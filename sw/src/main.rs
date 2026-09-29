#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use core::hint::spin_loop;
use core::ptr::write_volatile;

use pac::rcc::vals::Hpre;
use embassy_executor::Spawner;
use embassy_time::Timer;
use pac::dma::vals::{Dir, Pl, Size};
use pac::gpio::vals::Cnf;
use pac::gpio::vals::Mode;
use pac::{AFIO, DMA1, GPIOA};
use pac::{FLASH, RCC};
use qingke::riscv::asm::delay;
use qingke_rt::interrupt;

#[interrupt]
fn PIOC() {}

#[embassy_executor::main(entry = "qingke_rt::entry")]
async fn main(_spawner: Spawner) -> ! {
    // flash needs 3 wait states when core/bus freq >24MHz
    FLASH.actlr().write(|w| w.set_latency(2));
    // set core/bus freq to 1 prescale (48MHz)
    RCC.cfgr0().write(|w| w.set_hpre(Hpre::DIV1));

    time_driver::init();

    Timer::after_millis(20).await; // brick-proof

    RCC.apb2pcenr().modify(|w| {
        w.set_iopaen(true);
        w.set_afioen(true);
    });

    // disable SWD, setup IO pins
    AFIO.pcfr1().modify(|w| w.set_sw_cfg(0b100));

    GPIOA.cfglr().modify(|w| {
        w.set_mode(0, Mode::OUTPUT_50MHZ);
        w.set_cnf(0, Cnf::ANALOG_IN__PUSH_PULL_OUT);
    });

    loop {
        led_act_on();
        Timer::after_millis(100).await;
        led_act_off();
        Timer::after_millis(900).await;
    }
}

/// src and dst must be 32-bit aligned
unsafe fn memcpy_dma(src: *const u8, dst: *mut u8, len: usize) {
    let dma = DMA1.ch(7);
    dma.cr().write(|w| {
        w.set_en(false);
    });
    dma.cr().write(|w| {
        w.set_mem2mem(true);
        w.set_pl(Pl::LOW);
        w.set_msize(Size::BITS32);
        w.set_psize(Size::BITS32);
        w.set_minc(true);
        w.set_pinc(true);
        w.set_dir(Dir::FROMPERIPHERAL);
    });
    dma.ndtr().write(|w| w.set_ndt((len / 4) as u16));
    dma.par().write_value(src as u32);
    dma.mar().write_value(dst as u32);
    DMA1.ifcr().write(|w| w.set_gif(7, true));
    dma.cr().modify(|w| {
        w.set_en(true);
    });
    if !len.is_multiple_of(4) {
        let offset = len / 4 * 4;
        for i in 0..(len % 4) {
            unsafe {
                *dst.add(offset + i) = *src.add(offset + i);
            }
        }
    }
    while !DMA1.isr().read().tcif(7) {
        delay(len as u32);
        while !DMA1.isr().read().gif(7) {
            spin_loop();
        }
        if DMA1.isr().read().teif(7) {
            panic!();
        }
        DMA1.ifcr().write(|w| w.set_htif(7, true));
    }
    DMA1.ifcr().write(|w| w.set_gif(7, true));
}

#[inline]
fn led_act_on() {
    GPIOA.bshr().write(|w| w.set_br(0, true));
}

#[inline]
fn led_act_off() {
    GPIOA.bshr().write(|w| w.set_bs(0, true));
}

#[inline]
fn led_act_toggle() {
    GPIOA.outdr().modify(|w| w.set_odr(0, !w.odr(0)));
}

#[inline]
fn iwdg_feed() {
    const IWDG_CTLR: *mut u16 = 0x40003000 as *mut u16;
    unsafe { write_volatile(IWDG_CTLR, 0xaaaa) };
}
