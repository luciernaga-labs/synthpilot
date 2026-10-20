mod sysex;
pub mod yamaha;

pub use sysex::{END, START, Segment, SysExError, SysExMessage, hex};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProtocolError {
    #[error("device number {0} is outside 0..=15")]
    DeviceNumberOutOfRange(u8),
    #[error("address byte {0:#04X} is not a 7-bit value")]
    InvalidAddressByte(u8),
    #[error("data size {0} is not supported")]
    UnsupportedDataSize(usize),
    #[error("value {value} does not fit in {size} data byte(s)")]
    ValueTooLarge { value: u32, size: usize },
    #[error("data byte {0:#04X} is not a 7-bit value")]
    InvalidDataByte(u8),
    #[error("the message is not a recognized parameter change or bulk dump")]
    Unrecognized,
    #[error("the bulk dump declares {declared} data bytes but carries {actual}")]
    ByteCountMismatch { declared: usize, actual: usize },
    #[error("the bulk dump checksum does not match")]
    ChecksumMismatch,
    #[error(transparent)]
    SysEx(#[from] SysExError),
}
