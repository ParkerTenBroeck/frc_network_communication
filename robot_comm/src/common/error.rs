use std::{error::Error, fmt::Display};

use util::{buffer_reader::BufferReaderError, buffer_writter::BufferWritterError};

use super::joystick::JoystickParseError;

#[derive(Debug)]
pub enum RobotPacketParseError {
    BufferWriteError(BufferWritterError),
    BufferReaderError(BufferReaderError),
    // driver to robot
    DriverToRobotInvalidCommVersion(u8),
    InvalidControlCode(u8),
    InvalidRequestCode(u8),
    InvalidStationCode(u8),
    JoystickParseError(JoystickParseError),
    DriverToRobotInvalidExtraTag(u8),
    // robot to driver
    RobotToDriverInvalidCommVersion(u8),
    RobotToDriverInvalidUsageTag(u8),
    InvalidTimeZoneData,
    InvalidTimeData,
    
}

impl Display for RobotPacketParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for RobotPacketParseError {}

impl From<BufferReaderError> for RobotPacketParseError {
    fn from(value: BufferReaderError) -> Self {
        Self::BufferReaderError(value)
    }
}

impl From<BufferWritterError> for RobotPacketParseError {
    fn from(value: BufferWritterError) -> Self {
        Self::BufferWriteError(value)
    }
}

impl From<JoystickParseError> for RobotPacketParseError{
    fn from(value: JoystickParseError) -> Self {
        Self::JoystickParseError(value)
    }
}