pub mod error;

use std::borrow::Cow;

use util::{
    buffer_reader::{BufferReaderError, ReadFromBuff},
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
};

use self::error::{Errors, Warnings};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportKind<'a> {
    LibCVersion(Cow<'a, str>),
    ImageVersion(Cow<'a, str>),
    Empty(Cow<'a, str>),
}

impl<'a> ReportKind<'a> {
    pub fn get_tag(&self) -> &'static str {
        match self {
            ReportKind::LibCVersion(_) => "FRC_Lib_Version",
            ReportKind::ImageVersion(_) => "roboRIO Image",
            ReportKind::Empty(_) => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageKind<'a> {
    ZeroCode {
        msg: Cow<'a, str>,
    },
    Report {
        kind: ReportKind<'a>,
    },
    Message {
        ms: u32,
        msg_num: u16,
        msg: Cow<'a, str>,
    },
    Error {
        ms: u32,
        msg_num: u16,
        err: Errors,
        msg: Cow<'a, str>,
        loc: Cow<'a, str>,
        stack: Cow<'a, str>,
    },
    Warning {
        ms: u32,
        msg_num: u16,
        warn: Warnings,
        msg: Cow<'a, str>,
        loc: Cow<'a, str>,
        stack: Cow<'a, str>,
    },
    Unknown0x0D {
        disable_5v: u16,
        // when this is 2 the top row on the power/can metrics has a red underline
        top_signal: u8,
        // when this is 2 the second top row on the power/can metrics has a red underline
        second_top_signal: u8,
        // when this is 2 the third top row on the power/can metrics has a red underline
        third_top_signal: u8,
    },
}

impl<'a> MessageKind<'a> {
    fn get_code(&self) -> u8 {
        match self {
            MessageKind::ZeroCode { .. } => 0x00,
            MessageKind::Report { .. } => 0x0A,
            MessageKind::Error { .. } | MessageKind::Warning { .. } => 0x0B,
            MessageKind::Message { .. } => 0x0C,
            MessageKind::Unknown0x0D { .. } => 0x0D,
        }
    }
}

#[derive(Debug)]
pub struct Message<'a> {
    pub kind: MessageKind<'a>,
}

impl<'a> Message<'a> {
    pub fn info(message: impl Into<Cow<'a, str>>) -> Self {
        Self {
            kind: MessageKind::Message {
                ms: 0,
                msg_num: 0,
                msg: message.into(),
            },
        }
    }

    pub fn warn(
        message: impl Into<Cow<'a, str>>,
        warn: Warnings,
        location: impl Into<Cow<'a, str>>,
        stack: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            kind: MessageKind::Warning {
                ms: 0,
                msg_num: 0,
                warn,
                msg: message.into(),
                loc: location.into(),
                stack: stack.into(),
            },
        }
    }

    pub fn error(
        message: impl Into<Cow<'a, str>>,
        err: Errors,
        location: impl Into<Cow<'a, str>>,
        stack: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            kind: MessageKind::Error {
                ms: 0,
                msg_num: 0,
                err,
                msg: message.into(),
                loc: location.into(),
                stack: stack.into(),
            },
        }
    }

    pub fn set_ms(&mut self, time_ms: u32) {
        match &mut self.kind {
            MessageKind::Message { ms, .. }
            | MessageKind::Warning { ms, .. }
            | MessageKind::Error { ms, .. } => {
                *ms = time_ms;
            }
            _ => {}
        }
    }

    pub fn set_msg_num(&mut self, msg_number: u16) {
        match &mut self.kind {
            MessageKind::Message { msg_num, .. }
            | MessageKind::Warning { msg_num, .. }
            | MessageKind::Error { msg_num, .. } => {
                *msg_num = msg_number;
            }
            _ => {}
        }
    }
}

impl Clone for Message<'_> {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
        }
    }
}

#[derive(Debug)]
pub enum MessageReadError {
    BufferReaderError(BufferReaderError),
    InvalidDataValue,
    InvalidReportTag(String),
    ReportStartValueNonZero,
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
        println!("{:?}", buf.raw_buff());
        // tells us how to treat the rest of the data
        let msg_code = buf.read_u8()?;

