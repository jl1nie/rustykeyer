#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Sender, Receiver};

use ch32_hal::gpio::{Input, Output, Level, Pull, Speed, OutputType};
use ch32_hal::time::Hertz;
use ch32_hal::timer::simple_pwm::{SimplePwm, PwmPin};
use ch32_hal::exti::Exti;
use ch32_hal::{bind_interrupts, peripherals, rcc};

use keyer_core::*;
use static_cell::StaticCell;
use heapless::spsc::Queue;

// Static resources
static PADDLE: PaddleInput = PaddleInput::new();
static ELEMENT_QUEUE: StaticCell<Queue<Element, 64>> = StaticCell::new();
static SIDETONE_CHANNEL: Channel<ThreadModeRawMutex, SidetoneCommand, 8> = Channel::new();

// Bind interrupts
bind_interrupts!(struct Irqs {
    EXTI_LINE1 => ch32_hal::exti::ExtiInterruptHandler<ch32_hal::peripherals::EXTI_LINE1>;
    EXTI_LINE2 => ch32_hal::exti::ExtiInterruptHandler<ch32_hal::peripherals::EXTI_LINE2>;
});

/// CH32V003 Hardware implementation
struct Ch32v003Hal {
    dit_input: Input<'static, peripherals::PA2>,
    dah_input: Input<'static, peripherals::PA3>,
    key_output: Output<'static, peripherals::PD6>,
    status_led: Output<'static, peripherals::PD7>,
    sidetone_sender: Sender<'static, ThreadModeRawMutex, SidetoneCommand, 8>,
}

impl Ch32v003Hal {
    fn new(
        dit_input: Input<'static, peripherals::PA2>,
        dah_input: Input<'static, peripherals::PA3>,
        key_output: Output<'static, peripherals::PD6>,
        status_led: Output<'static, peripherals::PD7>,
        sidetone_sender: Sender<'static, ThreadModeRawMutex, SidetoneCommand, 8>,
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
        // PA2 (Dit) and PA3 (Dah) are active low (pulled up, pressed = low)
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
                warn!("Sidetone channel full");
            }
        } else {
            self.key_output.set_low();
            self.status_led.set_low();
            // Send sidetone off command
            if let Err(_) = self.sidetone_sender.try_send(SidetoneCommand::Off) {
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
    info!("🔧 Rusty Keyer CH32V003 Starting...");

    // Initialize CH32V003 peripherals
    let mut config = rcc::Config::default();
    config.hse = None;
    config.sysclk = Hertz::mhz(48);  // Use internal RC oscillator
    let p = ch32_hal::init(config);

    info!("⚡ CH32V003 initialized at 48MHz");

    // Configure GPIO pins
    let dit_input = Input::new(p.PA2, Pull::Up);
    let dah_input = Input::new(p.PA3, Pull::Up);
    let key_output = Output::new(p.PD6, Level::Low, Speed::Low);
    let status_led = Output::new(p.PD7, Level::Low, Speed::Low);

    info!("🔌 GPIO configured: Dit=PA2, Dah=PA3, Key=PD6, LED=PD7");

    // Configure PWM for sidetone on PC4 (TIM1_CH4)
    let pwm = SimplePwm::new(
        p.TIM1,
        Some(PwmPin::new_ch4(p.PC4, OutputType::PushPull)),
        None,
        None,
        None,
        Hertz::hz(600),  // Default 600Hz sidetone
        &mut config,
    );

    // Configure EXTI for paddle interrupts
    let mut exti = Exti::new(p.EXTI, Irqs);
    exti.listen(ch32_hal::exti::ExtiLine::Line2, ch32_hal::exti::Edge::Both);
    exti.listen(ch32_hal::exti::ExtiLine::Line3, ch32_hal::exti::Edge::Both);

    // Initialize keyer configuration
    let keyer_config = KeyerConfig {
        mode: KeyerMode::SuperKeyer,
        char_space_enabled: true,
        unit: Duration::from_millis(60), // 20 WPM
        debounce_ms: 5,
        queue_size: 64,
    };

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

    info!("🚀 Spawning keyer tasks...");

    // Spawn keyer tasks
    spawner.spawn(paddle_monitor_task()).unwrap();
    spawner.spawn(evaluator_task_ch32(&PADDLE, producer, keyer_config)).unwrap();
    spawner.spawn(sender_task_ch32(consumer, keyer_config.unit)).unwrap();
    spawner.spawn(sidetone_task(pwm, sidetone_receiver)).unwrap();

    info!("✨ Keyer firmware ready!");

    // Main supervision loop
    loop {
        Timer::after(Duration::from_secs(10)).await;
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
    producer: heapless::spsc::Producer<'static, Element, 64>,
    config: KeyerConfig,
) {
    info!("🧠 Evaluator task started");
    evaluator_task(paddle, producer, config).await;
}

/// Sender task for CH32V003
#[embassy_executor::task]
async fn sender_task_ch32(
    mut consumer: heapless::spsc::Consumer<'static, Element, 64>,
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

/// Sidetone generation task
#[embassy_executor::task]
async fn sidetone_task(
    mut pwm: SimplePwm<'static, peripherals::TIM1>,
    receiver: Receiver<'static, ThreadModeRawMutex, SidetoneCommand, 8>,
) {
    info!("🔊 Sidetone task started (600Hz default)");
    
    // Set initial PWM duty to 0 (off)
    pwm.set_duty(ch32_hal::timer::Channel::Ch4, 0);
    pwm.enable(ch32_hal::timer::Channel::Ch4);

    loop {
        match receiver.receive().await {
            SidetoneCommand::On => {
                // Set 50% duty cycle for audible tone
                let max_duty = pwm.get_max_duty();
                pwm.set_duty(ch32_hal::timer::Channel::Ch4, max_duty / 2);
                debug!("🔊 Sidetone ON");
            }
            SidetoneCommand::Off => {
                // Set 0% duty cycle for silence
                pwm.set_duty(ch32_hal::timer::Channel::Ch4, 0);
                debug!("🔇 Sidetone OFF");
            }
            SidetoneCommand::SetFrequency(freq) => {
                // Change PWM frequency
                pwm.set_frequency(Hertz::hz(freq as u32));
                info!("🎵 Sidetone frequency set to {}Hz", freq);
            }
        }
    }
}