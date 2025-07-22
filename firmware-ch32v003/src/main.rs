#![no_std]
#![no_main]

// Logging support
#[cfg(feature = "defmt")]
use defmt::{debug, info, warn};
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use panic_halt as _;

// Define simple logging macros when defmt is not available
#[cfg(not(feature = "defmt"))]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

// Core imports
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use core::cell::RefCell;
use riscv_rt::entry;
use keyer_core::{
    KeyerFSM, PaddleInput, PaddleSide, KeyerConfig, KeyerMode, Element,
    hal::{Duration, Instant, InputPaddle, OutputKey, HalError}
};
use heapless::spsc::Queue;

// Critical section implementation for RISC-V
struct RiscvCriticalSection;
critical_section::set_impl!(RiscvCriticalSection);

unsafe impl critical_section::Impl for RiscvCriticalSection {
    unsafe fn acquire() -> critical_section::RawRestoreState {
        let mstatus = riscv::register::mstatus::read();
        riscv::register::mstatus::clear_mie();
        mstatus.mie() as u8
    }

    unsafe fn release(was_enabled: critical_section::RawRestoreState) {
        if was_enabled != 0 {
            riscv::register::mstatus::set_mie();
        }
    }
}

// ========================================
// CH32V003 Hardware Definitions
// ========================================

/// CH32V003 Memory Map and Register Base Addresses
const RCC_BASE: u32 = 0x4002_1000;
const GPIOA_BASE: u32 = 0x4001_0800;
const GPIOC_BASE: u32 = 0x4001_1000;  
const GPIOD_BASE: u32 = 0x4001_1400;
const AFIO_BASE: u32 = 0x4001_0000;
const EXTI_BASE: u32 = 0x4001_0400;
const NVIC_BASE: u32 = 0xE000_E000;
const TIM1_BASE: u32 = 0x4001_2C00;
const SYSTICK_BASE: u32 = 0xE000_E010;

/// RCC Register offsets
const RCC_APB2PCENR: u32 = 0x18; // APB2 peripheral clock enable register

/// GPIO Register offsets
const GPIO_CRL: u32 = 0x00;    // Control Register Low
const GPIO_CRH: u32 = 0x04;    // Control Register High  
const GPIO_IDR: u32 = 0x08;    // Input Data Register
const GPIO_ODR: u32 = 0x0C;    // Output Data Register
const GPIO_BSHR: u32 = 0x10;   // Bit Set/Reset Register
const GPIO_BCR: u32 = 0x14;    // Bit Reset Register
const GPIO_LCKR: u32 = 0x18;   // Lock Register

/// AFIO Register offsets
const AFIO_PCFR1: u32 = 0x04;  // Port configuration register 1

/// EXTI Register offsets  
const EXTI_IMR: u32 = 0x00;    // Interrupt Mask Register
const EXTI_EMR: u32 = 0x04;    // Event Mask Register
const EXTI_RTSR: u32 = 0x08;   // Rising Trigger Selection Register
const EXTI_FTSR: u32 = 0x0C;   // Falling Trigger Selection Register
const EXTI_SWIER: u32 = 0x10;  // Software Interrupt Event Register
const EXTI_PR: u32 = 0x14;     // Pending Register

/// TIM1 Register offsets for PWM
const TIM_CR1: u32 = 0x00;     // Control Register 1
const TIM_PSC: u32 = 0x28;     // Prescaler
const TIM_ARR: u32 = 0x2C;     // Auto-reload Register
const TIM_CCR1: u32 = 0x34;    // Capture/Compare Register 1
const TIM_CCMR1: u32 = 0x18;   // Capture/Compare Mode Register 1
const TIM_CCER: u32 = 0x20;    // Capture/Compare Enable Register

/// SysTick Register offsets
const SYSTICK_CSR: u32 = 0x00;  // Control and Status Register
const SYSTICK_RVR: u32 = 0x04;  // Reload Value Register  
const SYSTICK_CVR: u32 = 0x08;  // Current Value Register

// ========================================
// Hardware Abstraction Layer
// ========================================

