use util::{
    buffer_reader::{BufferReader, ReadFromBuff},
    buffer_writter::WriteToBuff,
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
    pub time_data: TimeData,
    pub joystick_data: Joysticks,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverstationToRobotCorePacketDate {
    pub packet: u16,
    pub tag_comm_version: u8,
    pub control_code: ControlCode,
    pub request_code: RobotRequestCode,
    pub station: AllianceStation,
}

impl Default for DriverstationToRobotCorePacketDate {
    fn default() -> Self {
        Self {
            packet: 0,
            tag_comm_version: 1,
            control_code: Default::default(),
            request_code: Default::default(),
            station: Default::default(),
        }
    }
}

impl<'a> ReadFromBuff<'a> for DriverstationToRobotPacket {
    type Error = RobotPacketParseError;

    fn read_from_buff(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        let mut read = Self {
            core_data: DriverstationToRobotCorePacketDate {
                packet: buf.read_u16()?,
                tag_comm_version: {
                    let read = buf.read_u8()?;
                    if read != 1 {
                        Err(RobotPacketParseError::InvalidCommVersion(read))?
                    }
                    read
                },
                control_code: ControlCode::from_bits(buf.read_u8()?),
                request_code: RobotRequestCode::from_bits(buf.read_u8()?),
                station: AllianceStation::try_from(buf.read_u8()?)?,
            },
            time_data: TimeData::default(),
            joystick_data: Joysticks::default(),
        };

        while buf.has_more() {
            let length = buf.read_u8()? - 1;
            let extra_id = buf.read_u8()?;
            let mut buf = BufferReader::new(buf.read_amount(length as usize)?);
            match extra_id {
                15 => {
                    read.time_data.read_time_data(buf)?;
                }
                16 => {
                    read.time_data.read_time_zone_date(buf)?;
                }
                12 => {
                    read.joystick_data
                        .push_joystick(Joystick::read_from_buff(&mut buf)?);
                }
                invalid => Err(RobotPacketParseError::InvalidTag(invalid))?,
            }
        }

        Ok(read)
    }
}

impl<'a> WriteToBuff<'a> for DriverstationToRobotPacket {
    type Error = RobotPacketParseError;

    fn write_to_buff(
        &self,
        buf: &mut util::buffer_writter::BufferWritter<'a>,
    ) -> Result<(), Self::Error> {
        buf.write_u16(self.core_data.packet)?;
        buf.write_u8(self.core_data.tag_comm_version)?;
        buf.write_u8(self.core_data.control_code.to_bits())?;
        buf.write_u8(self.core_data.request_code.to_bits())?;
        buf.write_u8(self.core_data.station as u8)?;
        self.time_data.write_to_buff(buf)?;
        self.joystick_data.write_to_buff(buf)?;

        Ok(())
    }
}
