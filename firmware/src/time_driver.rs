//! Simple embassy time driver for CH32V003

use embassy_time_driver::{AlarmHandle, Driver};
use portable_atomic::{AtomicU32, Ordering};

/// Simple time driver using system tick counter
pub struct SimpleTimeDriver {
    tick_count: AtomicU32,
}

impl SimpleTimeDriver {
    const fn new() -> Self {
        Self {
            tick_count: AtomicU32::new(0),
        }
    }
    
    /// Increment tick count (called from system timer interrupt)
    pub fn tick(&self) {
        self.tick_count.fetch_add(1, Ordering::Relaxed);
    }
}

impl Driver for SimpleTimeDriver {
    fn now(&self) -> u64 {
        self.tick_count.load(Ordering::Relaxed) as u64
    }

    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
        // For simplicity, we don't support alarms in this basic implementation
        None
    }

    fn set_alarm_callback(&self, _alarm: AlarmHandle, _callback: fn(*mut ()), _ctx: *mut ()) {
        // Not implemented
    }

    fn set_alarm(&self, _alarm: AlarmHandle, _timestamp: u64) -> bool {
        // Not implemented
        false
    }
}

// Export the driver  
embassy_time_driver::time_driver_impl!(static DRIVER: SimpleTimeDriver = SimpleTimeDriver::new());

// Critical section implementation for single-core RISC-V
critical_section::set_impl!(RiscvCriticalSection);

struct RiscvCriticalSection;

unsafe impl critical_section::Impl for RiscvCriticalSection {
    unsafe fn acquire() -> u8 {
        let mut mstatus: usize;
        core::arch::asm!("csrrci {}, mstatus, 8", out(reg) mstatus);
        (mstatus & 8) as u8
    }

    unsafe fn release(was_active: u8) {
        if was_active != 0 {
            core::arch::asm!("csrsi mstatus, 8");
        }
    }
}