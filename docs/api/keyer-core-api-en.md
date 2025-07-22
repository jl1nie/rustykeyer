# keyer-core API Reference

**Rust Iambic Keyer Core Library** - `no_std` compatible embedded keyer library

## 📋 Overview

`keyer-core` is a `no_std` compatible library providing core functionality for iambic keyers. It supports three keyer modes: Mode A, Mode B (Curtis A), and SuperKeyer, with high-precision timing control and HAL abstraction.

### 🎯 Key Features
- **3 Keyer Modes**: A (Basic), B (Curtis A), SuperKeyer (Dah Priority)
- **HAL Abstraction**: Portability across different hardware platforms
- **Real-time Control**: 1ms precision timing management
- **Type Safety**: Compile-time verification through Rust's type system

## 📦 Module Structure

```rust
pub mod types;        // Data type definitions
pub mod fsm;          // Finite state machine
pub mod controller;   // Input control & SuperKeyer
pub mod hal;          // HAL abstraction
```

## 🔧 Basic Usage

### Configuration and Setup
```rust
use keyer_core::*;

// Default configuration (20 WPM, Mode A - Unified settings)
let config = keyer_core::default_config();

// Custom configuration
let config = KeyerConfig {
    mode: KeyerMode::ModeA,  // Unified default (V203/V003 compatible)
    unit: Duration::from_millis(60), // 20 WPM
    char_space_enabled: true,
    debounce_ms: 10,  // Unified debounce (practical noise immunity)
    queue_size: 4, // For low-memory MCUs
};

// Initialize FSM and queue
let mut fsm = KeyerFSM::new(config);
let (mut producer, mut consumer) = queue.split();
```

### Main Loop Implementation
```rust
loop {
    // Read paddle state + FSM update
    let dit_pressed = /* Read from GPIO */;
    let dah_pressed = /* Read from GPIO */;
    
    let paddle = PaddleInput::new();
    paddle.update(PaddleSide::Dit, dit_pressed, system_time_ms);
    paddle.update(PaddleSide::Dah, dah_pressed, system_time_ms);
    
    fsm.update(&paddle, &mut producer);
    
    // Process output elements
    if let Some(element) = consumer.dequeue() {
        match element {
            Element::Dit => send_dit(config.unit),
            Element::Dah => send_dah(config.unit * 3),
            Element::CharSpace => delay(config.unit * 3),
        }
    }
}
```

## 📚 API Details

### 🎛️ Core Types (`types`)

#### `Element` - Morse Code Elements
```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Element {
    Dit,        // Dot (1 unit)
    Dah,        // Dash (3 units)
    CharSpace,  // Character space (3 units)
}

impl Element {
    pub const fn duration_units(&self) -> u32;  // Unit duration
    pub const fn is_keyed(&self) -> bool;       // Key output element check
    pub const fn opposite(&self) -> Element;    // Get opposite element
}
```

#### `KeyerMode` - Keyer Operating Modes
```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyerMode {
    ModeA,      // Basic iambic (no memory)
    ModeB,      // Curtis A (1-element memory)
    SuperKeyer, // Dah priority (advanced memory)
}
```

#### `KeyerConfig` - Keyer Configuration
```rust
#[derive(Copy, Clone, Debug)]
pub struct KeyerConfig {
    pub mode: KeyerMode,
    pub char_space_enabled: bool,  // Auto character spacing
    pub unit: Duration,            // Base unit time
    pub debounce_ms: u32,          // Debounce time
    pub queue_size: usize,         // Output queue size
}

impl KeyerConfig {
    pub fn wpm(&self) -> u32;      // Calculate WPM
    pub fn validate(&self) -> Result<(), &'static str>;
}
```

#### `PaddleSide` - Paddle Identification
```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PaddleSide {
    Dit,  // Dit side paddle
    Dah,  // Dah side paddle
}
```

### 🎚️ Paddle Input (`controller`)

#### `PaddleInput` - Paddle Input Management
```rust
pub struct PaddleInput {
    // Internal state (Atomic operations)
}

impl PaddleInput {
    pub const fn new() -> Self;
    pub fn update(&self, side: PaddleSide, state: bool, now_ms: u32);
    pub fn dit(&self) -> bool;                    // Get Dit state
    pub fn dah(&self) -> bool;                    // Get Dah state
    pub fn both_pressed(&self) -> bool;           // Check simultaneous press
    pub fn both_released(&self) -> bool;          // Check simultaneous release
    pub fn current_single_element(&self) -> Option<Element>;  // Single element check
    pub fn get_press_times(&self) -> (Option<u32>, Option<u32>);  // Press timestamps
}
```

#### `SuperKeyerController` - SuperKeyer Control
```rust
pub struct SuperKeyerController {
    // Internal history & priority management
}

impl SuperKeyerController {
    pub fn new() -> Self;
    pub fn update(&mut self, paddle_input: &PaddleInput);
    pub fn next_element(&mut self, squeeze: bool, last_element: Option<Element>) -> Option<Element>;
    pub fn set_memory(&mut self, element: Element);      // Set memory
    pub fn clear_history(&mut self);                     // Clear history
}
```

### 🔄 Finite State Machine (`fsm`)

#### `KeyerFSM` - Main State Machine
```rust
pub struct KeyerFSM {
    // Configuration, state, controller
}

impl KeyerFSM {
    pub fn new(config: KeyerConfig) -> Self;
    pub fn update<P>(&mut self, paddle: &PaddleInput, producer: &mut P)
    where P: Producer<Element>;
    pub fn reset(&mut self);                             // Reset state
    pub fn config(&self) -> &KeyerConfig;               // Get configuration
}
```

