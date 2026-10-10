use core::{
    num::NonZeroU8,
    sync::atomic::{AtomicI32, Ordering},
};

use constcat::concat_bytes;
use coro::UnsafeStatic;
use pac::{
    AFIO,
    Interrupt::{USBFS, USBFS_WKUP},
    usb::{
        Usbd,
        vals::{EpRxResponse, EpTxResponse, UsbToken},
    },
};
use qingke::{
    interrupt::Priority,
    pfic::{enable_interrupt, set_priority},
};
use qingke_rt::interrupt;
use static_buffers::{BufState, DoubleBufState, StaticBuf, StaticDoubleBuf};
use usb::{
    descriptor_type,
    standard_request::{GET_DESCRIPTOR, SET_ADDRESS, SET_CONFIGURATION},
};
use usb_t::{
    class::cdc::{
        self,
        desc::{AbsCtrlMgmtCap, CallMgmtCap},
    },
    desc::{self, DevAttr, EpAddr, EpAttr, MaxPacketSize, TransferTy},
    setup::{Direction, Recipient, RequestType, SetupRequest},
};
use utf16_lit::utf16;

use crate::{
    io::{self, lnk_off, lnk_on},
    uart,
    util::read_serial_number,
};

const DEVICE_DESC: desc::Dev = desc::Dev {
    len: desc::Dev::LEN,
    desc_ty: desc::Dev::TYPE,
    bcd_usb: 0x200,
    dev_cls: 0xef,
    dev_sub_cls: 0x02,
    dev_proto: 0x01,
    max_packet_size_ep0: 64,
    id_vendor: 0x0000,
    id_product: 0x0000,
    bcd_dev: 0x001,
    i_manufacturer: 1,
    i_product: 2,
    i_serial_number: 3,
    num_configs: 1,
};
const CONFIG_DESC: desc::Cfg = desc::Cfg {
    len: desc::Cfg::LEN,
    desc_ty: desc::Cfg::TYPE,
    total_len: 98,
    num_ifs: 3,
    cfg_value: 1,
    i_cfg: 0,
    attr: DevAttr::new()
        .with_self_powered(false)
        .with_remote_wakeup(false),
    max_power_in_2ma: 250,
};
// CMSIS-DAP: EP1 IN & OUT (double buffer, bulk)
const INTERFACE_0_DESC: desc::If = desc::If {
    len: desc::If::LEN,
    desc_ty: desc::If::TYPE,
    if_num: 0,
    alternate_setting: 0,
    num_eps: 2,
    if_cls: 0xff,
    if_sub_cls: 0x00,
    if_proto: 0x00,
    i_if: 4,
};
const ENDPOINT_1_IN_DESC: desc::Ep = desc::Ep {
    len: desc::Ep::LEN,
    desc_ty: desc::Ep::TYPE,
    ep_addr: EpAddr::new().with_num(1).with_dir(Direction::In),
    attr: EpAttr::new().with_transfer_ty(TransferTy::Bulk),
    max_packet_size: MaxPacketSize::new().with_transfer_ty(64),
    interval: 0,
};
const ENDPOINT_1_OUT_DESC: desc::Ep = desc::Ep {
    len: desc::Ep::LEN,
    desc_ty: desc::Ep::TYPE,
    ep_addr: EpAddr::new().with_num(1).with_dir(Direction::Out),
    attr: EpAttr::new().with_transfer_ty(TransferTy::Bulk),
    max_packet_size: MaxPacketSize::new().with_transfer_ty(64),
    interval: 0,
};
// CDC-ACM Data: EP2 IN & OUT (double buffer, bulk)
const IAD_CDC: desc::IfAssoc = desc::IfAssoc {
    len: desc::IfAssoc::LEN,
    desc_ty: desc::IfAssoc::TYPE,
    first_if: 1,
    if_count: 2,
    func_cls: 0x02,
    func_sub_cls: 0x00,
    func_proto: 0x00,
    i_func: 5,
};
const INTERFACE_1_DESC: desc::If = desc::If {
    len: desc::If::LEN,
    desc_ty: desc::If::TYPE,
    if_num: 1,
    alternate_setting: 0,
    num_eps: 2,
    if_cls: 0x0a,
    if_sub_cls: 0x00,
    if_proto: 0x00,
    i_if: 5,
};
const ENDPOINT_2_IN_DESC: desc::Ep = desc::Ep {
    len: desc::Ep::LEN,
    desc_ty: desc::Ep::TYPE,
    ep_addr: EpAddr::new().with_num(2).with_dir(Direction::In),
    attr: EpAttr::new().with_transfer_ty(TransferTy::Bulk),
    max_packet_size: MaxPacketSize::new().with_transfer_ty(64),
    interval: 0,
};
const ENDPOINT_2_OUT_DESC: desc::Ep = desc::Ep {
    len: desc::Ep::LEN,
    desc_ty: desc::Ep::TYPE,
    ep_addr: EpAddr::new().with_num(2).with_dir(Direction::Out),
    attr: EpAttr::new().with_transfer_ty(TransferTy::Bulk),
    max_packet_size: MaxPacketSize::new().with_transfer_ty(64),
    interval: 0,
};
// CDC-ACM Comm: EP3 IN (interrupt)
const INTERFACE_2_DESC: desc::If = desc::If {
    len: desc::If::LEN,
    desc_ty: desc::If::TYPE,
    if_num: 2,
    alternate_setting: 0,
    num_eps: 1,
    if_cls: 0x02,
    if_sub_cls: 0x02,
    if_proto: 0x00,
    i_if: 5,
};
const CDC_HEADER_DESC: cdc::desc::Header = cdc::desc::Header {
    len: cdc::desc::Header::LEN,
    desc_ty: cdc::desc::Header::TYPE,
    desc_sub_ty: cdc::desc::Header::SUB_TYPE,
    bcd_cdc: 0x120,
};
const CDC_CALL_MGMT_DESC: cdc::desc::CallMgmt = cdc::desc::CallMgmt {
    len: cdc::desc::CallMgmt::LEN,
    desc_ty: cdc::desc::CallMgmt::TYPE,
    desc_sub_ty: cdc::desc::CallMgmt::SUB_TYPE,
    cap: CallMgmtCap::new().with_mgmt_over_data(true),
    data_if: 1,
};
const CDC_ABS_CTRL_MGMT_DESC: cdc::desc::AbsCtrlMgmt = cdc::desc::AbsCtrlMgmt {
    len: cdc::desc::AbsCtrlMgmt::LEN,
    desc_ty: cdc::desc::AbsCtrlMgmt::TYPE,
    desc_sub_ty: cdc::desc::AbsCtrlMgmt::SUB_TYPE,
    cap: AbsCtrlMgmtCap::new(),
};
const CDC_UNION_DESC: cdc::desc::Union<1> = cdc::desc::Union::<1> {
    len: cdc::desc::Union::<1>::LEN,
    desc_ty: cdc::desc::Union::<1>::TYPE,
    desc_sub_ty: cdc::desc::Union::<1>::SUB_TYPE,
    control_if: 2,
    subordinate_if: [1],
};
const ENDPOINT_3_IN_DESC: desc::Ep = desc::Ep {
    len: desc::Ep::LEN,
    desc_ty: desc::Ep::TYPE,
    ep_addr: EpAddr::new().with_num(3).with_dir(Direction::In),
    attr: EpAttr::new().with_transfer_ty(TransferTy::Interrupt),
    max_packet_size: MaxPacketSize::new().with_transfer_ty(64),
    interval: 255,
};
const CONFIGURATION: &[u8] = concat_bytes!(
    CONFIG_DESC.as_slice(),
    INTERFACE_0_DESC.as_slice(),
    ENDPOINT_1_IN_DESC.as_slice(),
    ENDPOINT_1_OUT_DESC.as_slice(),
    IAD_CDC.as_slice(),
    INTERFACE_1_DESC.as_slice(),
    ENDPOINT_2_IN_DESC.as_slice(),
    ENDPOINT_2_OUT_DESC.as_slice(),
    INTERFACE_2_DESC.as_slice(),
    CDC_HEADER_DESC.as_slice(),
    CDC_CALL_MGMT_DESC.as_slice(),
    CDC_ABS_CTRL_MGMT_DESC.as_slice(),
    CDC_UNION_DESC.as_slice(),
    ENDPOINT_3_IN_DESC.as_slice(),
);
// rust-analyzer doesn't like #![feature(const_block_items)]
const _: () = const { assert!(CONFIG_DESC.total_len as usize == CONFIGURATION.len()) };