// ========================================
// New Data Structures
// ========================================

/// System tick counter for timing (updated by SysTick interrupt)
static SYSTEM_TICK_MS: AtomicU32 = AtomicU32::new(0);

/// System event flags for power-efficient operation
static SYSTEM_EVENTS: AtomicU32 = AtomicU32::new(0);
const EVENT_PADDLE: u32 = 0x01;      // Paddle state changed

/// Transmission controller (12 bytes)
struct TxController {
    state: AtomicU8,           // Idle(0) / Transmitting(1)
    element_end_ms: AtomicU32, // Current element end time
    next_allowed_ms: AtomicU32, // Next transmission allowed time (space control)
}

impl TxController {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(0), // Idle
            element_end_ms: AtomicU32::new(0),
            next_allowed_ms: AtomicU32::new(0),
        }
    }
    
    fn is_idle(&self) -> bool {
        self.state.load(Ordering::Relaxed) == 0
    }
    
    fn is_transmitting(&self) -> bool {
        self.state.load(Ordering::Relaxed) == 1
    }
    
    fn set_transmitting(&self, end_time: u32) {
        self.state.store(1, Ordering::Release);
        self.element_end_ms.store(end_time, Ordering::Release);
    }
    
    fn set_idle_with_constraint(&self, next_allowed: u32) {
        self.state.store(0, Ordering::Release);
        self.next_allowed_ms.store(next_allowed, Ordering::Release);
    }
    
    fn can_start_transmission(&self, now_ms: u32) -> bool {
        self.is_idle() && now_ms >= self.next_allowed_ms.load(Ordering::Relaxed)
    }
    
    fn is_element_finished(&self, now_ms: u32) -> bool {
        now_ms >= self.element_end_ms.load(Ordering::Relaxed)
    }
}

/// Global state
static TX_CONTROLLER: TxController = TxController::new();
static LAST_ACTIVITY_MS: AtomicU32 = AtomicU32::new(0);
static PADDLE_CHANGED: AtomicBool = AtomicBool::new(false);
static PADDLE_STATE: critical_section::Mutex<RefCell<PaddleInput>> = 
    critical_section::Mutex::new(RefCell::new(PaddleInput::new()));
static KEYER_FSM_INSTANCE: critical_section::Mutex<RefCell<Option<KeyerFSM>>> = 
    critical_section::Mutex::new(RefCell::new(None));

/// Element queue for FSM communication
static mut ELEMENT_QUEUE: Queue<Element, 4> = Queue::new();

// ========================================
// Helper Functions
// ========================================

/// Get current system time as Instant
fn get_current_instant() -> Instant {
    let ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
    Instant::from_millis(ms as i64)
}

/// Record activity for power management
fn record_activity() {
    let now_ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
    LAST_ACTIVITY_MS.store(now_ms, Ordering::Relaxed);
}

/// Get unit duration in milliseconds (20 WPM = 60ms per unit)
fn get_unit_duration_ms() -> u32 {
    60 // Fixed 20 WPM for now
}

/// Debug logging for transmission (feature-gated)
#[cfg(feature = "debug")]
macro_rules! tx_debug {
    ($($arg:tt)*) => {
        debug!($($arg)*);
    };
}

#[cfg(not(feature = "debug"))]
macro_rules! tx_debug {
    ($($arg:tt)*) => {};
}

/// Initialize keyer FSM
fn initialize_keyer_fsm() {
    critical_section::with(|cs| {
        let config = KeyerConfig {
            mode: KeyerMode::ModeA,  // Unified to ModeA for compatibility
            char_space_enabled: true,
            unit: Duration::from_millis(60),
            debounce_ms: 10,  // Unified 10ms debounce for noise immunity
            queue_size: 4,
        };
        let fsm = KeyerFSM::new(config);
        *KEYER_FSM_INSTANCE.borrow(cs).borrow_mut() = Some(fsm);
    });
    info!("🎛️ Keyer FSM initialized");
}

