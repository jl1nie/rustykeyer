# 📖 Rusty Keyer Documentation

**Comprehensive Documentation** - From design to implementation of Rust iambic keyer

## 📋 Documentation Structure

### 🚀 Main Documents
- **[README.md](../README.en.md)** - Project overview & quick start
- **[Kiro Specifications](../.kiro/specs/keyer-main/)** - Requirements, design & implementation status

### 🔌 Hardware Implementation
- **[CH32V003 Bare Metal Implementation Guide](hardware/CH32V003_BAREMENTAL_GUIDE_EN.md)** 📍 **Must Read**
  - Complete implementation in 1KB Flash / 2KB RAM
  - Direct register control, interrupt & PWM details
  - Memory allocation, performance measurement & hardware preparation
- **[CH32V003 Implementation Guide (Japanese)](hardware/CH32V003_BAREMENTAL_GUIDE.md)**
- **[Circuit Diagram with TLP785 Design](hardware/CH32V003_CIRCUIT_DIAGRAM_EN.md)** - Optocoupler safe connection
- **[Circuit Diagram with TLP785 (Japanese)](hardware/CH32V003_CIRCUIT_DIAGRAM.md)**

### 🦀 API Specifications
- **[keyer-core API Reference](api/keyer-core-api-en.md)** - Complete library specification
- **[keyer-core API Reference (Japanese)](api/keyer-core-api.md)** - コアライブラリ完全仕様
- **[HAL Abstraction Details](api/keyer-core-api-en.md#🔌-hardware-abstraction-layer-hal)** - Hardware abstraction layer

### 📋 Development Records
- **[Session Records](archive/)** - Detailed implementation process records
- **[Memory Analysis](archive/)** - Footprint optimization process
- **[Test Reports](archive/)** - 21 test verification results

## 🎯 Level-Based Guide

### 🔰 For Beginners
1. **[README.md](../README.en.md)** - Overview understanding
2. **[Requirements Spec](../.kiro/specs/keyer-main/requirements.en.md)** - Feature understanding  
3. **[Quick Start](../README.en.md#🚀-quick-start)** - Build & test

### ⚡ For Implementers
1. **[Technical Design](../.kiro/specs/keyer-main/design.en.md)** - Architecture understanding
2. **[CH32V003 Guide](hardware/CH32V003_BAREMENTAL_GUIDE_EN.md)** - Detailed implementation
3. **[API Specification](api/keyer-core-api-en.md)** - Library utilization

### 🎛️ For Advanced Users & Customization
1. **[HAL Abstraction Design](api/keyer-core-api-en.md)** - Portability support
2. **[Session Records](archive/)** - Optimization technique learning
3. **[FSM Details](api/keyer-core-api-en.md)** - SuperKeyer extension

## 🌟 Featured Documents

### 📍 **CH32V003 Bare Metal Implementation Guide**
**Ultimate optimization implementation** - This implementation achieves:

- **Flash Usage**: 1,070B (73% below 4KB target)
- **RAM Usage**: 2,048B (perfect fit to 2KB constraint)
- **Features**: Full iambic keyer functionality (3 modes)
- **Performance**: 1ms real-time control, 21 tests passed

**Technical Significance**: Practical example of extreme optimization in Rust embedded development

### 📊 Implementation Results Summary

| Phase | Duration | Major Results | Documentation |
|-------|----------|---------------|---------------|
| **Phase 1** | 1 day | Embassy implementation, HAL design | [Design Document](../.kiro/specs/keyer-main/design.en.md) |
| **Phase 2** | 1 day | Test integration, 21 tests passed | [Test Reports](archive/) |  
| **Phase 3** | 1 day | Bare metal implementation, extreme optimization | [Implementation Guide](hardware/CH32V003_BAREMENTAL_GUIDE_EN.md) |

**Total Development**: 3 days  
**Tests Passed**: 21/21 (100%)  
**Supported MCUs**: 2 chips (CH32V003/V203)  
**Implementation Methods**: 2 types (Embassy/Bare Metal)

## 🔧 How to Use

### Project Participants
- Implementation: [Technical Design](../.kiro/specs/keyer-main/design.en.md) → [Implementation Guide](hardware/CH32V003_BAREMENTAL_GUIDE_EN.md)
- Testing: [Test Reports](archive/) → `cargo test` execution
- Customization: [API Specification](api/keyer-core-api-en.md) → Core feature extension

### Learners & Researchers
- Rust Embedded Learning: [Bare Metal Implementation](hardware/CH32V003_BAREMENTAL_GUIDE_EN.md)
- Memory Optimization Techniques: [Analysis Reports](archive/)
- RISC-V Usage Examples: [Session Records](archive/)

### Commercial Use Consideration
- Commercialization Feasibility: [Implementation Guide](hardware/CH32V003_BAREMENTAL_GUIDE_EN.md#🚀-commercialization-potential)
- Cost Analysis: Hardware specifications & bill of materials
- Technology Transfer: Open source license (MIT)

---

## 📞 Support

- **GitHub Issues**: Technical questions & bug reports
- **Documentation**: `cargo doc --open --package keyer-core`
- **Community**: Amateur radio & Rust embedded communities

**This is a project systematically developed through Kiro Spec-Driven Development.**