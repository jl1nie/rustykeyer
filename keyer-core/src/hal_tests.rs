//! HAL layer tests with mock implementations

#[cfg(test)]
use crate::hal::*;
#[cfg(test)]
use crate::hal::mock::*;
#[cfg(test)]
use crate::types::*;

#[test]
fn test_mock_paddle_basic_operations() {
    let mut paddle = MockPaddle::new();
    
    // Initially not pressed
    assert!(!paddle.is_pressed().unwrap());
    assert!(paddle.last_edge_time().is_none());
    
    // Press paddle
    paddle.set_pressed(true);
    assert!(paddle.is_pressed().unwrap());
    assert!(paddle.last_edge_time().is_some());
    
    // Release paddle
    paddle.set_pressed(false);
    assert!(!paddle.is_pressed().unwrap());
}

#[test]
fn test_mock_paddle_debounce_configuration() {
    let mut paddle = MockPaddle::new();
    
    // Set valid debounce time
    assert!(paddle.set_debounce_time(10).is_ok());
    assert!(paddle.set_debounce_time(50).is_ok());
    assert!(paddle.set_debounce_time(100).is_ok());
    
    // MockPaddle doesn't validate debounce time limits
    // The validation is done in the EmbeddedHalPaddle implementation
    assert!(paddle.set_debounce_time(101).is_ok());
}

#[test]
fn test_mock_paddle_interrupts() {
    let mut paddle = MockPaddle::new();
    
    // Interrupt operations should succeed on mock
    assert!(paddle.enable_interrupt().is_ok());
    assert!(paddle.disable_interrupt().is_ok());
}

#[test]
fn test_mock_key_output_operations() {
    let mut key = MockKeyOutput::new();
    
    // Initially off
    assert!(!key.get_state().unwrap());
    assert!(!key.is_active());
    
    // Turn on
    assert!(key.set_state(true).is_ok());
    assert!(key.get_state().unwrap());
    assert!(key.is_active());
    
    // Turn off
    assert!(key.set_state(false).is_ok());
    assert!(!key.get_state().unwrap());
    assert!(!key.is_active());
}

#[test]
fn test_mock_key_output_toggle() {
    let mut key = MockKeyOutput::new();
    
    // Initially off
    assert!(!key.get_state().unwrap());
    
    // Toggle on
    assert!(key.toggle().is_ok());
    assert!(key.get_state().unwrap());
    
    // Toggle off
    assert!(key.toggle().is_ok());
    assert!(!key.get_state().unwrap());
}

#[test]
fn test_noop_interrupt_controller() {
    let mut ctrl = NoOpInterruptController;
    
    // All operations should succeed
    assert!(ctrl.configure_paddle_interrupt(
        PaddleSide::Dit, true, true
    ).is_ok());
    
    assert!(ctrl.configure_paddle_interrupt(
        PaddleSide::Dah, false, true
    ).is_ok());
    
    assert!(ctrl.set_interrupt_priority(
        PaddleSide::Dit, 1
    ).is_ok());
    
    assert!(ctrl.set_interrupt_priority(
        PaddleSide::Dah, 255
    ).is_ok());
    
    assert!(ctrl.enable_paddle_interrupt(
        PaddleSide::Dit, true
    ).is_ok());
    
    assert!(ctrl.enable_paddle_interrupt(
        PaddleSide::Dah, false
    ).is_ok());
}

#[test]
fn test_hal_error_types() {
    // Verify all error types are distinct
    let errors = [
        HalError::GpioError,
        HalError::TimingError,
        HalError::InterruptError,
        HalError::NotInitialized,
        HalError::InvalidConfig,
    ];
    
    for (i, e1) in errors.iter().enumerate() {
        for (j, e2) in errors.iter().enumerate() {
            if i == j {
                assert_eq!(e1, e2);
            } else {
                assert_ne!(e1, e2);
            }
        }
    }
}