/// CH32V003 GPIO Input implementation with real register access and debouncing
struct Ch32v003Input {
    /// GPIO port base address
    port: u32,
    /// Pin number (0-15)
    pin: u8,
    /// Last edge time
    last_edge: AtomicU32,
    /// Last stable state for debouncing
    last_stable_state: AtomicBool,
    /// Debounce time in milliseconds
    debounce_ms: u32,
}

impl Ch32v003Input {
    const fn new(port: u32, pin: u8) -> Self {
        Self {
            port,
            pin,
            last_edge: AtomicU32::new(0),
            last_stable_state: AtomicBool::new(true), // Default to released (high)
            debounce_ms: 10, // 10ms debounce for noise immunity
        }
    }
    
    fn is_low(&self) -> bool {
        // Read current GPIO state
        let idr = unsafe { core::ptr::read_volatile((self.port + 0x08) as *const u32) };
        let current_raw = (idr & (1 << self.pin)) == 0; // Active low
        
        // Get timing information
        let now_ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
        let last_edge_ms = self.last_edge.load(Ordering::Relaxed);
        let last_stable = self.last_stable_state.load(Ordering::Relaxed);
        
        // If enough time has passed since last edge, update stable state
        if now_ms.saturating_sub(last_edge_ms) >= self.debounce_ms {
            if current_raw != last_stable {
                self.last_stable_state.store(current_raw, Ordering::Relaxed);
                return current_raw;
            }
        }
        
        // Return last stable state during debounce period
        last_stable
    }
    
    /// Called from EXTI interrupt handler
    fn update_from_interrupt(&self) {
        let now_ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
        self.last_edge.store(now_ms, Ordering::Relaxed);
    }
}

/// CH32V003 GPIO Output implementation with real register access
struct Ch32v003Output {
    /// GPIO port base address
    port: u32,
    /// Pin number (0-15)
    pin: u8,
}

impl Ch32v003Output {
    const fn new(port: u32, pin: u8) -> Self {
        Self {
            port,
            pin,
        }
    }
    
    fn set_high(&self) {
        // Write to GPIO BSHR (Bit Set/Reset Register) at offset 0x10
        // Set bit using BSHR high part (bits 16-31 reset, bits 0-15 set)
        unsafe {
            core::ptr::write_volatile((self.port + 0x10) as *mut u32, 1 << self.pin);
        }
    }
    
    fn set_low(&self) {
        // Write to GPIO BSHR (Bit Set/Reset Register) at offset 0x10
        // Reset bit using BSHR high part (bits 16-31 reset, bits 0-15 set)
        unsafe {
            core::ptr::write_volatile((self.port + 0x10) as *mut u32, 1 << (self.pin + 16));
        }
    }
    
    fn is_set_high(&self) -> bool {
        // Read GPIO ODR (Output Data Register) at offset 0x0C
        let odr = unsafe { core::ptr::read_volatile((self.port + 0x0C) as *const u32) };
        (odr & (1 << self.pin)) != 0
    }
}

/// CH32V003 PWM for sidetone
struct Ch32v003Pwm {
    enabled: AtomicBool,
    duty: AtomicU32,
    frequency: AtomicU32,
}

impl Ch32v003Pwm {
    const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            duty: AtomicU32::new(0),
            frequency: AtomicU32::new(600), // Default 600Hz
        }
    }
    
    fn set_duty(&self, duty: u16) {
        self.duty.store(duty as u32, Ordering::Relaxed);
        unsafe {
            // Calculate duty cycle value: (duty / 1000) * ARR
            // For 50% duty cycle (500), CCR1 = 1666 / 2 = 833
            let tim_arr = (TIM1_BASE + TIM_ARR) as *const u32;
            let arr_value = core::ptr::read_volatile(tim_arr);
            let ccr_value = (duty as u32 * arr_value) / 1000;
            
            let tim_ccr1 = (TIM1_BASE + TIM_CCR1) as *mut u32;
            core::ptr::write_volatile(tim_ccr1, ccr_value);
        }
    }
    
    fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        unsafe {
            let tim_ccer = (TIM1_BASE + TIM_CCER) as *mut u32;
            let ccer = core::ptr::read_volatile(tim_ccer);
            core::ptr::write_volatile(tim_ccer, ccer | 1); // Enable CC1E
        }
    }
    
    fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        unsafe {
            let tim_ccer = (TIM1_BASE + TIM_CCER) as *mut u32;
            let ccer = core::ptr::read_volatile(tim_ccer);
            core::ptr::write_volatile(tim_ccer, ccer & !1); // Disable CC1E
        }
    }
    
    fn set_frequency(&self, freq: u32) {
        self.frequency.store(freq, Ordering::Relaxed);
        unsafe {
            // Calculate new ARR value: 1MHz / freq - 1
            let arr_value = (1_000_000 / freq) - 1;
            let tim_arr = (TIM1_BASE + TIM_ARR) as *mut u32;
            core::ptr::write_volatile(tim_arr, arr_value);
        }
    }
}

