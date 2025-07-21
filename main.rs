#![no_std]
#![no_main]

use keyer::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use heapless::spsc::Queue;

use ch32v0_hal::{
    gpio::{Level, PullUp, Input, Output, Pin, AnyPin},
    pac,
    prelude::*,
};

use cortex_m_rt::interrupt;
use static_cell::StaticCell;

static PADDLE: PaddleInput = PaddleInput::new();

static KEY_QUEUE: StaticCell<Queue<Element, 64>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let dp = pac::Peripherals::take().unwrap();
    let mut rcc = dp.RCC.configure().freeze();
    let mut gpioa = dp.GPIOA.split(&mut rcc);

    // パドル入力ピン（割り込み用）
    let dit_pin = gpioa.pa0.into_pull_up_input();
    let dah_pin = gpioa.pa1.into_pull_up_input();

    // キー出力ピン
    let mut key_pin = gpioa.pa2.into_push_pull_output();

    // 割り込み設定（仮想例：EXTI0, EXTI1）
    // ※ 実装時は CH32V PAC の割り込みAPIに従って設定してください
    dit_pin.enable_interrupt();
    dah_pin.enable_interrupt();

    // 送信キューの初期化
    let queue = KEY_QUEUE.init(Queue::new());
    let (mut prod, mut cons) = queue.split();

    let config = KeyerConfig {
        mode: KeyerMode::SuperKeyer,
        char_space_enabled: true,
        unit: Duration::from_millis(60),
    };

    spawner.spawn(evaluator_fsm(&PADDLE, &mut prod, &config)).unwrap();
    spawner.spawn(sender_task(&mut key_pin, &mut cons, config.unit)).unwrap();

    // 無限ループで入力を監視する必要なし（割り込み駆動）
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

// ISR割り込みハンドラ（実装は HALに応じて調整）

#[interrupt]
fn EXTI0() {
    // Dit入力（押下をtrueとする例）
    PADDLE.update(PaddleSide::Dit, true);
}

#[interrupt]
fn EXTI1() {
    // Dah入力（押下）
    PADDLE.update(PaddleSide::Dah, true);
}