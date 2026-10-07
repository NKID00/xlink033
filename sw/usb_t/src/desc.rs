use core::slice::from_raw_parts;

use bitfield_struct::{bitenum, bitfield};

use crate::setup::Direction;

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dev {
    pub len: u8,
    pub desc_ty: u8,
    pub bcd_usb: u16,
    pub dev_cls: u8,
    pub dev_sub_cls: u8,
    pub dev_proto: u8,
    pub max_packet_size_ep0: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_dev: u16,
    pub i_manufacturer: u8,
    pub i_product: u8,
    pub i_serial_number: u8,
    pub num_configs: u8,
}

impl Dev {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = usb::descriptor_type::DEVICE;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    pub len: u8,
    pub desc_ty: u8,
    pub total_len: u16,
    pub num_ifs: u8,
    pub cfg_value: u8,
    pub i_cfg: u8,
    pub attr: DevAttr,
    pub max_power_in_2ma: u8,
}

impl Cfg {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = usb::descriptor_type::CONFIGURATION;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[bitfield(u8, order = Msb)]
#[derive(PartialEq, Eq)]
pub struct DevAttr {
    #[bits(default = true)]
    pub __reserved1: bool,
    pub self_powered: bool,
    pub remote_wakeup: bool,
    #[bits(5)]
    pub __reserved0: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str<const N: usize> {
    pub len: u8,
    pub desc_ty: u8,
    pub value: [u16; N],
}

impl<const N: usize> Str<N> {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = usb::descriptor_type::STRING;

    pub const fn from_value(value: [u16; N]) -> Self {
        Self {
            len: Self::LEN,
            desc_ty: Self::TYPE,
            value,
        }
    }

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct If {
    pub len: u8,
    pub desc_ty: u8,
    pub if_num: u8,
    pub alternate_setting: u8,
    pub num_eps: u8,
    pub if_cls: u8,
    pub if_sub_cls: u8,
    pub if_proto: u8,
    pub i_if: u8,
}

impl If {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = usb::descriptor_type::INTERFACE;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ep {
    pub len: u8,
    pub desc_ty: u8,
    pub ep_addr: EpAddr,
    pub attr: EpAttr,
    pub max_packet_size: MaxPacketSize,
    pub interval: u8,
}

impl Ep {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = usb::descriptor_type::ENDPOINT;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

#[bitfield(u8)]
#[derive(PartialEq, Eq)]
pub struct EpAddr {
    #[bits(4)]
    pub num: u8,
    #[bits(3)]
    pub __reserved0: u8,
    #[bits(1)]
    pub dir: Direction,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq)]
pub struct EpAttr {
    #[bits(2)]
    pub transfer_ty: TransferTy,
    #[bits(2)]
    pub sync_ty: SyncTy,
    #[bits(2)]
    pub usage_ty: UsageTy,
    #[bits(2)]
    pub __reserved0: u8,
}

#[bitenum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferTy {
    #[fallback]
    Control = 0b00,
    Isochronous = 0b01,
    Bulk = 0b10,
    Interrupt = 0b11,
}

#[bitenum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTy {
    #[fallback]
    NoSync = 0b00,
    Async = 0b01,
    Adaptive = 0b10,
    Sync = 0b11,
}

#[bitenum]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageTy {
    #[fallback]
    Data = 0b00,
    Feedback = 0b01,
    ImplicitFeedback = 0b10,
    __Reserved = 0b11,
}

#[bitfield(u16)]
#[derive(PartialEq, Eq)]
pub struct MaxPacketSize {
    #[bits(11)]
    pub transfer_ty: u16,
    #[bits(2)]
    pub additional_transactions: u8,
    #[bits(3)]
    pub __reserved0: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfAssoc {
    pub len: u8,
    pub desc_ty: u8,
    pub first_if: u8,
    pub if_count: u8,
    pub func_cls: u8,
    pub func_sub_cls: u8,
    pub func_proto: u8,
    pub i_func: u8,
}

impl IfAssoc {
    pub const LEN: u8 = size_of::<Self>() as u8;
    pub const TYPE: u8 = usb::descriptor_type::INTERFACE_ASSOCIATION;

    pub const fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}
