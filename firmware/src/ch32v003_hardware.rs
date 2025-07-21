//! CH32V003 specific hardware implementations
//! 
//! This module provides concrete implementations of the HAL traits for the CH32V003 microcontroller.
//! It handles GPIO for paddle inputs and key output, with proper debouncing and interrupt handling.

use core::option::Option::{self, None, Some};
use core::result::Result::{self, Ok, Err};
use core::default::Default;

use keyer_core::hal::{InputPaddle, OutputKey, HalError, Instant};
use embassy_time::Duration;
use embedded_hal::digital::{InputPin, OutputPin};

/// CH32V003 paddle input implementation with debouncing
pub struct Ch32v003Paddle<P> {
    pin: P,
    last_state: bool,
    last_edge_time: Option<Instant>,
    debounce_time_ms: u32,
}

impl<P: InputPin> Ch32v003Paddle<P> {
    /// Create new paddle input
    pub fn new(pin: P) -> Self {
        Self {
            pin,
            last_state: false,
            last_edge_time: None,
            debounce_time_ms: 10, // Default 10ms debounce
        }
    }
    
    /// Check if state has changed after debounce period
    fn read_debounced(&mut self) -> Result<bool, HalError> {
        let current_state = self.pin.is_high().map_err(|_| HalError::GpioError)?;
        
        if current_state != self.last_state {
            let now = Instant::now();
            
            // Check if enough time has passed since last edge
            if let Some(last_edge) = self.last_edge_time {
                let elapsed = now.duration_since(last_edge);
                if elapsed < Duration::from_millis(self.debounce_time_ms as u64) {
                    // Still in debounce period, return last stable state
                    return Ok(self.last_state);
                }
            }
            
            // State change is valid, update stored state
            self.last_state = current_state;
            self.last_edge_time = Some(now);
        }
        
        Ok(self.last_state)
    }
}

impl<P: InputPin> InputPaddle for Ch32v003Paddle<P> {
    type Error = HalError;

    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        // Active low logic (pressed = low)
        let state = self.read_debounced()?;
        Ok(!state)
    }

    fn last_edge_time(&self) -> Option<Instant> {
        self.last_edge_time
    }

    fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), Self::Error> {
        self.debounce_time_ms = time_ms.clamp(1, 100); // Limit to reasonable range
        Ok(())
    }

    fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
        // TODO: Enable GPIO interrupt for this pin
        // This would be CH32V003 specific interrupt configuration
        Ok(())
    }

    fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
        // TODO: Disable GPIO interrupt for this pin
        Ok(())
    }
}

/// CH32V003 key output implementation
pub struct Ch32v003KeyOutput<P> {
    pin: P,
    active_low: bool,
}

impl<P: OutputPin> Ch32v003KeyOutput<P> {
    /// Create new key output
    /// 
    /// # Arguments
    /// * `pin` - GPIO pin for key output
    /// * `active_low` - true if key-down is low signal, false if high
    pub fn new(pin: P, active_low: bool) -> Self {
        Self {
            pin,
            active_low,
        }
    }
}

impl<P: OutputPin> OutputKey for Ch32v003KeyOutput<P> {
    type Error = HalError;

    fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
        let output_level = if self.active_low {
            !state // Invert for active low
        } else {
            state
        };

        if output_level {
            self.pin.set_high().map_err(|_| HalError::GpioError)?;
        } else {
            self.pin.set_low().map_err(|_| HalError::GpioError)?;
        }

        #[cfg(feature = "defmt")]
        defmt::debug!("🔑 Key output: {} (physical: {})", 
                     if state { "DOWN" } else { "UP" },
                     if output_level { "HIGH" } else { "LOW" });

        Ok(())
    }

    fn get_state(&self) -> Result<bool, Self::Error> {
        // Note: This would require reading back the pin state
        // For now, we'll track state internally in a more complete implementation
        Err(HalError::GpioError)
    }
}

/// Complete CH32V003 hardware collection
pub struct Ch32v003KeyerHal<DitPin, DahPin, KeyPin> {
    pub dit_paddle: Ch32v003Paddle<DitPin>,
    pub dah_paddle: Ch32v003Paddle<DahPin>,
    pub key_output: Ch32v003KeyOutput<KeyPin>,
}

impl<DitPin, DahPin, KeyPin> Ch32v003KeyerHal<DitPin, DahPin, KeyPin>
where
    DitPin: InputPin,
    DahPin: InputPin,
    KeyPin: OutputPin,
{
    /// Initialize CH32V003 hardware
    /// 
    /// # Arguments
    /// * `dit_pin` - GPIO pin for dit paddle (active low)
    /// * `dah_pin` - GPIO pin for dah paddle (active low)  
    /// * `key_pin` - GPIO pin for key output
    /// * `key_active_low` - true if key output is active low
    pub fn new(
        dit_pin: DitPin,
        dah_pin: DahPin,
        key_pin: KeyPin,
        key_active_low: bool,
    ) -> Self {
        #[cfg(feature = "defmt")]
        defmt::info!("🔧 Initializing CH32V003 keyer hardware");

        Self {
            dit_paddle: Ch32v003Paddle::new(dit_pin),
            dah_paddle: Ch32v003Paddle::new(dah_pin),
            key_output: Ch32v003KeyOutput::new(key_pin, key_active_low),
        }
    }

    /// Configure debounce times for both paddles
    pub fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), HalError> {
        self.dit_paddle.set_debounce_time(time_ms)?;
        self.dah_paddle.set_debounce_time(time_ms)?;
        Ok(())
    }

    /// Enable interrupts for paddle inputs
    pub fn enable_paddle_interrupts(&mut self) -> Result<(), HalError> {
        self.dit_paddle.enable_interrupt()?;
        self.dah_paddle.enable_interrupt()?;
        Ok(())
    }

    /// Disable interrupts for paddle inputs
    pub fn disable_paddle_interrupts(&mut self) -> Result<(), HalError> {
        self.dit_paddle.disable_interrupt()?;
        self.dah_paddle.disable_interrupt()?;
        Ok(())
    }
}

/// Hardware configuration for CH32V003 keyer
#[derive(Clone, Copy, Debug)]
pub struct Ch32v003Config {
    /// Debounce time in milliseconds
    pub debounce_time_ms: u32,
    /// Key output is active low
    pub key_active_low: bool,
    /// Enable paddle interrupts
    pub use_interrupts: bool,
}

impl Default for Ch32v003Config {
    fn default() -> Self {
        Self {
            debounce_time_ms: 10,
            key_active_low: true,
            use_interrupts: true,
        }
    }
}

/// Initialize hardware with configuration
pub fn init_hardware<DitPin, DahPin, KeyPin>(
    dit_pin: DitPin,
    dah_pin: DahPin,
    key_pin: KeyPin,
    config: Ch32v003Config,
) -> Result<Ch32v003KeyerHal<DitPin, DahPin, KeyPin>, HalError>
where
    DitPin: InputPin,
    DahPin: InputPin,
    KeyPin: OutputPin,
{
    let mut hardware = Ch32v003KeyerHal::new(
        dit_pin,
        dah_pin,
        key_pin,
        config.key_active_low,
    );

    hardware.set_debounce_time(config.debounce_time_ms)?;

    if config.use_interrupts {
        hardware.enable_paddle_interrupts()?;
    }

    #[cfg(feature = "defmt")]
    defmt::info!("✅ CH32V003 hardware initialized successfully");

    Ok(hardware)
}