#![no_std]

//! Firmware library for exposing mock hardware and tasks for testing

use core::option::Option::Some;

pub use embassy_executor::Spawner;
pub use embassy_time::Duration;
pub use heapless::spsc::Queue;
pub use static_cell::StaticCell;

pub use keyer_core::*;

// Re-export hardware implementations
pub use crate::mock_hardware::*;
pub use crate::ch32v203_hardware::*;
pub use crate::tasks::*;

// Mock hardware module
pub mod mock_hardware {
    use keyer_core::hal::{InputPaddle, OutputKey, HalError};
    
    /// Mock paddle implementation
    #[derive(Debug)]
    pub struct MockPaddle {
        pressed: bool,
    }
    
    impl MockPaddle {
        pub fn new() -> Self {
            Self { pressed: false }
        }
        
        /// Set paddle state for testing
        pub fn set_pressed(&mut self, pressed: bool) {
            self.pressed = pressed;
        }
    }
    
    impl InputPaddle for MockPaddle {
        type Error = HalError;
    
        fn is_pressed(&mut self) -> Result<bool, Self::Error> {
            Ok(self.pressed)
        }
    
        fn last_edge_time(&self) -> Option<crate::hal::Instant> {
            None
        }
    
        fn set_debounce_time(&mut self, _time_ms: u32) -> Result<(), Self::Error> {
            Ok(())
        }
    
        fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    
        fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    
    /// Mock key output implementation
    #[derive(Debug)]
    pub struct MockKeyOutput {
        state: bool,
    }
    
    impl MockKeyOutput {
        pub fn new() -> Self {
            Self { state: false }
        }
        
        /// Get current key state for testing
        pub fn is_active(&self) -> bool {
            self.state
        }
    }
    
    impl OutputKey for MockKeyOutput {
        type Error = HalError;
    
        fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
            #[cfg(feature = "defmt")]
            if state != self.state {
                defmt::info!("🔑 Key: {}", if state { "DOWN" } else { "UP" });
            }
            self.state = state;
            Ok(())
        }
    
        fn get_state(&self) -> Result<bool, Self::Error> {
            Ok(self.state)
        }
    }
    
    /// Mock hardware collection
    #[derive(Debug)]
    pub struct MockKeyerHal {
        pub dit_paddle: MockPaddle,
        pub dah_paddle: MockPaddle,
        pub key_output: MockKeyOutput,
    }
    
    impl MockKeyerHal {
        pub fn new() -> Self {
            #[cfg(feature = "defmt")]
            defmt::info!("🧪 Using mock hardware (for testing)");
            Self {
                dit_paddle: MockPaddle::new(),
                dah_paddle: MockPaddle::new(), 
                key_output: MockKeyOutput::new(),
            }
        }
    }
}

// Embassy tasks module
pub mod tasks {
    use super::*;
    use heapless::spsc::{Producer, Consumer};
    
    /// Evaluator task wrapper
    #[embassy_executor::task]
    pub async fn evaluator_task_wrapper(
        paddle: &'static PaddleInput,
        producer: Producer<'static, Element, 8>,
        config: KeyerConfig,
    ) {
        #[cfg(feature = "defmt")]
        defmt::info!("🧠 Evaluator task started");
        keyer_core::fsm::evaluator_task::<8>(paddle, producer, config).await;
    }
    
    /// Sender task for key output
    #[embassy_executor::task]
    pub async fn sender_task_with_mock(
        mut consumer: Consumer<'static, Element, 8>,
        unit: Duration,
        key_output: &'static mut crate::mock_hardware::MockKeyOutput,
    ) {
        #[cfg(feature = "defmt")]
        defmt::info!("📤 Sender task started");
    
        loop {
            if let Some(element) = consumer.dequeue() {
                let (on_time, element_name) = match element {
                    Element::Dit => (unit, "Dit"),
                    Element::Dah => (unit * 3, "Dah"),
                    Element::CharSpace => (Duration::from_millis(0), "Space"),
                };
    
                if element.is_keyed() {
                    #[cfg(feature = "defmt")]
                    defmt::debug!("📡 Sending {}", element_name);
                    
                    // Key down
                    key_output.set_state(true).ok();
                    embassy_time::Timer::after(on_time).await;
                    
                    // Key up
                    key_output.set_state(false).ok();
                    
                    // Inter-element space (except for CharSpace)
                    embassy_time::Timer::after(unit).await;
                } else {
                    // Character space - just wait
                    #[cfg(feature = "defmt")]
                    defmt::debug!("⏸️ Character space");
                    embassy_time::Timer::after(unit * 3).await;
                }
            } else {
                // No elements in queue, brief pause
                embassy_time::Timer::after(unit / 8).await;
            }
        }
    }
}

// CH32V203 hardware module
pub mod ch32v203_hardware;

// Time driver for embassy
mod time_driver;