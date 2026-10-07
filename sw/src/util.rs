use core::future::poll_fn;
use core::ptr::write_volatile;
use core::sync::atomic::{Ordering, compiler_fence};
use core::task::Poll;

use aligned::{A4, Aligned};
use embassy_sync::blocking_mutex::{CriticalSectionMutex, Mutex};
use embassy_sync::waitqueue::MultiWakerRegistration;
use pac::DMA1;
use pac::dma::vals::{Dir, Pl, Size};
use qingke_rt::interrupt;

#[macro_export]
macro_rules! spin_until {
    ($cond:expr) => {
        while !$cond {
            ::core::hint::spin_loop();
        }
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::Acquire);
    };
}

// static DMA_MEMCPY_WAKER: CriticalSectionMutex<MultiWakerRegistration<4>> =
//     CriticalSectionMutex::new(MultiWakerRegistration::new());

// #[interrupt]
// fn DMA1_CHANNEL8() {
//     unsafe { DMA_MEMCPY_WAKER.lock_mut(|waker| waker.wake()) };
// }

// pub async unsafe fn memcpy_dma(src: *const Aligned<A4, u8>, dst: *mut Aligned<A4, u8>, len: usize) {
//     let dma = DMA1.ch(7);
//     poll_fn(move |ctx| {
//         if dma.ndtr().read().ndt() > 0 {
//             DMA_MEMCPY_WAKER.register(ctx.waker());
//             Poll::Pending
//         } else {
//             compiler_fence(Ordering::Acquire);
//             Poll::Ready(())
//         }
//     })
//     .await;
//     dma.cr().write(|w| {
//         w.set_en(false);
//     });
//     dma.par().write_value(src as u32);
//     dma.mar().write_value(dst as u32);
//     dma.ndtr().write(|w| w.set_ndt((len / 4) as u16));
//     dma.cr().write(|w| {
//         w.set_mem2mem(true);
//         w.set_pl(Pl::LOW);
//         w.set_msize(Size::BITS32);
//         w.set_psize(Size::BITS32);
//         w.set_minc(true);
//         w.set_pinc(true);
//         w.set_dir(Dir::FROMPERIPHERAL);
//         w.set_tcie(true);
//         w.set_en(true);
//     });
//     if !len.is_multiple_of(4) {
//         let offset = len / 4 * 4;
//         for i in 0..(len % 4) {
//             unsafe {
//                 *dst.add(offset + i) = *src.add(offset + i);
//             }
//         }
//     }
//     poll_fn(move |ctx| {
//         if dma.ndtr().read().ndt() > 0 {
//             DMA_MEMCPY_WAKER.register(ctx.waker());
//             Poll::Pending
//         } else {
//             compiler_fence(Ordering::Acquire);
//             Poll::Ready(())
//         }
//     })
//     .await;
// }

pub unsafe fn memcpy_dma_blocking(
    src: *const Aligned<A4, u8>,
    dst: *mut Aligned<A4, u8>,
    len: usize,
) {
    let dma = DMA1.ch(7);
    dma.cr().write(|w| {
        w.set_en(false);
    });
    dma.par().write_value(src as u32);
    dma.mar().write_value(dst as u32);
    dma.ndtr().write(|w| w.set_ndt((len / 4) as u16));
    dma.cr().write(|w| {
        w.set_mem2mem(true);
        w.set_pl(Pl::LOW);
        w.set_msize(Size::BITS32);
        w.set_psize(Size::BITS32);
        w.set_minc(true);
        w.set_pinc(true);
        w.set_dir(Dir::FROMPERIPHERAL);
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
    spin_until!(dma.ndtr().read().ndt() == 0);
}

#[inline]
pub fn iwdg_feed() {
    const IWDG_CTLR: *mut u16 = 0x40003000 as *mut u16;
    unsafe { write_volatile(IWDG_CTLR, 0xaaaa) };
}

pub fn read_serial_number() -> [u8; 12] {
    const ESIG_UNIID: *const u32 = 0x1FFFF7E8 as *const u32;
    let mut buf = [0; 12];
    for (chunk, offset) in buf.chunks_exact_mut(4).zip(0..) {
        chunk.copy_from_slice(&unsafe { ESIG_UNIID.add(offset).read_volatile() }.to_le_bytes());
    }
    buf
}
