// Temporarily enable std for compilation testing
// Will be changed to #![no_std] #![no_main] for actual embedded target

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_time::Duration;
use heapless::spsc::Queue;
use static_cell::StaticCell;

use keyer_core::*;

// Static resources
static PADDLE: PaddleInput = PaddleInput::new();
static KEY_QUEUE: StaticCell<Queue<Element, 64>> = StaticCell::new();

/// Main firmware entry point
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    defmt::info!("🔧 Rusty Keyer Firmware Starting...");

    // Initialize hardware (placeholder for now)
    let _hal = init_hardware().await;
    defmt::info!("✅ Hardware initialized");

    // Initialize keyer configuration
    let config = KeyerConfig {
        mode: KeyerMode::SuperKeyer,
        char_space_enabled: true,
        unit: Duration::from_millis(60), // 20 WPM
        debounce_ms: 10,
        queue_size: 64,
    };
    defmt::info!("⚙️ Keyer config: {:?} WPM, Mode: {:?}", 
                config.wpm(), config.mode);

    // Initialize element queue
    let queue = KEY_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();

    // Spawn keyer tasks
    defmt::info!("🚀 Spawning keyer tasks...");
    
    spawner.must_spawn(evaluator_task_wrapper(&PADDLE, producer, config));
    spawner.must_spawn(sender_task(consumer, config.unit));

    defmt::info!("✨ Keyer firmware ready!");

    // Main supervision loop
    loop {
        embassy_time::Timer::after(Duration::from_secs(1)).await;
        defmt::trace!("💓 Heartbeat");
    }
}

/// Initialize hardware abstraction layer
async fn init_hardware() -> MockKeyerHal {
    defmt::info!("🔌 Initializing hardware...");
    
    // For now, use mock hardware for compilation
    // Real CH32V implementation will replace this
    MockKeyerHal::new()
}

/// Mock hardware implementation for testing
struct MockKeyerHal {
    dit_paddle: MockPaddle,
    dah_paddle: MockPaddle,
    key_output: MockKeyOutput,
}

impl MockKeyerHal {
    fn new() -> Self {
        defmt::info!("🧪 Using mock hardware (for testing)");
        Self {
            dit_paddle: MockPaddle::new(),
            dah_paddle: MockPaddle::new(), 
            key_output: MockKeyOutput::new(),
        }
    }
}

/// Mock paddle implementation
struct MockPaddle {
    pressed: bool,
}

impl MockPaddle {
    fn new() -> Self {
        Self { pressed: false }
    }
}

impl InputPaddle for MockPaddle {
    type Error = HalError;

    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        Ok(self.pressed)
    }

    fn last_edge_time(&self) -> Option<crate::hal::Instant> {
        None
    }

    fn set_debounce_time(&mut self, _time_ms: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Mock key output implementation
struct MockKeyOutput {
    state: bool,
}

impl MockKeyOutput {
    fn new() -> Self {
        Self { state: false }
    }
}

impl OutputKey for MockKeyOutput {
    type Error = HalError;

    fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
        if state != self.state {
            defmt::info!("🔑 Key: {}", if state { "DOWN" } else { "UP" });
            self.state = state;
        }
        Ok(())
    }

    fn get_state(&self) -> Result<bool, Self::Error> {
        Ok(self.state)
    }
}

/// Evaluator task wrapper
#[embassy_executor::task]
async fn evaluator_task_wrapper(
    paddle: &'static PaddleInput,
    producer: heapless::spsc::Producer<'static, Element, 64>,
    config: KeyerConfig,
) {
    defmt::info!("🧠 Evaluator task started");
    evaluator_task(paddle, producer, config).await;
}

/// Sender task for key output
#[embassy_executor::task]
async fn sender_task(
    mut consumer: heapless::spsc::Consumer<'static, Element, 64>,
    unit: Duration,
) {
    defmt::info!("📤 Sender task started");
    let mut key_output = MockKeyOutput::new();

    loop {
        if let Some(element) = consumer.dequeue() {
            let (on_time, element_name) = match element {
                Element::Dit => (unit, "Dit"),
                Element::Dah => (unit * 3, "Dah"),
                Element::CharSpace => (Duration::from_millis(0), "Space"),
            };

            if element.is_keyed() {
                defmt::debug!("📡 Sending {}", element_name);
                
                // Key down
                key_output.set_state(true).ok();
                embassy_time::Timer::after(on_time).await;
                
                // Key up
                key_output.set_state(false).ok();
                
                // Inter-element space (except for CharSpace)
                embassy_time::Timer::after(unit).await;
            } else {
                // Character space - just wait
                defmt::debug!("⏸️ Character space");
                embassy_time::Timer::after(unit * 3).await;
            }
        } else {
            // No elements in queue, brief pause
            embassy_time::Timer::after(unit / 8).await;
        }
    }
}