#### `FSMState` - FSM Internal States
```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum FSMState {
    Idle,           // Idle state
    SendingDit,     // Sending Dit
    SendingDah,     // Sending Dah
    InterElement,   // Inter-element space
    InterCharacter, // Inter-character space
    Squeezed,       // Squeeze processing
}
```

### 🔌 Hardware Abstraction Layer (`hal`)

#### Trait Definitions
```rust
/// GPIO input abstraction
pub trait InputPaddle {
    type Error;
    fn is_pressed(&mut self) -> Result<bool, Self::Error>;
    fn last_edge_time(&self) -> Option<Instant>;
    fn set_debounce_time(&mut self, time_ms: u32) -> Result<(), Self::Error>;
    fn enable_interrupt(&mut self) -> Result<(), Self::Error>;
    fn disable_interrupt(&mut self) -> Result<(), Self::Error>;
}

/// GPIO output abstraction  
pub trait OutputKey {
    type Error;
    fn set_state(&mut self, state: bool) -> Result<(), Self::Error>;
    fn get_state(&self) -> Result<bool, Self::Error>;
}
```

#### Time & Duration Types
```rust
/// System time (1ms precision)
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Instant(i64);

impl Instant {
    pub fn from_millis(ms: i64) -> Self;
    pub fn elapsed(&self) -> Duration;
    pub fn duration_since(&self, earlier: Instant) -> Duration;
}

/// Duration (1ms precision)
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Duration(u64);

impl Duration {
    pub const fn from_millis(ms: u64) -> Self;
    pub const fn as_millis(&self) -> u64;
    pub const fn from_secs(secs: u64) -> Self;
}
```

## 🎯 Keyer Mode Details

### Mode A - Basic Iambic
```rust
// Features:
// - Alternating transmission during squeeze (DitDahDitDah...)
// - Immediate stop when paddles are released
// - No memory function
// - For beginners and precise control

let config = KeyerConfig {
    mode: KeyerMode::ModeA,  // Unified default (V203/V003 compatible)
    // Other settings...
};
```

### Mode B - Curtis A
```rust
// Features:
// - Mode A + 1-element memory function
// - Sends opposite element once after squeeze release
// - Accu-Keyer compatible
// - Most common setting

let config = KeyerConfig {
    mode: KeyerMode::ModeB,
    // Other settings...
};
```

### SuperKeyer - Dah Priority
```rust
// Features:
// - Dah Priority: Always sends Dah on simultaneous press
// - Advanced Memory: Control based on press history
// - Timestamp priority determination
// - For advanced users and high-speed operation

let config = KeyerConfig {
    mode: KeyerMode::SuperKeyer,
    // Other settings...
};
```

## 📊 Performance Characteristics

### Memory Usage
```
Flash Usage: 2.9KB-6.4KB (varies by platform & features)
RAM Usage: 2KB-20KB (V003: 2KB, V203: 20KB)
Stack Usage: ~256-512B (function call depth)
Efficiency: V003 18.6%, V203 10.2% (Flash utilization)
```

### Timing Accuracy
```
Base Precision: 1ms (depends on HAL implementation)
WPM Range: 5-100 WPM (recommended: 10-50 WPM)
Jitter: ±0.1ms (with stable system clock)
Responsiveness: <1ms (interrupt-driven)
```

## 🧪 Testing

### Test Execution
```bash
# Run all tests
cargo test -p keyer-core --no-default-features

# HAL integration tests (21 tests)
cargo test -p keyer-core --no-default-features hal_tests

# Squeeze functionality tests
cargo test -p keyer-core --no-default-features squeeze
```

### Test Coverage
- ✅ **21/21 Tests Passed** - Full functionality verification
- ✅ **Basic HAL Functions** - GPIO & timing control
- ✅ **Squeeze Operations** - All 3 modes fully verified  
- ✅ **Boundary Conditions** - Timing boundaries & error handling
- ✅ **Integration Operations** - FSM & Controller coordination

## 🔧 Implementation Examples

### CH32V003 Bare Metal Implementation
```rust
// hardware.rs
use keyer_core::*;

struct Ch32v003Hal;

impl InputPaddle for Ch32v003Hal {
    type Error = HalError;
    
    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        // Direct GPIO read
        Ok(read_dit_gpio() || read_dah_gpio())
    }
}

impl OutputKey for Ch32v003Hal {
    type Error = HalError;
    
    fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
        // Direct GPIO control
        write_key_gpio(state);
        write_led_gpio(state);
        set_sidetone_pwm(if state { 500 } else { 0 });
        Ok(())
    }
}
```

### Embassy Async Implementation
```rust
// embassy_main.rs
use keyer_core::*;
use embassy_executor::Spawner;

#[embassy_executor::task]
async fn keyer_task(mut fsm: KeyerFSM, producer: Producer<Element>) {
    loop {
        let paddle = read_paddle_state().await;
        fsm.update(&paddle, &producer);
        Timer::after(Duration::from_millis(1)).await;
    }
}

#[embassy_executor::task] 
async fn sender_task(mut consumer: Consumer<Element>, unit: Duration) {
    while let Some(element) = consumer.dequeue() {
        send_element(element, unit).await;
    }
}
```

## 📖 Related Documentation

- **[CH32V003 Implementation Guide](../hardware/CH32V003_BAREMENTAL_GUIDE_EN.md)** - Bare metal implementation details
- **[Circuit Diagram](../hardware/CH32V003_CIRCUIT_DIAGRAM_EN.md)** - Hardware circuit examples
- **[Design Specification](../../.kiro/specs/keyer-main/design.en.md)** - Architecture details

**keyer-core provides implementation examples of HAL abstraction and real-time control in Rust embedded development.**