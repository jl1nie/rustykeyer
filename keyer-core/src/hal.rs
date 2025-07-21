//! Hardware Abstraction Layer for keyer implementation

// Re-export time types based on feature
#[cfg(feature = "embassy-time")]
pub use embassy_time::{Duration, Instant};

#[cfg(not(feature = "embassy-time"))]
pub use self::mock_time::{Duration, Instant};

#[cfg(not(feature = "embassy-time"))]
mod mock_time {
    /// Mock instant type for compilation without embassy-time
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Instant(u64);

    impl Instant {
        pub fn now() -> Self {
            Self(0) // Placeholder implementation
        }
        
        pub fn from_millis(ms: i64) -> Self {
            Self(ms as u64)
        }
        
        pub fn duration_since(&self, other: Instant) -> Duration {
            Duration::from_millis(self.0.saturating_sub(other.0))
        }
        
        pub fn as_millis(&self) -> u64 {
            self.0
        }
    }

    /// Mock duration type
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Duration(u64);

    impl Duration {
        pub fn from_millis(ms: u64) -> Self {
            Self(ms)
        }
        
        pub fn as_millis(&self) -> u64 {
            self.0
        }
    }

    impl core::ops::Div<u32> for Duration {
        type Output = Duration;
        
        fn div(self, rhs: u32) -> Duration {
            Duration(self.0 / rhs as u64)
        }
    }

    impl core::ops::Mul<u32> for Duration {
        type Output = Duration;
        
        fn mul(self, rhs: u32) -> Duration {
            Duration(self.0 * rhs as u64)
        }
    }
}
use embedded_hal::digital::{InputPin, OutputPin};
use crate::types::PaddleSide;

/// Error types for HAL operations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HalError {
    /// GPIO operation failed
    GpioError,
    /// Timing operation failed
    TimingError,
    /// Interrupt configuration failed
    InterruptError,
    /// Hardware not initialized
    NotInitialized,
    /// Invalid configuration
    InvalidConfig,
}

#[cfg(feature = "std")]
impl core::fmt::Display for HalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HalError::GpioError => write!(f, "GPIO operation failed"),
            HalError::TimingError => write!(f, "Timing operation failed"),
            HalError::InterruptError => write!(f, "Interrupt configuration failed"),
            HalError::NotInitialized => write!(f, "Hardware not initialized"),
            HalError::InvalidConfig => write!(f, "Invalid configuration"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HalError {}

/// Trait for paddle input handling
pub trait InputPaddle {
    type Error: From<HalError>;

    /// Check if paddle is currently pressed
    fn is_pressed(&mut self) -> Result<bool, Self::Error>;
    
    /// Get timestamp of last edge transition
    fn last_edge_time(&self) -> Option<Instant>;
    
    /// Configure debounce time
    fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), Self::Error>;
    
    /// Enable edge interrupts for this paddle
    fn enable_interrupt(&mut self) -> Result<(), Self::Error>;
    
    /// Disable edge interrupts for this paddle
    fn disable_interrupt(&mut self) -> Result<(), Self::Error>;
}

/// Trait for key output control
pub trait OutputKey {
    type Error: From<HalError>;

    /// Set key output state (true = key down, false = key up)
    fn set_state(&mut self, state: bool) -> Result<(), Self::Error>;
    
    /// Get current key output state
    fn get_state(&self) -> Result<bool, Self::Error>;
    
    /// Toggle key output state
    fn toggle(&mut self) -> Result<(), Self::Error> {
        let current = self.get_state()?;
        self.set_state(!current)
    }
}

/// Trait for interrupt configuration
pub trait InterruptConfig {
    type Error: From<HalError>;

    /// Configure edge detection for paddle input
    fn configure_paddle_interrupt(
        &mut self, 
        paddle: PaddleSide,
        rising: bool,
        falling: bool
    ) -> Result<(), Self::Error>;
    
    /// Set interrupt priority
    fn set_interrupt_priority(&mut self, paddle: PaddleSide, priority: u8) -> Result<(), Self::Error>;
    
    /// Enable/disable specific paddle interrupt
    fn enable_paddle_interrupt(&mut self, paddle: PaddleSide, enable: bool) -> Result<(), Self::Error>;
}

/// Complete keyer HAL interface
pub trait KeyerHal {
    type DitPaddle: InputPaddle;
    type DahPaddle: InputPaddle;
    type KeyOutput: OutputKey;
    type InterruptCtrl: InterruptConfig;
    type Error: From<HalError>;

    /// Initialize hardware
    fn initialize(&mut self) -> Result<(), Self::Error>;
    
    /// Access to Dit paddle
    fn dit_paddle(&mut self) -> &mut Self::DitPaddle;
    
