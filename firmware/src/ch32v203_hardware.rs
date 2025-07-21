//! CH32V203 Hardware Implementation
//! 
//! 64KB Flash / 20KB RAM - Embassy-optimized implementation

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embassy_time::Instant;
use keyer_core::types::PaddleSide;
use static_cell::StaticCell;

use keyer_core::{KeyerHal, HalError, InputPaddle, OutputKey, InterruptConfig};

/// CH32V203 hardware abstraction layer implementation
pub struct Ch32v203KeyerHal {
    dit_pin: DitInputPin,
    dah_pin: DahInputPin, 
    key_output: KeyOutputPin,
    interrupt_ctrl: NoOpInterruptCtrl,
    last_update: Instant,
}

impl Ch32v203KeyerHal {
    /// Initialize CH32V203 hardware
    pub fn new() -> Self {
        Self {
            dit_pin: DitInputPin::new(),
            dah_pin: DahInputPin::new(),
            key_output: KeyOutputPin::new(),
            interrupt_ctrl: NoOpInterruptCtrl,
            last_update: Instant::now(),
        }
    }
}

impl KeyerHal for Ch32v203KeyerHal {
    type DitPaddle = DitInputPin;
    type DahPaddle = DahInputPin;
    type KeyOutput = KeyOutputPin;
    type InterruptCtrl = NoOpInterruptCtrl;
    type Error = HalError;
    
    fn initialize(&mut self) -> Result<(), Self::Error> {
        // GPIO initialization
        self.dit_pin.init().map_err(|_| HalError::GpioError)?;
        self.dah_pin.init().map_err(|_| HalError::GpioError)?;
        self.key_output.init().map_err(|_| HalError::GpioError)?;
        
        #[cfg(feature = "defmt")]
        defmt::info!("🔌 CH32V203 HAL initialized");
        
        Ok(())
    }
    
    fn dit_paddle(&mut self) -> &mut Self::DitPaddle {
        &mut self.dit_pin
    }
    
    fn dah_paddle(&mut self) -> &mut Self::DahPaddle {
        &mut self.dah_pin
    }
    
    fn key_output(&mut self) -> &mut Self::KeyOutput {
        &mut self.key_output
    }
    
    fn interrupt_controller(&mut self) -> &mut Self::InterruptCtrl {
        &mut self.interrupt_ctrl
    }
    
    fn shutdown(&mut self) -> Result<(), Self::Error> {
        #[cfg(feature = "defmt")]
        defmt::info!("🔌 CH32V203 HAL shutdown");
        Ok(())
    }
}

// No-op interrupt controller for CH32V203
pub struct NoOpInterruptCtrl;

impl InterruptConfig for NoOpInterruptCtrl {
    type Error = HalError;

    fn configure_paddle_interrupt(
        &mut self,
        _paddle: PaddleSide,
        _rising: bool,
        _falling: bool,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_interrupt_priority(&mut self, _paddle: PaddleSide, _priority: u8) -> Result<(), Self::Error> {
        Ok(())
    }

    fn enable_paddle_interrupt(&mut self, _paddle: PaddleSide, _enable: bool) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Ch32v203KeyerHal {
}

/// Dit paddle input pin (PA0)
pub struct DitInputPin {
    pressed: AtomicBool,
    last_edge: AtomicU32,
    debounce_ms: u32,
}

impl DitInputPin {
    fn new() -> Self {
        Self {
            pressed: AtomicBool::new(false),
            last_edge: AtomicU32::new(0),
            debounce_ms: 10,
        }
    }
    
    fn init(&self) -> Result<(), ()> {
        // Configure PA0 as input with pull-up (active-low)
        // Enable EXTI0 interrupt on both edges (press and release detection)
        // Implementation would configure:
        // 1. GPIO PA0 as input with pull-up
        // 2. EXTI0 for both rising and falling edges (like V003)
        // 3. NVIC interrupt enable for EXTI0
        Ok(())
    }
    
    /// Called from EXTI0 interrupt handler (both edges)
    pub fn on_interrupt(&self, pressed: bool) {
        self.pressed.store(pressed, Ordering::Relaxed);
        // Store timestamp as microseconds since boot
        let now_us = Instant::now().as_micros() as u32;
        self.last_edge.store(now_us, Ordering::Relaxed);
    }
}

impl InputPaddle for DitInputPin {
    type Error = HalError;
    
    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        Ok(self.pressed.load(Ordering::Relaxed))
    }
    
    fn last_edge_time(&self) -> Option<Instant> {
        let edge_us = self.last_edge.load(Ordering::Relaxed);
        if edge_us == 0 {
            None
        } else {
            Some(Instant::from_micros(edge_us as u64))
        }
    }
    
    fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), Self::Error> {
        self.debounce_ms = time_ms;
        Ok(())
    }
    
    fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    
    fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Dah paddle input pin (PA1)
pub struct DahInputPin {
    pressed: AtomicBool,
    last_edge: AtomicU32,
    debounce_ms: u32,
}

impl DahInputPin {
    fn new() -> Self {
        Self {
            pressed: AtomicBool::new(false),
            last_edge: AtomicU32::new(0),
            debounce_ms: 10,
        }
    }
    