// ========================================
// Hardware Instances - CH32V003 Pin Mapping
// ========================================

// Pin assignments:
// PA2 = Dit paddle input (active low with pull-up)
// PA3 = Dah paddle input (active low with pull-up)  
// PD6 = Key output (active high)
// PD7 = Status LED (active high)
// PA1 = Sidetone PWM output (TIM1_CH1)

static DIT_INPUT: Ch32v003Input = Ch32v003Input::new(GPIOA_BASE, 2);  // PA2
static DAH_INPUT: Ch32v003Input = Ch32v003Input::new(GPIOA_BASE, 3);  // PA3
static KEY_OUTPUT: Ch32v003Output = Ch32v003Output::new(GPIOD_BASE, 6); // PD6
static STATUS_LED: Ch32v003Output = Ch32v003Output::new(GPIOD_BASE, 7); // PD7
static SIDETONE_PWM: Ch32v003Pwm = Ch32v003Pwm::new();

/// Combined HAL implementation for keyer-core integration
struct Ch32v003KeyerHal;

impl InputPaddle for Ch32v003KeyerHal {
    type Error = HalError;
    
    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        // Check both dit and dah inputs (active low)
        Ok(DIT_INPUT.is_low() || DAH_INPUT.is_low())
    }
    
    fn last_edge_time(&self) -> Option<Instant> {
        let dit_time = DIT_INPUT.last_edge.load(Ordering::Relaxed);
        let dah_time = DAH_INPUT.last_edge.load(Ordering::Relaxed);
        let latest = dit_time.max(dah_time);
        
        if latest > 0 {
            Some(Instant::from_millis(latest as i64))
        } else {
            None
        }
    }
    
    fn set_debounce_time(&mut self, _time_ms: u32) -> Result<(), Self::Error> {
        // Debounce handled in interrupt handlers
        Ok(())
    }
    
    fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
        // EXTI configuration will be done in hardware_init()
        Ok(())
    }
    
    fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
        // EXTI configuration will be done in hardware_init()
        Ok(())
    }
}

impl OutputKey for Ch32v003KeyerHal {
    type Error = HalError;
    
    fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
        if state {
            KEY_OUTPUT.set_high();
            STATUS_LED.set_high();
            // Enable sidetone
            SIDETONE_PWM.set_duty(500); // 50% duty cycle
        } else {
            KEY_OUTPUT.set_low();
            STATUS_LED.set_low(); 
            // Disable sidetone
            SIDETONE_PWM.set_duty(0);
        }
        Ok(())
    }
    
    fn get_state(&self) -> Result<bool, Self::Error> {
        Ok(KEY_OUTPUT.is_set_high())
    }
}

// ========================================
// Main Application
// ========================================

// ========================================
// New Transmission FSM
// ========================================

