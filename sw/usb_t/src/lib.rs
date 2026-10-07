#![no_std]

#[cfg(not(target_endian = "little"))]
compile_error!("usb_t requires a little endian target.");

pub mod class;
pub mod desc;
pub mod setup;
