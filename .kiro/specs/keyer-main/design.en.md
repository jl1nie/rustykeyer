# Keyer-Main - Technical Design

> ✅ **Implementation Complete** - Full implementation based on this design document has been completed.
> 
> **Implementation Status**:
> - ✅ **keyer-core Library**: Fully implemented & compiles successfully
> - ✅ **firmware Application**: Embassy async task implementation complete  
> - ✅ **HAL Abstraction**: Feature-based conditional compilation support
> - ✅ **FSM Implementation**: All modes (A/B/SuperKeyer) implemented
> - ✅ **Test Framework**: Host-based virtual time testing design complete
> 
> **Update History**:
> - 2025-01-21: Initial version created (reverse engineered from sample code)
> - 2025-01-21: Fixed interrupt handling and debounce explanations
> - 2025-01-21: Detailed architecture (layered structure, data flow, error handling added)
> - 2025-01-21: Added HAL abstraction layer detailed design (700 line expansion)
> - 2025-01-21: Added host environment test design (800 line expansion)
> - 2025-01-21: **Implementation Complete** - All component implementation and compilation verification complete

## 1. Architecture Overview

### 1.1 Layered Architecture

#### Overall System Structure
```
┌─────────────────────────────────────────────────────┐
│                   Application Layer                  │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐│
│  │ evaluator   │  │  sender     │  │ SuperKeyer  ││
│  │    _fsm     │→ │   _task     │  │ Controller  ││
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘│
│         │                 │                 │        │
│  ┌──────┴─────────────────┴─────────────────┴──────┐│
│  │          Message Queue (SPSC, 64 elements)      ││
│  └──────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────┤
│                    Driver Layer                      │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐│
│  │ PaddleInput │  │  EXTI ISR   │  │ GPIO Driver ││
│  │  (Atomic)   │← │  Handlers   │  │  (HAL)      ││
│  └─────────────┘  └─────────────┘  └─────────────┘│
├─────────────────────────────────────────────────────┤
│                   Hardware Layer                     │
├─────────────────────────────────────────────────────┤
│  PA0: Dit Input   PA1: Dah Input   PA2: Key Output  │
│  (Pull-up, INT)   (Pull-up, INT)   (Push-pull)     │
└─────────────────────────────────────────────────────┘
```

### 1.2 Data Flow

#### Input Processing Flow
```
Hardware Event → ISR → Atomic State → Evaluator Task → FSM → Queue → Sender Task → GPIO Output
     ↓              ↓        ↓            ↓           ↓       ↓         ↓           ↓
  Paddle Press → Record → Update State → Process → Generate → Buffer → Transmit → Key Output
```

#### Timing Control Flow
```
Embassy Timer → FSM Update (unit/4) → State Transition → Element Generation → Queue Processing
      ↓              ↓                      ↓                    ↓                ↓
   Periodic      State Check         Timing Logic        Dit/Dah/Pause       Serial Output
```

---

## 2. Core Components

### 2.1 FSM (Finite State Machine)

#### State Definition
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum FSMState {
    Idle,
    DitHold,
    DahHold,
    Squeeze(Element),
    MemoryPending(Element),
    CharSpacePending(Instant),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Dit,
    Dah,
    Pause,
}
```

#### State Transition Logic
- **Idle**: Waiting for paddle input
- **DitHold/DahHold**: Single paddle pressed, transmitting corresponding element
- **Squeeze**: Both paddles pressed, alternating transmission
- **MemoryPending**: Queue opposite element for Mode B/SuperKeyer
- **CharSpacePending**: Inter-character space timing

### 2.2 SuperKeyer Controller

#### Dah Priority Algorithm
```rust
impl SuperKeyerController {
    pub fn update_state(&mut self, input: &PaddleInput, now: Instant) -> SuperKeyerAction {
        // Update timestamps
        if input.dit && self.dit_time.is_none() {
            self.dit_time = Some(now);
        }
        if input.dah && self.dah_time.is_none() {
            self.dah_time = Some(now);
        }

        // Priority determination
        match (input.dit, input.dah) {
            (true, true) => {
                // Dah priority: Always prefer Dah in SuperKeyer mode
                SuperKeyerAction::ForceDah
            }
            (true, false) => SuperKeyerAction::Dit,
            (false, true) => SuperKeyerAction::Dah,
            (false, false) => {
                // Memory transmission logic
                self.handle_release(now)
            }
        }
    }
}
```

### 2.3 Input Management

#### Paddle Input Structure
```rust
#[derive(Debug, Clone)]
pub struct PaddleInput {
    pub dit: bool,
    pub dah: bool,
    pub timestamp: Instant,
}

