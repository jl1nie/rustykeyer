# CH32V003/V203 Iambic Keyer Circuit Diagram

**TLP785 Optocoupler Design** - Safe Radio Connection Circuit (Dual Platform Support)

## 📋 Overview

Implementation circuit for iambic keyer supporting both CH32V003 and CH32V203 platforms. By using TLP785 optocoupler, complete electrical isolation from the radio is achieved, providing safe and reliable key control.

### 🎯 Design Philosophy
- **Electrical Isolation**: Complete separation from radio via optocoupler
- **Minimal Configuration**: Minimize external components (7 components)
- **Reliability**: Compatible with standard amateur radio key inputs
- **Expandability**: With sidetone and LED indication

## 🔌 Circuit Diagram

### Complete Circuit
```
                    CH32V003F4P6
                  ┌─────────────┐
    +3.3V ────────┤20 VCC   PD7├─19─┤220Ω├─── Status LED (Red)
                  │              │
    GND ──────────┤10 VSS   PD6├─18─┤1kΩ ├─── Key Output → TLP785
                  │              │
    Dit Paddle ───┤19 PA2   PA1├─17──── PWM Sidetone → Piezo Buzzer
        (10kΩ↑)  │              │
    Dah Paddle ───┤ 9 PA3   PA0├─16
        (10kΩ↑)  │              │
                  └─────────────┘

    Pin Assignment:
    PA1: TIM1_CH1 (PWM Sidetone output)
    PA2: Dit input (Pull-up, EXTI2)
    PA3: Dah input (Pull-up, EXTI3)
    PD6: Key control output → TLP785
    PD7: Status LED
```

### Paddle Input Circuit
```
Dit Paddle Input:
    +3.3V
      │
    10kΩ (Pull-up)
      │
      ├─── PA2 (CH32V003)
      │
    [Dit Paddle] ──── GND

Dah Paddle Input:
    +3.3V
      │
    10kΩ (Pull-up)
      │
      ├─── PA3 (CH32V003)
      │
    [Dah Paddle] ──── GND

Operation:
- Normal: PA2/PA3 = HIGH (3.3V)
- Paddle pressed: PA2/PA3 = LOW (0V)
- EXTI interrupt: Detected on falling edge
```

### TLP785 Key Output Circuit
```
Key Output Circuit:
                        TLP785
                    ┌─────────────┐
PD6 ──1kΩ──── LED-1 │2           5│ ──── Key Tip (to Radio)
              (internal)│           │
GND ─────────── LED-2 │3           4│ ──── Key Ring (to Radio)
                    └─────────────┘
                    
TLP785 Features:
- Isolation voltage: 2500Vrms (complete input-output isolation)
- Switching voltage: 55V max (collector-emitter)
- Switching current: 50mA max (sufficient for radio key input)
- Input forward current: 20mA (typ)
- Response time: 18μs (typ)
- DIP-4 package

Calculation:
Forward voltage VF = 1.2V (typ)
Forward current IF = (3.3V - 1.2V) / 1kΩ = 2.1mA
→ Sufficient drive current secured

Amateur Radio Compatibility:
- Typical radio key input voltage: 5-12V (55V capability provides ample safety margin)
- Typical radio key input current: <10mA (50mA capability provides headroom)
- Isolation: Complete MCU-radio separation ensures safety
```

### Sidetone Circuit
```
Sidetone Circuit:
PA1 (PWM) ────┐
              │  Piezo Buzzer
              │  (600Hz resonant)
              │
GND ──────────┘

Alternative:

PA1 (PWM) ──┤│──1μF──[Speaker 8Ω]──GND
            (DC cut)

Operation:
- PWM frequency: 600Hz (audible tone)
- Duty cycle: 50% (key pressed)
- Duty cycle: 0% (key released)
```

### Status LED
```
Status LED Circuit:
PD7 ─────220Ω──── LED (Red) ──── GND

Operation:
- Key pressed: LED on
- Key released: LED off
- Forward current: (3.3V - 2.0V) / 220Ω = 5.9mA
```

## 🔧 Bill of Materials (BOM)

### Essential Components
| Component | Qty | Part No./Spec | Purpose |
|-----------|-----|---------------|---------|
| **CH32V003F4P6** | 1pc | TSSOP-20 | Main MCU |
| **TLP785** | 1pc | DIP-4 | Optocoupler |
| **Resistor 1kΩ** | 1pc | 1/4W | LED drive limiting |
| **Resistor 10kΩ** | 2pcs | 1/4W | Paddle pull-up |
| **Resistor 220Ω** | 1pc | 1/4W | LED current limiting |
| **LED Red** | 1pc | 3mm | Status indication |

### Optional Components
| Component | Qty | Part No./Spec | Purpose |
|-----------|-----|---------------|---------|
| **Piezo Buzzer** | 1pc | 600Hz resonant | Sidetone |
| **Capacitor 1μF** | 1pc | Ceramic | DC cut |
| **Speaker** | 1pc | 8Ω 0.5W | Sidetone |

### Connection Components
| Component | Qty | Part No./Spec | Purpose |
|-----------|-----|---------------|---------|
| **3.5mm Jack** | 1pc | Stereo | Radio key connection |
| **3.5mm Jack** | 2pcs | Mono | Paddle connection |
| **Power Connector** | 1pc | DC Jack/USB | 3.3V power |

## 🔌 Connection Specifications

