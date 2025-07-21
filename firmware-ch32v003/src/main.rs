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
    KeyerFSM, PaddleInput, KeyerConfig, KeyerMode, Element,
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
// Hardware Abstraction Layer
// ========================================

/// System tick counter for timing (updated by SysTick interrupt)
static SYSTEM_TICK_MS: AtomicU32 = AtomicU32::new(0);

/// Global paddle state (updated by EXTI interrupts)
static PADDLE: PaddleInput = PaddleInput::new();

/// Element queue for FSM communication
static mut ELEMENT_QUEUE: Queue<Element, 4> = Queue::new();

/// Get current system time as Instant
fn get_current_instant() -> Instant {
    let ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
    Instant::from_millis(ms as i64)
}

/// CH32V003 GPIO Input implementation
struct Ch32v003Input {
    /// GPIO pin state (mock for now - will be real GPIO registers)
    state: AtomicBool,
    last_edge: AtomicU32,
}

impl Ch32v003Input {
    const fn new() -> Self {
        Self {
            state: AtomicBool::new(false),
            last_edge: AtomicU32::new(0),
        }
    }
    
    fn is_low(&self) -> bool {
        !self.state.load(Ordering::Relaxed)
    }
    
    /// Called from EXTI interrupt handler
    fn update_from_interrupt(&self, pressed: bool) {
        self.state.store(pressed, Ordering::Relaxed);
        let now_ms = SYSTEM_TICK_MS.load(Ordering::Relaxed);
        self.last_edge.store(now_ms, Ordering::Relaxed);
    }
}

/// CH32V003 GPIO Output implementation  
struct Ch32v003Output {
    /// GPIO pin state (mock for now - will be real GPIO registers)
    state: AtomicBool,
}

impl Ch32v003Output {
    const fn new() -> Self {
        Self {
            state: AtomicBool::new(false),
        }
    }
    
    fn set_high(&self) {
        self.state.store(true, Ordering::Relaxed);
        // TODO: Write to actual GPIO register
    }
    
    fn set_low(&self) {
        self.state.store(false, Ordering::Relaxed);
        // TODO: Write to actual GPIO register
    }
    
    fn is_set_high(&self) -> bool {
        self.state.load(Ordering::Relaxed)
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
        // TODO: Write to TIM1 duty register
    }
    
    fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        // TODO: Enable TIM1 PWM output
    }
    
    fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        // TODO: Disable TIM1 PWM output  
    }
    
    fn set_frequency(&self, freq: u32) {
        self.frequency.store(freq, Ordering::Relaxed);
        // TODO: Update TIM1 prescaler/period
    }
}

// ========================================
// Hardware Instances
// ========================================

static DIT_INPUT: Ch32v003Input = Ch32v003Input::new();
static DAH_INPUT: Ch32v003Input = Ch32v003Input::new();
static KEY_OUTPUT: Ch32v003Output = Ch32v003Output::new();
static STATUS_LED: Ch32v003Output = Ch32v003Output::new();
static SIDETONE_PWM: Ch32v003Pwm = Ch32v003Pwm::new();

/// Combined HAL implementation for keyer-core integration
struct Ch32v003KeyerHal;

impl InputPaddle for Ch32v003KeyerHal {
    type Error = HalError;
    
    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        // Check both dit and dah inputs (active low)
        Ok(!DIT_INPUT.state.load(Ordering::Relaxed) || !DAH_INPUT.state.load(Ordering::Relaxed))
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
        // Update FSM with paddle input (暫定的にmock実装)
        // TODO: PADDLE状態をGPIOから更新する実装が必要
        // fsm.update(&PADDLE, &mut producer);
        
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
    // TODO: Real hardware initialization
    // - Configure system clock
    // - Initialize GPIO pins
    // - Configure SysTick timer
    // - Configure TIM1 for PWM sidetone
    // - Configure EXTI for paddle interrupts
    
    // Enable SysTick for 1ms interrupts
    // systick_init();
    
    // Configure PWM sidetone at 600Hz
    SIDETONE_PWM.set_frequency(600);
    SIDETONE_PWM.enable();
    
    info!("🔌 GPIO configured: Dit=PA2, Dah=PA3, Key=PD6, LED=PD7");
    info!("🎵 PWM sidetone configured (600Hz)");
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

/// EXTI0 interrupt handler - Dit paddle
#[no_mangle] 
extern "C" fn EXTI0() {
    let pressed = true; // TODO: Read actual GPIO state
    DIT_INPUT.update_from_interrupt(pressed);
    // TODO: Update PADDLE state for dit side
}

/// EXTI1 interrupt handler - Dah paddle  
#[no_mangle]
extern "C" fn EXTI1() {
    let pressed = true; // TODO: Read actual GPIO state
    DAH_INPUT.update_from_interrupt(pressed);
    // TODO: Update PADDLE state for dah side
}