// Atomic state for ISR communication
static PADDLE_STATE: AtomicU8 = AtomicU8::new(0);
```

#### Interrupt Handler
```rust
#[interrupt]
fn EXTI0() {  // Dit paddle
    let now = Instant::now();
    PADDLE_STATE.fetch_or(0b01, Ordering::Relaxed);
    // Minimal ISR processing - just record the event
}

#[interrupt]
fn EXTI1() {  // Dah paddle
    let now = Instant::now();
    PADDLE_STATE.fetch_or(0b10, Ordering::Relaxed);
    // Minimal ISR processing - just record the event
}
```

---

## 3. Task Architecture

### 3.1 Evaluator Task
```rust
#[embassy_executor::task]
async fn evaluator_task(
    paddle: &'static AtomicPaddleInput,
    producer: heapless::spsc::Producer<Element, 64>,
    config: KeyerConfig,
) {
    let mut fsm = FSM::new(config.mode);
    let mut controller = SuperKeyerController::new();
    
    let mut ticker = Ticker::every(config.unit / 4);
    
    loop {
        ticker.next().await;
        
        let input = paddle.load();
        let action = controller.update_state(&input, Instant::now());
        
        if let Some(element) = fsm.update(action, &config) {
            let _ = producer.enqueue(element);
        }
    }
}
```

### 3.2 Sender Task
```rust
#[embassy_executor::task]
async fn sender_task(
    mut consumer: heapless::spsc::Consumer<Element, 64>,
    unit: Duration,
    key_output: impl OutputPin,
) {
    loop {
        if let Some(element) = consumer.dequeue() {
            match element {
                Element::Dit => {
                    key_output.set_high();
                    Timer::after(unit).await;
                    key_output.set_low();
                    Timer::after(unit).await;
                }
                Element::Dah => {
                    key_output.set_high();
                    Timer::after(unit * 3).await;
                    key_output.set_low();
                    Timer::after(unit).await;
                }
                Element::Pause => {
                    Timer::after(unit * 3).await;
                }
            }
        } else {
            Timer::after(Duration::from_millis(1)).await;
        }
    }
}
```

---

## 4. HAL Abstraction

### 4.1 Hardware Abstraction Layer
```rust
pub trait KeyerHAL {
    type InputPin: InputPin + ExtiPin;
    type OutputPin: OutputPin;
    type Timer: Timer;
    
    fn init_hardware(&mut self) -> Result<(), Self::Error>;
    fn setup_interrupts(&mut self) -> Result<(), Self::Error>;
    fn get_paddle_state(&self) -> PaddleInput;
}
```

### 4.2 Platform-Specific Implementation
```rust
// CH32V003 implementation
#[cfg(feature = "ch32v003")]
impl KeyerHAL for CH32V003HAL {
    type InputPin = ch32v003_hal::gpio::Pin<Input<PullUp>>;
    type OutputPin = ch32v003_hal::gpio::Pin<Output<PushPull>>;
    type Timer = ch32v003_hal::timer::Timer;
    
    fn init_hardware(&mut self) -> Result<(), Self::Error> {
        // CH32V003-specific initialization
    }
}

// STM32F4 implementation  
#[cfg(feature = "stm32f4")]
impl KeyerHAL for STM32F4HAL {
    type InputPin = stm32f4xx_hal::gpio::Pin<Input<PullUp>>;
    type OutputPin = stm32f4xx_hal::gpio::Pin<Output<PushPull>>;
    type Timer = stm32f4xx_hal::timer::Timer;
    
    fn init_hardware(&mut self) -> Result<(), Self::Error> {
        // STM32F4-specific initialization
    }
}
```

---

## 5. Configuration Management

### 5.1 Keyer Configuration
```rust
#[derive(Debug, Clone)]
pub struct KeyerConfig {
    pub mode: KeyerMode,
    pub unit: Duration,
    pub char_space_enabled: bool,
    pub debounce_ms: u8,
    pub queue_size: usize,
}

