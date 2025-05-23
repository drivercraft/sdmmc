// ===== Types and Structures =====

use core::fmt;

#[derive(Debug)]
pub enum SdError {
    Timeout,
    Crc,
    EndBit,
    Index,
    DataTimeout,
    DataCrc,
    DataEndBit,
    BusPower,
    Acmd12Error,
    AdmaError,
    InvalidResponse,
    NoCard,
    UnsupportedCard,
    IoError,
    CommandError,
    TransferError,
    InvalidResponseType,
    CurrentLimit,
    DataError,
    TuningFailed,
    VoltageSwitchFailed,
    BadMessage,
    InvalidArgument,
    BufferOverflow,
    MemoryError,
    BusWidth,
    CardError(u32, &'static str), // 包含错误状态和描述
}

impl fmt::Display for SdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SdError::Timeout => write!(f, "Command timeout error"),
            SdError::Crc => write!(f, "Command CRC error"),
            SdError::EndBit => write!(f, "Command end bit error"),
            SdError::Index => write!(f, "Command index error"),
            SdError::DataTimeout => write!(f, "Data timeout error"),
            SdError::DataCrc => write!(f, "Data CRC error"),
            SdError::DataEndBit => write!(f, "Data end bit error"),
            SdError::BusPower => write!(f, "Bus power error"),
            SdError::Acmd12Error => write!(f, "ACMD12 error"),
            SdError::AdmaError => write!(f, "ADMA error"),
            SdError::InvalidResponse => write!(f, "Invalid response"),
            SdError::NoCard => write!(f, "No card detected"),
            SdError::UnsupportedCard => write!(f, "Unsupported card"),
            SdError::IoError => write!(f, "I/O error"),
            SdError::CommandError => write!(f, "Command error"),
            SdError::TransferError => write!(f, "Transfer error"),
            SdError::InvalidResponseType => write!(f, "Invalid response type"),
            SdError::CurrentLimit => write!(f, "Current limit error"),
            SdError::DataError => write!(f, "Data error"),
            SdError::TuningFailed => write!(f, "Tuning failed"),
            SdError::VoltageSwitchFailed => write!(f, "Voltage switch failed"),
            SdError::BadMessage => write!(f, "Bad message"),
            SdError::InvalidArgument => write!(f, "Invalid argument"),
            SdError::BufferOverflow => write!(f, "Buffer overflow"),
            SdError::MemoryError => write!(f, "Memory error"),
            SdError::BusWidth => write!(f, "Bus width error"),
            SdError::CardError(status, desc) => write!(f, "Card error: 0x{:X} ({})", status, desc),
        }
    }
}
