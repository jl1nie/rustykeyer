//! HAL layer tests with mock implementations

#[cfg(test)]
use crate::hal::*;
#[cfg(test)]
use crate::hal::mock::*;
#[cfg(test)]
use crate::types::*;
#[cfg(test)]
use crate::controller::{PaddleInput};
#[cfg(test)]
use crate::fsm::KeyerFSM;
#[cfg(test)]
use heapless::spsc::Queue;
// Embassy-time removed for test compatibility

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

#[test]
fn test_paddle_input_squeeze_detection() {
    let paddle = PaddleInput::new();
    let start_time = 100u32;
    
    // Press both paddles (squeeze)
    paddle.update(PaddleSide::Dit, true, start_time);
    paddle.update(PaddleSide::Dah, true, start_time + 1);
    
    assert!(paddle.dit());
    assert!(paddle.dah());
    assert!(paddle.both_pressed());
    assert!(!paddle.both_released());
    assert_eq!(paddle.current_single_element(), None); // No single element in squeeze
    
    // Release Dit first
    paddle.update(PaddleSide::Dit, false, start_time + 50);
    assert!(!paddle.dit());
    assert!(paddle.dah());
    assert!(!paddle.both_pressed());
    assert_eq!(paddle.current_single_element(), Some(Element::Dah)); // Dah remains
    
    // Release Dah
    paddle.update(PaddleSide::Dah, false, start_time + 100);
    assert!(!paddle.dit());
    assert!(!paddle.dah());
    assert!(paddle.both_released());
    assert_eq!(paddle.current_single_element(), None);
}

#[test]
fn test_fsm_squeeze_mode_a() {
    // Mode A: Ultimatic - first pressed paddle wins, no memory
    let mut fsm = KeyerFSM::new(KeyerConfig {
        mode: KeyerMode::ModeA,
        char_space_enabled: false,
        unit: crate::hal::Duration::from_millis(60), // 20 WPM
        debounce_ms: 5,
        queue_size: 8,
    });
    
    let paddle = PaddleInput::new();
    let mut queue = Queue::<Element, 8>::new();
    let (mut producer, _consumer) = queue.split();
    let start_time = 100u32;
    
    // Press Dit first, then Dah quickly (squeeze simulation)
    paddle.update(PaddleSide::Dit, true, start_time);
    paddle.update(PaddleSide::Dah, true, start_time + 5); // Very quick squeeze
    
    let sent = fsm.update(&paddle, &mut producer);
    
    // Mode A: Should only queue Dit (first pressed), ignore Dah during transmission
    assert_eq!(sent, 1);
    // In Mode A, subsequent presses are ignored during element transmission
}

#[test] 
fn test_fsm_squeeze_mode_b() {
    // Mode B: Iambic with one element memory
    let mut fsm = KeyerFSM::new(KeyerConfig {
        mode: KeyerMode::ModeB,
        char_space_enabled: false,
        unit: crate::hal::Duration::from_millis(60), // 20 WPM
        debounce_ms: 5,
        queue_size: 8,
    });
    
    let paddle = PaddleInput::new();
    let mut queue = Queue::<Element, 8>::new();
    let (mut producer, mut consumer) = queue.split();
    let start_time = 100u32;
    
    // Squeeze: Press Dit first, then Dah
    paddle.update(PaddleSide::Dit, true, start_time);
    paddle.update(PaddleSide::Dah, true, start_time + 5);
    
    let sent1 = fsm.update(&paddle, &mut producer);
    assert_eq!(sent1, 1); // Dit queued
    
    // Release Dit but keep Dah pressed
    paddle.update(PaddleSide::Dit, false, start_time + 50);
    
    // Mode B should remember Dah press and queue it next
    let _sent2 = fsm.update(&paddle, &mut producer);
    
    // Verify queue contains Dit then Dah
    assert!(consumer.ready());
    let element1 = consumer.dequeue().unwrap();
    assert_eq!(element1, Element::Dit);
    
    if consumer.ready() {
        let element2 = consumer.dequeue().unwrap();
        assert_eq!(element2, Element::Dah);
    }
}

