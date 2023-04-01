pub mod reader;

use util::{
    buffer_reader::{BufferReader, CreateFromBuf, ReadFromBuf},
    buffer_writter::{BufferWritter, WriteToBuff},
};

use crate::common::{
    alliance_station::AllianceStation,
    control_code::ControlCode,
    error::RobotPacketParseError,
    joystick::{Joystick, Joysticks},
    request_code::RobotRequestCode,
    time_data::TimeData,
};

#[derive(Default, Debug)]
pub struct DriverstationToRobotPacket {
    pub core_data: DriverstationToRobotCorePacketDate,
    pub countdown: Option<f32>,
    pub time_data: TimeData,
    pub joystick_data: Joysticks,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverstationToRobotCorePacketDate {
    pub sequence: u16,
    pub tag_comm_version: u8,
    pub control_code: ControlCode,
    pub request_code: RobotRequestCode,
    pub station: AllianceStation,
}

impl<'a> ReadFromBuf<'a> for DriverstationToRobotCorePacketDate {
    type Error = RobotPacketParseError;

    fn read_into_from_buf(
        &mut self,
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<&mut Self, Self::Error> {
        self.sequence = buf.read_u16()?;
        self.tag_comm_version = buf.read_u8()?;
        if self.tag_comm_version != 1 {
            Err(RobotPacketParseError::RobotToDriverInvalidCommVersion(1))?
        }
        self.control_code = ControlCode::from_bits(buf.read_u8()?);
        self.request_code = RobotRequestCode::from_bits(buf.read_u8()?);
        self.station = AllianceStation::try_from(buf.read_u8()?)?;
        Ok(self)
    }
}

impl<'a> CreateFromBuf<'a> for DriverstationToRobotCorePacketDate {
    fn create_from_buf(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            sequence: buf.read_u16()?,
            tag_comm_version: {
                let read = buf.read_u8()?;
                if read != 1 {
                    Err(RobotPacketParseError::RobotToDriverInvalidUsageTag(read))?
                }
                read
            },
            control_code: ControlCode::from_bits(buf.read_u8()?),
            request_code: RobotRequestCode::from_bits(buf.read_u8()?),
            station: AllianceStation::try_from(buf.read_u8()?)?,
        })
    }
}

impl Default for DriverstationToRobotCorePacketDate {
    fn default() -> Self {
        Self {
            sequence: 0,
            tag_comm_version: 1,
            control_code: Default::default(),
            request_code: Default::default(),
            station: Default::default(),
        }
    }
}

impl<'a> ReadFromBuf<'a> for DriverstationToRobotPacket {
    type Error = RobotPacketParseError;

    fn read_into_from_buf(
        &mut self,
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<&mut Self, Self::Error> {
        self.core_data.sequence = buf.read_u16()?;
        self.core_data.tag_comm_version = buf.read_u8()?;
        if self.core_data.tag_comm_version != 1 {
            Err(RobotPacketParseError::RobotToDriverInvalidCommVersion(1))?
        }
        self.core_data.control_code = ControlCode::from_bits(buf.read_u8()?);
        self.core_data.request_code = RobotRequestCode::from_bits(buf.read_u8()?);
        self.core_data.station = AllianceStation::try_from(buf.read_u8()?)?;

        let mut joy_index = 0;
        while buf.has_more() {
            let mut buf = buf.read_known_length_u16()?;
            if buf.is_empty() {
                continue;
            }
            let tag = buf.read_u8()?;
            self.countdown = None;
            self.time_data.empty();
            match tag {
                7 => {
                    self.countdown = Some(buf.read_f32()?);
                    println!("{:?}", self.countdown)
                }
                15 => {
                    self.time_data.read_time_data(&mut buf)?;
                }
                16 => {
                    self.time_data.read_time_zone_date(&mut buf)?;
                }
                12 => {
                    if let Some(joy) = self.joystick_data.get_o_mut(joy_index) {
                        *joy = None;
                    } else {
                        Err(RobotPacketParseError::TooManyJoysticksInPacket)?
                    }
                    joy_index += 1;
                }
                invalid => Err(RobotPacketParseError::DriverToRobotInvalidExtraTag(invalid))?,
            }

            buf.assert_empty()?;
        }
        for i in joy_index..6 {
            self.joystick_data.delete(i);
        }
        Ok(self)
    }
}

impl<'a> CreateFromBuf<'a> for DriverstationToRobotPacket {
    fn create_from_buf(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        let mut read = Self {
            core_data: DriverstationToRobotCorePacketDate {
                sequence: buf.read_u16()?,
                tag_comm_version: {
                    let read = buf.read_u8()?;
                    if read != 1 {
                        Err(RobotPacketParseError::RobotToDriverInvalidUsageTag(read))?
                    }
                    read
                },
                control_code: ControlCode::from_bits(buf.read_u8()?),
                request_code: RobotRequestCode::from_bits(buf.read_u8()?),
                station: AllianceStation::try_from(buf.read_u8()?)?,
            },
            time_data: TimeData::default(),
            joystick_data: Joysticks::default(),
            countdown: None,
        };

        if read.core_data.control_code.is_invalid() {
            Err(RobotPacketParseError::InvalidControlCode(
                read.core_data.control_code.to_bits(),
            ))?
        }
        if read.core_data.request_code.is_invalid() {
            Err(RobotPacketParseError::InvalidRequestCode(
                read.core_data.request_code.to_bits(),
            ))?
        }

        while buf.has_more() {
            let length = buf.read_u8()? - 1;
            let extra_id = buf.read_u8()?;
            let mut buf = BufferReader::new(buf.read_amount(length as usize)?);
            match extra_id {
                7 => {
                    println!("{:?}", read.countdown);
                    read.countdown = Some(buf.read_f32()?);
                    println!("{:?}", read.countdown)
                }
                15 => {
                    read.time_data.read_time_data(&mut buf)?;
                }
                16 => {
                    read.time_data.read_time_zone_date(&mut buf)?;
                }
                12 => {
                    read.joystick_data.insert(
                        read.joystick_data.count(),
                        Joystick::create_from_buf(&mut buf)?,
                    );
                }
                invalid => Err(RobotPacketParseError::DriverToRobotInvalidExtraTag(invalid))?,
            }
            buf.assert_empty()?;
        }

        Ok(read)
    }
}

impl<'a> WriteToBuff<'a> for DriverstationToRobotPacket {
    type Error = RobotPacketParseError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        buf.write_u16(self.core_data.sequence)?;
        buf.write_u8(self.core_data.tag_comm_version)?;
        buf.write_u8(self.core_data.control_code.to_bits())?;
        buf.write_u8(self.core_data.request_code.to_bits())?;
        buf.write_u8(self.core_data.station as u8)?;
        // self.time_data.write_to_buf(buf)?;
        // self.joystick_data.write_to_buf(buf)?;

        Ok(())
    }
}
