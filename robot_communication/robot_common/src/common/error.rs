use std::{error::Error, fmt::Display};

use util::{buffer_reader::BufferReaderError, buffer_writter::BufferWritterError};

#[derive(Debug)]
pub enum RobotPacketParseError {
    InvalidDataLength(usize),
    InvalidCommVersion(u8),
    InvalidControlCode(u8),
    InvalidRequestCode(u8),
    InvalidStationCode(u8),
    InvalidTimeData,
    InvalidJoystickData,
    InvalidTag(u8),
    GeneralError(Box<dyn Error + Send>),
    BufferTooSmall,
    MalformedData(&'static str)
}

impl Display for RobotPacketParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RobotPacketParseError::InvalidDataLength(len) => write!(f, "InvalidDataLength: {len}"),
            RobotPacketParseError::InvalidCommVersion(version) => {
                write!(f, "InvalidCommVersion: {version}")
            }
            RobotPacketParseError::InvalidControlCode(code) => {
                write!(f, "InvalidControlCode: {code:#02x}")
            }
            RobotPacketParseError::InvalidRequestCode(code) => {
                write!(f, "InvalidRequestCode: {code:#02x}")
            }
            RobotPacketParseError::InvalidStationCode(code) => {
                write!(f, "InvalidStationCode: {code:#02x}")
            }
            RobotPacketParseError::InvalidTimeData => write!(f, "InvalidTimeData"),
            RobotPacketParseError::InvalidJoystickData => write!(f, "InvalidJoystickData"),
            RobotPacketParseError::InvalidTag(tag) => write!(f, "InvalidTag: {tag}"),
            RobotPacketParseError::GeneralError(err) => write!(f, "GeneralError: {err}"),
            RobotPacketParseError::BufferTooSmall => write!(f, "BufferTooSmall"),
            RobotPacketParseError::MalformedData(mesg) => write!(f, "MalformedData: {mesg}"),
        }
    }
}

impl Error for RobotPacketParseError {}

impl From<BufferReaderError> for RobotPacketParseError {
    fn from(value: BufferReaderError) -> Self {
        match value {
            BufferReaderError::BufferReadOverflow {
                actual_buffer_length,
                ..
            } => RobotPacketParseError::InvalidDataLength(actual_buffer_length),
            BufferReaderError::GeneralError(err) => RobotPacketParseError::GeneralError(err),
        }
    }
}

impl From<BufferWritterError> for RobotPacketParseError{
    fn from(value: BufferWritterError) -> Self {
        match value {
            BufferWritterError::BufferTooSmall => Self::BufferTooSmall,
            BufferWritterError::InvalidData(value) => Self::MalformedData(value),
        }
    }
}