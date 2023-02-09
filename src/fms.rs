use std::{error::Error, fmt::Display};

use super::util::{BufferReader, BufferReaderError, ReadFromBuff};

#[derive(Debug)]
pub struct FMSPacket {
    pub packet_count: u16,
    pub ds_version: u8,
    pub fms_control: u8,
    pub team_number: TeamNumber,
    pub robot_voltage: RobotVoltage,
}

#[derive(Debug)]
pub enum FMSPacketParseError {
    InvalidDataLength,
    InvalidData,
}

impl Display for FMSPacketParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<BufferReaderError> for FMSPacketParseError {
    fn from(_: BufferReaderError) -> Self {
        Self::InvalidDataLength
    }
}

impl Error for FMSPacketParseError {}

impl<'a> ReadFromBuff<'a> for FMSPacket {
    type Error = FMSPacketParseError;

    fn read_from_buff(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            packet_count: buf.read_u16()?,
            ds_version: buf.read_u8()?,
            fms_control: buf.read_u8()?,
            team_number: TeamNumber(buf.read_u16()?),
            robot_voltage: RobotVoltage {
                int: buf.read_u8()?,
                dec: buf.read_u8()?,
            },
        })
    }
}