    fn init(&self) -> Result<(), ()> {
        // Configure PA1 as input with pull-up (active-low)
        // Enable EXTI1 interrupt on both edges (press and release detection)
        // Implementation would configure:
        // 1. GPIO PA1 as input with pull-up
        // 2. EXTI1 for both rising and falling edges (like V003)
        // 3. NVIC interrupt enable for EXTI1
        Ok(())
    }
    
    /// Called from EXTI1 interrupt handler (both edges)
    pub fn on_interrupt(&self, pressed: bool) {
        self.pressed.store(pressed, Ordering::Relaxed);
        // Store timestamp as microseconds since boot
        let now_us = Instant::now().as_micros() as u32;
        self.last_edge.store(now_us, Ordering::Relaxed);
    }
}

impl InputPaddle for DahInputPin {
    type Error = HalError;
    
    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        Ok(self.pressed.load(Ordering::Relaxed))
    }
    
    fn last_edge_time(&self) -> Option<Instant> {
        let edge_us = self.last_edge.load(Ordering::Relaxed);
        if edge_us == 0 {
            None
        } else {
            Some(Instant::from_micros(edge_us as u64))
        }
    }
    
    fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), Self::Error> {
        self.debounce_ms = time_ms;
        Ok(())
    }
    
    fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    
    fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Key output pin (PA2)
pub struct KeyOutputPin {
    state: AtomicBool,
}

impl KeyOutputPin {
    fn new() -> Self {
        Self {
            state: AtomicBool::new(false),
        }
    }
    
    fn init(&self) -> Result<(), ()> {
        // Configure PA2 as push-pull output
        Ok(())
    }
}

impl OutputKey for KeyOutputPin {
    type Error = HalError;
    
    fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
        self.state.store(state, Ordering::Relaxed);
        // TODO: Actual GPIO write
        #[cfg(feature = "defmt")]
        defmt::trace!("🔑 Key output: {}", state);
        Ok(())
    }
    
    fn get_state(&self) -> Result<bool, Self::Error> {
        Ok(self.state.load(Ordering::Relaxed))
    }
}

/// Global hardware instance for interrupt handlers
static CH32V203_HAL: StaticCell<Ch32v203KeyerHal> = StaticCell::new();

/// Initialize global hardware instance
pub fn init_global_hal() -> &'static mut Ch32v203KeyerHal {
    CH32V203_HAL.init(Ch32v203KeyerHal::new())
}

// Interrupt handlers (to be connected to actual EXTI handlers)

/// EXTI0 interrupt handler for Dit paddle
pub fn handle_dit_interrupt() {
    // In a real implementation, this would:
    // 1. Read GPIO state to determine press/release
    // 2. Call dit_pin.on_interrupt(pressed) to update atomic state
    // 3. Handle both rising and falling edges like V003
    
    // Pseudo-implementation:
    // let pressed = !read_gpio_pa0(); // Active-low with pull-up
    // if let Some(hal) = get_global_hal() {
    //     hal.dit_pin.on_interrupt(pressed);
    // }
}

/// EXTI1 interrupt handler for Dah paddle  
pub fn handle_dah_interrupt() {
    // In a real implementation, this would:
    // 1. Read GPIO state to determine press/release
    // 2. Call dah_pin.on_interrupt(pressed) to update atomic state
    // 3. Handle both rising and falling edges like V003
    
    // Pseudo-implementation:
    // let pressed = !read_gpio_pa1(); // Active-low with pull-up
    // if let Some(hal) = get_global_hal() {
    //     hal.dah_pin.on_interrupt(pressed);
    // }
}

/// CH32V203-specific timing utilities
pub mod timing {
    use embassy_time::{Duration, Timer};
    
    /// High-precision async delay
    pub async fn delay_precise(duration: Duration) {
        Timer::after(duration).await;
    }
    
    /// Calculate WPM timing
    pub fn wpm_to_unit_duration(wpm: u16) -> Duration {
        // PARIS method: 1 word = 50 units
        let unit_ms = 60_000 / (wpm as u64 * 50);
        Duration::from_millis(unit_ms)
    }
}

/// CH32V203 pin configuration constants
pub mod pins {
    /// Dit paddle input pin
    pub const DIT_PIN: u8 = 0; // PA0
    
    /// Dah paddle input pin  
    pub const DAH_PIN: u8 = 1; // PA1
    
    /// Key output pin
    pub const KEY_PIN: u8 = 2; // PA2
    
    /// Optional sidetone output pin
    pub const SIDETONE_PIN: u8 = 3; // PA3
}

/// CH32V203 memory layout information
pub mod memory {
    /// Available Flash memory (actual usable)
    pub const FLASH_SIZE: u32 = 60 * 1024; // 60KB usable
    
    /// Available RAM
    pub const RAM_SIZE: u32 = 20 * 1024; // 20KB
    
    /// Recommended Embassy task arena size
    pub const TASK_ARENA_SIZE: u32 = 8 * 1024; // 8KB
    
    /// Recommended queue sizes
    pub const LARGE_QUEUE_SIZE: usize = 64;
    pub const MEDIUM_QUEUE_SIZE: usize = 32;
    pub const SMALL_QUEUE_SIZE: usize = 16;
}