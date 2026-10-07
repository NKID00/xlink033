use core::slice::from_raw_parts;

use bitfield_struct::bitenum;

pub const SET_LINE_CODING: u8 = 0x20;

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineCoding {
    pub data_term_rate: u32,
    pub char_fmt: CharFmt,
    pub parity_ty: ParityTy,
    pub data_bits: u8,
}

impl LineCoding {
    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }

    pub unsafe fn from_ptr<'a>(ptr: *const u8) -> &'a Self {
        unsafe { (ptr as *const Self).as_ref_unchecked() }
    }
}

#[bitenum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharFmt {
    #[fallback]
    StopBits1 = 0,
    StopBits1AndHalf = 1,
    StopBits2 = 2,
}

#[bitenum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParityTy {
    #[fallback]
    None = 0,
    Odd = 1,
    Even = 2,
    Mark = 3,
    Space = 4,
}