    /// Access to Dah paddle  
    fn dah_paddle(&mut self) -> &mut Self::DahPaddle;
    
    /// Access to key output
    fn key_output(&mut self) -> &mut Self::KeyOutput;
    
    /// Access to interrupt controller
    fn interrupt_controller(&mut self) -> &mut Self::InterruptCtrl;
    
    /// Shutdown hardware
    fn shutdown(&mut self) -> Result<(), Self::Error>;
}

/// Generic implementation for embedded-hal compatible pins
pub struct EmbeddedHalPaddle<P> {
    pin: P,
    last_edge: Option<Instant>,
    debounce_ms: u32,
}

impl<P> EmbeddedHalPaddle<P>
where
    P: InputPin,
{
    pub fn new(pin: P) -> Self {
        Self {
            pin,
            last_edge: None,
            debounce_ms: 10,
        }
    }

    /// Update edge time (called from interrupt handler)
    pub fn update_edge_time(&mut self, time: Instant) {
        self.last_edge = Some(time);
    }
}

impl<P> InputPaddle for EmbeddedHalPaddle<P>
where
    P: InputPin,
    P::Error: Into<HalError>,
{
    type Error = HalError;

    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        // Assuming active low (pulled up, grounded when pressed)
        self.pin.is_low().map_err(|_| HalError::GpioError)
    }

    fn last_edge_time(&self) -> Option<Instant> {
        self.last_edge
    }

    fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), Self::Error> {
        if time_ms > 100 {
            return Err(HalError::InvalidConfig);
        }
        self.debounce_ms = time_ms;
        Ok(())
    }

    fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
        // Platform-specific implementation required
        Err(HalError::InterruptError)
    }

    fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
        // Platform-specific implementation required  
        Err(HalError::InterruptError)
    }
}

/// Generic implementation for embedded-hal compatible output pins
pub struct EmbeddedHalKeyOutput<P> {
    pin: P,
    inverted: bool,
}

impl<P> EmbeddedHalKeyOutput<P>
where
    P: OutputPin,
{
    pub fn new(pin: P, inverted: bool) -> Self {
        Self { pin, inverted }
    }
}

impl<P> OutputKey for EmbeddedHalKeyOutput<P>
where
    P: OutputPin,
    P::Error: Into<HalError>,
{
    type Error = HalError;

    fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
        let output_state = if self.inverted { !state } else { state };
        if output_state {
            self.pin.set_high().map_err(|_| HalError::GpioError)
        } else {
            self.pin.set_low().map_err(|_| HalError::GpioError)
        }
    }

    fn get_state(&self) -> Result<bool, Self::Error> {
        // Note: embedded-hal doesn't provide input reading for output pins
        // Platform-specific implementation may be needed
        Err(HalError::GpioError)
    }
}

/// No-op interrupt controller for basic implementations
pub struct NoOpInterruptController;

impl InterruptConfig for NoOpInterruptController {
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

#[cfg(any(test, feature = "test-utils"))]
pub mod mock {
    //! Mock implementations for testing
    
    use super::*;
    use core::cell::RefCell;
    
    #[derive(Default)]
    pub struct MockPaddle {
        pressed: RefCell<bool>,
        last_edge: RefCell<Option<Instant>>,
        debounce_ms: RefCell<u32>,
    }
    
    impl MockPaddle {
        pub fn new() -> Self {
            Self::default()
        }
        
        pub fn set_pressed(&self, pressed: bool) {
            *self.pressed.borrow_mut() = pressed;
            if pressed {
                *self.last_edge.borrow_mut() = Some(Instant::now());
            }
        }
    }
    
    impl InputPaddle for MockPaddle {
        type Error = HalError;
        
        fn is_pressed(&mut self) -> Result<bool, Self::Error> {
            Ok(*self.pressed.borrow())
        }
        
        fn last_edge_time(&self) -> Option<Instant> {
            *self.last_edge.borrow()
        }
        
        fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), Self::Error> {
            *self.debounce_ms.borrow_mut() = time_ms;
            Ok(())
        }
        
        fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        
        fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    
    #[derive(Default)]
    pub struct MockKeyOutput {
        state: RefCell<bool>,
    }
    
    impl MockKeyOutput {
        pub fn new() -> Self {
            Self::default()
        }
        
        pub fn is_active(&self) -> bool {
            *self.state.borrow()
        }
    }
    
    impl OutputKey for MockKeyOutput {
        type Error = HalError;
        
        fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
            *self.state.borrow_mut() = state;
            Ok(())
        }
        
        fn get_state(&self) -> Result<bool, Self::Error> {
            Ok(*self.state.borrow())
        }
    }
}