use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use embassy_time::{Duration, Instant, Timer};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use heapless::spsc::{Consumer, Producer};

// ==== キーヤー構成定義 ====

#[derive(Copy, Clone, PartialEq)]
pub enum Element {
    Dit,
    Dah,
}

#[derive(Copy, Clone)]
pub enum KeyerMode {
    ModeA,
    ModeB,
    SuperKeyer,
}

#[derive(Copy, Clone)]
pub enum PaddleSide {
    Dit,
    Dah,
}

#[derive(Copy, Clone)]
enum FSMState {
    Idle,
    DitHold,
    DahHold,
    Squeeze(Element),
    MemoryPending(Element),
    CharSpacePending(Instant),
}

pub struct KeyerConfig {
    pub mode: KeyerMode,
    pub char_space_enabled: bool,
    pub unit: Duration,
}

// ==== 割り込み入力デバウンス構造 ====

const DEBOUNCE_MS: u64 = 10;

pub struct PaddleInput {
    dit_pressed: AtomicBool,
    dah_pressed: AtomicBool,
    dit_last_edge: AtomicU64,
    dah_last_edge: AtomicU64,
}

impl PaddleInput {
    pub const fn new() -> Self {
        Self {
            dit_pressed: AtomicBool::new(false),
            dah_pressed: AtomicBool::new(false),
            dit_last_edge: AtomicU64::new(0),
            dah_last_edge: AtomicU64::new(0),
        }
    }

    pub fn update(&self, side: PaddleSide, state: bool) {
        let now = Instant::now().as_millis() as u64;
        match side {
            PaddleSide::Dit => {
                let last = self.dit_last_edge.load(Ordering::Relaxed);
                if now - last >= DEBOUNCE_MS {
                    self.dit_pressed.store(state, Ordering::Relaxed);
                    self.dit_last_edge.store(now, Ordering::Relaxed);
                }
            }
            PaddleSide::Dah => {
                let last = self.dah_last_edge.load(Ordering::Relaxed);
                if now - last >= DEBOUNCE_MS {
                    self.dah_pressed.store(state, Ordering::Relaxed);
                    self.dah_last_edge.store(now, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn dit(&self) -> bool {
        self.dit_pressed.load(Ordering::Relaxed)
    }

    pub fn dah(&self) -> bool {
        self.dah_pressed.load(Ordering::Relaxed)
    }
}

// ==== SuperKeyer 優先制御 ====

pub struct SuperKeyerController {
    dit_time: Option<Instant>,
    dah_time: Option<Instant>,
}

impl SuperKeyerController {
    pub fn new() -> Self {
        Self { dit_time: None, dah_time: None }
    }

    pub fn record_press(&mut self, dit: bool, dah: bool) {
        let now = Instant::now();
        if dit { self.dit_time = Some(now); }
        if dah { self.dah_time = Some(now); }
    }

    pub fn evaluate_priority(&self) -> Option<Element> {
        match (self.dit_time, self.dah_time) {
            (Some(dt), Some(ht)) => if ht <= dt { Some(Element::Dah) } else { Some(Element::Dit) },
            (Some(_), None) => Some(Element::Dit),
            (None, Some(_)) => Some(Element::Dah),
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.dit_time = None;
        self.dah_time = None;
    }
}

// ==== FSMタスク ====

pub async fn evaluator_fsm(
    paddle: &PaddleInput,
    queue: &mut Producer<'_, Element, 64>,
    config: &KeyerConfig,
) {
    let mut state = FSMState::Idle;
    let mut superkeyer = SuperKeyerController::new();

    loop {
        let dit_now = paddle.dit();
        let dah_now = paddle.dah();
        let now = Instant::now();

        match state {
            FSMState::Idle => {
                if dit_now && dah_now {
                    let start = match config.mode {
                        KeyerMode::SuperKeyer => superkeyer.evaluate_priority().unwrap_or(Element::Dit),
                        _ => Element::Dit,
                    };
                    queue.enqueue(start).ok();
                    state = FSMState::Squeeze(start);
                } else if dit_now {
                    queue.enqueue(Element::Dit).ok();
                    state = FSMState::DitHold;
                } else if dah_now {
                    queue.enqueue(Element::Dah).ok();
                    state = FSMState::DahHold;
                }
            }

            FSMState::DitHold => {
                if !dit_now {
                    state = FSMState::Idle;
                } else {
                    queue.enqueue(Element::Dit).ok();
                }
                if dah_now {
                    state = FSMState::Squeeze(Element::Dit);
                }
            }

            FSMState::DahHold => {
                if !dah_now {
                    state = FSMState::Idle;
                } else {
                    queue.enqueue(Element::Dah).ok();
                }
                if dit_now {
                    state = FSMState::Squeeze(Element::Dah);
                }
            }

            FSMState::Squeeze(last) => {
                if dit_now && dah_now {
                    let next = match config.mode {
                        KeyerMode::SuperKeyer => {
                            superkeyer.record_press(dit_now, dah_now);
                            superkeyer.evaluate_priority().unwrap_or(last)
                        }
                        _ => if last == Element::Dit { Element::Dah } else { Element::Dit }
                    };
                    queue.enqueue(next).ok();
                    state = FSMState::Squeeze(next);
                } else if dit_now {
                    queue.enqueue(Element::Dit).ok();
                    state = FSMState::DitHold;
                } else if dah_now {
                    queue.enqueue(Element::Dah).ok();
                    state = FSMState::DahHold;
                } else {
                    match config.mode {
                        KeyerMode::ModeB | KeyerMode::SuperKeyer => {
                            let mem = if last == Element::Dit { Element::Dah } else { Element::Dit };
                            state = FSMState::MemoryPending(mem);
                        }
                        _ => {
                            state = if config.char_space_enabled {
                                FSMState::CharSpacePending(now)
                            } else {
                                FSMState::Idle
                            };
                        }
                    }
                }
            }

            FSMState::MemoryPending(mem) => {
                queue.enqueue(mem).ok();
                superkeyer.clear();
                state = if config.char_space_enabled {
                    FSMState::CharSpacePending(now)
                } else {
                    FSMState::Idle
                };
            }

            FSMState::CharSpacePending(start_time) => {
                if dit_now || dah_now {
                    let elapsed = now.duration_since(start_time);
                    if elapsed >= config.unit * 3 {
                        let next = if dah_now { Element::Dah } else { Element::Dit };
                        queue.enqueue(next).ok();
                        state = match (dit_now, dah_now) {
                            (true, true) => FSMState::Squeeze(next),
                            (true, false) => FSMState::DitHold,
                            (false, true) => FSMState::DahHold,
                            _ => FSMState::Idle,
                        };
                    }
                } else if now.duration_since(start_time) >= config.unit * 3 {
                    state = FSMState::Idle;
                }
            }
        }

        Timer::after(config.unit / 4).await;
    }
}

// ==== Senderタスク ====

pub async fn sender_task(
    key: &mut impl OutputPin,
    queue: &mut Consumer<'_, Element, 64>,
    unit: Duration,
) {
    loop {
        if let Some(elem) = queue.dequeue() {
            let on_time = match elem {
                Element::Dit => unit,
                Element::Dah => unit * 3,
            };

            key.set_high().ok();
            Timer::after(on_time).await;
            key.set_low().ok();
            Timer::after(unit).await;
        } else {
            Timer::after(unit / 2).await;
        }
    }
}
