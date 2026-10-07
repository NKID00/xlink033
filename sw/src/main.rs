#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(coroutines)]
#![feature(coroutine_trait)]

mod io;
mod rcc;
mod uart;
mod usb;
mod util;

use embassy_executor::{Spawner, main, task};
use embassy_time::Timer;

use crate::io::{led_act, led_lnk_off, led_lnk_on};

#[main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    rcc::init();
    // brick-proof as SWD will be disabled next
    Timer::after_millis(20).await;
    io::init();
    usb::init();

    spawner.spawn(led_act_task().unwrap());

    loop {
        led_lnk_on();
        Timer::after_millis(50).await;
        led_lnk_off();
        Timer::after_millis(950).await;
    }
}

#[task]
async fn led_act_task() -> ! {
    loop {
        for i in 0..100 {
            led_act(i);
            Timer::after_millis(5).await;
        }
        for i in (0..100).rev() {
            led_act(i);
            Timer::after_millis(5).await;
        }
    }
}