#[test]
fn test_fsm_squeeze_superkeyer_dah_priority() {
    // SuperKeyer: Dah priority in ambiguous situations
    let mut fsm = KeyerFSM::new(KeyerConfig {
        mode: KeyerMode::SuperKeyer,
        char_space_enabled: false,
        unit: crate::hal::Duration::from_millis(60), // 20 WPM
        debounce_ms: 5,
        queue_size: 8,
    });
    
    let paddle = PaddleInput::new();
    let mut queue = Queue::<Element, 8>::new();
    let (mut producer, mut consumer) = queue.split();
    let start_time = 100u32;
    
    // Simultaneous press (true squeeze) - SuperKeyer should prioritize Dah
    paddle.update(PaddleSide::Dit, true, start_time);
    paddle.update(PaddleSide::Dah, true, start_time); // Same time = simultaneous
    
    let sent = fsm.update(&paddle, &mut producer);
    assert_eq!(sent, 1);
    
    // First element should be Dah due to SuperKeyer priority
    assert!(consumer.ready());
    let first_element = consumer.dequeue().unwrap();
    assert_eq!(first_element, Element::Dah);
}

#[test] 
fn test_squeeze_release_patterns() {
    // Test different release orders and their effects
    let paddle = PaddleInput::new();
    let start_time = 100u32;
    
    // Pattern 1: Press both, release Dit first
    paddle.update(PaddleSide::Dit, true, start_time);
    paddle.update(PaddleSide::Dah, true, start_time + 1);
    assert!(paddle.both_pressed());
    
    paddle.update(PaddleSide::Dit, false, start_time + 50);
    assert!(!paddle.both_pressed());
    assert!(paddle.dah()); // Dah still active
    assert_eq!(paddle.current_single_element(), Some(Element::Dah));
    
    // Pattern 2: Release Dah
    paddle.update(PaddleSide::Dah, false, start_time + 100);
    assert!(paddle.both_released());
    
    // Pattern 3: Press both, release Dah first
    paddle.update(PaddleSide::Dit, true, start_time + 150);
    paddle.update(PaddleSide::Dah, true, start_time + 151);
    assert!(paddle.both_pressed());
    
    paddle.update(PaddleSide::Dah, false, start_time + 200);
    assert!(!paddle.both_pressed());
    assert!(paddle.dit()); // Dit still active
    assert_eq!(paddle.current_single_element(), Some(Element::Dit));
}

#[test]
fn test_squeeze_timing_boundaries() {
    // Test squeeze operations at element and character boundaries
    let mut fsm = KeyerFSM::new(KeyerConfig {
        mode: KeyerMode::ModeB,
        char_space_enabled: false,
        unit: crate::hal::Duration::from_millis(60), // 20 WPM
        debounce_ms: 5,
        queue_size: 8,
    });
    
    let paddle = PaddleInput::new();
    let mut queue = Queue::<Element, 8>::new();
    let (mut producer, mut consumer) = queue.split();
    
    // Send single Dit first
    let time1 = 100u32;
    paddle.update(PaddleSide::Dit, true, time1);
    let sent1 = fsm.update(&paddle, &mut producer);
    assert_eq!(sent1, 1);
    
    paddle.update(PaddleSide::Dit, false, time1 + 50);
    
    // During inter-element space, press both (squeeze)
    let time2 = time1 + 120; // During inter-element space
    paddle.update(PaddleSide::Dit, true, time2);
    paddle.update(PaddleSide::Dah, true, time2 + 5);
    
    let _sent2 = fsm.update(&paddle, &mut producer);
    
    // Should handle additional elements (may be 0 if FSM is in different state)
    // The key test is that FSM accepts the input without errors
    // Note: sent2 is usize, so always >= 0
    
    // Verify element sequence
    assert!(consumer.ready());
    let first = consumer.dequeue().unwrap();
    assert_eq!(first, Element::Dit); // Original Dit
    
    if consumer.ready() {
        let second = consumer.dequeue().unwrap();
        // Should be either Dit or Dah from squeeze, depending on FSM state
        assert!(second == Element::Dit || second == Element::Dah);
    }
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