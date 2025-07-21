#![no_std]
#![no_main]

#[cfg(feature = "defmt")]
use defmt::{debug, info, warn};
#[cfg(feature = "defmt")]
use defmt_rtt as _;
use panic_halt as _;

// Define simple logging macros when defmt is not available
#[cfg(not(feature = "defmt"))]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender, Receiver};

// Mock CH32V003 HAL for Embassy footprint testing
use core::sync::atomic::{AtomicBool, Ordering};

use keyer_core::{*, evaluator_task};
use static_cell::StaticCell;
use heapless::spsc::Queue;

// Static resources
static PADDLE: PaddleInput = PaddleInput::new();
static ELEMENT_QUEUE: StaticCell<Queue<Element, 8>> = StaticCell::new();
static SIDETONE_CHANNEL: Channel<CriticalSectionRawMutex, SidetoneCommand, 8> = Channel::new();

// Mock GPIO types for Embassy footprint testing
struct MockInput {
    state: AtomicBool,
}

struct MockOutput {
    state: AtomicBool,
}

struct MockPwm;

impl MockInput {
    fn new() -> Self {
        Self { state: AtomicBool::new(false) }
    }
    
    fn is_low(&self) -> bool {
        !self.state.load(Ordering::Relaxed)
    }
}

impl MockOutput {
    fn new() -> Self {
        Self { state: AtomicBool::new(false) }
    }
    
    fn set_high(&self) {
        self.state.store(true, Ordering::Relaxed);
    }
    
    fn set_low(&self) {
        self.state.store(false, Ordering::Relaxed);
    }
    
    fn is_set_high(&self) -> bool {
        self.state.load(Ordering::Relaxed)
    }
}

impl MockPwm {
    fn new() -> Self { Self }
    fn set_duty(&mut self, _duty: u16) {}
    fn enable(&mut self) {}
    fn get_max_duty(&self) -> u16 { 1000 }
    fn set_frequency(&mut self, _freq: u32) {}
}

/// CH32V003 Hardware implementation (Mock for footprint testing)
struct Ch32v003Hal {
    dit_input: MockInput,
    dah_input: MockInput,
    key_output: MockOutput,
    status_led: MockOutput,
    sidetone_sender: Sender<'static, CriticalSectionRawMutex, SidetoneCommand, 8>,
}

impl Ch32v003Hal {
    fn new(
        dit_input: MockInput,
        dah_input: MockInput,
        key_output: MockOutput,
        status_led: MockOutput,
        sidetone_sender: Sender<'static, CriticalSectionRawMutex, SidetoneCommand, 8>,
    ) -> Self {
        Self {
            dit_input,
            dah_input,
            key_output,
            status_led,
            sidetone_sender,
        }
    }
}

impl InputPaddle for Ch32v003Hal {
    type Error = HalError;

    fn is_pressed(&mut self) -> Result<bool, Self::Error> {
        // Mock implementation - Dit and Dah are active low
        Ok(self.dit_input.is_low() || self.dah_input.is_low())
    }

    fn last_edge_time(&self) -> Option<crate::hal::Instant> {
        // Edge time tracking implemented via EXTI interrupts
        None
    }

    fn set_debounce_time(&mut self, _time_ms: u32) -> Result<(), Self::Error> {
        // Debounce handled in software
        Ok(())
    }

    fn enable_interrupt(&mut self) -> Result<(), Self::Error> {
        // EXTI interrupts enabled in main
        Ok(())
    }

    fn disable_interrupt(&mut self) -> Result<(), Self::Error> {
        // EXTI interrupts managed in main
        Ok(())
    }
}

impl OutputKey for Ch32v003Hal {
    type Error = HalError;

    fn set_state(&mut self, state: bool) -> Result<(), Self::Error> {
        if state {
            self.key_output.set_high();
            self.status_led.set_high();
            // Send sidetone on command
            if let Err(_) = self.sidetone_sender.try_send(SidetoneCommand::On) {
                #[cfg(feature = "defmt")]
            warn!("Sidetone channel full");
            }
        } else {
            self.key_output.set_low();
            self.status_led.set_low();
            // Send sidetone off command
            if let Err(_) = self.sidetone_sender.try_send(SidetoneCommand::Off) {
                #[cfg(feature = "defmt")]
            warn!("Sidetone channel full");
            }
        }
        Ok(())
    }

    fn get_state(&self) -> Result<bool, Self::Error> {
        Ok(self.key_output.is_set_high())
    }
}

/// Sidetone control commands
#[derive(Debug, Copy, Clone)]
enum SidetoneCommand {
    On,
    Off,
    SetFrequency(u16),
}

