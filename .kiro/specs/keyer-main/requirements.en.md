# 📟 Iambic Keyer Technical Specification

## 1. Overview
This specification defines the design and operational specifications for an iambic keyer using Rust + Embassy. Through a structure emphasizing real-time performance, memory efficiency, mode extensibility, and high responsiveness, it realizes CW control suitable for amateur radio operation and embedded communication.

Supported Modes:
- **Mode A**: Simple iambic squeeze transmission
- **Mode B**: Transmits opposite element once on squeeze release (Accu-Keyer style)
- **SuperKeyer**: Dah priority + press history control

Auxiliary Functions:
- FSM (Finite State Machine) for state transition management
- Element output via GPIO + timer control
- CharSpace (inter-character space) control (toggle support)

---

## 2. Input Management and Interrupts
- Paddle presses (Dit/Dah) are detected via GPIO edge interrupts (ISR)
- ISR does not manipulate transmission queue, only records press state and timestamp
- State is passed asynchronously to evaluator task

---

## 3. FSM Design
```rust
enum FSMState {
  Idle,
  DitHold,
  DahHold,
  Squeeze(Element),
  MemoryPending(Element),
  CharSpacePending(Instant),
}
```

- State transitions updated every unit/4 cycle
- Squeeze: Alternating transmission on simultaneous Dit+Dah press
- MemoryPending: Transmits opposite element after release in ModeB/SK modes
- CharSpacePending: All release → wait unit×3 → start next transmission

## 4. SuperKeyerController Structure
```rust
struct SuperKeyerController {
  dit_time: Option<Instant>,
  dah_time: Option<Instant>,
}
```
- Dah priority judgment: Compare by press timestamp
- Memory transmission: Insert opposite element after squeeze release

---

## 5. Timing Requirements

### Basic Timing
- **Unit Duration**: 60ms (20 WPM) to 17ms (70 WPM)
- **Dit**: 1 unit on + 1 unit off
- **Dah**: 3 units on + 1 unit off
- **Character Space**: 3 units off (after all paddles released)

### Update Frequencies
- **FSM Update**: Every unit/4 (15ms @ 20WPM)
- **Interrupt Response**: < 10μs
- **ISR Execution**: < 5μs

---

## 6. Mode Specifications

### Mode A (Basic Iambic)
- Single element transmission on paddle press
- Alternating transmission on squeeze (Dit+Dah simultaneous)
- Immediate stop on paddle release
- No memory function

### Mode B (Curtis A Compatible)
- All Mode A functionality
- One-element memory: Transmits opposite element once after squeeze release
- Compatible with commercial Accu-Keyer behavior

### SuperKeyer (Advanced)
- **Dah Priority**: On simultaneous press, Dah takes priority regardless of timing
- **Advanced Memory**: Based on press history and timing
- **Timestamp Control**: Uses precise timing for priority determination

---

## 7. Hardware Interface

### GPIO Configuration
```
PA0: Dit Paddle Input  (Pull-up, EXTI0)
PA1: Dah Paddle Input  (Pull-up, EXTI1)
PA2: Key Output        (Push-pull)
PA3: Sidetone Output   (Optional)
```

### Timing Requirements
- Debounce: 5-15ms (configurable)
- Key Output: Active HIGH or LOW (configurable)
- Pull-up resistors: 10kΩ internal

---

## 8. Performance Requirements

### Memory Constraints
- Total RAM usage: < 2KB
- Stack usage: < 512 bytes per task
- Queue size: 32-64 elements (configurable)

### Real-time Constraints
- Interrupt latency: < 10μs
- State update jitter: < ±1%
- Key output timing accuracy: ±1%

---

## 9. Configuration Parameters

```rust
struct KeyerConfig {
    mode: KeyerMode,
    unit: Duration,
    char_space_enabled: bool,
    debounce_ms: u8,
    queue_size: usize,
}
```

### Default Values
- Mode: ModeB
- Unit: 60ms (20 WPM)
- Character space: Enabled
- Debounce: 10ms
- Queue size: 64

---

## 10. Error Handling

### Input Validation
- Unit duration: 17ms - 200ms
- Debounce: 1ms - 50ms
- Queue size: 8 - 256

### Fault Recovery
- Queue overflow: Drop oldest elements
- Timing drift: Auto-correction via Embassy time
- Hardware fault: Graceful degradation

---

## 11. Testing Requirements

### Unit Tests
- FSM state transitions
- Timing accuracy
- Mode behavior verification
- Configuration validation

### Integration Tests
- Hardware simulation
- Real-time performance
- Memory usage validation
- Interrupt behavior

---

## 12. Compliance and Standards

### Amateur Radio Standards
- ITU-R M.1172 (Morse code timing)
- ARRL contest rules compliance
- Commercial keyer compatibility (Mode B)

### Embedded Standards
- Real-time constraints
- Memory efficiency
- Power consumption optimization
- Hardware abstraction