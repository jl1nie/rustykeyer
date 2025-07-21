//! Core data types for the iambic keyer

use crate::hal::{Duration, Instant};

/// Morse code elements
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum Element {
    /// Dit (short element)
    Dit,
    /// Dah (long element) 
    Dah,
    /// Character space (inter-character pause)
    CharSpace,
}

impl Element {
    /// Returns the duration of this element in units
    pub const fn duration_units(&self) -> u32 {
        match self {
            Element::Dit => 1,
            Element::Dah => 3,
            Element::CharSpace => 3,
        }
    }

    /// Returns true if this element produces key output
    pub const fn is_keyed(&self) -> bool {
        match self {
            Element::Dit | Element::Dah => true,
            Element::CharSpace => false,
        }
    }

    /// Returns the opposite element (Dit <-> Dah), CharSpace unchanged
    pub const fn opposite(&self) -> Element {
        match self {
            Element::Dit => Element::Dah,
            Element::Dah => Element::Dit,
            Element::CharSpace => Element::CharSpace,
        }
    }
}

/// Keyer operating modes
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyerMode {
    /// Mode A: Basic iambic squeeze, no memory
    ModeA,
    /// Mode B: Curtis A - one opposite element memory after squeeze release
    ModeB, 
    /// SuperKeyer: Dah priority with advanced memory
    SuperKeyer,
}

impl KeyerMode {
    /// Returns true if this mode supports memory after squeeze release
    pub const fn has_memory(&self) -> bool {
        match self {
            KeyerMode::ModeA => false,
            KeyerMode::ModeB | KeyerMode::SuperKeyer => true,
        }
    }

    /// Returns true if this mode uses priority logic
    pub const fn has_priority(&self) -> bool {
        match self {
            KeyerMode::ModeA | KeyerMode::ModeB => false,
            KeyerMode::SuperKeyer => true,
        }
    }
}

/// Paddle side identification
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PaddleSide {
    /// Dit paddle (typically left side)
    Dit,
    /// Dah paddle (typically right side)
    Dah,
}

impl PaddleSide {
    /// Convert to corresponding Element
    pub const fn to_element(&self) -> Element {
        match self {
            PaddleSide::Dit => Element::Dit,
            PaddleSide::Dah => Element::Dah,
        }
    }

    /// Returns the opposite paddle side
    pub const fn opposite(&self) -> PaddleSide {
        match self {
            PaddleSide::Dit => PaddleSide::Dah,
            PaddleSide::Dah => PaddleSide::Dit,
        }
    }
}

/// FSM states for the keyer
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FSMState {
    /// No paddles pressed, waiting for input
    Idle,
    /// Dit paddle held, sending Dit elements
    DitHold,
    /// Dah paddle held, sending Dah elements  
    DahHold,
    /// Both paddles pressed, alternating elements
    Squeeze(Element),
    /// Memory pending after squeeze release (Mode B/SuperKeyer)
    MemoryPending(Element),
    /// Character space timing, waiting for next character
    CharSpacePending(Instant),
}

impl FSMState {
    /// Returns true if this state represents active paddle input
    pub const fn has_paddle_input(&self) -> bool {
        match self {
            FSMState::Idle | FSMState::MemoryPending(_) | FSMState::CharSpacePending(_) => false,
            FSMState::DitHold | FSMState::DahHold | FSMState::Squeeze(_) => true,
        }
    }

    /// Returns the current element being sent, if any
    pub const fn current_element(&self) -> Option<Element> {
        match self {
            FSMState::DitHold => Some(Element::Dit),
            FSMState::DahHold => Some(Element::Dah),
            FSMState::Squeeze(element) => Some(*element),
            FSMState::MemoryPending(element) => Some(*element),
            FSMState::Idle | FSMState::CharSpacePending(_) => None,
        }
    }
}

/// Keyer configuration parameters
#[derive(Copy, Clone, Debug)]
pub struct KeyerConfig {
    /// Operating mode
    pub mode: KeyerMode,
    /// Enable character spacing
    pub char_space_enabled: bool,
    /// Basic timing unit (Dit duration)
    pub unit: Duration,
    /// Debounce time in milliseconds
    pub debounce_ms: u64,
    /// Queue size for element buffer
    pub queue_size: usize,
}

impl Default for KeyerConfig {
    fn default() -> Self {
        Self {
            mode: KeyerMode::ModeB,
            char_space_enabled: true,
            unit: Duration::from_millis(60), // 20 WPM
            debounce_ms: 10,
            queue_size: 64,
        }
    }
}

impl KeyerConfig {
    /// Create a new configuration with validation
    pub fn new(
        mode: KeyerMode,
        char_space_enabled: bool,
        wpm: u32,
        debounce_ms: u64,
        queue_size: usize,
    ) -> Result<Self, &'static str> {
        if wpm == 0 || wpm > 100 {
            return Err("WPM must be between 1 and 100");
        }
        if debounce_ms > 100 {
            return Err("Debounce must be <= 100ms");
        }
        if queue_size < 8 || queue_size > 1024 {
            return Err("Queue size must be between 8 and 1024");
        }

        // Calculate unit time from WPM (PARIS standard: 50 units per word)
        let unit = Duration::from_millis(1200 / wpm as u64);

        Ok(Self {
            mode,
            char_space_enabled,
            unit,
            debounce_ms,
            queue_size,
        })
    }

    /// Get Words Per Minute from current unit timing
    pub fn wpm(&self) -> u32 {
        (1200 / self.unit.as_millis() as u32).max(1)
    }

    /// Get inter-element space duration
    pub fn inter_element_space(&self) -> Duration {
        self.unit
    }

    /// Get character space duration  
    pub fn char_space_duration(&self) -> Duration {
        Duration::from_millis(self.unit.as_millis() * 3)
    }
}