const LANG_ID_ARRAY: desc::Str<1> = desc::Str::from_value([0x0409]);
const MANUFACTURER_STR: desc::Str<12> = desc::Str::from_value(utf16!("Example Labs"));
const PRODUCT_STR: desc::Str<18> = desc::Str::from_value(utf16!("xlink033 CMSIS-DAP"));
static SERIAL_NUMBER_STR: UnsafeStatic<desc::Str<24>> = UnsafeStatic::uninit();
const INTERFACE_1_STR: desc::Str<9> = desc::Str::from_value(utf16!("CMSIS-DAP"));
const INTERFACE_2_STR: desc::Str<24> = desc::Str::from_value(utf16!("CDC-ACM UART Transceiver"));
const STRINGS: &[&[u8]] = &[
    LANG_ID_ARRAY.as_slice(),
    MANUFACTURER_STR.as_slice(),
    PRODUCT_STR.as_slice(),
    &[],
    INTERFACE_1_STR.as_slice(),
    INTERFACE_2_STR.as_slice(),
];

const USBFSD: Usbd = unsafe { Usbd::from_ptr(pac::USBFS.as_ptr()) };

pub static EP0_BUF: StaticBuf<64> = StaticBuf::new(); // IN & OUT
pub static EP1_BUF: [StaticDoubleBuf<64>; 2] = [StaticDoubleBuf::new(), StaticDoubleBuf::new()]; // OUT0, OUT1, IN0, IN1
pub static EP2_BUF: [StaticDoubleBuf<64>; 2] = [StaticDoubleBuf::new(), StaticDoubleBuf::new()]; // OUT0, OUT1, IN0, IN1
pub static EP3_BUF: StaticBuf<64> = StaticBuf::new(); // IN
pub const EP1_BUF_OUT: &'static StaticDoubleBuf<64> = &EP1_BUF[0];
pub const EP1_BUF_IN: &'static StaticDoubleBuf<64> = &EP1_BUF[1];
pub const EP2_BUF_OUT: &'static StaticDoubleBuf<64> = &EP2_BUF[0];
pub const EP2_BUF_IN: &'static StaticDoubleBuf<64> = &EP2_BUF[1];
pub static EP1_OUT_STATE: DoubleBufState<NonZeroU8> = DoubleBufState::new();
pub static EP1_IN_STATE: DoubleBufState<NonZeroU8> = DoubleBufState::new();
pub static EP2_OUT_STATE: DoubleBufState<NonZeroU8> = DoubleBufState::new();
pub static EP2_IN_STATE: DoubleBufState<NonZeroU8> = DoubleBufState::new();