        Ok(match msg_code {
            0x00 => Self {
                kind: MessageKind::ZeroCode {
                    msg: Cow::Borrowed(buf.read_str(buf.remaining_packet_data())?),
                },
            },

            0x0A => Self {
                kind: MessageKind::Report {
                    kind: {
                        let _zero = buf.read_u32()?;
                        if _zero != 0 {
                            Err(MessageReadError::ReportStartValueNonZero)?
                        }
                        let tag = buf.read_short_str()?;
                        match tag {
                            "roboRIO Image" => {
                                ReportKind::ImageVersion(Cow::Borrowed(buf.read_short_str()?))
                            }
                            "FRC_Lib_Version" => {
                                ReportKind::LibCVersion(Cow::Borrowed(buf.read_short_str()?))
                            }
                            "" => ReportKind::Empty(Cow::Borrowed(buf.read_short_str()?)),
                            _ => Err(MessageReadError::InvalidReportTag(tag.to_owned()))?,
                        }
                    },
                },
            },
            0x0B | 0x0C => {
                let ms = buf.read_u32()?;
                let msg_num = buf.read_u16()?;

                if msg_code == 0x0B {
                    let should_be_one = buf.read_u16()?;
                    if should_be_one != 1 {
                        return Err(MessageReadError::InvalidDataValue);
                    }
                    let err_num = buf.read_u32()? as i32;
                    let err = buf.read_u8()?;

                    let msg_len = buf.read_u16()?;

                    let message = buf.read_str(msg_len as usize)?;
                    let loc_len = buf.read_u16()?;
                    let location = buf.read_str(loc_len as usize)?;
                    let stack_len = buf.read_u16()?;
                    let stack = buf.read_str(stack_len as usize)?;

                    let kind = if err == 1 {
                        MessageKind::Error {
                            ms,
                            msg_num,
                            err: Errors::from(err_num),
                            msg: Cow::Borrowed(message),
                            loc: Cow::Borrowed(location),
                            stack: Cow::Borrowed(stack),
                        }
                    } else {
                        MessageKind::Warning {
                            ms,
                            msg_num,
                            warn: Warnings::from(err_num),
                            msg: Cow::Borrowed(message),
                            loc: Cow::Borrowed(location),
                            stack: Cow::Borrowed(stack),
                        }
                    };

                    Self { kind }

                // regular shmegular message
                } else {
                    let msg = buf.read_str(buf.remaining_packet_data())?;
                    Self {
                        kind: MessageKind::Message {
                            ms,
                            msg_num,
                            msg: Cow::Borrowed(msg),
                        },
                    }
                }
            }
            0x0D => Self {
                kind: MessageKind::Unknown0x0D {
                    disable_5v: buf.read_u16()?,
                    top_signal: buf.read_u8()?,
                    second_top_signal: buf.read_u8()?,
                    third_top_signal: buf.read_u8()?,
                },
            },
            _ => panic!("invalid msg_code: {msg_code}"),
        })
    }
}

impl<'a, 'm> WriteToBuff<'a> for Message<'m> {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        buf.write_u8(self.kind.get_code())?;

        // all these message types record the time in ms and the message number so just do it now
        match &self.kind {
            MessageKind::Message { ms, msg_num, .. }
            | MessageKind::Warning { ms, msg_num, .. }
            | MessageKind::Error { ms, msg_num, .. } => {
                buf.write_u32(*ms)?;
                buf.write_u16(*msg_num)?;
            }
            _ => {}
        }

        match &self.kind {
            MessageKind::ZeroCode { msg } => {
                buf.write_buf(msg.as_bytes())?;
            }
            MessageKind::Message { msg, .. } => {
                // we already wrote the time and msg number so just write the message
                buf.write_buf(msg.as_bytes())?;
            }
            MessageKind::Error {
                msg, loc, stack, ..
            }
            | MessageKind::Warning {
                msg, loc, stack, ..
            } => {
                // we already wrote the time and msg number so we just need to worry about the rest

                // unknown value but it seems to always be 1
                buf.write_u16(1)?;

                match &self.kind {
                    MessageKind::Error { err, .. } => {
                        buf.write_i32((*err).into())?;
                        buf.write_u8(1)?;
                    }
                    MessageKind::Warning { warn, .. } => {
                        buf.write_i32((*warn).into())?;
                        buf.write_u8(0)?;
                    }
                    _ => {}
                }

                buf.write_u16(msg.len() as u16)?;
                buf.write_buf(msg.as_bytes())?;
                buf.write_u16(loc.len() as u16)?;
                buf.write_buf(loc.as_bytes())?;
                buf.write_u16(stack.len() as u16)?;
                buf.write_buf(stack.as_bytes())?;
            }
            MessageKind::Report { kind } => {
                buf.write_u32(0)?;

                buf.write_short_str(kind.get_tag())?;
                let msg = match kind {
                    ReportKind::LibCVersion(val) => &**val,
                    ReportKind::ImageVersion(val) => &**val,
                    ReportKind::Empty(val) => &**val,
                };
                buf.write_short_str(msg)?;
            }
            MessageKind::Unknown0x0D {
                disable_5v,
                top_signal,
                second_top_signal,
                third_top_signal,
            } => {
                buf.write_u16(*disable_5v)?;
                buf.write_u8(*top_signal)?;
                buf.write_u8(*second_top_signal)?;
                buf.write_u8(*third_top_signal)?;
            }
        }
        Ok(())
    }
}
