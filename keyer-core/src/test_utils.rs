//! Test utilities for keyer core functionality

#[cfg(all(feature = "test-utils", feature = "std", feature = "embassy-time"))]
pub mod virtual_time {
    //! Virtual time simulation for deterministic testing
    
    use embassy_time::{Duration, Instant};
    use std::sync::{Arc, Mutex};
    use std::collections::BinaryHeap;
    use std::cmp::Reverse;
    
    /// Virtual time controller for testing
    #[derive(Clone)]
    pub struct VirtualTime {
        inner: Arc<Mutex<VirtualTimeInner>>,
    }
    
    struct VirtualTimeInner {
        current_time: u64, // milliseconds since start
        scheduled_events: BinaryHeap<Reverse<ScheduledEvent>>,
    }
    
    #[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
    struct ScheduledEvent {
        time: u64,
        id: usize,
    }
    
    impl VirtualTime {
        pub fn new() -> Self {
            Self {
                inner: Arc::new(Mutex::new(VirtualTimeInner {
                    current_time: 0,
                    scheduled_events: BinaryHeap::new(),
                })),
            }
        }
        
        /// Get current virtual time
        pub fn now(&self) -> Instant {
            let inner = self.inner.lock().unwrap();
            Instant::from_millis(inner.current_time)
        }
        
        /// Advance virtual time by duration
        pub fn advance(&self, duration: Duration) {
            let mut inner = self.inner.lock().unwrap();
            inner.current_time += duration.as_millis() as u64;
        }
        
        /// Schedule an event at specific time
        pub fn schedule_event(&self, delay: Duration) -> usize {
            let mut inner = self.inner.lock().unwrap();
            let event_time = inner.current_time + delay.as_millis() as u64;
            let event_id = inner.scheduled_events.len();
            
            inner.scheduled_events.push(Reverse(ScheduledEvent {
                time: event_time,
                id: event_id,
            }));
            
            event_id
        }
        
        /// Get next scheduled event time
        pub fn next_event_time(&self) -> Option<Duration> {
            let inner = self.inner.lock().unwrap();
            inner.scheduled_events.peek().map(|event| {
                Duration::from_millis((event.0.time - inner.current_time) as u64)
            })
        }
        
        /// Advance to next scheduled event
        pub fn advance_to_next_event(&self) -> Option<usize> {
            let mut inner = self.inner.lock().unwrap();
            if let Some(Reverse(event)) = inner.scheduled_events.pop() {
                inner.current_time = event.time;
                Some(event.id)
            } else {
                None
            }
        }
    }
}

#[cfg(all(feature = "test-utils", feature = "embassy-time"))]
pub mod paddle_simulator {
    //! Paddle input simulation for testing
    
    use crate::types::PaddleSide;
    use crate::controller::PaddleInput;
    use embassy_time::Duration;
    use heapless::{Vec, String};
    
    /// Paddle event for simulation
    #[derive(Debug, Clone)]
    pub struct PaddleEvent {
        pub time: Duration,
        pub side: PaddleSide,
        pub pressed: bool,
    }
    
    /// Paddle pattern for simulation
    #[derive(Debug, Clone)]
    pub struct PaddlePattern {
        pub events: Vec<PaddleEvent, 64>,
        pub description: String<32>,
    }
    
    impl PaddlePattern {
        /// Create a simple Dit pattern
        pub fn dit(unit: Duration) -> Self {
            Self {
                events: Vec::from_slice(&[
                    PaddleEvent { time: Duration::from_millis(0), side: PaddleSide::Dit, pressed: true },
                    PaddleEvent { time: unit, side: PaddleSide::Dit, pressed: false },
                ]).unwrap(),
                description: String::try_from("Dit").unwrap(),
            }
        }
        
        /// Create a simple Dah pattern
        pub fn dah(unit: Duration) -> Self {
            Self {
                events: Vec::from_slice(&[
                    PaddleEvent { time: Duration::from_millis(0), side: PaddleSide::Dah, pressed: true },
                    PaddleEvent { time: unit * 3, side: PaddleSide::Dah, pressed: false },
                ]).unwrap(),
                description: String::try_from("Dah").unwrap(),
            }
        }
        