#[interrupt]
fn USBFS() {
    let flags = USBFSD.int_fg().read();
    if flags.transfer() {
        let packet = parse_packet();
        match packet {
            packet if packet.ep() == 0 => unsafe { SETUP_STATE_MACHINE.wrapping_resume(packet) },
            UsbPacket::In { ep: 1, toggle } => {}
            UsbPacket::Out { ep: 1, toggle, len } => {}
            UsbPacket::In { ep: 2, toggle } => {
                critical_section::with(|cs| {
                    uart::on_ep2_in(cs, toggle);
                });
            }
            UsbPacket::Out { ep: 2, toggle, len } => {
                critical_section::with(|cs| {
                    // static COUNTER: AtomicI32 = AtomicI32::new(0);
                    // let counter = COUNTER.load(Ordering::SeqCst) + 1;
                    // COUNTER.store(counter, Ordering::SeqCst);
                    // io::act_on();
                    // match counter {
                    //     1 => {
                    //         assert!(toggle == false);
                    //         assert!(len == 8);
                    //         assert!(
                    //             unsafe { &EP2_BUF_OUT.as_ref(toggle)[..len as usize] }
                    //                 == [0, 1, 2, 3, 4, 5, 6, 7]
                    //         );
                    //         assert!(EP2_OUT_STATE.get(cs, toggle) == BufState::Empty);
                    //         assert!(EP2_OUT_STATE.get(cs, !toggle) == BufState::Empty);
                    //     }
                    //     2 => {
                    //         assert!(toggle == true);
                    //         assert!(len == 1);
                    //         assert!(unsafe { &EP2_BUF_OUT.as_ref(toggle)[..len as usize] } == [8]);
                    //         assert!(EP2_OUT_STATE.get(cs, toggle) == BufState::Empty);
                    //         assert!(
                    //             EP2_OUT_STATE.get(cs, !toggle)
                    //                 == BufState::Filled {
                    //                     len: NonZeroU8::new(8).unwrap()
                    //                 }
                    //         );
                    //     }
                    //     3 => {
                    //         assert!(toggle == false);
                    //         assert!(len == 1);
                    //         assert!(unsafe { &EP2_BUF_OUT.as_ref(toggle)[..len as usize] } == [9]);
                    //         assert!(EP2_OUT_STATE.get(cs, toggle) == BufState::Empty);
                    //         assert!(EP2_OUT_STATE.get(cs, !toggle) == BufState::Empty);
                    //     }
                    //     _ => {}
                    // }
                    // io::act_off();
                    // if len == 0 {
                    //     panic!();
                    // }
                    if len > 0 {
                        assert!(EP2_OUT_STATE.get(cs, toggle) == BufState::Empty);
                        EP2_OUT_STATE.mark_filled(cs, toggle, NonZeroU8::new(len).unwrap());
                    }
                    if EP2_OUT_STATE.get(cs, !toggle) == BufState::Empty {
                        // dma is idle, start new transfer
                        if len > 0 {
                            // io::lnk_off();
                            uart::start_tx_dma(toggle, len);
                        }
                        prepare_out_ack_flip_toggle::<2>();
                    } else {
                        // io::act_flash();
                        prepare_out_nak_flip_toggle::<2>();
                    }
                });
            }
            UsbPacket::In { ep: 3, toggle } => prepare_in_nak_flip_toggle::<3>(),
            _ => {}
        }
    }
    if flags.suspend() {}
    if flags.bus_rst() {
        io::lnk_flash();
        reset();
    }
    USBFSD.int_fg().write_value(flags);
}

