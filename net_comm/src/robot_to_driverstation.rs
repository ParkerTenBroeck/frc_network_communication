pub mod error;

use std::borrow::Cow;

use util::{
    buffer_reader::{BufferReaderError, CreateFromBuf, ReadFromBuf},
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
    team_number::TeamNumber,
};

use self::error::{Errors, Warnings};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionInfo<'a> {
    LibCVersion(Cow<'a, str>),
    ImageVersion(Cow<'a, str>),
    CANTalon(u16, u8),
    PDP(u16, u8),
    PCM(u16, u8),
    Empty(Cow<'a, str>),
}

impl<'a> VersionInfo<'a> {
    pub fn get_tag(&self) -> &'static str {
        match self {
            VersionInfo::LibCVersion(_) => "FRC_Lib_Version",
            VersionInfo::ImageVersion(_) => "roboRIO Image",
            VersionInfo::Empty(_)
            | VersionInfo::CANTalon(..)
            | VersionInfo::PDP(..)
            | VersionInfo::PCM(..) => "",
        }
    }

    fn device_id(&self) -> u8 {
        match self {
            VersionInfo::LibCVersion(_) | VersionInfo::ImageVersion(_) => 0,
            VersionInfo::CANTalon(..) => 2,
            VersionInfo::PDP(..) => 8,
            VersionInfo::PCM(..) => 9,
            VersionInfo::Empty(_) => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageKind<'a> {
    ZeroCode {
        msg: Cow<'a, str>,
    },
    VersionInfo {
        kind: VersionInfo<'a>,
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
    UnderlineAnd5VDisable {
        disable_5v: u16,
        /// when this is 2 the top row on the power/can metrics has a red underline
        top_signal: u8,
        /// when this is 2 the second top row on the power/can metrics has a red underline
        second_top_signal: u8,
        /// when this is 2 the third top row on the power/can metrics has a red underline
        third_top_signal: u8,
    },
    DisableFaults {
        comms: u16,
        fault_12v: u16,
    },
    RailFaults {
        short_6v: u16,
        short_5v: u16,
        short_3_3v: u16,
    },
    UsageReport {
        team: TeamNumber,
        unknwon: u8,
        usage: (),
    },
}

struct Test<'a> {
    data: &'a u8,
}

pub enum MessageKindBorrowed<'a> {
    ZeroCode {
        msg: &'a str,
    },
    VersionInfo {
        // kind: VersionInfo<'a>,
    },
    Message {
        ms: u32,
        msg_num: u16,
        msg: &'a str,
    },
    Error {
        ms: u32,
        msg_num: u16,
        err: u32,
        msg: &'a str,
        loc: &'a str,
        stack: &'a str,
    },
    // Warning {
    //     ms: u32,
    //     msg_num: u16,
    //     warn: u32,
    //     msg: &'a str,
    //     loc: &'a str,
    //     stack: &'a str,
    // },
    UnderlineAnd5VDisable {
        disable_5v: u16,
        /// when this is 2 the top row on the power/can metrics has a red underline
        top_signal: u8,
        /// when this is 2 the second top row on the power/can metrics has a red underline
        second_top_signal: u8,
        /// when this is 2 the third top row on the power/can metrics has a red underline
        third_top_signal: u8,
    },
    DisableFaults {
        comms: u16,
        fault_12v: u16,
    },
    RailFaults {
        short_6v: u16,
        short_5v: u16,
        short_3_3v: u16,
    },
    UsageReport {
        team: TeamNumber,
        unknwon: u8,
        usage: (),
    },
}

impl<'a> MessageKind<'a> {
    fn get_code(&self) -> u8 {
        match self {
            MessageKind::ZeroCode { .. } => 0x00,
            MessageKind::UsageReport { .. } => 0x01,
            MessageKind::DisableFaults { .. } => 0x04,
            MessageKind::RailFaults { .. } => 0x05,
            MessageKind::VersionInfo { .. } => 0x0A,
            MessageKind::Error { .. } | MessageKind::Warning { .. } => 0x0B,
            MessageKind::Message { .. } => 0x0C,
            MessageKind::UnderlineAnd5VDisable { .. } => 0x0D,
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
    InvalidMsgCode(u8),
    InvalidVersionDeviceTag(u8),
}

impl From<BufferReaderError> for MessageReadError {
    fn from(value: BufferReaderError) -> Self {
        Self::BufferReaderError(value)
    }
}

impl<'a> ReadFromBuf<'a> for Message<'a> {
    type Error = MessageReadError;

    fn read_into_from_buf(
        &mut self,
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<&mut Self, Self::Error> {
        todo!()
    }
}

impl<'a> CreateFromBuf<'a> for Message<'a> {
    fn create_from_buf(
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<Self, Self::Error> {
        // tells us how to treat the rest of the data
        let msg_code = buf.read_u8()?;

        let ok = Ok(match msg_code {
            0x00 => Self {
                kind: MessageKind::ZeroCode {
                    msg: Cow::Borrowed(buf.read_str(buf.remaining_buf_len())?),
                },
            },
            0x04 => Self {
                kind: MessageKind::DisableFaults {
                    comms: buf.read_u16()?,
                    fault_12v: buf.read_u16()?,
                },
            },
            0x05 => Self {
                kind: MessageKind::RailFaults {
                    short_6v: buf.read_u16()?,
                    short_5v: buf.read_u16()?,
                    short_3_3v: buf.read_u16()?,
                },
            },

            0x0A => Self {
                kind: MessageKind::VersionInfo {
                    kind: {
                        let device_id = buf.read_u8()?;

                        match device_id {
                            0x00 => {
                                buf.assert_n_zero(3)?;
                                let tag = buf.read_short_str()?;
                                match tag {
                                    "roboRIO Image" => VersionInfo::ImageVersion(Cow::Borrowed(
                                        buf.read_short_str()?,
                                    )),
                                    "FRC_Lib_Version" => VersionInfo::LibCVersion(Cow::Borrowed(
                                        buf.read_short_str()?,
                                    )),
                                    "" => VersionInfo::Empty(Cow::Borrowed(buf.read_short_str()?)),
                                    _ => Err(MessageReadError::InvalidReportTag(tag.to_owned()))?,
                                }
                            }
                            2 => {
                                let idk = buf.read_u16()?;
                                //maybe the can id ??
                                let can_id = buf.read_u8()?;
                                buf.assert_n_zero(2)?;
                                buf.assert_empty()?;

                                
                                VersionInfo::CANTalon(idk, can_id)
                            }
                            8 => {
                                let idk = buf.read_u16()?;
                                //maybe the can id ??
                                let can_id = buf.read_u8()?;
                                buf.assert_empty()?;
                                VersionInfo::PDP(idk, can_id)
                            }
                            9 => {
                                let idk = buf.read_u16()?;
                                //maybe the can id ??
                                let can_id = buf.read_u8()?;
                                buf.assert_n_zero(2)?;
                                buf.assert_empty()?;
                                VersionInfo::PCM(idk, can_id)
                            }
                            _ => Err(MessageReadError::InvalidVersionDeviceTag(device_id))?,
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
                    let msg = buf.read_str(buf.remaining_buf_len())?;
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
                kind: MessageKind::UnderlineAnd5VDisable {
                    disable_5v: buf.read_u16()?,
                    top_signal: buf.read_u8()?,
                    second_top_signal: buf.read_u8()?,
                    third_top_signal: buf.read_u8()?,
                },
            },
            _ => {
                println!("{:?}", buf.read_amount(buf.remaining_buf_len())?);
                Err(MessageReadError::InvalidMsgCode(msg_code))?
            }
        });
        buf.assert_empty()?;
        ok
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
            MessageKind::VersionInfo { kind } => {
                // buf.write_u32(0)?;
                buf.write_u8(kind.device_id())?;

                match kind {
                    VersionInfo::LibCVersion(msg) | VersionInfo::ImageVersion(msg) => {
                        buf.write_u16(0)?;
                        buf.write_u8(0)?;
                        buf.write_short_str(kind.get_tag())?;
                        buf.write_short_str(msg)?;
                    }
                    VersionInfo::CANTalon(idk, can_id)
                    | VersionInfo::PDP(idk, can_id)
                    | VersionInfo::PCM(idk, can_id) => {
                        buf.write_u16(*idk)?;
                        buf.write_u8(*can_id)?;
                        //tags and strings
                        buf.write_u16(0)?;
                    }
                    VersionInfo::Empty(str) => {
                        // this shidz emptyyy
                        buf.write(4)?.fill(0);
                        buf.write_short_str(str)?;
                    }
                }
            }
            MessageKind::UnderlineAnd5VDisable {
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
            MessageKind::RailFaults {
                short_6v,
                short_5v,
                short_3_3v,
            } => {
                // buf.write_buf(&*data)?;
                buf.write_u16(*short_6v)?;
                buf.write_u16(*short_5v)?;
                buf.write_u16(*short_3_3v)?;
            }
            MessageKind::DisableFaults { comms, fault_12v } => {
                buf.write_u16(*comms)?;
                buf.write_u16(*fault_12v)?;
            }
            MessageKind::UsageReport {
                team,
                unknwon,
                usage,
            } => {
                todo!("NOT ACTUALLY WORKING YET :3");
            }
        }
        Ok(())
    }
}