        /// Create a squeeze pattern (both paddles)
        pub fn squeeze(unit: Duration, duration: Duration) -> Self {
            Self {
                events: Vec::from_slice(&[
                    PaddleEvent { time: Duration::from_millis(0), side: PaddleSide::Dit, pressed: true },
                    PaddleEvent { time: Duration::from_millis(10), side: PaddleSide::Dah, pressed: true },
                    PaddleEvent { time: duration, side: PaddleSide::Dit, pressed: false },
                    PaddleEvent { time: duration + Duration::from_millis(10), side: PaddleSide::Dah, pressed: false },
                ]).unwrap(),
                description: String::try_from("Squeeze").unwrap(),
            }
        }
        
        /// Create letter 'A' pattern (Dit-Dah)
        pub fn letter_a(unit: Duration) -> Self {
            let inter_element = unit;
            Self {
                events: Vec::from_slice(&[
                    // Dit
                    PaddleEvent { time: Duration::from_millis(0), side: PaddleSide::Dit, pressed: true },
                    PaddleEvent { time: unit, side: PaddleSide::Dit, pressed: false },
                    // Inter-element space
                    // Dah
                    PaddleEvent { time: unit + inter_element, side: PaddleSide::Dah, pressed: true },
                    PaddleEvent { time: unit + inter_element + unit * 3, side: PaddleSide::Dah, pressed: false },
                ]).unwrap(),
                description: String::try_from("Letter A (Dit-Dah)").unwrap(),
            }
        }
        
        /// Combine multiple patterns with timing
        pub fn sequence(patterns: &[(PaddlePattern, Duration)]) -> Self {
            let mut events = Vec::new();
            let mut offset = Duration::from_millis(0);
            let mut descriptions = Vec::<String<32>, 16>::new();
            
            for (pattern, delay) in patterns {
                for event in &pattern.events {
                    events.push(PaddleEvent {
                        time: offset + event.time,
                        side: event.side,
                        pressed: event.pressed,
                    }).ok();
                }
                descriptions.push(pattern.description.clone()).ok();
                offset += *delay;
            }
            
            Self {
                events,
                description: String::try_from("Sequence").unwrap(),
            }
        }
    }
    
    /// Execute a paddle pattern against a PaddleInput
    pub fn execute_pattern(paddle: &PaddleInput, pattern: &PaddlePattern, debounce_ms: u64) {
        for event in &pattern.events {
            // In real test, this would use virtual time
            paddle.update(event.side, event.pressed, debounce_ms);
        }
    }
}

#[cfg(all(feature = "test-utils", feature = "embassy-time"))]
pub mod output_capture {
    //! Output capture and analysis for testing
    
    use crate::types::Element;
    use embassy_time::{Duration, Instant};
    use std::collections::VecDeque;
    use heapless::{Vec, String};
    
    /// Captured keyer output event
    #[derive(Debug, Clone, PartialEq)]
    pub struct OutputEvent {
        pub element: Element,
        pub start_time: Instant,
        pub duration: Duration,
    }
    
    /// Output capture buffer
    #[derive(Debug)]
    pub struct OutputCapture {
        events: VecDeque<OutputEvent>,
        current_element: Option<Element>,
        element_start: Option<Instant>,
    }
    
    impl OutputCapture {
        pub fn new() -> Self {
            Self {
                events: VecDeque::new(),
                current_element: None,
                element_start: None,
            }
        }
        
        /// Record key down event
        pub fn key_down(&mut self, element: Element, time: Instant) {
            if self.current_element.is_some() {
                // End previous element
                self.key_up(time);
            }
            
            self.current_element = Some(element);
            self.element_start = Some(time);
        }
        
        /// Record key up event
        pub fn key_up(&mut self, time: Instant) {
            if let (Some(element), Some(start)) = (self.current_element.take(), self.element_start.take()) {
                let duration = time.duration_since(start);
                self.events.push_back(OutputEvent {
                    element,
                    start_time: start,
                    duration,
                });
            }
        }
        
        /// Get all captured events
        pub fn events(&self) -> &VecDeque<OutputEvent> {
            &self.events
        }
        
        /// Clear capture buffer
        pub fn clear(&mut self) {
            self.events.clear();
            self.current_element = None;
            self.element_start = None;
        }
        
        /// Analyze timing accuracy
        pub fn analyze_timing(&self, expected_unit: Duration) -> TimingAnalysis {
            let mut dit_durations = Vec::<Duration, 32>::new();
            let mut dah_durations = Vec::<Duration, 32>::new();
            let mut inter_element_gaps = Vec::<Duration, 32>::new();
            
            for (i, event) in self.events.iter().enumerate() {
                match event.element {
                    Element::Dit => { dit_durations.push(event.duration).ok(); },
                    Element::Dah => { dah_durations.push(event.duration).ok(); },
                    Element::CharSpace => {}
                }
                
                // Calculate inter-element gap
                if i + 1 < self.events.len() {
                    let next_event = &self.events[i + 1];
                    let gap = next_event.start_time.duration_since(
                        event.start_time + event.duration
                    );
                    inter_element_gaps.push(gap).ok();
                }
            }
            
            TimingAnalysis {
                expected_unit,
                dit_durations,
                dah_durations,
                inter_element_gaps,
            }
        }
        
