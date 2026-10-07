use core::slice::from_raw_parts;

use bitfield_struct::bitfield;

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub len: u8,
    pub desc_ty: u8,
    pub desc_sub_ty: u8,
    pub bcd_cdc: u16,
}

impl Header {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = 0x24;
    pub const SUB_TYPE: u8 = 0x00;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Union<const N: usize> {
    pub len: u8,
    pub desc_ty: u8,
    pub desc_sub_ty: u8,
    pub control_if: u8,
    pub subordinate_if: [u8; N],
}

impl<const N: usize> Union<N> {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = 0x24;
    pub const SUB_TYPE: u8 = 0x06;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallMgmt {
    pub len: u8,
    pub desc_ty: u8,
    pub desc_sub_ty: u8,
    pub cap: CallMgmtCap,
    pub data_if: u8,
}

impl CallMgmt {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = 0x24;
    pub const SUB_TYPE: u8 = 0x01;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
pub struct CallMgmtCap {
    #[bits(6)]
    pub __reserved0: u8,
    pub mgmt_over_data: bool,
    pub self_mgmt: bool,
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsCtrlMgmt {
    pub len: u8,
    pub desc_ty: u8,
    pub desc_sub_ty: u8,
    pub cap: AbsCtrlMgmtCap,
}

impl AbsCtrlMgmt {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = 0x24;
    pub const SUB_TYPE: u8 = 0x02;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
pub struct AbsCtrlMgmtCap {
    #[bits(4)]
    pub __reserved0: u8,
    pub net_conn: bool,
    pub send_brk: bool,
    pub line_state: bool,
    pub comm_feature: bool,
}
