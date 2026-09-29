#![no_std]

use core::cell::UnsafeCell;
use core::task::Waker;

use critical_section::{CriticalSection, Mutex};
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
use pac::SYSTICK;
use pac::systick::vals::{Mode, Stclk};
use qingke::interrupt::Priority;
use qingke::pfic::{enable_interrupt, set_priority};
use qingke_rt::{CoreInterrupt, interrupt};

#[interrupt(core)]
fn SysTick() {
    // clear interrupt flag
    SYSTICK.sr().write(|w| w.set_cntif(false));
    critical_section::with(|cs| DRIVER.update_alarm(cs));
}

embassy_time_driver::time_driver_impl!(static DRIVER: TimeDriver = TimeDriver{queue: Mutex::new(UnsafeCell::new(Queue::new()))});

struct TimeDriver {
    queue: Mutex<UnsafeCell<Queue>>,
}

impl TimeDriver {
    #[inline]
    fn update_alarm(&self, cs: CriticalSection) {
        // Safety: Even if critical section is nested, the outer one is
        // neither this block nor schedule_wake, because in this
        // block schedule_wake is never called, interrupt
        // never triggers and no user code is executed. Therefore the
        // private queue is never borrowed at this point.
        let queue = unsafe { self.queue.borrow(cs).as_mut_unchecked() };
        loop {
            let next = queue.next_expiration(self.now());
            if self.set_alarm(cs, next) {
                break;
            }
        }
    }

    /// Returns false if alarm time has already passed.
    #[inline]
    fn set_alarm(&self, _cs: CriticalSection, at: u64) -> bool {
        if self.now() >= at {
            return false;
        }
        SYSTICK.cmp().write_value(at);
        // we have critical section token, interrupts are disabled
        // hence alarm will never trigger here
        true
    }
}

impl Driver for TimeDriver {
    #[inline]
    fn now(&self) -> u64 {
        SYSTICK.cnt().read()
    }

    #[inline]
    fn schedule_wake(&self, at: u64, waker: &Waker) {
        critical_section::with(|cs| {
            // Safety: Even if critical section is nested, the outer one is
            // neither this block nor SysTick interrupt, because in this
            // block this function itself is never called again, interrupt
            // never triggers and no user code is executed. Therefore the
            // private queue is never borrowed at this point.
            let queue = unsafe { self.queue.borrow(cs).as_mut_unchecked() };
            if queue.schedule_wake(at, waker) {
                self.update_alarm(cs);
            }
        });
    }
}

pub fn init() {
    SYSTICK.cmp().write_value(u64::MAX);
    SYSTICK.ctlr().write(|w| {
        w.set_init(true); // init to 0
        w.set_mode(Mode::UPCOUNT);
        w.set_stre(false); // no auto reload
        w.set_stclk(Stclk::HCLK_DIV8);
        w.set_stie(true); // enable interrupt
        w.set_ste(true); // enable systick
    });
    // clear interrupt flag
    SYSTICK.sr().write(|w| w.set_cntif(false));
    unsafe {
        // lowest priority
        set_priority(CoreInterrupt::SysTick as u8, Priority::P15 as u8);
        enable_interrupt(CoreInterrupt::SysTick as u8);
    };
}
