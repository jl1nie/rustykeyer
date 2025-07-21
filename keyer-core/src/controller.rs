//! Paddle input and SuperKeyer controller implementations

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use crate::hal::Instant;
use crate::types::{Element, PaddleSide};

/// Atomic paddle input state management
/// Safe for use in interrupt contexts
pub struct PaddleInput {
    dit_pressed: AtomicBool,
    dah_pressed: AtomicBool,
    dit_last_edge: AtomicU32,
    dah_last_edge: AtomicU32,
}

impl PaddleInput {
    /// Create new paddle input manager
    pub const fn new() -> Self {
        Self {
            dit_pressed: AtomicBool::new(false),
            dah_pressed: AtomicBool::new(false),
            dit_last_edge: AtomicU32::new(0),
            dah_last_edge: AtomicU32::new(0),
        }
    }

    /// Update paddle state (called from interrupt handler)
    /// 
    /// # Safety
    /// This function is safe to call from interrupt context
    pub fn update(&self, side: PaddleSide, state: bool, debounce_ms: u32) {
        let now = Instant::now().as_millis() as u32;
        
        match side {
            PaddleSide::Dit => {
                let last = self.dit_last_edge.load(Ordering::Relaxed);
                if now.saturating_sub(last) >= debounce_ms {
                    self.dit_pressed.store(state, Ordering::Relaxed);
                    self.dit_last_edge.store(now, Ordering::Relaxed);
                }
            }
            PaddleSide::Dah => {
                let last = self.dah_last_edge.load(Ordering::Relaxed);
                if now.saturating_sub(last) >= debounce_ms {
                    self.dah_pressed.store(state, Ordering::Relaxed);
                    self.dah_last_edge.store(now, Ordering::Relaxed);
                }
            }
        }
    }

    /// Check if Dit paddle is pressed
    pub fn dit(&self) -> bool {
        self.dit_pressed.load(Ordering::Relaxed)
    }

    /// Check if Dah paddle is pressed  
    pub fn dah(&self) -> bool {
        self.dah_pressed.load(Ordering::Relaxed)
    }

    /// Check if both paddles are pressed (squeeze condition)
    pub fn both_pressed(&self) -> bool {
        self.dit() && self.dah()
    }

    /// Check if both paddles are released
    pub fn both_released(&self) -> bool {
        !self.dit() && !self.dah()
    }

    /// Get press times for priority determination
    pub fn get_press_times(&self) -> (Option<u32>, Option<u32>) {
        let dit_time = if self.dit() {
            Some(self.dit_last_edge.load(Ordering::Relaxed))
        } else {
            None
        };
        
        let dah_time = if self.dah() {
            Some(self.dah_last_edge.load(Ordering::Relaxed))
        } else {
            None
        };
        
        (dit_time, dah_time)
    }

    /// Get the currently pressed element (if exactly one paddle is pressed)
    pub fn current_single_element(&self) -> Option<Element> {
        match (self.dit(), self.dah()) {
            (true, false) => Some(Element::Dit),
            (false, true) => Some(Element::Dah),
            _ => None,
        }
    }

    /// Reset all paddle states (for testing)
    #[cfg(feature = "test-utils")]
    pub fn reset(&self) {
        self.dit_pressed.store(false, Ordering::Relaxed);
        self.dah_pressed.store(false, Ordering::Relaxed);
        self.dit_last_edge.store(0, Ordering::Relaxed);
        self.dah_last_edge.store(0, Ordering::Relaxed);
    }
}

impl Default for PaddleInput {
    fn default() -> Self {
        Self::new()
    }
}

/// SuperKeyer mode controller with Dah priority and memory
#[derive(Debug)]
pub struct SuperKeyerController {
    dit_time: Option<Instant>,
    dah_time: Option<Instant>,
    memory_element: Option<Element>,
}

impl SuperKeyerController {
    /// Create new SuperKeyer controller
    pub fn new() -> Self {
        Self {
            dit_time: None,
            dah_time: None,
            memory_element: None,
        }
    }

    /// Record paddle press events with timestamps
    pub fn record_press(&mut self, dit_pressed: bool, dah_pressed: bool) {
        let now = Instant::now();
        
        if dit_pressed && self.dit_time.is_none() {
            self.dit_time = Some(now);
        }
        if dah_pressed && self.dah_time.is_none() {
            self.dah_time = Some(now);
        }

        // Clear times for released paddles
        if !dit_pressed {
            self.dit_time = None;
        }
        if !dah_pressed {
            self.dah_time = None;
        }
    }