### Radio Connection (3.5mm Stereo Jack)
```
Tip: Key line (key signal)
Ring: Usually unused (PTT on some models)
Sleeve: GND (common ground)

Compatible Models:
- ICOM: IC-7300, IC-705, IC-9700
- YAESU: FT-991A, FT-710, FT-DX10
- Kenwood: TS-890S, TS-590SG
- Elecraft: K3S, KX2, KX3
```

### Paddle Connection (3.5mm Mono Jack × 2)
```
Dit Paddle:
  Tip: Dit contact
  Sleeve: GND

Dah Paddle:
  Tip: Dah contact  
  Sleeve: GND

Compatible Paddles:
- Bencher BY-1, BY-2
- Vibroplex Iambic
- Kent Twin Paddle
- N3ZN Keys
```

## 🔧 PCB Layout

### Board Size
```
Recommended: 40mm × 30mm (single-sided)
Thickness: 1.6mm
Material: FR-4
```

### Layout Strategy
```
[Power] ─ [MCU] ─ [Output]
    │        │       │
    └─ [LED] │   [Optocoupler]
            │
        [Paddle Input]
            │
      [Sidetone]
```

### Ground Plane
```
- Maximize copper area for single-sided board
- Place GND plane directly under MCU
- Short traces for RF noise reduction
```

## ⚡ Power Specifications

### Power Requirements
```
Input Voltage: 3.3V ± 5%
Current Consumption: 
  - Standby: 5mA (typ)
  - Key operation: 8mA (typ)
  - Maximum: 15mA (LED + buzzer simultaneous)

Power Options:
1. AC Adapter (3.3V 100mA+)
2. USB power + 3.3V regulator
3. Battery (2×AA + regulator)
```

### Power Regulation
```
Recommended Configuration:
5V (USB) → AMS1117-3.3 → 3.3V
                │
              100μF ─ 0.1μF (bypass)
```

## 🧪 Testing & Adjustment

### Basic Operation Test
```
1. Power on → LED brief flash (startup confirmation)
2. Dit paddle → LED on + key output
3. Dah paddle → LED on + key output  
4. Simultaneous press → alternating operation
5. Sidetone → 600Hz tone confirmation
```

### TLP785 Operation Test
```
Measurement Points:
1. PD6 output voltage: 0V/3.3V switching
2. LED forward current: 2-3mA measurement
3. Output isolation: Isolation between radio GND and board GND
4. Response time: <20μs confirmation (oscilloscope)
```

### Radio Connection Test
```
Pre-connection Check:
1. Verify radio key specifications (voltage/current)
2. Polarity verification (Tip/Ring/Sleeve)
3. Isolation verification (multimeter)

Operation Test:
1. Low power CW transmission test
2. Timing accuracy verification (20WPM reference)
3. Continuous operation stability test
```

## 🔒 Safety Considerations

### Electrical Safety
- **Isolation Verification**: Maintain complete isolation via optocoupler
- **Voltage Verification**: Within radio's allowable voltage/current range
- **Polarity Verification**: Essential polarity check before connection

### RF Safety
- **Grounding**: Separate board ground from radio ground
- **Shielding**: Use metal case if necessary
- **Wiring**: Minimize key line length

## 🔧 CH32V203 Circuit Layout (NEW!)

### 🏆 V203-Specific Pin Assignment

CH32V203 uses different pin assignment:

```
                    CH32V203 (48-pin)
        Pin Assignment:
        PA0: Dit input (Pull-up, EXTI0) ← V003's PA2
        PA1: Dah input (Pull-up, EXTI1) ← V003's PA3  
        PA2: Key control output → TLP785 ← V003's PD6
        PA3: PWM Sidetone output        ← V003's PA1
        
        Status LED: Any GPIO pin (e.g., PC13)
```

### 📊 Platform Comparison

| **Signal** | **CH32V003** | **CH32V203** | **Function** |
|:----------:|:------------:|:------------:|:------------:|
| **Dit Input** | PA2 (EXTI2) | PA0 (EXTI0) | Paddle detection |
| **Dah Input** | PA3 (EXTI3) | PA1 (EXTI1) | Paddle detection |
| **Key Output** | PD6 | PA2 | TLP785 control |
| **PWM Audio** | PA1 (TIM1_CH1) | PA3 (TIM1_CH3) | Sidetone |
| **LED** | PD7 | PC13 (example) | Status indication |

### 🔄 Unified Edge Detection Implementation

Unified operation across both platforms:

```
Common Features:
✅ Both-edge (rising/falling) detection
✅ Complete paddle press/release tracking  
✅ 1ms precision timing control
✅ Complete TLP785 isolation
✅ 600Hz PWM sidetone
```

## 📖 Related Documentation

- **[CH32V003 Implementation Guide](CH32V003_BAREMENTAL_GUIDE_EN.md)** - V003 software implementation details
- **[CH32V203 Implementation Guide](CH32V203_EMBASSY_GUIDE_EN.md)** - V203 software implementation details (future addition)
- **[API Reference](../api/keyer-core-api-en.md)** - Library specifications
- **[Assembly Guide](CH32V003_ASSEMBLY_GUIDE_EN.md)** - Implementation procedures (future addition)

---

## 🎯 Implementation Results

This circuit achieves:
- **Complete Isolation**: Radio protection via optocoupler
- **Minimal Configuration**: Full functionality with 7 components
- **High Reliability**: Professional-grade isolation and protection circuit
- **Cost Efficiency**: V003: ~$5, V203: ~$8
- **Cross-platform**: Optimal choices for different use cases

**Ultimate cost-performance iambic keyer realized with CH32V003/V203 + TLP785**