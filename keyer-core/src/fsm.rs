//! Finite State Machine implementation for iambic keyer

use crate::hal::Instant;
use heapless::spsc::Producer;
use crate::types::{Element, FSMState, KeyerConfig, KeyerMode};
use crate::controller::{PaddleInput, SuperKeyerController};

/// Main keyer FSM implementation
pub struct KeyerFSM {
    state: FSMState,
    config: KeyerConfig,
    superkeyer: SuperKeyerController,
}

impl KeyerFSM {
    /// Create new FSM with given configuration
    pub fn new(config: KeyerConfig) -> Self {
        Self {
            state: FSMState::Idle,
            config,
            superkeyer: SuperKeyerController::new(),
        }
    }

    /// Get current FSM state
    pub fn current_state(&self) -> FSMState {
        self.state
    }

    /// Update FSM state and generate output elements
    /// Returns the number of elements enqueued
    pub fn update<const N: usize>(&mut self, paddle: &PaddleInput, queue: &mut Producer<'_, Element, N>) -> usize {
        let dit_now = paddle.dit();
        let dah_now = paddle.dah();
        let both_pressed = dit_now && dah_now;
        let both_released = !dit_now && !dah_now;
        let now = Instant::now();
        
        // Update SuperKeyer controller if in SuperKeyer mode
        if self.config.mode == KeyerMode::SuperKeyer {
            self.superkeyer.update(paddle);
        }

        let mut elements_sent = 0;

        // State machine transitions
        match self.state {
            FSMState::Idle => {
                elements_sent += self.handle_idle_state(dit_now, dah_now, both_pressed, queue);
            }

            FSMState::DitHold => {
                elements_sent += self.handle_dit_hold_state(dit_now, dah_now, both_pressed, queue);
            }

            FSMState::DahHold => {
                elements_sent += self.handle_dah_hold_state(dit_now, dah_now, both_pressed, queue);
            }

            FSMState::Squeeze(last_element) => {
                elements_sent += self.handle_squeeze_state(dit_now, dah_now, both_pressed, both_released, last_element, now, queue);
            }

            FSMState::MemoryPending(memory_element) => {
                elements_sent += self.handle_memory_pending_state(memory_element, now, queue);
            }

            FSMState::CharSpacePending(start_time) => {
                elements_sent += self.handle_char_space_pending_state(dit_now, dah_now, both_pressed, start_time, now, queue);
            }
        }

        elements_sent
    }

    /// Handle Idle state transitions
    fn handle_idle_state<const N: usize>(&mut self, dit_now: bool, dah_now: bool, both_pressed: bool, queue: &mut Producer<'_, Element, N>) -> usize {
        if both_pressed {
            let start_element = self.determine_squeeze_start();
            if queue.enqueue(start_element).is_ok() {
                self.state = FSMState::Squeeze(start_element);
                return 1;
            }
        } else if dit_now {
            if queue.enqueue(Element::Dit).is_ok() {
                self.state = FSMState::DitHold;
                return 1;
            }
        } else if dah_now {
            if queue.enqueue(Element::Dah).is_ok() {
                self.state = FSMState::DahHold;
                return 1;
            }
        }
        0
    }

    /// Handle DitHold state transitions
    fn handle_dit_hold_state<const N: usize>(&mut self, dit_now: bool, _dah_now: bool, both_pressed: bool, queue: &mut Producer<'_, Element, N>) -> usize {
        if both_pressed {
            self.state = FSMState::Squeeze(Element::Dit);
            0
        } else if !dit_now {
            self.transition_to_idle_or_char_space();
            0
        } else {
            // Continue holding Dit - send another Dit element
            if queue.enqueue(Element::Dit).is_ok() {
                1
            } else {
                0
            }
        }
    }

    /// Handle DahHold state transitions
    fn handle_dah_hold_state<const N: usize>(&mut self, _dit_now: bool, dah_now: bool, both_pressed: bool, queue: &mut Producer<'_, Element, N>) -> usize {
        if both_pressed {
            self.state = FSMState::Squeeze(Element::Dah);
            0
        } else if !dah_now {
            self.transition_to_idle_or_char_space();
            0
        } else {
            // Continue holding Dah - send another Dah element
            if queue.enqueue(Element::Dah).is_ok() {
                1
            } else {
                0
            }
        }
    }

    /// Handle Squeeze state transitions
    fn handle_squeeze_state<const N: usize>(
        &mut self,
        dit_now: bool,
        dah_now: bool,
        both_pressed: bool,
        both_released: bool,
        last_element: Element,
        now: Instant,
        queue: &mut Producer<'_, Element, N>
    ) -> usize {
        if both_pressed {
            // Continue squeeze - send alternating element
            let next_element = self.determine_next_squeeze_element(last_element);
            if queue.enqueue(next_element).is_ok() {
                self.state = FSMState::Squeeze(next_element);
                return 1;
            }
        } else if dit_now {
            // Only Dit pressed - transition to DitHold
            if queue.enqueue(Element::Dit).is_ok() {
                self.state = FSMState::DitHold;
                return 1;
            }
        } else if dah_now {
            // Only Dah pressed - transition to DahHold
            if queue.enqueue(Element::Dah).is_ok() {
                self.state = FSMState::DahHold;
                return 1;
            }
        } else if both_released {
            // Squeeze released - handle memory based on mode
            self.handle_squeeze_release(last_element, now);
        }
        0
    }