#[interrupt]
fn USBFS_WKUP() {}

static SETUP_STATE_MACHINE: UnsafeStatic<SetupStateMachine> = UnsafeStatic::uninit();

#[coro::yields]
fn SetupStateMachine(packet: UsbPacket) {
    use UsbPacket::*;
    let Setup { req } = packet else {
        return prepare_stall::<0>();
    };
    match req.req_ty.direction() {
        Direction::In => {
            let result: Result<&'static [u8], ()> =
                match (req.req_ty.req_ty(), req.req_ty.recipient(), req.request) {
                    (RequestType::Standard, Recipient::Device, GET_DESCRIPTOR) => {
                        let desc_ty = (req.value >> 8) as u8;
                        let desc_index = req.value as u8;
                        let _lang_id = req.index;
                        match desc_ty {
                            descriptor_type::DEVICE => Ok(DEVICE_DESC.as_slice()),
                            descriptor_type::CONFIGURATION => Ok(&CONFIGURATION),
                            descriptor_type::STRING if desc_index == 3 => {
                                Ok(unsafe { SERIAL_NUMBER_STR.as_ref() }.as_slice())
                            }
                            descriptor_type::STRING if desc_index <= 6 => {
                                Ok(STRINGS[desc_index as usize])
                            }
                            _ => Err(()),
                        }
                    }
                    _ => Err(()),
                };
            match result {
                Ok(data) => {
                    let mut data = &data[..data.len().min(req.len as usize)];
                    // data stage starts with IN1
                    let mut toggle = true;
                    while data.len() > 0 {
                        let packet_len = data.len().min(64);
                        (unsafe { EP0_BUF.as_mut() }[..packet_len])
                            .copy_from_slice(&data[..packet_len]);
                        data = &data[packet_len..];
                        prepare_in_ack_with_toggle::<0>(toggle, packet_len);
                        toggle = !toggle;
                        let packet = yield;
                        let In { .. } = packet else {
                            return prepare_stall::<0>();
                        };
                    }
                    // OUT1 zero-length status stage
                    prepare_out_ack_with_toggle::<0>(true);
                    yield;
                    prepare_out_nak::<0>();
                }
                Err(()) => {
                    return prepare_stall::<0>();
                }
            }
        }
        Direction::Out => {
            if req.len > 64 {
                // current implementation only accepts single data packet
                return prepare_stall::<0>();
            }
            let data: &'static [u8] = if req.len > 0 {
                // OUT1 data stage
                prepare_out_ack_with_toggle::<0>(true);
                let packet = yield;
                let Out { len, .. } = packet else {
                    return prepare_stall::<0>();
                };
                if len as u16 != req.len {
                    return prepare_stall::<0>();
                }
                unsafe { &EP0_BUF.as_ref()[..len as usize] }
            } else {
                &[]
            };
            match (req.req_ty.req_ty(), req.req_ty.recipient(), req.request) {
                (RequestType::Standard, Recipient::Device, SET_ADDRESS) => {}
                (RequestType::Standard, Recipient::Device, SET_CONFIGURATION) => {
                    prepare_out_ack::<1>();
                    prepare_out_ack::<2>();
                }
                (RequestType::Class, Recipient::Interface, _) if req.index == 2 => {
                    if let Err(()) = uart::on_ctrl_write(&req, data) {
                        return prepare_stall::<0>();
                    }
                }
                _ => return prepare_stall::<0>(),
            };

            // IN1 zero-length status stage
            prepare_in_ack_with_toggle::<0>(true, 0);
            yield;
            prepare_in_nak::<0>();

            if req.request == SET_ADDRESS {
                USBFSD
                    .dev_ad()
                    .write(|w| w.set_mask_usb_addr(req.value as u8));
            }
        }
    }
}

