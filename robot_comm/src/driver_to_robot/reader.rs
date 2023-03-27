use util::buffer_reader::{BufferReader, CreateFromBuf};

use crate::common::{error::RobotPacketParseError, joystick::Joystick, time_data::TimeData};

use super::DriverstationToRobotCorePacketDate;

pub struct DriverToRobotPacketReader<'a, 'b> {
    reader: &'b mut BufferReader<'a>,
}

impl<'a, 'b> DriverToRobotPacketReader<'a, 'b> {
    pub fn new(
        reader: &'b mut BufferReader<'a>,
    ) -> Result<(DriverstationToRobotCorePacketDate, Self), RobotPacketParseError> {
        Ok((
            DriverstationToRobotCorePacketDate::create_from_buf(reader)?,
            Self { reader },
        ))
    }

    pub fn read_tags<T: PacketTagAcceptor>(
        self,
        mut acceptor: T,
    ) -> Result<(), RobotPacketParseError> {
        let buf = self.reader;
        let mut joystick_index = 0;
        let mut timedata = TimeData::default();
        let mut countdown = None;
        while buf.has_more() {
            let mut buf = buf.read_known_length_u8()?;
            if buf.is_empty() {
                continue;
            }
            let tag = buf.read_u8()?;

            match tag {
                7 => {
                    countdown = Some(buf.read_f32()?);
                }
                15 => {
                    timedata.read_time_data(&mut buf)?;
                }
                16 => {
                    timedata.read_time_zone_date(&mut buf)?;
                }
                12 => {
                    if joystick_index >= 6 {
                        Err(RobotPacketParseError::TooManyJoysticksInPacket)?
                    } else {
                        acceptor.accept_joystick(
                            joystick_index,
                            Some(Joystick::create_from_buf(&mut buf)?),
                        );
                    };
                    joystick_index += 1;
                }
                invalid => Err(RobotPacketParseError::DriverToRobotInvalidExtraTag(invalid))?,
            }
            buf.assert_empty()?;
        }
        acceptor.accept_countdown(countdown);
        for i in joystick_index..6 {
            acceptor.accept_joystick(i, None)
        }
        acceptor.accept_time_data(timedata);
        Ok(())
    }
}

pub trait PacketTagAcceptor {
    fn accept_joystick(&mut self, index: usize, joystick: Option<Joystick>);
    fn accept_countdown(&mut self, countdown: Option<f32>);
    fn accept_time_data(&mut self, timedata: TimeData);
}
