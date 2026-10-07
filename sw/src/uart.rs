use pac::{
    DMA1, Interrupt, USART2,
    dma::{
        Ch,
        vals::{Dir, Pl, Size},
    },
    usart::regs::{Brr, Statr},
};
use qingke::{
    interrupt::Priority,
    pfic::{enable_interrupt, set_priority},
};
use qingke_rt::interrupt;
use usb_t::class::cdc::setup::{CharFmt, LineCoding, ParityTy};

use crate::{
    io::led_act_on,
    usb::{self, EP2_BUF, EP3_BUF},
};

// const DMA_RX: Ch = DMA1.ch(5);
// const DMA_TX: Ch = DMA1.ch(6);
const DMA_RX: usize = 5; // Channel 6 for USART2_RX
const DMA_TX: usize = 6; // Channel 7 for USART2_TX

// USART2_RX complete
#[interrupt]
fn DMA1_CHANNEL6() {
    let dma = DMA1.ch(DMA_RX);
    DMA1.ifcr().write(|w| w.set_gif(DMA_RX, true));
}

// USART2_TX complete
#[interrupt]
fn DMA1_CHANNEL7() {
    usb::prepare_out_ack_flip_toggle::<2>();
    DMA1.ifcr().write(|w| w.set_gif(DMA_TX, true));
}

#[interrupt]
fn USART2() {
    USART2.statr().write_value(Statr(0));
}

pub fn on_ep2_in() {}

pub fn on_ep2_out(buf: &'static [u8]) {
    let dma = DMA1.ch(DMA_TX);
    dma.cr().write(|w| {
        w.set_en(false);
    });
    dma.mar().write_value(buf.as_ptr() as u32);
    dma.par().write_value(USART2.datar().as_ptr() as u32);
    dma.ndtr().write(|w| w.set_ndt(buf.len() as u16));
    dma.cr().write(|w| {
        w.set_mem2mem(true);
        w.set_pl(Pl::HIGH);
        w.set_msize(Size::BITS8);
        w.set_psize(Size::BITS8);
        w.set_minc(true);
        w.set_pinc(true);
        w.set_dir(Dir::FROMMEMORY);
        w.set_tcie(true);
        w.set_en(true);
    });
}

pub fn on_ctrl_set_line_coding(data: &'static [u8]) -> Result<(), ()> {
    if data.len() != size_of::<LineCoding>() {
        return Err(());
    }
    // avoid invalid enum ub
    if data[4] > 2 || data[5] > 4 {
        return Err(());
    }
    let line_coding = unsafe { LineCoding::from_ptr(data.as_ptr()) };
    if line_coding.data_bits != 8 {
        return Err(());
    }
    let parity = match line_coding.parity_ty {
        ParityTy::None => Parity::None,
        ParityTy::Odd => Parity::Odd,
        ParityTy::Even => Parity::Even,
        ParityTy::Mark | ParityTy::Space => return Err(()),
    };
    let stop_bits = match line_coding.char_fmt {
        CharFmt::StopBits1 => StopBits::Bits1,
        CharFmt::StopBits1AndHalf => StopBits::Bits1AndHalf,
        CharFmt::StopBits2 => StopBits::Bits2,
    };
    reset(line_coding.data_term_rate, parity, stop_bits);
    Ok(())
}

pub enum Parity {
    None,
    Odd,
    Even,
}

pub enum StopBits {
    Bits1 = 0b00,
    BitsHalf = 0b01,
    Bits2 = 0b10,
    Bits1AndHalf = 0b11,
}

pub fn reset(baud_rate: u32, parity: Parity, stop_bits: StopBits) {
    // disable UART
    USART2.ctlr1().modify(|w| w.set_ue(false));
    // stop dma
    let dma_rx = DMA1.ch(DMA_RX);
    let dma_tx = DMA1.ch(DMA_TX);
    dma_rx.cr().write(|w| {
        w.set_en(false);
    });
    dma_tx.cr().write(|w| {
        w.set_en(false);
    });

    let div = (48_000_000 * 2 / baud_rate + 1) / 2;
    USART2.brr().write_value(Brr(div as u32));
    USART2.ctlr1().write(|w| {
        w.set_ue(false);
        match parity {
            Parity::None => w.set_pce(false),
            Parity::Odd => {
                w.set_pce(true);
                w.set_ps(true);
            }
            Parity::Even => {
                w.set_pce(true);
                w.set_ps(false);
            }
        }
        w.set_rxneie(true); // enable rx interrupt
        w.set_te(true); // enable tx
        w.set_re(true); // enable rx
    });
    USART2.ctlr2().write(|w| {
        w.set_stop(stop_bits as u8);
    });
    USART2.ctlr3().write(|w| {
        w.set_dmat(true); // enable tx dma
        w.set_dmar(true); // enable rx dma
    });
    // clear interrupt flags
    USART2.statr().write_value(Statr(0));

    // enable UART
    USART2.ctlr1().modify(|w| w.set_ue(true));

    // enable interrupts
    unsafe {
        set_priority(Interrupt::DMA1_CHANNEL6 as u8, Priority::P8 as u8);
        set_priority(Interrupt::DMA1_CHANNEL7 as u8, Priority::P8 as u8);
        set_priority(Interrupt::USART2 as u8, Priority::P9 as u8);
        enable_interrupt(Interrupt::DMA1_CHANNEL6 as u8);
        enable_interrupt(Interrupt::DMA1_CHANNEL7 as u8);
        enable_interrupt(Interrupt::USART2 as u8);
    }
}