/// Update paddle state from interrupt events
fn update_paddle_state() {
    PADDLE_CHANGED.store(false, Ordering::Relaxed);
    
    let dit_pressed = DIT_INPUT.is_low();
    let dah_pressed = DAH_INPUT.is_low();
    let now_ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
    
    critical_section::with(|cs| {
        let paddle = PADDLE_STATE.borrow(cs).borrow_mut();
        paddle.update(PaddleSide::Dit, dit_pressed, now_ms);
        paddle.update(PaddleSide::Dah, dah_pressed, now_ms);
    });
    
    record_activity();
    
    #[cfg(feature = "debug")]
    {
        if dit_pressed || dah_pressed {
            tx_debug!("🎮 Paddle: Dit={}, Dah={}", dit_pressed, dah_pressed);
        }
    }
}

/// Update keyer-core FSM
fn update_keyer_fsm() {
    critical_section::with(|cs| {
        let paddle = PADDLE_STATE.borrow(cs).borrow();
        
        if let Some(ref mut fsm) = *KEYER_FSM_INSTANCE.borrow(cs).borrow_mut() {
            let mut producer = unsafe { ELEMENT_QUEUE.split().0 };
            fsm.update(&*paddle, &mut producer);
        }
    });
    
    record_activity();
}

/// Transmission FSM update
fn update_transmission_fsm(now_ms: u32) {
    if TX_CONTROLLER.is_transmitting() {
        if TX_CONTROLLER.is_element_finished(now_ms) {
            end_element_transmission(now_ms);
        }
    } else {
        if TX_CONTROLLER.can_start_transmission(now_ms) {
            let mut consumer = unsafe { ELEMENT_QUEUE.split().1 };
            if let Some(element) = consumer.dequeue() {
                start_element_transmission(element, now_ms);
            }
        }
    }
}

/// Start element transmission
fn start_element_transmission(element: Element, now_ms: u32) {
    let unit_ms = get_unit_duration_ms();
    
    match element {
        Element::Dit => {
            KEY_OUTPUT.set_high();
            STATUS_LED.set_high();
            SIDETONE_PWM.set_duty(500);
            TX_CONTROLLER.set_transmitting(now_ms + unit_ms);
            record_activity();
            tx_debug!("🟢 Dit start: {}ms", unit_ms);
        }
        
        Element::Dah => {
            KEY_OUTPUT.set_high();
            STATUS_LED.set_high();
            SIDETONE_PWM.set_duty(500);
            TX_CONTROLLER.set_transmitting(now_ms + (unit_ms * 3));
            record_activity();
            tx_debug!("🟢 Dah start: {}ms", unit_ms * 3);
        }
        
        Element::CharSpace => {
            TX_CONTROLLER.set_idle_with_constraint(now_ms + (unit_ms * 2));
            record_activity();
            tx_debug!("⏸️ CharSpace: +{}ms", unit_ms * 2);
        }
    }
}

/// End current element transmission
fn end_element_transmission(now_ms: u32) {
    KEY_OUTPUT.set_low();
    STATUS_LED.set_low();
    SIDETONE_PWM.set_duty(0);
    
    let unit_ms = get_unit_duration_ms();
    TX_CONTROLLER.set_idle_with_constraint(now_ms + unit_ms);
    
    tx_debug!("🔴 Element end, space: {}ms", unit_ms);
}

/// Check if can enter low power mode
fn can_enter_low_power(now_ms: u32) -> bool {
    let tx_idle = TX_CONTROLLER.is_idle();
    let queue_empty = unsafe { ELEMENT_QUEUE.is_empty() };
    let no_pending_events = SYSTEM_EVENTS.load(Ordering::Relaxed) == 0;
    let last_activity = LAST_ACTIVITY_MS.load(Ordering::Relaxed);
    let idle_long_enough = now_ms.saturating_sub(last_activity) >= 5000;
    
    tx_idle && queue_empty && no_pending_events && idle_long_enough
}

/// Debug heartbeat (feature-gated)
#[cfg(feature = "debug")]
fn debug_heartbeat(last_heartbeat: &mut Instant) {
    let now_instant = get_current_instant();
    if now_instant.duration_since(*last_heartbeat).as_millis() >= 10000 {
        info!("💓 Heartbeat - Tx: {}, Queue: {}, Activity: {}ms ago", 
              TX_CONTROLLER.is_transmitting(),
              unsafe { ELEMENT_QUEUE.len() },
              now_instant.as_millis() - LAST_ACTIVITY_MS.load(Ordering::Relaxed) as i64);
        *last_heartbeat = now_instant;
    }
}

