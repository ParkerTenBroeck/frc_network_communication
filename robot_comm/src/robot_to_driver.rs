use util::{
    buffer_reader::{BufferReader, ReadFromBuff},
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
}

trait RobotToDriverPacketAdditions<'a>: BufferWritter<'a>{
    const ID: u8;
}

pub struct RobotToDriverCpuUsage<const CPUS: usize>{
    pub usages: [CpuUsage; CPUS]
}

#[repr(packed(1))]
pub struct CpuUsage{
    pub user: f32,
    pub _unknown1: f32,
    pub _unknown2: f32,
    pub system: f32,
}

pub struct RobotToDriverRamUsage{
    pub bytes_used: u64,
}

pub struct RobotToDriverDiskUsage{
    pub bytes_used: u64,
}

pub enum UsageReport<'a>{
    DiskUsage(RobotToDriverDiskUsage),
    RamUsage(RobotToDriverRamUsage),
    CanUsage(RobotToDriverCanUsage),
    CpuUsage(&'a [CpuUsage])
}

#[derive(Default)]
pub struct RobotToDriverCanUsage{
    pub utilization: f32,
    pub bus_off: u32,
    pub tx_full: u32,
    pub rx: u8,
    pub tx: u8
}


impl<'a> WriteToBuff<'a> for RobotToDriverstationPacket {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        buf.write_u16(self.packet)?;
        buf.write_u8(self.tag_comm_version)?;
        buf.write_u8(self.control_code.to_bits())?;
        buf.write_u8(self.status.to_bits())?;
        buf.write_u8(self.battery.int)?;
        buf.write_u8(self.battery.dec)?;
        buf.write_u8(self.request.to_bits())?;

        let mut buf = buf.create_u8_size_guard()?;
        match self.packet & 3 {
            0 => {
                // ram
                buf.write_u8(6)?;
                // num bytes free
                buf.write_u64((self.packet as u64) << 22)?;
            }
            1 => {
                // disk
                buf.write_u8(4)?;
                // num bytes free
                buf.write_u64((self.packet as u64) << 31)?;
            }
            2 => {
                // can usage
                buf.write_u8(0x0e)?;

                // utilization % [0, 1.0]
                buf.write_f32(1.0)?;
                // Bus Off
                buf.write_u32(5)?;
                // TX Full
                buf.write_u32(10)?;
                // Recieve
                buf.write_u8(23)?;
                // Transmit
                buf.write_u8(32)?;
            }
            _ => {
                buf.write_u8(0x05)?;

                let num = 2;
                buf.write_u8(num + 5)?;
                for _ in 0..num{
                    buf.write_f32(0.0)?;
                    buf.write_f32(100.0)?;
                    buf.write_f32(0.0)?;
                    buf.write_f32(0.0)?;
                }
            }
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

        if buf.remaining_packet_data() > 0 {
            //println!("remaining: {:?}", buf.read_amount(buf.remaining_packet_data())?);
        }

        while buf.has_more() {
            let length = buf.read_u8()? - 1;
            let extra_id = buf.read_u8()?;
            let mut buf = BufferReader::new(buf.read_amount(length as usize)?);
            //println!("id: {extra_id} -> {:?}", buf.raw_buff());
            match extra_id {
                4 => {
                    let usage = buf.read_u64()?;
                    println!("disk usage: {usage}")
                }
                5 => {
                    let cpus = buf.read_u8()?;
                    for i in 0..cpus{
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
                invalid => Err(RobotPacketParseError::InvalidTag(invalid))?,
            }
        }

        Ok(base)
    }
}