        /// Convert to sequence of elements
        pub fn to_element_sequence(&self) -> Vec<Element, 32> {
            let mut result = Vec::new();
            for event in &self.events {
                result.push(event.element).ok();
            }
            result
        }
        
        /// Convert to morse code string
        pub fn to_morse_string(&self) -> String<32> {
            let mut result = String::new();
            for event in &self.events {
                let ch = match event.element {
                    Element::Dit => ".",
                    Element::Dah => "-",
                    Element::CharSpace => " ",
                };
                result.push_str(ch).ok();
            }
            result
        }
    }
    
    /// Timing analysis results
    #[derive(Debug)]
    pub struct TimingAnalysis {
        pub expected_unit: Duration,
        pub dit_durations: Vec<Duration, 32>,
        pub dah_durations: Vec<Duration, 32>,
        pub inter_element_gaps: Vec<Duration, 32>,
    }
    
    impl TimingAnalysis {
        /// Calculate Dit timing accuracy (percentage error)
        pub fn dit_accuracy(&self) -> f64 {
            if self.dit_durations.is_empty() { return 0.0; }
            
            let expected = self.expected_unit.as_millis() as f64;
            let average = self.dit_durations.iter()
                .map(|d| d.as_millis() as f64)
                .sum::<f64>() / self.dit_durations.len() as f64;
            
            ((average - expected).abs() / expected) * 100.0
        }
        
        /// Calculate Dah timing accuracy (should be 3x unit)
        pub fn dah_accuracy(&self) -> f64 {
            if self.dah_durations.is_empty() { return 0.0; }
            
            let expected = (self.expected_unit.as_millis() * 3) as f64;
            let average = self.dah_durations.iter()
                .map(|d| d.as_millis() as f64)
                .sum::<f64>() / self.dah_durations.len() as f64;
            
            ((average - expected).abs() / expected) * 100.0
        }
        
        /// Calculate inter-element spacing accuracy
        pub fn spacing_accuracy(&self) -> f64 {
            if self.inter_element_gaps.is_empty() { return 0.0; }
            
            let expected = self.expected_unit.as_millis() as f64;
            let average = self.inter_element_gaps.iter()
                .map(|d| d.as_millis() as f64)
                .sum::<f64>() / self.inter_element_gaps.len() as f64;
            
            ((average - expected).abs() / expected) * 100.0
        }
    }
}

#[cfg(all(feature = "test-utils", feature = "embassy-time"))]
pub mod test_scenarios {
    //! Common test scenarios
    
    use super::paddle_simulator::{PaddlePattern, PaddleEvent};
    use crate::types::{KeyerMode, PaddleSide};
    use embassy_time::Duration;
    use heapless::{Vec, String};
    
    /// Generate test scenario for CQ calling
    pub fn cq_call_pattern(unit: Duration) -> PaddlePattern {
        // CQ CQ CQ = (-.-. --.- / -.-. --.- / -.-. --.-) 
        PaddlePattern {
            events: Vec::from_slice(&[
                // C: -.-. 
                PaddleEvent { time: Duration::from_millis(0), side: PaddleSide::Dah, pressed: true },
                // ... (implement full CQ pattern)
            ]).unwrap(),
            description: String::try_from("CQ CQ CQ call").unwrap(),
        }
    }
    
    /// Contest-style rapid input
    pub fn contest_pattern(unit: Duration) -> PaddlePattern {
        PaddlePattern {
            events: Vec::new(),
            description: String::try_from("Contest rapid input").unwrap(),
        }
    }
    
    /// Test all three keyer modes with same input
    pub fn mode_comparison_scenarios() -> Vec<(KeyerMode, PaddlePattern), 8> {
        let unit = Duration::from_millis(60);
        Vec::from_slice(&[
            (KeyerMode::ModeA, PaddlePattern::squeeze(unit, unit * 5)),
            (KeyerMode::ModeB, PaddlePattern::squeeze(unit, unit * 5)),
            (KeyerMode::SuperKeyer, PaddlePattern::squeeze(unit, unit * 5)),
        ]).unwrap()
    }
}