/// Main firmware entry point
#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    #[cfg(feature = "defmt")]
    info!("🔧 Rusty Keyer CH32V003 Starting...");

    // Mock CH32V003 initialization for Embassy footprint testing
    #[cfg(feature = "defmt")]
    info!("⚡ CH32V003 Mock initialized");

    // Mock GPIO configuration
    let dit_input = MockInput::new();
    let dah_input = MockInput::new();
    let key_output = MockOutput::new();
    let status_led = MockOutput::new();

    #[cfg(feature = "defmt")]
    info!("🔌 Mock GPIO configured: Dit=PA2, Dah=PA3, Key=PD6, LED=PD7");

    // Mock PWM for sidetone
    let pwm = MockPwm::new();

    #[cfg(feature = "defmt")]
    info!("🎵 Mock PWM configured for sidetone");

    // Initialize keyer configuration
    let keyer_config = KeyerConfig {
        mode: KeyerMode::SuperKeyer,
        char_space_enabled: true,
        unit: Duration::from_millis(60), // 20 WPM
        debounce_ms: 5,
        queue_size: 8,
    };

    #[cfg(feature = "defmt")]
    info!("⚙️ Keyer config: {:?} WPM, Mode: {:?}", 
          keyer_config.wpm(), keyer_config.mode);

    // Initialize element queue
    let queue = ELEMENT_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();

    // Get sidetone channel endpoints
    let sidetone_sender = SIDETONE_CHANNEL.sender();
    let sidetone_receiver = SIDETONE_CHANNEL.receiver();

    // Create HAL instance
    let hal = Ch32v003Hal::new(
        dit_input,
        dah_input, 
        key_output,
        status_led,
        sidetone_sender,
    );

    #[cfg(feature = "defmt")]
    info!("🚀 Spawning keyer tasks...");

    // Spawn keyer tasks
    spawner.spawn(paddle_monitor_task()).unwrap();
    spawner.spawn(evaluator_task_ch32(&PADDLE, producer, keyer_config)).unwrap();
    spawner.spawn(sender_task_ch32(consumer, keyer_config.unit)).unwrap();
    spawner.spawn(sidetone_task(pwm, sidetone_receiver)).unwrap();

    #[cfg(feature = "defmt")]
    info!("✨ Keyer firmware ready!");

    // Main supervision loop
    loop {
        Timer::after(Duration::from_secs(10)).await;
        #[cfg(feature = "defmt")]
        info!("💓 Heartbeat - System running");
    }
}

/// Paddle monitoring task using EXTI interrupts
#[embassy_executor::task]
async fn paddle_monitor_task() {
    info!("🎮 Paddle monitor task started");
    
    loop {
        // Wait for EXTI interrupt (simulated with timer for now)
        Timer::after(Duration::from_millis(1)).await;
        
        // Update paddle state in atomic structure
        // This will be implemented with actual EXTI handlers
    }
}

/// Evaluator task wrapper for CH32V003
#[embassy_executor::task]
async fn evaluator_task_ch32(
    paddle: &'static PaddleInput,
    producer: heapless::spsc::Producer<'static, Element, 8>,
    config: KeyerConfig,
) {
    info!("🧠 Evaluator task started");
    evaluator_task::<8>(paddle, producer, config).await;
}

/// Sender task for CH32V003
#[embassy_executor::task]
async fn sender_task_ch32(
    mut consumer: heapless::spsc::Consumer<'static, Element, 8>,
    unit: Duration,
) {
    info!("📤 Sender task started");

    loop {
        if let Some(element) = consumer.dequeue() {
            let (on_time, element_name) = match element {
                Element::Dit => (unit, "Dit"),
                Element::Dah => (unit * 3, "Dah"),
                Element::CharSpace => (Duration::from_millis(0), "Space"),
            };

            if element.is_keyed() {
                debug!("📡 Sending {}", element_name);
                
                // Key timing handled by HAL
                Timer::after(on_time).await;
                Timer::after(unit).await; // Inter-element space
            } else {
                // Character space - just wait
                debug!("⏸️ Character space");
                Timer::after(unit * 3).await;
            }
        } else {
            // No elements in queue, brief pause
            Timer::after(unit / 8).await;
        }
    }
}

/// Sidetone generation task (Mock implementation)
#[embassy_executor::task]
async fn sidetone_task(
    mut pwm: MockPwm,
    receiver: Receiver<'static, CriticalSectionRawMutex, SidetoneCommand, 8>,
) {
    info!("🔊 Mock Sidetone task started (600Hz default)");
    
    // Mock PWM initialization
    pwm.set_duty(0);
    pwm.enable();

    loop {
        match receiver.receive().await {
            SidetoneCommand::On => {
                // Mock PWM on
                let max_duty = pwm.get_max_duty();
                pwm.set_duty(max_duty / 2);
                debug!("🔊 Mock Sidetone ON");
            }
            SidetoneCommand::Off => {
                // Mock PWM off
                pwm.set_duty(0);
                debug!("🔇 Mock Sidetone OFF");
            }
            SidetoneCommand::SetFrequency(freq) => {
                // Mock frequency change
                pwm.set_frequency(freq as u32);
                info!("🎵 Mock Sidetone frequency set to {}Hz", freq);
            }
        }
    }
}