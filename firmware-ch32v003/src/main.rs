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
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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

/// System tick counter for timing (updated by SysTick interrupt)
static SYSTEM_TICK_MS: AtomicU32 = AtomicU32::new(0);

/// Paddle state is managed locally in main loop for simplicity

/// Element queue for FSM communication
static mut ELEMENT_QUEUE: Queue<Element, 4> = Queue::new();

/// Get current system time as Instant
fn get_current_instant() -> Instant {
    let ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
    Instant::from_millis(ms as i64)
}

/// CH32V003 GPIO Input implementation with real register access
struct Ch32v003Input {
    /// GPIO port base address
    port: u32,
    /// Pin number (0-15)
    pin: u8,
    /// Last edge time
    last_edge: AtomicU32,
}

impl Ch32v003Input {
    const fn new(port: u32, pin: u8) -> Self {
        Self {
            port,
            pin,
            last_edge: AtomicU32::new(0),
        }
    }
    
    fn is_low(&self) -> bool {
        // Read GPIO IDR (Input Data Register) at offset 0x08
        let idr = unsafe { core::ptr::read_volatile((self.port + 0x08) as *const u32) };
        (idr & (1 << self.pin)) == 0 // Active low
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

#[entry]
fn main() -> ! {
    info!("🔧 Rusty Keyer CH32V003 Starting (Bare Metal)...");
    
    // Initialize hardware
    hardware_init();
    
    info!("⚡ CH32V003 Hardware initialized");
    
    // Initialize keyer configuration
    let keyer_config = KeyerConfig {
        mode: KeyerMode::SuperKeyer,
        char_space_enabled: true,
        unit: Duration::from_millis(60), // 20 WPM
        debounce_ms: 5,
        queue_size: 4, // Reduced for CH32V003
    };
    
    info!("⚙️ Keyer config: {:?} WPM, Mode: {:?}", 
          keyer_config.wpm(), keyer_config.mode);
    
    // Initialize FSM and queue
    let mut fsm = KeyerFSM::new(keyer_config);
    let (mut producer, mut consumer) = unsafe { ELEMENT_QUEUE.split() };
    
    info!("🚀 Keyer system ready!");
    
    // Main application loop
    let mut last_heartbeat = get_current_instant();
    
    loop {
        // Update FSM with current paddle state
        critical_section::with(|_| {
            // Read current paddle state safely
            let dit_pressed = DIT_INPUT.is_low();
            let dah_pressed = DAH_INPUT.is_low();
            
            // Create temporary paddle state for FSM
            let current_paddle = PaddleInput::new();
            let now_ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
            
            // Update paddle state based on current GPIO readings
            current_paddle.update(PaddleSide::Dit, dit_pressed, now_ms);
            current_paddle.update(PaddleSide::Dah, dah_pressed, now_ms);
            
            // Update FSM
            fsm.update(&current_paddle, &mut producer);
        });
        
        // Process output queue
        if let Some(element) = consumer.dequeue() {
            process_element(element, keyer_config.unit);
        }
        
        // Heartbeat every 10 seconds
        let now = get_current_instant();
        if now.duration_since(last_heartbeat).as_millis() >= 10000 {
            info!("💓 Heartbeat - System running");
            last_heartbeat = now;
        }
        
        // Brief CPU pause (RISC-V version)
        unsafe { riscv::asm::wfi(); }
    }
}

/// Process a single element from the queue
fn process_element(element: Element, unit: Duration) {
    match element {
        Element::Dit => {
            debug!("📡 Sending Dit");
            send_element(unit);
        }
        Element::Dah => {
            debug!("📡 Sending Dah");
            send_element(unit * 3);
        }
        Element::CharSpace => {
            debug!("⏸️ Character space");
            delay_ms(unit.as_millis() as u32 * 3);
        }
    }
}

/// Send a keyed element with timing
fn send_element(duration: Duration) {
    // Key down
    KEY_OUTPUT.set_high();
    STATUS_LED.set_high();
    SIDETONE_PWM.set_duty(500);
    
    // Element duration
    delay_ms(duration.as_millis() as u32);
    
    // Key up
    KEY_OUTPUT.set_low();
    STATUS_LED.set_low();
    SIDETONE_PWM.set_duty(0);
    
    // Inter-element space (1 unit)
    delay_ms(60); // 1 unit at 20 WPM
}

/// Simple delay implementation using system tick
fn delay_ms(ms: u32) {
    let start = SYSTEM_TICK_MS.load(Ordering::Relaxed);
    while SYSTEM_TICK_MS.load(Ordering::Relaxed).saturating_sub(start) < ms {
        // RISC-V nop
        unsafe { riscv::asm::nop(); }
    }
}

/// Initialize CH32V003 hardware
fn hardware_init() {
    // 1. Enable peripheral clocks
    enable_peripheral_clocks();
    
    // 2. Configure GPIO pins
    configure_gpio_pins();
    
    // 3. Configure SysTick timer for 1ms interrupts
    configure_systick();
    
    // 4. Configure EXTI for paddle interrupts
    configure_exti_interrupts();
    
    // 5. Configure TIM1 for PWM sidetone
    configure_pwm_sidetone();
    
    info!("🔌 GPIO configured: Dit=PA2, Dah=PA3, Key=PD6, LED=PD7");
    info!("🎵 PWM sidetone configured (600Hz)");
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
    // Configure PA2 and PA3 as inputs with pull-up (Dit/Dah paddles)
    unsafe {
        let gpioa_crl = (GPIOA_BASE + GPIO_CRL) as *mut u32;
        let mut crl = core::ptr::read_volatile(gpioa_crl);
        
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
        
        // Enable EXTI2 and EXTI3 interrupts (falling edge for active-low paddles)
        let exti_imr = (EXTI_BASE + EXTI_IMR) as *mut u32;
        let exti_ftsr = (EXTI_BASE + EXTI_FTSR) as *mut u32;
        
        let imr = core::ptr::read_volatile(exti_imr);
        core::ptr::write_volatile(exti_imr, imr | (1 << 2) | (1 << 3)); // Enable EXTI2 and EXTI3
        
        let ftsr = core::ptr::read_volatile(exti_ftsr);
        core::ptr::write_volatile(exti_ftsr, ftsr | (1 << 2) | (1 << 3)); // Falling edge trigger
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
        let ccmr1 = core::ptr::read_volatile(tim_ccmr1);
        core::ptr::write_volatile(tim_ccmr1, ccmr1 | (0x6 << 4) | (1 << 3)); // PWM mode 1, preload enable
        
        // Enable Channel 1 output
        let tim_ccer = (TIM1_BASE + TIM_CCER) as *mut u32;
        let ccer = core::ptr::read_volatile(tim_ccer);
        core::ptr::write_volatile(tim_ccer, ccer | 1); // Enable CC1E
        
        // Enable timer
        let tim_cr1 = (TIM1_BASE + TIM_CR1) as *mut u32;
        core::ptr::write_volatile(tim_cr1, 1); // Enable counter
    }
    
    SIDETONE_PWM.set_frequency(600);
    SIDETONE_PWM.enable();
}

// ========================================
// Interrupt Handlers (Stubs)
// ========================================

/// SysTick interrupt handler - increment system tick counter
#[no_mangle]
extern "C" fn SysTick() {
    let current = SYSTEM_TICK_MS.load(Ordering::Relaxed);
    SYSTEM_TICK_MS.store(current.wrapping_add(1), Ordering::Relaxed);
}

/// EXTI2 interrupt handler - Dit paddle (PA2)
#[no_mangle] 
extern "C" fn EXTI7_0_IRQHandler() {
    unsafe {
        let exti_pr = (EXTI_BASE + EXTI_PR) as *mut u32;
        let pending = core::ptr::read_volatile(exti_pr);
        
        // Check EXTI2 (PA2 - Dit paddle)
        if pending & (1 << 2) != 0 {
            DIT_INPUT.update_from_interrupt();
            // Clear EXTI2 pending bit
            core::ptr::write_volatile(exti_pr, 1 << 2);
        }
        
        // Check EXTI3 (PA3 - Dah paddle)
        if pending & (1 << 3) != 0 {
            DAH_INPUT.update_from_interrupt();
            // Clear EXTI3 pending bit
            core::ptr::write_volatile(exti_pr, 1 << 3);
        }
    }
}