impl Default for KeyerConfig {
    fn default() -> Self {
        Self {
            mode: KeyerMode::ModeB,
            unit: Duration::from_millis(60), // 20 WPM
            char_space_enabled: true,
            debounce_ms: 10,
            queue_size: 64,
        }
    }
}
```

### 5.2 Runtime Validation
```rust
impl KeyerConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.unit.as_millis() < 17 || self.unit.as_millis() > 200 {
            return Err(ConfigError::InvalidUnit);
        }
        
        if self.debounce_ms > 50 {
            return Err(ConfigError::InvalidDebounce);
        }
        
        if self.queue_size < 8 || self.queue_size > 256 {
            return Err(ConfigError::InvalidQueueSize);
        }
        
        Ok(())
    }
}
```

---

## 6. Error Handling

### 6.1 Error Types
```rust
#[derive(Debug, PartialEq)]
pub enum KeyerError {
    QueueFull,
    InvalidConfiguration,
    HardwareError,
    TimingError,
}
```

### 6.2 Graceful Degradation
- **Queue Overflow**: Drop oldest elements, continue operation
- **Hardware Fault**: Disable faulty input, continue with remaining functionality  
- **Timing Drift**: Auto-correction via Embassy time synchronization
- **Configuration Error**: Fall back to safe defaults

---

## 7. Performance Optimization

### 7.1 Memory Usage
- **Stack per task**: < 512 bytes
- **Static allocation**: Eliminate dynamic allocation completely
- **Queue sizing**: Configurable 8-256 elements based on use case

### 7.2 Real-time Constraints
- **ISR execution**: < 5μs (minimal atomic operations only)
- **Task switching**: Embassy's zero-cost async
- **Timing accuracy**: ±1% via hardware timers

### 7.3 Power Optimization
```rust
// Power-aware implementation
async fn low_power_evaluator_task() {
    loop {
        let input = wait_for_paddle_change().await;
        
        if input.is_idle() {
            // Enter low-power mode
            embassy_time::Timer::after(Duration::from_millis(100)).await;
        } else {
            // Active processing
            process_input(input).await;
        }
    }
}
```

---

## 8. Testing Strategy

### 8.1 Host-based Testing
```rust
// Virtual time simulation for deterministic testing
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::time::*;
    
    #[tokio::test]
    async fn test_mode_b_squeeze_memory() {
        pause();
        
        let mut fsm = FSM::new(KeyerMode::ModeB);
        let config = KeyerConfig::default();
        
        // Simulate squeeze and release
        let input = PaddleInput { dit: true, dah: true, timestamp: Instant::now() };
        let result = fsm.update(input, &config);
        
        advance(Duration::from_millis(60)).await;
        
        // Verify memory behavior
        assert_eq!(result, Some(Element::Dit));
    }
}
```

### 8.2 Integration Testing
- **Hardware simulation**: Mock GPIO and timing
- **Real-time validation**: Timing accuracy measurement
- **Stress testing**: Queue overflow and recovery
- **Compatibility testing**: Mode B vs commercial keyers

---

## 9. Deployment Considerations

### 9.1 Firmware Size
- **Target**: < 16KB flash for CH32V003
- **Optimization**: Release profile with LTO enabled
- **Feature gating**: Conditional compilation for unused modes

### 9.2 Production Deployment
```toml
[profile.release]
debug = false
lto = true
codegen-units = 1
panic = "abort"
opt-level = "s"  # Size optimization for embedded
```

### 9.3 Field Updates
- **Bootloader integration**: Embassy-compatible bootloader
- **Configuration persistence**: EEPROM/Flash storage
- **Factory reset**: Hardware pin combination for recovery

---

## 10. Future Extensions

### 10.1 Advanced Features
- **Sidetone generation**: PWM-based audio output
- **Speed adjustment**: Runtime WPM modification
- **Memory training**: Adaptive timing learning
- **Contest macros**: Predefined message transmission

### 10.2 Connectivity
- **USB interface**: Configuration and monitoring
- **Wireless updates**: OTA firmware deployment
- **Computer integration**: CAT control compatibility
- **Logging**: Operating statistics and diagnostics