enum UsbPacket {
    In { ep: u8, toggle: bool },
    Out { ep: u8, toggle: bool, len: u8 },
    Setup { req: SetupRequest },
    Err { ep: u8 },
}

impl UsbPacket {
    fn ep(&self) -> u8 {
        use UsbPacket::*;
        match self {
            In { ep, .. } => *ep,
            Out { ep, .. } => *ep,
            Setup { .. } => 0,
            Err { ep } => *ep,
        }
    }
}

fn parse_packet() -> UsbPacket {
    use UsbPacket::*;
    use UsbToken::*;
    let status = USBFSD.int_st().read();
    // bit 8 is SETUP_ACT
    // status.0 & (1 << 7) != 0 &&
    if status.mask_token() == SETUP {
        let req = unsafe { SetupRequest::from_ptr(EP0_BUF.as_ptr()) };
        // immutable borrow to the ep0 tx/rx shared buffer must be dropped
        // because we will soon overwrite next packet to the buffer
        return Setup { req: req.clone() };
    }
    let ep = status.mask_uis_endp();
    if ep > 3 || !status.tog_ok() {
        return Err { ep };
    }
    match status.mask_token() {
        IN => In {
            ep,
            toggle: get_in_toggle(ep as usize),
        },
        OUT => Out {
            ep,
            toggle: get_out_toggle(ep as usize),
            len: USBFSD.rx_len().read().rx_len() as u8,
        },
        _ => Err { ep },
    }
}

#[inline]
pub fn get_in_toggle(ep: usize) -> bool {
    USBFSD.uep01234_ctrl(ep).read().t_tog()
}

#[inline]
pub fn get_out_toggle(ep: usize) -> bool {
    USBFSD.uep01234_ctrl(ep).read().r_tog()
}

#[inline]
pub fn prepare_in_ack<const EP: usize>(len: usize) {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_t_len(EP).write(|w| w.set_t_len(len as u8));
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_t_res(EpTxResponse::ACK);
    });
}

#[inline]
pub fn prepare_out_ack<const EP: usize>() {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_r_res(EpRxResponse::ACK);
    });
}

#[inline]
pub fn prepare_in_ack_flip_toggle<const EP: usize>(len: usize) {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_t_len(EP).write(|w| w.set_t_len(len as u8));
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_t_tog(!w.t_tog());
        w.set_t_res(EpTxResponse::ACK);
    });
}

#[inline]
pub fn prepare_out_ack_flip_toggle<const EP: usize>() {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_r_tog(!w.r_tog());
        w.set_r_res(EpRxResponse::ACK);
    });
}

#[inline]
pub fn prepare_in_ack_with_toggle<const EP: usize>(toggle: bool, len: usize) {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_t_len(EP).write(|w| w.set_t_len(len as u8));
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_t_tog(toggle);
        w.set_t_res(EpTxResponse::ACK);
    });
}

#[inline]
pub fn prepare_out_ack_with_toggle<const EP: usize>(toggle: bool) {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_r_tog(toggle);
        w.set_r_res(EpRxResponse::ACK);
    });
}

#[inline]
pub fn prepare_nak<const EP: usize>() {
    prepare_in_nak::<EP>();
    prepare_out_nak::<EP>();
}

#[inline]
pub fn prepare_in_nak<const EP: usize>() {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_t_res(EpTxResponse::NAK);
    });
}

#[inline]
pub fn prepare_out_nak<const EP: usize>() {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_r_res(EpRxResponse::NAK);
    });
}

#[inline]
pub fn prepare_in_nak_flip_toggle<const EP: usize>() {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_t_tog(!w.t_tog());
        w.set_t_res(EpTxResponse::NAK);
    });
}