#[cfg(not(feature = "debug"))]
fn debug_heartbeat(_last_heartbeat: &mut ()) {}

/// Main execution loop
fn main_loop() {
    let mut last_keyer_update = 0u32;
    
    #[cfg(feature = "debug")]
    let mut last_heartbeat = get_current_instant();
    #[cfg(not(feature = "debug"))]
    let mut last_heartbeat = ();
    
    info!("🚀 Main loop started");
    
    loop {
        let now_ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
        
        // Phase 1: Paddle change processing (highest priority)
        if PADDLE_CHANGED.load(Ordering::Relaxed) {
            update_paddle_state();
            update_keyer_fsm();
            last_keyer_update = now_ms;
        }
        
        // Phase 2: Periodic FSM update (10ms cycle)
        else if now_ms.wrapping_sub(last_keyer_update) >= 10 {
            update_keyer_fsm();
            last_keyer_update = now_ms;
        }
        
        // Phase 3: Transmission FSM update (always active)
        update_transmission_fsm(now_ms);
        
        // Phase 4: Debug heartbeat
        debug_heartbeat(&mut last_heartbeat);
        
        // Phase 5: Power saving
        if can_enter_low_power(now_ms) {
            unsafe { riscv::asm::wfi(); }
        }
    }
}

/// Hardware initialization wrapper
fn hardware_init() {
    enable_peripheral_clocks();
    configure_gpio_pins();
    configure_systick();
    configure_exti_interrupts();
    configure_pwm_sidetone();
    initialize_keyer_fsm();
    
    info!("✅ Hardware initialization complete");
}

/// Enable required peripheral clocks
fn enable_peripheral_clocks() {
    unsafe {
        let rcc_apb2pcenr = (RCC_BASE + RCC_APB2PCENR) as *mut u32;
        let current = core::ptr::read_volatile(rcc_apb2pcenr);
        // Enable GPIOA, GPIOD, AFIO, TIM1 clocks
        // Bit 2 = GPIOA, Bit 5 = GPIOD, Bit 0 = AFIO, Bit 11 = TIM1
        core::ptr::write_volatile(rcc_apb2pcenr, current | (1 << 2) | (1 << 5) | (1 << 0) | (1 << 11));
    }
}

/// Configure GPIO pins for inputs and outputs
fn configure_gpio_pins() {
    // Configure PA1 as AF push-pull output for TIM1_CH1 (PWM)
    // Configure PA2 and PA3 as inputs with pull-up (Dit/Dah paddles)
    unsafe {
        let gpioa_crl = (GPIOA_BASE + GPIO_CRL) as *mut u32;
        let mut crl = core::ptr::read_volatile(gpioa_crl);
        
        // PA1: CNF=10 (AF push-pull), MODE=11 (50MHz output)
        crl &= !(0xF << (1 * 4)); // Clear PA1 configuration
        crl |= 0xB << (1 * 4);    // Set PA1 as AF push-pull 50MHz
        
        // PA2: CNF=10 (input with pull-up), MODE=00 (input)
        crl &= !(0xF << (2 * 4)); // Clear PA2 configuration
        crl |= 0x8 << (2 * 4);    // Set PA2 as input pull-up
        
        // PA3: CNF=10 (input with pull-up), MODE=00 (input)  
        crl &= !(0xF << (3 * 4)); // Clear PA3 configuration
        crl |= 0x8 << (3 * 4);    // Set PA3 as input pull-up
        
        core::ptr::write_volatile(gpioa_crl, crl);
        
        // Set pull-up resistors for PA2 and PA3
        let gpioa_odr = (GPIOA_BASE + GPIO_ODR) as *mut u32;
        let odr = core::ptr::read_volatile(gpioa_odr);
        core::ptr::write_volatile(gpioa_odr, odr | (1 << 2) | (1 << 3));
    }
    
    // Configure PD6 and PD7 as outputs (Key output and Status LED)
    unsafe {
        let gpiod_crl = (GPIOD_BASE + GPIO_CRL) as *mut u32;
        let mut crl = core::ptr::read_volatile(gpiod_crl);
        
        // PD6: CNF=00 (push-pull output), MODE=11 (50MHz output)
        crl &= !(0xF << (6 * 4)); // Clear PD6 configuration
        crl |= 0x3 << (6 * 4);    // Set PD6 as 50MHz push-pull output
        
        // PD7: CNF=00 (push-pull output), MODE=11 (50MHz output)
        crl &= !(0xF << (7 * 4)); // Clear PD7 configuration  
        crl |= 0x3 << (7 * 4);    // Set PD7 as 50MHz push-pull output
        
        core::ptr::write_volatile(gpiod_crl, crl);
    }
}

