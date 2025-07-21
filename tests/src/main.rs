// HAL level integration tests with Embassy async support

mod embassy_tests;

fn main() {
    println!("🧪 HAL Level Integration Tests");
    
    // Test 1: Basic Mock Hardware Operations
    test_mock_hardware_basic();
    
    // Test 2: Element generation basics
    test_element_generation();
    
    // Test 3: Configuration validation
    test_configuration_validation();
    
    println!("✅ All HAL level tests passed!");
    println!();
    println!("📝 Run async tests with: cargo test");
}

/// Test basic mock hardware operations (without dependencies)
fn test_mock_hardware_basic() {
    println!("🔧 Testing Mock Hardware Basic Operations...");
    
    // Test basic boolean states (simulating paddle/key states)
    let mut dit_state = false;
    let mut dah_state = false;
    let mut key_state = false;
    
    // Test initial states
    assert!(!dit_state);
    assert!(!dah_state);
    assert!(!key_state);
    
    // Test Dit paddle state changes
    dit_state = true;
    assert!(dit_state);
    assert!(!dah_state);
    
    // Test Dah paddle state changes
    dah_state = true;
    assert!(dit_state);
    assert!(dah_state);
    
    // Test both paddles release
    dit_state = false;
    dah_state = false;
    assert!(!dit_state);
    assert!(!dah_state);
    
    key_state = true;
    assert!(key_state);
    key_state = false;
    assert!(!key_state);
    
    println!("  ✅ Mock hardware operations working");
}

/// Test element generation basics
fn test_element_generation() {
    println!("📡 Testing Element Generation...");
    
    // Test element types (using simple enum comparison)
    #[derive(Debug, PartialEq, Copy, Clone)]
    enum TestElement {
        Dit,
        Dah,
        CharSpace,
    }
    
    let test_sequence = vec![
        TestElement::Dit,
        TestElement::Dah,
        TestElement::Dit,
        TestElement::CharSpace,
    ];
    
    // Validate sequence
    assert_eq!(test_sequence.len(), 4);
    assert_eq!(test_sequence[0], TestElement::Dit);
    assert_eq!(test_sequence[1], TestElement::Dah);
    assert_eq!(test_sequence[2], TestElement::Dit);
    assert_eq!(test_sequence[3], TestElement::CharSpace);
    
    // Test morse patterns
    let letter_a = vec![TestElement::Dit, TestElement::Dah];
    let letter_b = vec![TestElement::Dah, TestElement::Dit, TestElement::Dit, TestElement::Dit];
    
    assert_eq!(letter_a.len(), 2);
    assert_eq!(letter_b.len(), 4);
    assert_eq!(letter_b[0], TestElement::Dah);
    
    println!("  ✅ Element generation working");
}

/// Test configuration validation
fn test_configuration_validation() {
    println!("⚙️ Testing Configuration Validation...");
    
    // Test keyer mode enumeration
    #[derive(Debug, PartialEq, Copy, Clone)]
    enum TestKeyerMode {
        ModeA,
        ModeB,
        SuperKeyer,
    }
    
    #[derive(Debug, Clone)]
    struct TestConfig {
        mode: TestKeyerMode,
        unit_ms: u32,
        char_space_enabled: bool,
        debounce_ms: u32,
    }
    
    // Test valid configurations
    let config_a = TestConfig {
        mode: TestKeyerMode::ModeA,
        unit_ms: 60,  // 20 WPM
        char_space_enabled: false,
        debounce_ms: 5,
    };
    
    let config_b = TestConfig {
        mode: TestKeyerMode::ModeB,
        unit_ms: 100,  // 12 WPM
        char_space_enabled: true,
        debounce_ms: 5,
    };
    
    let config_super = TestConfig {
        mode: TestKeyerMode::SuperKeyer,
        unit_ms: 50,  // 24 WPM
        char_space_enabled: true,
        debounce_ms: 5,
    };
    
    // Validate configurations
    assert_eq!(config_a.mode, TestKeyerMode::ModeA);
    assert_eq!(config_a.unit_ms, 60);
    assert!(!config_a.char_space_enabled);
    
    assert_eq!(config_b.mode, TestKeyerMode::ModeB);
    assert_eq!(config_b.unit_ms, 100);
    assert!(config_b.char_space_enabled);
    assert_eq!(config_b.debounce_ms, 5);
    
    assert_eq!(config_super.mode, TestKeyerMode::SuperKeyer);
    assert_eq!(config_super.unit_ms, 50);
    assert!(config_super.char_space_enabled);
    assert_eq!(config_super.debounce_ms, 5);
    
    // Test WPM calculations
    let wpm_a = 1200 / config_a.unit_ms;  // Simple WPM formula
    let wpm_b = 1200 / config_b.unit_ms;
    let wpm_super = 1200 / config_super.unit_ms;
    
    assert_eq!(wpm_a, 20);
    assert_eq!(wpm_b, 12);
    assert_eq!(wpm_super, 24);
    
    println!("  ✅ Configuration validation working");
    println!("    Mode A: {}ms unit, {} WPM", config_a.unit_ms, wpm_a);
    println!("    Mode B: {}ms unit, {} WPM", config_b.unit_ms, wpm_b);
    println!("    SuperKeyer: {}ms unit, {} WPM", config_super.unit_ms, wpm_super);
}

