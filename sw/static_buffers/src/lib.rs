#![no_std]

use core::{
    cell::{Cell, UnsafeCell},
    num::NonZeroUsize,
};

use aligned::{A4, Aligned, Alignment};
use critical_section::CriticalSection;

/// A buffer consists of N consecutive bytes with alignment A (defaults to
/// 4 bytes aligned).
#[repr(transparent)]
pub struct StaticBuf<const N: usize, A: Alignment = A4>(UnsafeCell<Aligned<A, [u8; N]>>);

impl<const N: usize, A: Alignment> StaticBuf<N, A> {
    #[inline]
    pub const fn new() -> Self {
        Self(UnsafeCell::new(Aligned([0; N])))
    }

    #[inline]
    pub const fn len() -> usize {
        N
    }

    #[inline]
    pub const fn get(&'static self) -> *const u8 {
        self.0.get() as _
    }

    #[inline]
    pub const fn get_aligned(&'static self) -> *const Aligned<A, [u8; N]> {
        self.get() as _
    }

    #[inline]
    pub const fn as_ptr(&'static self) -> *const [u8; N] {
        self.get() as _
    }

    /// Safety: ensure there is no other mutable reference to the buffer.
    #[inline]
    pub const unsafe fn as_ref(&'static self) -> &'static [u8; N] {
        unsafe { self.as_ptr().as_ref_unchecked() }
    }

    /// Safety: ensure there is no other reference to the buffer.
    #[inline]
    pub const unsafe fn as_mut(&'static self) -> &'static mut [u8; N] {
        unsafe { self.as_ptr().cast_mut().as_mut_unchecked() }
    }
}

// Safety: all methods providing buffer mutation are already unsafe.
unsafe impl<const N: usize, A: Alignment> Sync for StaticBuf<N, A> {}

/// A buffer consists of 2N consecutive bytes with alignment A (defaults to
/// 4 bytes aligned). First half numbered 0 (false), second half numbered 1
/// (true). The order can be reversed if REVERSE set to true.
#[repr(C)]
pub struct StaticDoubleBuf<const N: usize, A: Alignment = A4, const REVERSE: bool = false> {
    buf0: StaticBuf<N, A>,
    buf1: StaticBuf<N, A>,
}

impl<const N: usize, A: Alignment, const REVERSE: bool> StaticDoubleBuf<N, A, REVERSE> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            buf0: StaticBuf::new(),
            buf1: StaticBuf::new(),
        }
    }

    #[inline]
    pub const fn len() -> usize {
        2 * N
    }

    #[inline]
    pub const fn base(&'static self) -> *const u8 {
        self.buf0.get() as _
    }

    #[inline]
    pub const fn get(&'static self, which: bool) -> *const u8 {
        if which ^ REVERSE {
            self.buf1.get() as _
        } else {
            self.buf0.get() as _
        }
    }

    #[inline]
    pub const fn get_aligned(&'static self, which: bool) -> *const Aligned<A, [u8; N]> {
        self.get(which) as _
    }

    #[inline]
    pub const fn as_ptr(&'static self, which: bool) -> *const [u8; N] {
        self.get(which) as _
    }

    /// Safety: ensure there is no other mutable reference to the buffer.
    #[inline]
    pub const unsafe fn as_ref(&'static self, which: bool) -> &'static [u8; N] {
        unsafe { self.as_ptr(which).as_ref_unchecked() }
    }

    /// Safety: ensure there is no other reference to the buffer.
    #[inline]
    pub const unsafe fn as_mut(&'static self, which: bool) -> &'static mut [u8; N] {
        unsafe { self.as_ptr(which).cast_mut().as_mut_unchecked() }
    }
}

// Safety: all methods providing buffer mutation are already unsafe.
unsafe impl<const N: usize, A: Alignment> Sync for StaticDoubleBuf<N, A> {}

/// State of a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufState<LEN: Copy = NonZeroUsize> {
    Empty,
    Filled { len: LEN },
}

/// Current states of the two buffers.
pub struct DoubleBufState<LEN: Copy = NonZeroUsize> {
    state0: Cell<BufState<LEN>>,
    state1: Cell<BufState<LEN>>,
}

impl<LEN: Copy> DoubleBufState<LEN> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            state0: Cell::new(BufState::Empty),
            state1: Cell::new(BufState::Empty),
        }
    }

    #[inline]
    pub fn get(&self, _cs: CriticalSection, which: bool) -> BufState<LEN> {
        if which {
            self.state1.get()
        } else {
            self.state0.get()
        }
    }

    #[inline]
    pub fn set(&self, _cs: CriticalSection, which: bool, state: BufState<LEN>) {
        if which {
            self.state1.set(state);
        } else {
            self.state0.set(state);
        }
    }

    #[inline]
    pub fn mark_filled(&self, cs: CriticalSection, which: bool, len: LEN) {
        self.set(cs, which, BufState::Filled { len });
    }

    #[inline]
    pub fn mark_empty(&self, cs: CriticalSection, which: bool) {
        self.set(cs, which, BufState::Empty);
    }

    /// Mark both buffers as empty.
    #[inline]
    pub fn clear(&self, cs: CriticalSection) {
        self.set(cs, false, BufState::Empty);
        self.set(cs, true, BufState::Empty);
    }
}

// Safety: all methods are guarded behind a critical section.
unsafe impl<LEN: Copy> Sync for DoubleBufState<LEN> {}
