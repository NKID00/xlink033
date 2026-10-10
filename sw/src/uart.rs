use core::{
    assert_matches,
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicI32, Ordering},
};

use critical_section::CriticalSection;
use pac::{
    DMA1, Interrupt, PFIC, USART2,
    dma::{
        Ch,
        vals::{Dir, Pl, Size},
    },
    usart::regs::{Brr, Statr},
};
use qingke::{
    interrupt::Priority,
    pfic::{enable_interrupt, set_priority},
    riscv::asm::delay,
};
use qingke_rt::interrupt;
use static_buffers::BufState;
use usb_t::{
    class::cdc::setup::{CharFmt, LineCoding, ParityTy, SET_LINE_CODING},
    setup::SetupRequest,
};

use crate::{
    io,
    usb::{self, EP2_BUF_OUT, EP2_IN_STATE, EP2_OUT_STATE},
};

// const DMA_RX: Ch = DMA1.ch(5);
// const DMA_TX: Ch = DMA1.ch(6);
const DMA_RX_CH: usize = 5; // Channel 6 for USART2_RX
const DMA_TX_CH: usize = 6; // Channel 7 for USART2_TX
static RX_DMA_WHICH: AtomicBool = AtomicBool::new(false);
static TX_DMA_WHICH: AtomicBool = AtomicBool::new(false);

// USART2_RX complete
#[interrupt]
fn DMA1_CHANNEL6() {
    if DMA1.isr().read().tcif(DMA_TX_CH) {
        let dma = DMA1.ch(DMA_RX_CH);
        DMA1.ifcr().write(|w| w.set_tcif(DMA_RX_CH, true));
    }
}

#[interrupt]
fn USART2() {
    if USART2.statr().read().rxne() {
        USART2.statr().modify(|w| w.set_rxne(false));
    }
}

// USART2_TX complete
#[interrupt]
fn DMA1_CHANNEL7() {
    if DMA1.isr().read().tcif(DMA_TX_CH) {
        // // if flags.teif(DMA_TX_CH)
        // //     || flags.tcif(DMA_TX_CH)
        // //     || flags.htif(DMA_TX_CH)
        // //     || flags.gif(DMA_TX_CH)
        // if flags.0 == 0 {
        // if !(unsafe { (PFIC.iactr1().as_ptr() as *const u32).read_volatile() } & (1 << 28) != 0
        //     && PFIC.gisr().read().gactsta()
        //     && PFIC.gisr().read().neststa() == 0x01
        //     && flags.0 != 0)
        // {
        //     io::lnk_on();
        //     panic!();
        // }
        critical_section::with(|cs| {
            let which = TX_DMA_WHICH.load(Ordering::SeqCst);
            // static COUNTER: AtomicI32 = AtomicI32::new(0);
            // let counter = COUNTER.load(Ordering::SeqCst) + 1;
            // COUNTER.store(counter, Ordering::SeqCst);
            // io::act_on();
            // let dma = DMA1.ch(DMA_TX_CH);
            // match counter {
            //     1 => {
            //         assert!(which == false);
            //         assert_matches!(EP2_OUT_STATE.get(cs, which), BufState::Filled { .. });
            //         assert_matches!(EP2_OUT_STATE.get(cs, !which), BufState::Filled { .. });
            //         assert!(DMA1.isr().read().tcif(DMA_TX_CH));
            //         // assert!(dma.cr().read().en() == true);
            //         // assert!(dma.ndtr().read().ndt() == 0);
            //     }
            //     2 => {
            //         assert!(which == true);
            //         assert_matches!(EP2_OUT_STATE.get(cs, which), BufState::Filled { .. });
            //         assert_matches!(EP2_OUT_STATE.get(cs, !which), BufState::Empty);
            //         let flags = DMA1.isr().read();
            //         assert!(
            //             flags.teif(DMA_TX_CH)
            //                 || flags.tcif(DMA_TX_CH)
            //                 || flags.htif(DMA_TX_CH)
            //                 || flags.gif(DMA_TX_CH)
            //         );
            //         // assert!(dma.cr().read().en() == true);
            //         // assert!(dma.ndtr().read().ndt() == 0);
            //     }
            //     _ => {},
            // }
            // io::act_off();
            EP2_OUT_STATE.mark_empty(cs, which);
            if let BufState::Filled { len } = EP2_OUT_STATE.get(cs, !which) {
                start_tx_dma(!which, len.into());
                // writer was blocked by slow reader, start writing again
                usb::prepare_out_ack::<2>();
                assert!(usb::get_out_toggle(2) == which);
            }
        });
        DMA1.ifcr().write(|w| w.set_tcif(DMA_TX_CH, true));
    }
}