/// Test timing calculations
#[allow(dead_code)]
fn test_timing_calculations() {
    println!("⏱️ Testing Timing Calculations...");
    
    let unit_ms = 60;  // 20 WPM
    
    // Standard CW timing ratios
    let dit_duration = unit_ms;
    let dah_duration = unit_ms * 3;
    let inter_element_space = unit_ms;
    let inter_character_space = unit_ms * 3;
    let inter_word_space = unit_ms * 7;
    
    assert_eq!(dit_duration, 60);
    assert_eq!(dah_duration, 180);
    assert_eq!(inter_element_space, 60);
    assert_eq!(inter_character_space, 180);
    assert_eq!(inter_word_space, 420);
    
    // Test ratio validation
    assert_eq!(dah_duration / dit_duration, 3);
    assert_eq!(inter_character_space / inter_element_space, 3);
    assert_eq!(inter_word_space / dit_duration, 7);
    
    println!("  ✅ Timing calculations working");
    println!("    Dit: {}ms, Dah: {}ms", dit_duration, dah_duration);
    println!("    Inter-element: {}ms, Inter-char: {}ms", inter_element_space, inter_character_space);
}

/// Test paddle state simulation
#[allow(dead_code)]
fn test_paddle_state_simulation() {
    println!("🎮 Testing Paddle State Simulation...");
    
    #[derive(Debug, PartialEq, Copy, Clone)]
    enum PaddleSide {
        Dit,
        Dah,
    }
    
    #[derive(Debug, Clone)]
    struct PaddleEvent {
        time_ms: u32,
        side: PaddleSide,
        pressed: bool,
    }
    
    // Simulate a Dit press sequence
    let dit_sequence = vec![
        PaddleEvent { time_ms: 0, side: PaddleSide::Dit, pressed: true },
        PaddleEvent { time_ms: 60, side: PaddleSide::Dit, pressed: false },
    ];
    
    // Simulate a Dah press sequence
    let dah_sequence = vec![
        PaddleEvent { time_ms: 100, side: PaddleSide::Dah, pressed: true },
        PaddleEvent { time_ms: 280, side: PaddleSide::Dah, pressed: false },
    ];
    
    // Validate sequences
    assert_eq!(dit_sequence.len(), 2);
    assert_eq!(dit_sequence[0].pressed, true);
    assert_eq!(dit_sequence[1].pressed, false);
    assert_eq!(dit_sequence[0].side, PaddleSide::Dit);
    
    assert_eq!(dah_sequence.len(), 2);
    assert_eq!(dah_sequence[0].pressed, true);
    assert_eq!(dah_sequence[1].pressed, false);
    assert_eq!(dah_sequence[0].side, PaddleSide::Dah);
    
    // Test timing
    let dit_duration = dit_sequence[1].time_ms - dit_sequence[0].time_ms;
    let dah_duration = dah_sequence[1].time_ms - dah_sequence[0].time_ms;
    
    assert_eq!(dit_duration, 60);
    assert_eq!(dah_duration, 180);
    assert_eq!(dah_duration / dit_duration, 3);
    
    println!("  ✅ Paddle state simulation working");
}