/// Configure SysTick for 1ms interrupts
fn configure_systick() {
    unsafe {
        // Assuming 24MHz system clock, 1ms = 24000 ticks
        let systick_rvr = (SYSTICK_BASE + SYSTICK_RVR) as *mut u32;
        core::ptr::write_volatile(systick_rvr, 24000 - 1); // 1ms at 24MHz
        
        let systick_cvr = (SYSTICK_BASE + SYSTICK_CVR) as *mut u32;
        core::ptr::write_volatile(systick_cvr, 0); // Clear current value
        
        let systick_csr = (SYSTICK_BASE + SYSTICK_CSR) as *mut u32;
        // Enable SysTick, enable interrupt, use core clock
        core::ptr::write_volatile(systick_csr, 0x7);
    }
}

/// Configure EXTI interrupts for paddle inputs
fn configure_exti_interrupts() {
    unsafe {
        // Configure AFIO to map PA2 and PA3 to EXTI2 and EXTI3
        let afio_pcfr1 = (AFIO_BASE + AFIO_PCFR1) as *mut u32;
        let pcfr1 = core::ptr::read_volatile(afio_pcfr1);
        // EXTI2 and EXTI3 map to Port A (0x0)
        core::ptr::write_volatile(afio_pcfr1, pcfr1);
        
        // Enable EXTI2 and EXTI3 interrupts (both edges for complete paddle detection)
        let exti_imr = (EXTI_BASE + EXTI_IMR) as *mut u32;
        let exti_ftsr = (EXTI_BASE + EXTI_FTSR) as *mut u32;
        let exti_rtsr = (EXTI_BASE + EXTI_RTSR) as *mut u32;
        
        // Enable interrupt mask for EXTI2 and EXTI3
        let imr = core::ptr::read_volatile(exti_imr);
        core::ptr::write_volatile(exti_imr, imr | (1 << 2) | (1 << 3));
        
        // Enable both falling and rising edge triggers
        let ftsr = core::ptr::read_volatile(exti_ftsr);
        core::ptr::write_volatile(exti_ftsr, ftsr | (1 << 2) | (1 << 3)); // Falling edge (press)
        
        let rtsr = core::ptr::read_volatile(exti_rtsr);
        core::ptr::write_volatile(exti_rtsr, rtsr | (1 << 2) | (1 << 3)); // Rising edge (release)
        
        // Enable NVIC for EXTI7_0 interrupt (covers EXTI0-7)
        // CH32V003 NVIC ISER register for interrupt 30 (EXTI7_0)
        let nvic_iser = (NVIC_BASE + 0x100) as *mut u32;
        let iser = core::ptr::read_volatile(nvic_iser);
        core::ptr::write_volatile(nvic_iser, iser | (1 << 30)); // Enable interrupt 30
    }
}

