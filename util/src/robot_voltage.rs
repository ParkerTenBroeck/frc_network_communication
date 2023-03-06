use std::fmt::Display;

use crate::{
    buffer_reader::{BufferReader, BufferReaderError, ReadFromBuff},
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
};

#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
pub struct RobotVoltage {
    pub int: u8,
    pub dec: u8,
}

impl RobotVoltage {
    pub fn from_f32(val: f32) -> Self {
        Self {
            int: (val - val.fract()) as u8,
            dec: (val.fract() * 255.0).round() as u8,
        }
    }

    pub fn to_f32(&self) -> f32 {
        self.int as f32 + self.dec as f32 / 255.0
    }
}

impl Display for RobotVoltage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(perc) = f.precision() {
            write!(f, "{:.perc$}v", self.to_f32())
        } else {
            write!(f, "{}v", self.to_f32())
        }
    }
}

impl<'a> ReadFromBuff<'a> for RobotVoltage {
    type Error = BufferReaderError;

    fn read_from_buff(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            int: buf.read_u8()?,
            dec: buf.read_u8()?,
        })
    }
}

impl<'a> WriteToBuff<'a> for RobotVoltage {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        buf.write_u8(self.dec)?;
        buf.write_u8(self.int)?;
        Ok(())
    }
}
