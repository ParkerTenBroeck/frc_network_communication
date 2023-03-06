pub mod error;

use std::borrow::Cow;

use util::{
    buffer_reader::{BufferReaderError, ReadFromBuff},
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
};

use self::error::{Errors, Warnings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Error(Errors, u8, u8, u8),
    Warning(Warnings, u8, u8, u8),
    Message,
    ZeroCode,
}

#[derive(Debug)]
pub struct Message<'a> {
    pub ms: u32,
    pub msg_num: u16,
    pub kind: MessageKind,
    pub message: Cow<'a, str>,
}

impl<'a> Message<'a> {
    pub fn info(message: impl Into<Cow<'a, str>>) -> Self {
        Self {
            ms: 0,
            msg_num: 0,
            kind: MessageKind::Message,
            message: message.into(),
        }
    }

    pub fn warn(message: impl Into<Cow<'a, str>>, warning: Warnings) -> Self {
        Self {
            ms: 0,
            msg_num: 0,
            kind: MessageKind::Warning(warning, 0, 0, 0),
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<Cow<'a, str>>, error: Errors) -> Self {
        Self {
            ms: 0,
            msg_num: 0,
            kind: MessageKind::Error(error, 0, 0, 0),
            message: message.into(),
        }
    }
}

impl Clone for Message<'_> {
    fn clone(&self) -> Self {
        Self {
            ms: self.ms,
            msg_num: self.msg_num,
            kind: self.kind,
            message: self.message.clone(),
        }
    }
}

#[derive(Debug)]
pub enum MessageReadError {
    BufferReaderError(BufferReaderError),
    InvalidDataValue,
}

impl From<BufferReaderError> for MessageReadError {
    fn from(value: BufferReaderError) -> Self {
        Self::BufferReaderError(value)
    }
}

impl<'a> ReadFromBuff<'a> for Message<'a> {
    type Error = MessageReadError;

    fn read_from_buff(
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<Self, Self::Error> {
        // the first parts of our message
        let msg_code = buf.read_u8()?;

        // if the message code is zero its some kind of "special"
        // message with no attached timing or numbering information (in binary)
        if msg_code == 0 {
            return Ok(Self {
                ms: 0,
                msg_num: 0,
                kind: MessageKind::ZeroCode,
                message: Cow::Borrowed(buf.read_str(buf.remaining_packet_data())?),
            });
        }

        // all packets start with a 32 bit timer value and 16 bit message number value
        let ms = buf.read_u32()?;
        let msg_num = buf.read_u16()?;

        let kind = match msg_code {
            0x0B => {
                //
                let should_be_one = buf.read_u16()?;
                if should_be_one != 1 {
                    return Err(MessageReadError::InvalidDataValue);
                }
                let err_num = buf.read_u32()? as i32;
                let _v1 = buf.read_u8()?;
                let _v2 = buf.read_u8()?;
                let _v3 = buf.read_u8()?;

                if err_num < 0 {
                    MessageKind::Error(Errors::from(err_num), _v1, _v2, _v3)
                } else {
                    MessageKind::Warning(Warnings::from(err_num), _v1, _v2, _v3)
                }
            }
            0x0C => MessageKind::Message,
            _ => {
                return Err(MessageReadError::InvalidDataValue);
            }
        };

        let message = buf.read_str(buf.remaining_packet_data())?;
        let message = Cow::Borrowed(message);

        Ok(Self {
            ms,
            msg_num,
            kind,
            message,
        })
    }
}

impl<'a> WriteToBuff<'a> for Message<'a> {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        match self.kind {
            MessageKind::ZeroCode => {
                buf.write_u8(0)?;
                buf.write_buf(self.message.as_bytes())?;
            }
            MessageKind::Message | MessageKind::Error(..) | MessageKind::Warning(..) => {
                if self.kind == MessageKind::Message {
                    buf.write_u8(0x0C)?;

                    buf.write_u32(self.ms)?;
                    buf.write_u16(self.msg_num)?;
                } else {
                    buf.write_u8(0x0B)?;

                    buf.write_u32(self.ms)?;
                    buf.write_u16(self.msg_num)?;
                    // unknown value but it seems to always be 1
                    buf.write_u16(1)?;
                    if let MessageKind::Error(ern, _v1, _v2, _v3) = self.kind {
                        buf.write_i32(ern.into())?;

                        buf.write_u8(_v1)?;
                        buf.write_u8(_v2)?;
                        buf.write_u8(_v3)?;
                    } else if let MessageKind::Warning(wrn, _v1, _v2, _v3) = self.kind {
                        buf.write_i32(wrn.into())?;

                        buf.write_u8(_v1)?;
                        buf.write_u8(_v2)?;
                        buf.write_u8(_v3)?;
                    }
                }
                // write our actual message
                buf.write_buf(self.message.as_bytes())?;
            }
        }
        Ok(())
    }
}