#[inline]
pub fn prepare_out_nak_flip_toggle<const EP: usize>() {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_r_tog(!w.r_tog());
        w.set_r_res(EpRxResponse::NAK);
    });
}

#[inline]
pub fn prepare_stall<const EP: usize>() {
    prepare_in_stall::<EP>();
    prepare_out_stall::<EP>();
}

#[inline]
pub fn prepare_in_stall<const EP: usize>() {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_t_len(EP).write(|w| w.set_t_len(0));
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_t_res(EpTxResponse::STALL);
    });
}

#[inline]
pub fn prepare_out_stall<const EP: usize>() {
    const { assert!(EP <= 3) };
    USBFSD.uep01234_ctrl(EP).modify(|w| {
        w.set_r_res(EpRxResponse::STALL);
    });
}

pub fn init() {
    // enable USBFS
    USBFSD.ctrl().write(|w| {
        w.set_host_mode(false);
        w.set_dev_pu_en(true);
        w.set_int_busy(true);
        w.set_reset_sie(false);
        w.set_clr_all(false);
    });

    // setup endpoints
    USBFSD.uep4_1_mod().write(|w| {
        w.set_tx_en(1, true);
        w.set_rx_en(1, true);
        w.set_buf_mod(1, true);
    });
    USBFSD.uep2_3_mod().write(|w| {
        w.set_tx_en(0, true);
        w.set_rx_en(0, true);
        w.set_buf_mod(0, true);
    });
    USBFSD
        .uep0123_dma(0)
        .write(|w| w.set_buf_addr(EP0_BUF.get() as u16));
    USBFSD
        .uep0123_dma(1)
        .write(|w| w.set_buf_addr(EP1_BUF[0].base() as u16));
    USBFSD
        .uep0123_dma(2)
        .write(|w| w.set_buf_addr(EP2_BUF[0].base() as u16));
    USBFSD
        .uep0123_dma(3)
        .write(|w| w.set_buf_addr(EP3_BUF.get() as u16));

    USBFSD.udev_ctrl().write(|w| {
        w.set_pd_dis(true); // disable pull-down
        w.set_port_en(true); // connect transceiver
    });
    USBFSD.ctrl().modify(|w| w.set_dma_en(true));

    // prepare serial number string
    let sn = read_serial_number();
    let mut value = [0; 24];
    for (nibble, hex) in sn
        .into_iter()
        .flat_map(|byte| [byte & 0xf, byte >> 4])
        .rev()
        .zip(value.iter_mut())
    {
        *hex = if nibble < 10 {
            '0' as u16 + nibble as u16
        } else {
            'A' as u16 + nibble as u16 - 10
        };
    }
    unsafe {
        SERIAL_NUMBER_STR.init(desc::Str::from_value(value));
    }

    unsafe { SETUP_STATE_MACHINE.init_default() };

    reset();

    // enable interrupts
    USBFSD.int_en().write(|w| {
        w.set_suspend(true);
        w.set_transfer(true);
        w.set_bus_rst(true);
    });
    unsafe {
        set_priority(USBFS as u8, Priority::P1 as u8);
        set_priority(USBFS_WKUP as u8, Priority::P2 as u8);
        enable_interrupt(USBFS as u8);
        enable_interrupt(USBFS_WKUP as u8);
    }

    // enable 1.5K pull-up resistor on D+ (PC17)
    AFIO.ctlr().modify(|w| w.set_udp_pue(0b11));
}

fn reset() {
    critical_section::with(|cs| {
        // reset endpoints
        for i in 0..=3 {
            USBFSD.uep01234_t_len(i).write(|w| w.set_t_len(0));
            USBFSD.uep01234_ctrl(i).write(|w| {
                w.set_r_tog(false);
                w.set_r_res(EpRxResponse::NAK);
                w.set_t_tog(false);
                w.set_t_res(EpTxResponse::NAK);
            });
        }
        // reset device address to 0
        USBFSD.dev_ad().write(|w| w.set_mask_usb_addr(0));
        // reset state machine
        unsafe { SETUP_STATE_MACHINE.reset() };
        EP1_IN_STATE.clear(cs);
        EP1_OUT_STATE.clear(cs);
        EP2_IN_STATE.clear(cs);
        EP2_OUT_STATE.clear(cs);
        // reset fifo and interrupt flags
        USBFSD.ctrl().modify(|w| {
            w.set_clr_all(true);
        });
        USBFSD.ctrl().modify(|w| {
            w.set_clr_all(false);
        });
    });
}
