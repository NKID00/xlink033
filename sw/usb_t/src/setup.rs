use bitfield_struct::{bitenum, bitfield};

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupRequest {
    pub req_ty: RequestTypeBits,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub len: u16,
}

impl SetupRequest {
    #[inline]
    pub unsafe fn from_ptr<'a, T>(ptr: *const T) -> &'a Self {
        unsafe { (ptr as *const Self).as_ref_unchecked() }
    }
}

#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
pub struct RequestTypeBits {
    #[bits(1)]
    pub direction: Direction,
    #[bits(2)]
    pub req_ty: RequestType,
    #[bits(5)]
    pub recipient: Recipient,
}

#[bitenum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    #[fallback]
    Out = 0,
    In = 1,
}

#[bitenum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    #[fallback]
    __Reserved = 3,
}

#[bitenum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
    #[fallback]
    __Reserved = 4,
}
