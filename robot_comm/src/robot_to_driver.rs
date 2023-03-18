use std::{mem::size_of};

use util::{
    buffer_reader::{CreateFromBuf, ReadFromBuf},
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
};

use crate::common::{
    control_code::ControlCode, error::RobotPacketParseError,
    request_code::DriverstationRequestCode, roborio_status_code::RobotStatusCode,
    robot_voltage::RobotVoltage,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RobotToDriverstationPacket {
    pub sequence: u16,
    pub tag_comm_version: u8,
    pub control_code: ControlCode,
    pub status: RobotStatusCode,
    pub battery: RobotVoltage,
    pub request: DriverstationRequestCode,
}

trait RobotToDriverPacketAdditions<'a>: BufferWritter<'a> {
    const ID: u8;
}


#[repr(C, packed(1))]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct CpuUsage {
    pub user: f32,
    pub _unknown1: f32,
    pub _unknown2: f32,
    pub system: f32,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RobotToDriverRamUsage {
    pub bytes_used: u64,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RobotToDriverDiskUsage {
    pub bytes_used: u64,
}

pub enum UsageReport<'a> {
    DiskUsage(RobotToDriverDiskUsage),
    RamUsage(RobotToDriverRamUsage),
    CanUsage(RobotToDriverCanUsage),
    CpuUsage(&'a [CpuUsage]),
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct RobotToDriverCanUsage {
    pub utilization: f32,
    pub bus_off: u32,
    pub tx_full: u32,
    pub rx: u8,
    pub tx: u8,
}

impl<'a> WriteToBuff<'a> for RobotToDriverstationPacket {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        buf.write_u16(self.sequence)?;
        buf.write_u8(self.tag_comm_version)?;
        buf.write_u8(self.control_code.to_bits())?;
        buf.write_u8(self.status.to_bits())?;
        buf.write_u8(self.battery.int)?;
        buf.write_u8(self.battery.dec)?;
        buf.write_u8(self.request.to_bits())?;

        // if true{
        //     return Ok(());
        // }
        // let counter = self.packet as u8;
        // {
        //     let mut buf = buf.create_u8_size_guard()?;
        //                     // can usage
        //                     buf.write_u8(0x0e)?;

        //                     // utilization % [0, 1.0]
        //                     buf.write_f32(counter as f32 / 255.0)?;
        //                     // Bus Off
        //                     buf.write_u32(5)?;
        //                     // TX Full
        //                     buf.write_u32(10)?;
        //                     // Recieve
        //                     buf.write_u8(23)?;
        //                     // Transmit
        //                     buf.write_u8(32)?;
        // }

        // let mut buf2 = buf.create_u8_size_guard()?;

        //         // pdp stuff power in possibly amps or something?
        //         buf2.write_u8(0x08)?;
        //         // buf.write_buf(&[self.packet as u8; 2])?;
        //         // let mut data = [1u8; 22];
        //         // for (index, b) in data.iter_mut().enumerate(){
        //         //     *b = (index.wrapping_add(self.packet as usize)) as u8
        //         // }
        //         // buf.write_buf(&data)?;
        //         buf2.write_u8(0)?;
        //         buf2.write_u8(0)?;
        //         buf2.write_u8(0)?;
        //         buf2.write_u8(counter)?;
        //         // for i in 0u32..10{
        //         //     buf.write_u16( 0)?;
        //         // }
        //         // buf.write_buf(&[0;3])?;
        //         let wbuf = buf2.write(26 - buf2.curr_buf_len())?;

        //         wbuf.fill(0);
        //         println!("{wbuf:?}");
        //         drop(buf2);

        // buf.write_u8(0x08)?;
        // // buf.write_buf(&[0; 22])?;
        // // buf.write_buf(&[255, 171, 85])?;
        // buf.write_buf(&[self.packet as u8; 25])?;
        let mut buf = buf.create_u8_size_guard()?;
        match self.sequence & 7 {
            0 => {
                // ram
                buf.write_u8(6)?;
                // num bytes free
                buf.write_u64((self.sequence as u64) << 22)?;
            }
            1 => {
                // disk
                buf.write_u8(4)?;
                // num bytes free
                buf.write_u64((self.sequence as u64) << 31)?;
            }
            2 => {
                // can usage
                buf.write_u8(0x0e)?;

                // utilization % [0, 1.0]
                buf.write_f32((self.sequence % 200) as f32 / 200.0)?;
                // Bus Off
                buf.write_u32(5)?;
                // TX Full
                buf.write_u32(10)?;
                // Recieve
                buf.write_u8(23)?;
                // Transmit
                buf.write_u8(32)?;
            }
            3 => {
                // PDP LOGGG
                buf.write_u8(0x08)?;

                // unknown (becuase I dont know :( )
                buf.write_u8(0)?;

                // 16 pdp values each being 10 bits each
                // with 4 bits of padding every 8 bytes
                let pdp = [self.sequence; 16];
                for i in 0..2 {
                    let mut chunck = 0u64;
                    for val in &pdp[(i * 5)..(i * 5 + 5)] {
                        chunck <<= 10;
                        chunck |= (*val & 0x3FF) as u64;
                    }
                    chunck <<= 4;
                    buf.write_u64(chunck)?;
                }

                let mut chunck = 0u64;
                for val in &pdp[12..16] {
                    chunck <<= 10;
                    chunck |= (*val & 0x3FF) as u64;
                }
                let bytes = &chunck.to_be_bytes()[..5];
                buf.write_buf(bytes)?;

                buf.write_buf(&[255, 171, 85])?;
            }
            4 => {
                buf.write_u8(0x09)?;
                buf.write_buf(&[66; 9])?;
                // println!("bruh 9");
            }
            _ => {
                buf.write_u8(0x05)?;

                let num = 2;
                buf.write_u8(num + 5)?;
                for _ in 0..num {
                    buf.write_f32(11.0)?;
                    buf.write_f32(22.0)?;
                    buf.write_f32(33.0)?;
                    buf.write_f32(04.0)?;
                }
            }
        }
        // self.extended.write_to_buff(buf)?;

        Ok(())
    }
}

impl<'a> ReadFromBuf<'a> for RobotToDriverstationPacket{
    type Error = RobotPacketParseError;

    fn read_into_from_buf(&mut self, buf: &mut util::buffer_reader::BufferReader<'a>) -> Result<(), Self::Error> {
        
        self.sequence = buf.read_u16()?;
        self.tag_comm_version = buf.read_u8()?;
        if self.tag_comm_version != 1{
            Err(RobotPacketParseError::RobotToDriverInvalidCommVersion(self.tag_comm_version))?
        }

        self.control_code = ControlCode::from_bits(buf.read_u8()?);
        self.status = RobotStatusCode::from_bits(buf.read_u8()?);
        self.battery.read_into_from_buf(buf)?;
        self.request = DriverstationRequestCode::from_bits(buf.read_u8()?);
        // std::slice::from_mut(s)

        while buf.has_more(){
            let mut buf = buf.sized_u8_reader()?;
            if buf.remaining_buf_len() == 0 {
                continue;
            }
            let extra_id = buf.read_u8()?;

            match extra_id {
                1 => {
                    let left_rumble = buf.read_u16()?;
                    let right_rumble = buf.read_u16()?;
                }
                4 => {
                    let usage = buf.read_u64()?;
                    println!("disk usage: {usage}")
                }
                5 => {
                    let cpus = buf.read_u8()? as usize;
                    // let buf = buf.read_amount(cpus * size_of::<CpuUsage>())?;
                    // let slice: &[CpuUsage] = unsafe{
                    //     std::slice::from_raw_parts(buf.as_ptr().cast(), cpus)
                    // };
                    // println!("CpuUsage: {slice:#?}")
                    for i in 0..cpus {
                        // so these should all sum together to get the total CPU% [0.0, 100.0]
                        let robot = buf.read_f32()?;
                        let f2 = buf.read_f32()?;
                        let f3 = buf.read_f32()?;
                        let system = buf.read_f32()?;

                        println!("cpu: {i} -> user: {robot:.2} {f2:.2} {f3:.2} system?: {system:.2} total: {}", robot + system + f2 + f3)
                    }
                }
                6 => {
                    let usage = buf.read_u64()?;
                    println!("ram usage: {}", usage)
                }
                8 => {
                    let zeros = buf.read_amount(22)?;
                    let ff = buf.read_u8()?;
                    let num = buf.read_u16()? as i16;
                    println!("8 usage: {:?}, 0xff: {:02X}, num: {}", zeros, ff, num);
                }
                9 => {
                    println!("9 usage: {:?}", buf.read_amount(buf.remaining_buf_len())?)
                }
                14 => {
                    // utilization % [0, 1.0]
                    let utilization = buf.read_f32()?;
                    // Bus Off
                    let bus_off = buf.read_u32()?;
                    // TX Full
                    let tx_full = buf.read_u32()?;
                    // Recieve
                    let recieve = buf.read_u8()?;
                    // Transmit
                    let transmit = buf.read_u8()?;

                    println!("uti %{utilization:.2}, bus_off: {bus_off}, tx_full: {tx_full}, rx: {recieve}, ts: {transmit}");
                }
                invalid => Err(RobotPacketParseError::RobotToDriverInvalidUsageTag(invalid))?,
            }
            
        }
        Ok(())
    }
}

impl<'a> CreateFromBuf<'a> for RobotToDriverstationPacket {

    fn create_from_buf(buf: &mut util::buffer_reader::BufferReader<'a>) -> Result<Self, Self::Error> {
        let base = Self {
            sequence: buf.read_u16()?,
            tag_comm_version: {
                let com = buf.read_u8()?;
                if com != 1 {
                    Err(RobotPacketParseError::RobotToDriverInvalidCommVersion(com))?
                }
                com
            },
            control_code: ControlCode::from_bits(buf.read_u8()?),
            status: RobotStatusCode::from_bits(buf.read_u8()?),
            battery: RobotVoltage::create_from_buf(buf)?,
            request: DriverstationRequestCode::from_bits(buf.read_u8()?),
        };

        while buf.has_more() {
            let mut buf = buf.sized_u8_reader()?;
            if buf.remaining_buf_len() == 0 {
                continue;
            }
            let extra_id = buf.read_u8()?;
            // let length = buf.read_u8()? - 1;
            // let extra_id = buf.read_u8()?;
            // let mut buf = BufferReader::new(buf.read_amount(length as usize)?);
            //println!("id: {extra_id} -> {:?}", buf.raw_buff());
            match extra_id {
                1 => {
                    let left_rumble = buf.read_u16()?;
                    let right_rumble = buf.read_u16()?;
                }
                4 => {
                    let usage = buf.read_u64()?;
                    println!("disk usage: {usage}")
                }
                5 => {
                    let cpus = buf.read_u8()?;
                    for i in 0..cpus {
                        // so these should all sum together to get the total CPU% [0.0, 100.0]
                        let robot = buf.read_f32()?;
                        let f2 = buf.read_f32()?;
                        let f3 = buf.read_f32()?;
                        let system = buf.read_f32()?;

                        println!("cpu: {i} -> user: {robot:.2} {f2:.2} {f3:.2} system?: {system:.2} total: {}", robot + system + f2 + f3)
                    }
                }
                6 => {
                    let usage = buf.read_u64()?;
                    println!("ram usage: {}", usage)
                }
                8 => {
                    let zeros = buf.read_amount(22)?;
                    let ff = buf.read_u8()?;
                    let num = buf.read_u16()? as i16;
                    println!("8 usage: {:?}, 0xff: {:02X}, num: {}", zeros, ff, num);
                }
                9 => {
                    println!("9 usage: {:?}", buf.read_amount(buf.remaining_buf_len())?)
                }
                14 => {
                    // utilization % [0, 1.0]
                    let utilization = buf.read_f32()?;
                    // Bus Off
                    let bus_off = buf.read_u32()?;
                    // TX Full
                    let tx_full = buf.read_u32()?;
                    // Recieve
                    let recieve = buf.read_u8()?;
                    // Transmit
                    let transmit = buf.read_u8()?;

                    println!("uti %{utilization:.2}, bus_off: {bus_off}, tx_full: {tx_full}, rx: {recieve}, ts: {transmit}");
                }
                invalid => Err(RobotPacketParseError::RobotToDriverInvalidUsageTag(invalid))?,
            }
            buf.assert_empty()?;
        }

        Ok(base)
    }
}

#[allow(unused)]
mod tests {
    use util::{
        buffer_reader::{BufferReader, CreateFromBuf},
        buffer_writter::{BufferWritter, SliceBufferWritter, WriteToBuff},
        robot_voltage::RobotVoltage,
    };

    use crate::common::{
        control_code::ControlCode,
        request_code::{DriverstationRequestCode, RobotRequestCode},
        roborio_status_code::RobotStatusCode,
    };

    use super::RobotToDriverstationPacket;

    #[test]
    pub fn packet_write_and_read() {
        let packet = RobotToDriverstationPacket {
            sequence: 0x1234,
            tag_comm_version: 1,
            control_code: *ControlCode::new()
                .set_enabled()
                .set_autonomus()
                .set_fms_attached(true),
            status: *RobotStatusCode::new().set_has_robot_code(true),
            battery: RobotVoltage { int: 5, dec: 50 },
            request: *DriverstationRequestCode::new().set_request_time(true),
        };

        let mut buf = [0; 10];
        let mut bufw = SliceBufferWritter::new(&mut buf);
        packet
            .write_to_buf(&mut bufw)
            .expect("Failed to write to buffer");

        let mut bufr = BufferReader::new(bufw.curr_buf());
        let read_packet = RobotToDriverstationPacket::create_from_buf(&mut bufr)
            .expect("Failed to read packet from buffer");

        assert_eq!(
            read_packet, packet,
            "Packets do not match when written and read from a buffer"
        )
    }
}
