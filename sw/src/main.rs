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

#[main(entry = "qingke_rt::entry")]
async fn main(spawner: Spawner) -> ! {
    rcc::init();
    // brick-proof as SWD will be disabled next
    Timer::after_millis(20).await;
    io::init(&spawner);
    uart::init();
    usb::init();

    loop {
        Timer::after_millis(100).await;
    }
}