pub fn on_ep2_in(cs: CriticalSection, toggle: bool) {
    usb::prepare_in_nak_flip_toggle::<2>();
}

pub fn start_tx_dma(which: bool, len: u8) {
    // critical_section::with(|cs| {
    // static COUNTER: AtomicI32 = AtomicI32::new(0);
    // let counter = COUNTER.load(Ordering::SeqCst) + 1;
    // COUNTER.store(counter, Ordering::SeqCst);
    // io::act_on();
    // match counter {
    //     1 => {
    //         assert!(which == false);
    //         assert_matches!(EP2_OUT_STATE.get(cs, which), BufState::Filled { .. });
    //         assert_matches!(EP2_OUT_STATE.get(cs, !which), BufState::Empty);
    //         assert!(dma.ndtr().read().ndt() == 0);
    //     }
    //     2 => {
    //         assert!(which == true);
    //         assert_matches!(EP2_OUT_STATE.get(cs, which), BufState::Filled { .. });
    //         assert_matches!(EP2_OUT_STATE.get(cs, !which), BufState::Empty);
    //         assert!(dma.ndtr().read().ndt() == 0);
    //     }
    //     3 => {
    //         assert!(which == false);
    //         assert_matches!(EP2_OUT_STATE.get(cs, which), BufState::Filled { .. });
    //         assert_matches!(EP2_OUT_STATE.get(cs, !which), BufState::Empty);
    //         // assert!(dma.ndtr().read().ndt() == 0);
    //     }
    //     _ => {}
    // }
    // io::act_off();
    // });
    let dma = DMA1.ch(DMA_TX_CH);
    assert!(dma.ndtr().read().ndt() == 0);
    dma.cr().write(|w| {
        w.set_en(false);
    });
    dma.mar().write_value(EP2_BUF_OUT.as_ptr(which) as u32);
    dma.par().write_value(USART2.datar().as_ptr() as u32);
    dma.ndtr().write(|w| w.set_ndt(len as u16));
    dma.cr().write(|w| {
        w.set_pl(Pl::HIGH);
        w.set_msize(Size::BITS8);
        w.set_psize(Size::BITS8);
        w.set_minc(true);
        w.set_pinc(false);
        w.set_dir(Dir::FROMMEMORY);
        w.set_tcie(true);
        w.set_en(true);
    });
    TX_DMA_WHICH.store(which, Ordering::SeqCst);
    io::act_flash();
}

pub fn on_ctrl_write(req: &SetupRequest, data: &'static [u8]) -> Result<(), ()> {
    match req.request {
        SET_LINE_CODING => {
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
            // reset(800, parity, stop_bits);
            Ok(())
        }
        _ => Err(()),
    }
}

pub fn init() {
    // disable UART
    USART2.ctlr1().write(|w| w.set_ue(false));
    USART2.ctlr3().write(|w| {
        w.set_dmat(true); // enable tx dma
        w.set_dmar(true); // enable rx dma
    });
    // clear interrupt flags
    USART2.statr().write_value(Statr(0));
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
    critical_section::with(|cs| {
        io::disable_uart_tx();
        USART2.ctlr1().modify(|w| w.set_ue(false));

        // stop dma
        let dma_rx = DMA1.ch(DMA_RX_CH);
        dma_rx.cr().write(|w| {
            w.set_en(false);
        });
        let dma_tx = DMA1.ch(DMA_TX_CH);
        dma_tx.cr().write(|w| {
            w.set_en(false);
        });

        EP2_IN_STATE.clear(cs);
        EP2_OUT_STATE.clear(cs);

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

        // clear interrupt flags
        USART2.statr().write_value(Statr(0));

        USART2.ctlr1().modify(|w| w.set_ue(true));
        io::enable_uart_tx();
    });
}