/// Configure TIM1 for PWM sidetone generation
fn configure_pwm_sidetone() {
    unsafe {
        // Configure TIM1 for PWM mode on Channel 1 (PA1)
        // Assuming 24MHz clock, prescaler=24 gives 1MHz timer clock
        // For 600Hz: ARR = 1MHz / 600Hz = 1667 - 1 = 1666
        
        let tim_psc = (TIM1_BASE + TIM_PSC) as *mut u32;
        core::ptr::write_volatile(tim_psc, 24 - 1); // 1MHz timer clock
        
        let tim_arr = (TIM1_BASE + TIM_ARR) as *mut u32;
        core::ptr::write_volatile(tim_arr, 1666); // 600Hz frequency
        
        let tim_ccr1 = (TIM1_BASE + TIM_CCR1) as *mut u32;
        core::ptr::write_volatile(tim_ccr1, 0); // Start with 0% duty cycle
        
        // Configure PWM mode 1 on Channel 1
        let tim_ccmr1 = (TIM1_BASE + TIM_CCMR1) as *mut u32;
        core::ptr::write_volatile(tim_ccmr1, (0x6 << 4) | (1 << 3)); // PWM mode 1, preload enable
        
        // Enable Channel 1 output
        let tim_ccer = (TIM1_BASE + TIM_CCER) as *mut u32;
        core::ptr::write_volatile(tim_ccer, 1); // Enable CC1E
        
        // Enable Main Output Enable (MOE) bit for advanced timer
        const TIM_BDTR: u32 = 0x44; // Break and Dead-time Register
        let tim_bdtr = (TIM1_BASE + TIM_BDTR) as *mut u32;
        core::ptr::write_volatile(tim_bdtr, 1 << 15); // Set MOE bit
        
        // Enable timer with auto-reload preload
        let tim_cr1 = (TIM1_BASE + TIM_CR1) as *mut u32;
        core::ptr::write_volatile(tim_cr1, (1 << 7) | 1); // ARPE=1, CEN=1
    }
    
    SIDETONE_PWM.set_frequency(600);
    SIDETONE_PWM.enable();
}

#[entry]
fn main() -> ! {
    hardware_init();
    
    info!("🚀 CH32V003 Keyer - Separated FSM Architecture");
    info!("📊 Memory: TxController={}B, Queue={}B", 
          core::mem::size_of::<TxController>(),
          core::mem::size_of::<Queue<Element, 4>>());
    
    main_loop();
    
    // This should never be reached
    loop {}
}

// ========================================
// Interrupt Handlers
// ========================================

/// SysTick interrupt handler (new architecture)
#[no_mangle]
extern "C" fn SysTick() {
    // 1ms tick update
    let current = SYSTEM_TICK_MS.load(Ordering::Relaxed);
    SYSTEM_TICK_MS.store(current.wrapping_add(1), Ordering::Release);
    
    // Power optimization: only wake from WFI when transmission active
    if TX_CONTROLLER.is_transmitting() {
        // Transmission FSM needs precise timing, auto-wake from WFI
    }
    // Idle time continues WFI for maximum power savings
}

/// EXTI interrupt handler for paddle edges (new architecture)
#[no_mangle]
extern "C" fn EXTI7_0_IRQHandler() {
    unsafe {
        let exti_pr = (EXTI_BASE + EXTI_PR) as *mut u32;
        let pending = core::ptr::read_volatile(exti_pr);
        
        // EXTI2 (PA2 - Dit) both edge detection
        if pending & (1 << 2) != 0 {
            DIT_INPUT.update_from_interrupt();
            core::ptr::write_volatile(exti_pr, 1 << 2);
            
            // Immediate notification to main loop
            PADDLE_CHANGED.store(true, Ordering::Release);
            let old_events = SYSTEM_EVENTS.load(Ordering::Relaxed);
            SYSTEM_EVENTS.store(old_events | EVENT_PADDLE, Ordering::Release);
        }
        
        // EXTI3 (PA3 - Dah) both edge detection  
        if pending & (1 << 3) != 0 {
            DAH_INPUT.update_from_interrupt();
            core::ptr::write_volatile(exti_pr, 1 << 3);
            
            // Immediate notification to main loop
            PADDLE_CHANGED.store(true, Ordering::Release);
            let old_events = SYSTEM_EVENTS.load(Ordering::Relaxed);
            SYSTEM_EVENTS.store(old_events | EVENT_PADDLE, Ordering::Release);
        }
    }
}