    /// Handle MemoryPending state
    fn handle_memory_pending_state<const N: usize>(&mut self, memory_element: Element, now: Instant, queue: &mut Producer<'_, Element, N>) -> usize {
        if queue.enqueue(memory_element).is_ok() {
            // Memory element sent, clear SuperKeyer history and transition
            if self.config.mode == KeyerMode::SuperKeyer {
                self.superkeyer.clear_history();
            }
            self.transition_to_idle_or_char_space_at_time(now);
            1
        } else {
            0
        }
    }

    /// Handle CharSpacePending state
    fn handle_char_space_pending_state<const N: usize>(
        &mut self,
        dit_now: bool,
        dah_now: bool,
        both_pressed: bool,
        start_time: Instant,
        now: Instant,
        queue: &mut Producer<'_, Element, N>
    ) -> usize {
        let elapsed = now.duration_since(start_time);
        let char_space_duration = self.config.char_space_duration();

        if dit_now || dah_now {
            if elapsed >= char_space_duration {
                // Character space complete, start new transmission
                return self.handle_idle_state(dit_now, dah_now, both_pressed, queue);
            }
            // Input too early, remain in CharSpacePending
        } else if elapsed >= char_space_duration {
            // Character space complete, return to Idle
            self.state = FSMState::Idle;
        }
        0
    }

    /// Determine which element to start with in squeeze mode
    fn determine_squeeze_start(&mut self) -> Element {
        match self.config.mode {
            KeyerMode::SuperKeyer => {
                self.superkeyer.determine_priority().unwrap_or(Element::Dit)
            }
            // For Mode A and B, use first-pressed priority (timestamp-based)
            KeyerMode::ModeA | KeyerMode::ModeB => {
                // This should be determined by the PaddleInput based on edge times
                // For now, default to Dit (will be enhanced with proper timestamp logic)
                Element::Dit
            }
        }
    }

    /// Determine next element in squeeze sequence
    fn determine_next_squeeze_element(&mut self, last_element: Element) -> Element {
        match self.config.mode {
            KeyerMode::SuperKeyer => {
                self.superkeyer.next_element(true, Some(last_element)).unwrap_or_else(|| last_element.opposite())
            }
            KeyerMode::ModeA | KeyerMode::ModeB => {
                // Standard alternating behavior
                last_element.opposite()
            }
        }
    }

    /// Handle squeeze release based on keyer mode
    fn handle_squeeze_release(&mut self, last_element: Element, now: Instant) {
        match self.config.mode {
            KeyerMode::ModeA => {
                // Mode A: immediate return to Idle/CharSpace
                self.transition_to_idle_or_char_space_at_time(now);
            }
            KeyerMode::ModeB => {
                // Mode B: send opposite element once
                let memory_element = last_element.opposite();
                self.state = FSMState::MemoryPending(memory_element);
            }
            KeyerMode::SuperKeyer => {
                // SuperKeyer: use controller to determine memory
                self.superkeyer.handle_squeeze_release(last_element);
                if let Some(memory) = self.superkeyer.take_memory() {
                    self.state = FSMState::MemoryPending(memory);
                } else {
                    self.transition_to_idle_or_char_space_at_time(now);
                }
            }
        }
    }

    /// Transition to Idle or CharSpacePending based on configuration
    fn transition_to_idle_or_char_space(&mut self) {
        self.transition_to_idle_or_char_space_at_time(Instant::now());
    }

    /// Transition to Idle or CharSpacePending at specific time
    fn transition_to_idle_or_char_space_at_time(&mut self, time: Instant) {
        if self.config.char_space_enabled {
            self.state = FSMState::CharSpacePending(time);
        } else {
            self.state = FSMState::Idle;
        }
    }

    /// Reset FSM to initial state
    pub fn reset(&mut self) {
        self.state = FSMState::Idle;
        self.superkeyer.clear_history();
    }

    /// Get current configuration
    pub fn config(&self) -> &KeyerConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: KeyerConfig) {
        self.config = config;
        if config.mode != KeyerMode::SuperKeyer {
            self.superkeyer.clear_history();
        }
    }
}

/// Async task for running the FSM evaluator
#[cfg(feature = "embassy-time")]
pub async fn evaluator_task<const N: usize>(
    paddle: &PaddleInput,
    mut queue_producer: Producer<'_, Element, N>,
    config: KeyerConfig,
) {
    use embassy_time::Timer;
    
    let mut fsm = KeyerFSM::new(config);
    let update_interval = config.unit / 4; // Update FSM at unit/4 intervals

    loop {
        let _elements_sent = fsm.update(paddle, &mut queue_producer);
        
        // Optional: Log state transitions for debugging
        #[cfg(feature = "defmt")]
        defmt::trace!("FSM State: {:?}", fsm.current_state());

        Timer::after(update_interval).await;
    }
}


// Skip tests that require embassy-time runtime for now
// Will be tested in integration tests with proper time driver setup