#[test]
fn test_mock_time_duration_operations() {
    let d1 = Duration::from_millis(100);
    let d2 = Duration::from_millis(300);
    
    // Basic properties
    assert_eq!(d1.as_millis(), 100);
    assert_eq!(d2.as_millis(), 300);
    
    // Division
    assert_eq!((d2 / 3).as_millis(), 100);
    assert_eq!((d1 / 2).as_millis(), 50);
    
    // Multiplication
    assert_eq!((d1 * 3).as_millis(), 300);
    assert_eq!((d1 * 2).as_millis(), 200);
    
    // Chained operations
    let d3 = (d1 * 2) / 4;
    assert_eq!(d3.as_millis(), 50);
}

#[test]
fn test_mock_time_instant_operations() {
    let t0 = Instant::from_millis(0);
    let t1 = Instant::from_millis(100);
    let t2 = Instant::from_millis(250);
    
    // Basic properties
    assert_eq!(t0.as_millis(), 0);
    assert_eq!(t1.as_millis(), 100);
    assert_eq!(t2.as_millis(), 250);
    
    // Duration calculations
    assert_eq!(t1.duration_since(t0).as_millis(), 100);
    assert_eq!(t2.duration_since(t1).as_millis(), 150);
    assert_eq!(t2.duration_since(t0).as_millis(), 250);
    
    // Saturating subtraction
    assert_eq!(t0.duration_since(t1).as_millis(), 0);
}

#[test]
fn test_mock_paddle_timing_sequence() {
    let mut paddle = MockPaddle::new();
    
    // Simulate timed sequence
    assert!(!paddle.is_pressed().unwrap());
    
    // Press at t=0
    paddle.set_pressed(true);
    let edge1 = paddle.last_edge_time();
    assert!(edge1.is_some());
    assert!(paddle.is_pressed().unwrap());
    
    // Release doesn't update edge time
    paddle.set_pressed(false);
    let edge2 = paddle.last_edge_time();
    assert_eq!(edge1, edge2);
    assert!(!paddle.is_pressed().unwrap());
}

#[test] 
fn test_complex_keying_scenario() {
    let mut dit_paddle = MockPaddle::new();
    let mut dah_paddle = MockPaddle::new();
    let mut key = MockKeyOutput::new();
    
    // Simulate sending 'A' (dit-dah)
    
    // Dit press
    dit_paddle.set_pressed(true);
    assert!(dit_paddle.is_pressed().unwrap());
    key.set_state(true).unwrap();
    assert!(key.is_active());
    
    // Dit release
    dit_paddle.set_pressed(false);
    key.set_state(false).unwrap();
    assert!(!key.is_active());
    
    // Inter-element space
    
    // Dah press
    dah_paddle.set_pressed(true);
    assert!(dah_paddle.is_pressed().unwrap());
    key.set_state(true).unwrap();
    assert!(key.is_active());
    
    // Dah release
    dah_paddle.set_pressed(false);
    key.set_state(false).unwrap();
    assert!(!key.is_active());
}

#[test]
fn test_squeeze_operation_mock() {
    let mut dit_paddle = MockPaddle::new();
    let mut dah_paddle = MockPaddle::new();
    
    // Both paddles pressed (squeeze)
    dit_paddle.set_pressed(true);
    dah_paddle.set_pressed(true);
    
    assert!(dit_paddle.is_pressed().unwrap());
    assert!(dah_paddle.is_pressed().unwrap());
    
    // Release dit first
    dit_paddle.set_pressed(false);
    assert!(!dit_paddle.is_pressed().unwrap());
    assert!(dah_paddle.is_pressed().unwrap());
    
    // Release dah
    dah_paddle.set_pressed(false);
    assert!(!dit_paddle.is_pressed().unwrap());
    assert!(!dah_paddle.is_pressed().unwrap());
}

#[cfg(feature = "std")]
#[test]
fn test_hal_error_display() {
    use std::error::Error;
    
    let errors = [
        (HalError::GpioError, "GPIO operation failed"),
        (HalError::TimingError, "Timing operation failed"),
        (HalError::InterruptError, "Interrupt configuration failed"),
        (HalError::NotInitialized, "Hardware not initialized"),
        (HalError::InvalidConfig, "Invalid configuration"),
    ];
    
    for (error, expected_msg) in errors {
        assert_eq!(format!("{}", error), expected_msg);
        // Verify Error trait is implemented
        let _: &dyn Error = &error;
    }
}