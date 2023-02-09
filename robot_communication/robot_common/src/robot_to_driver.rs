use util::{
    buffer_reader::{ReadFromBuff, BufferReader},
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
};

use crate::common::{
    control_code::ControlCode, error::RobotPacketParseError,
    request_code::DriverstationRequestCode, roborio_status_code::RobotStatusCode,
    robot_voltage::RobotVoltage,
};

#[derive(Debug)]
pub struct RobotToDriverstationPacket {
    pub packet: u16,
    pub tag_comm_version: u8,
    pub control_code: ControlCode,
    pub status: RobotStatusCode,
    pub battery: RobotVoltage,
    pub request: DriverstationRequestCode,
    //pub extended: RobotOutExtended,
}

impl<'a> WriteToBuff<'a> for RobotToDriverstationPacket {
    type Error = BufferWritterError;

    fn write_to_buff(&self, buf: &mut BufferWritter<'a>) -> Result<(), BufferWritterError> {
        buf.write_u16(self.packet)?;
        buf.write_u8(self.tag_comm_version)?;
        buf.write_u8(self.control_code.to_bits())?;
        buf.write_u8(self.status.to_bits())?;
        buf.write_u8(self.battery.int)?;
        buf.write_u8(self.battery.dec)?;
        buf.write_u8(self.request.to_bits())?;
        
        if self.packet & 1 == 1{
            //ram
            buf.write_u8(9)?;
            buf.write_u8(6)?;
            buf.write_u32(0)?;
            buf.write_u32(238776320)?;
        }else{
            //disk
            buf.write_u8(9)?;
            buf.write_u8(4)?;
            buf.write_u32(0)?;
            buf.write_u32(0)?;
            buf.write_u32(11_000_000)?;
        }
        // self.extended.write_to_buff(buf)?;

        Ok(())
    }
}

impl<'a> ReadFromBuff<'a> for RobotToDriverstationPacket {
    type Error = RobotPacketParseError;

    fn read_from_buff(
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<Self, Self::Error> {
        let base = Self {
            packet: buf.read_u16()?,
            tag_comm_version: {
                let com = buf.read_u8()?;
                if com != 1 {
                    Err(RobotPacketParseError::InvalidCommVersion(com))?
                }
                com
            },
            control_code: ControlCode::from_bits(buf.read_u8()?),
            status: RobotStatusCode::from_bits(buf.read_u8()?),
            battery: RobotVoltage::read_from_buff(buf)?,
            request: DriverstationRequestCode::from_bits(buf.read_u8()?),
        };

        if buf.remaining_packet_data() > 0{

            // println!("remaining: {:?}", buf.read_amount(buf.remaining_packet_data())?);
        }

        while buf.has_more() {
            let length = buf.read_u8()? - 1;
            let extra_id = buf.read_u8()?;
            let mut buf = BufferReader::new(buf.read_amount(length as usize)?);
            println!("id: {extra_id} -> {:?}", buf.raw_buff());
            match extra_id {
                4 => {
                    buf.skip(4);
                    let usage = buf.read_u32()?;
                    println!("disk usage: {usage}")
                }
                5 => {
                    
                }
                6 => {
                    let max_ram_bytes = 256000000;
                    buf.skip(4);
                    let usage = buf.read_u32()?;
                    println!("ram usage: {}", usage)
                }
                14 => {
                    // buf.skip(2);
                    // let usage = buf.read_f32()?;
                    // println!("can usage: {usage}")
                }
                invalid => Err(RobotPacketParseError::InvalidTag(invalid))?,
            }
        }

        Ok(base)
    }
}
