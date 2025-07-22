#![no_std]
#![no_main]

#[cfg(feature = "defmt")]
use defmt_rtt as _;

// RISC-V runtime
use riscv_rt as _;

// Panic handler
use panic_halt as _;

use embassy_executor::Spawner;
use embassy_time::Duration;
use heapless::spsc::Queue;
use static_cell::StaticCell;

use keyer_core::{*, evaluator_task};
use rustykeyer_firmware::*;

// Static resources
static PADDLE: PaddleInput = PaddleInput::new();
static KEY_QUEUE: StaticCell<Queue<Element, 8>> = StaticCell::new();

/// Main firmware entry point
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    #[cfg(feature = "defmt")]
    defmt::info!("🔧 Rusty Keyer Firmware Starting...");

    // Initialize CH32V203 hardware
    let _hal = init_hardware().await;
    #[cfg(feature = "defmt")]
    defmt::info!("✅ Hardware initialized");

    // Initialize keyer configuration - optimized for RAM
    let config = KeyerConfig {
        mode: KeyerMode::ModeA,  // Unified to ModeA for compatibility
        char_space_enabled: true,
        unit: Duration::from_millis(60), // 20 WPM
        debounce_ms: 10, // Unified 10ms debounce for noise immunity
        queue_size: 8,  // Match actual queue size
    };
    #[cfg(feature = "defmt")]
    defmt::info!("⚙️ Keyer config: {:?} WPM, Mode: {:?}", 
                config.wpm(), config.mode);

    // Initialize element queue
    let queue = KEY_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();

    // Spawn keyer tasks
    #[cfg(feature = "defmt")]
    defmt::info!("🚀 Spawning keyer tasks...");
    
    spawner.spawn(evaluator_task_spawn(&PADDLE, producer, config)).unwrap();
    spawner.spawn(sender_task(consumer, config.unit)).unwrap();

    #[cfg(feature = "defmt")]
    defmt::info!("✨ Keyer firmware ready!");

    // Main supervision loop - optimized for minimal resources
    loop {
        embassy_time::Timer::after(Duration::from_secs(10)).await;
        // Heartbeat removed for RAM optimization
    }
}

/// Evaluator task wrapper for CH32V203
#[embassy_executor::task]
async fn evaluator_task_spawn(
    paddle: &'static PaddleInput,
    producer: heapless::spsc::Producer<'static, Element, 8>,
    config: KeyerConfig,
) {
    #[cfg(feature = "defmt")]
    defmt::info!("🧠 Evaluator task started");
    evaluator_task::<8>(paddle, producer, config).await;
}

/// Initialize hardware abstraction layer
async fn init_hardware() -> MockKeyerHal {
    #[cfg(feature = "defmt")]
    defmt::info!("🔌 Initializing hardware...");
    
    // For now, use mock hardware for compilation
    // Real CH32V implementation will replace this
    MockKeyerHal::new()
}


/// Sender task for key output (local implementation)
#[embassy_executor::task]
async fn sender_task(
    mut consumer: heapless::spsc::Consumer<'static, Element, 8>,
    unit: Duration,
) {
    #[cfg(feature = "defmt")]
    defmt::info!("📤 Sender task started");
    // Use actual CH32V203 key output (through HAL)
    // Note: KeyOutput will be handled by HAL instance

    loop {
        if let Some(element) = consumer.dequeue() {
            let (on_time, element_name) = match element {
                Element::Dit => (unit, "Dit"),
                Element::Dah => (unit * 3, "Dah"),
                Element::CharSpace => (Duration::from_millis(0), "Space"),
            };

            if element.is_keyed() {
                #[cfg(feature = "defmt")]
                defmt::debug!("📡 Sending {}", element_name);
                
                // Key down - TODO: Access HAL instance for actual output
                // hal.set_key_output(true);
                embassy_time::Timer::after(on_time).await;
                
                // Key up
                // hal.set_key_output(false);
                
                // Inter-element space (except for CharSpace)
                embassy_time::Timer::after(unit).await;
            } else {
                // Character space - just wait
                #[cfg(feature = "defmt")]
                defmt::debug!("⏸️ Character space");
                embassy_time::Timer::after(unit * 3).await;
            }
        } else {
            // No elements in queue, brief pause
            embassy_time::Timer::after(unit / 8).await;
        }
    }
}