    /// Determine priority element based on Dah-priority rules
    /// Returns Dah if both pressed simultaneously or Dah pressed first
    pub fn determine_priority(&self) -> Option<Element> {
        match (self.dit_time, self.dah_time) {
            (Some(dit), Some(dah)) => {
                // Dah priority: if Dah was pressed first or simultaneously, choose Dah
                if dah <= dit {
                    Some(Element::Dah)
                } else {
                    Some(Element::Dit)
                }
            }
            (Some(_), None) => Some(Element::Dit),
            (None, Some(_)) => Some(Element::Dah),
            (None, None) => None,
        }
    }

    /// Set memory element for later transmission
    pub fn set_memory(&mut self, element: Element) {
        self.memory_element = Some(element);
    }

    /// Get and consume memory element
    pub fn take_memory(&mut self) -> Option<Element> {
        self.memory_element.take()
    }

    /// Check if memory element should be sent
    pub fn should_send_memory(&self) -> bool {
        self.memory_element.is_some()
    }

    /// Clear all history and memory
    pub fn clear_history(&mut self) {
        self.dit_time = None;
        self.dah_time = None;
        self.memory_element = None;
    }

    /// Update controller state based on current paddle input
    pub fn update(&mut self, paddle_input: &PaddleInput) {
        self.record_press(paddle_input.dit(), paddle_input.dah());
    }

    /// Get next element to send based on current state and mode logic
    pub fn next_element(&mut self, squeeze: bool, _last_element: Option<Element>) -> Option<Element> {
        if squeeze {
            // In squeeze mode, determine priority
            self.determine_priority()
        } else if let Some(memory) = self.take_memory() {
            // Send memory element if available
            Some(memory)
        } else {
            // Standard single paddle logic
            if let Some(priority) = self.determine_priority() {
                Some(priority)
            } else {
                None
            }
        }
    }

    /// Handle squeeze release for SuperKeyer mode
    pub fn handle_squeeze_release(&mut self, last_sent: Element) {
        // In SuperKeyer mode, remember opposite element for memory transmission
        let opposite = last_sent.opposite();
        if opposite != Element::CharSpace {
            self.set_memory(opposite);
        }
    }
}

impl Default for SuperKeyerController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "test-utils")]
impl SuperKeyerController {
    /// Reset controller state (for testing)
    pub fn reset(&mut self) {
        self.clear_history();
    }

    /// Get current memory element without consuming it (for testing)
    pub fn peek_memory(&self) -> Option<Element> {
        self.memory_element
    }

    /// Get current press times (for testing)
    pub fn get_press_times(&self) -> (Option<Instant>, Option<Instant>) {
        (self.dit_time, self.dah_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_paddle_input_basic() {
        let paddle = PaddleInput::new();
        
        // Initially released
        assert!(!paddle.dit());
        assert!(!paddle.dah());
        assert!(paddle.both_released());
        assert!(!paddle.both_pressed());
        
        // Press Dit - use current time in millis
        let now_ms = 10u64;
        paddle.update(PaddleSide::Dit, true, now_ms);
        assert!(paddle.dit());
        assert!(!paddle.dah());
        assert_eq!(paddle.current_single_element(), Some(Element::Dit));
        
        // Press Dah (squeeze)
        paddle.update(PaddleSide::Dah, true, now_ms + 5);
        assert!(paddle.dit());
        assert!(paddle.dah());
        assert!(paddle.both_pressed());
        assert_eq!(paddle.current_single_element(), None);
    }

    #[test]
    fn test_superkeyer_priority() {
        let mut controller = SuperKeyerController::new();
        
        // Test Dah priority
        controller.record_press(true, true);
        assert_eq!(controller.determine_priority(), Some(Element::Dah));
        
        // Test single paddle
        controller.clear_history();
        controller.record_press(true, false);
        assert_eq!(controller.determine_priority(), Some(Element::Dit));
        
        controller.clear_history();
        controller.record_press(false, true);
        assert_eq!(controller.determine_priority(), Some(Element::Dah));
    }

    #[test]
    fn test_superkeyer_memory() {
        let mut controller = SuperKeyerController::new();
        
        // Set memory
        controller.set_memory(Element::Dah);
        assert!(controller.should_send_memory());
        
        // Take memory
        assert_eq!(controller.take_memory(), Some(Element::Dah));
        assert!(!controller.should_send_memory());
        assert_eq!(controller.take_memory(), None);
    }
}