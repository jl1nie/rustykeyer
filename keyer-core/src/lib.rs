#![cfg_attr(not(feature = "std"), no_std)]

//! # Keyer Core
//! 
//! Iambic keyer core logic library for embedded systems.
//! Supports Mode A, Mode B, and SuperKeyer modes with high-precision timing.

pub mod types;
pub mod fsm;
pub mod controller;
pub mod hal;

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(test)]
mod hal_tests;

pub use types::*;
pub use fsm::*;
pub use controller::*;
pub use hal::{*, Instant, Duration};

/// Keyer library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration for most amateur radio applications
pub fn default_config() -> KeyerConfig {
    KeyerConfig {
        mode: KeyerMode::ModeB,
        char_space_enabled: true,
        unit: Duration::from_millis(60), // 20 WPM
        debounce_ms: 10,
        queue_size: 64,
    }
}