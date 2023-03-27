use std::collections::HashMap;

use util::{
    buffer_reader::{CreateFromBuf, ReadFromBuf},
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
};

use crate::common::{
    control_code::ControlCode, error::RobotPacketParseError,
    request_code::DriverstationRequestCode, roborio_status_code::RobotStatusCode,
    robot_voltage::RobotVoltage,
};

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct RobotToDriverstationPacket {
    pub sequence: u16,
    pub tag_comm_version: u8,
    pub control_code: ControlCode,
    pub status: RobotStatusCode,
    pub battery: RobotVoltage,
    pub request: DriverstationRequestCode,
}

#[derive(Debug)]
pub struct PdpPowerReport {
    pub inner: PdpPowerReportInner<[u8; 9]>,
}

use bitfield::*;
//MSB0 [u8]
bitfield! { // 9 bytes
    pub struct PdpPowerReportInner(MSB0 [u8]);
    u32;
    // always zero so maybe the can_id for PDP
    // (thats also normally zero)
    can_id, _: 7, 0; // 8 bits
    // always 20 so maybe number of PDP ports
    pdp_ports, _: 15, 8; // 8 bits
    // this is close or equal to the combined amps of each channel
    // of the PDP so its probably correct
    current_pdp_amps, _: 27, 16; // 12 bits
    // its close to the total times 12 so my guess is its watts
    current_pdp_wats, _: 43, 28; // 16 bits
    // this counts up whenever theres amps
    // no idea what unit it is
    // uhhhhh so this might be total A(ms)??
    // this acumulates roughtly equal to 20*current_pdp_amps every packet
    // and these packets are sent every ~20 ms????
    total_unknown, _: 71, 44; // 28 bits
}

impl<T: AsRef<[u8]>> std::fmt::Debug for PdpPowerReportInner<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PdpPowerUsageReport")
            .field("can_id", &self.can_id())
            .field("pdp_ports", &self.pdp_ports())
            .field("current_pdp_amps", &self.current_pdp_amps())
            .field("current_pdp_wats", &self.current_pdp_wats())
            .field("total_unknown", &self.total_unknown())
            .finish()
    }
}

#[derive(Debug)]
pub struct PdpPortReport {
    pub unknown_0: u8,
    pub port_amp_report: PdpPortAmpReport<[u8; 21]>,
    pub unknown_1: [u8; 3],
}

bitfield! { // 9 bytes
    pub struct PdpPortAmpReport(MSB0 [u8]);
    u16;
    port_00, _: 9, 0;
    port_01, _: 19, 10;
    port_02, _: 29, 20;
    port_03, _: 39, 30;
    port_04, _: 49, 40;
    port_05, _: 59, 50;
    pad1, _: 63, 60;
    port_06, _: 73, 64;
    port_07, _: 83, 74;
    port_08, _: 93, 84;
    port_09, _: 103, 94;
    port_10, _: 113, 104;
    port_11, _: 123, 114;
    pad2, _: 127, 124;
    port_12, _: 137, 128;
    port_13, _: 147, 138;
    port_14, _: 157, 148;
    port_15, _: 167, 158;
}

impl<T: AsRef<[u8]>> std::fmt::Debug for PdpPortAmpReport<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PdpPortAmpReport")
            .field("port_00", &self.port_00())
            .field("port_01", &self.port_01())
            .field("port_02", &self.port_02())
            .field("port_03", &self.port_03())
            .field("port_04", &self.port_04())
            .field("port_05", &self.port_05())
            .field("pad1", &self.pad1())
            .field("port_06", &self.port_06())
            .field("port_07", &self.port_07())
            .field("port_08", &self.port_08())
            .field("port_09", &self.port_09())
            .field("port_10", &self.port_10())
            .field("pad2", &self.pad2())
            .field("port_11", &self.port_11())
            .field("port_12", &self.port_12())
            .field("port_13", &self.port_13())
            .field("port_14", &self.port_14())
            .field("port_15", &self.port_15())
            .finish()
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq)]
pub struct CpuUsage {
    user: [u8; 4],
    _unknown1: [u8; 4],
    _unknown2: [u8; 4],
    system: [u8; 4],
}

impl std::fmt::Debug for CpuUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpuUsage")
            .field("user", &self.get_1())
            .field("_unknown1", &self.get_2())
            .field("_unknown2", &self.get_3())
            .field("system", &self.get_4())
            .finish()
    }
}

impl CpuUsage {
    pub fn get_1(&self) -> f32 {
        f32::from_be_bytes(self.user)
    }
    pub fn get_2(&self) -> f32 {
        f32::from_be_bytes(self._unknown1)
    }
    pub fn get_3(&self) -> f32 {
        f32::from_be_bytes(self._unknown2)
    }
    pub fn get_4(&self) -> f32 {
        f32::from_be_bytes(self.system)
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RobotToDriverRumble {
    no_idea: u32,
    left: u16,
    right: u16,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RobotToDriverRamUsage {
    pub bytes_free: u64,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RobotToDriverDiskUsage {
    pub bytes_free: u64,
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

        return Ok(());

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
        {
            let mut buf = buf.create_u8_size_guard()?;
            // ram
            buf.write_u8(6)?;
            // num bytes free
            buf.write_u64((self.sequence as u64) << 22)?;
        }

        {
            let mut buf = buf.create_u8_size_guard()?;
            // disk
            buf.write_u8(4)?;
            // num bytes free
            buf.write_u64((self.sequence as u64) << 31)?;
        }
        {
            let mut buf = buf.create_u8_size_guard()?;
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
        {
            let mut buf = buf.create_u8_size_guard()?;
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
        {
            buf.write_u8(0x09)?;
            buf.write_buf(&[66; 9])?;
            // println!("bruh 9");
        }
        {
            let mut buf = buf.create_u8_size_guard()?;
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

        // self.extended.write_to_buff(buf)?;

        Ok(())
    }
}

impl<'a> ReadFromBuf<'a> for RobotToDriverstationPacket {
    type Error = RobotPacketParseError;

    fn read_into_from_buf(
        &mut self,
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<&mut Self, Self::Error> {
        self.sequence = buf.read_u16()?;
        self.tag_comm_version = buf.read_u8()?;
        if self.tag_comm_version != 1 {
            Err(RobotPacketParseError::RobotToDriverInvalidCommVersion(
                self.tag_comm_version,
            ))?
        }

        self.control_code = ControlCode::from_bits(buf.read_u8()?);
        self.status = RobotStatusCode::from_bits(buf.read_u8()?);
        self.battery.read_into_from_buf(buf)?;
        self.request = DriverstationRequestCode::from_bits(buf.read_u8()?);
        // std::slice::from_mut(s)
        return Ok(self);
        println!("battery voltage: {}", self.battery.to_f32());

        while buf.has_more() {
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
                    let buf = buf.read_amount(cpus * size_of::<CpuUsage>())?;
                    let slice: &[CpuUsage] =
                        unsafe { std::slice::from_raw_parts(buf.as_ptr().cast(), cpus) };
                    println!("CpuUsage: {slice:#?}");
                    // for i in 0..cpus {
                    //     // so these should all sum together to get the total CPU% [0.0, 100.0]
                    //     let robot = buf.read_f32()?;
                    //     let f2 = buf.read_f32()?;
                    //     let f3 = buf.read_f32()?;
                    //     let system = buf.read_f32()?;

                    //     println!("cpu: {i} -> user: {robot:.2} {f2:.2} {f3:.2} system?: {system:.2} total: {}", robot + system + f2 + f3)
                    // }
                }
                6 => {
                    let usage = buf.read_u64()?;
                    println!("ram usage: {}", usage)
                }
                8 => {
                    let val = buf.read_u8()?;

                    let mut calculated_total = 0;

                    let mut curr_thing = 0;
                    for _ in 0..2 {
                        let mut val = buf.read_u64()?;
                        for _ in 0..8 {
                            let tmp = val >> (64 - 10);
                            print!("amp_{curr_thing}: {tmp}, ");
                            calculated_total += tmp;
                            val <<= 10;
                            curr_thing += 1;
                        }
                    }
                    let tmp = buf.read_u16()? as u64;
                    let mut val = (tmp << 16) | (buf.read_u24()? as u64);
                    val <<= 64 - 40;
                    for _ in 0..5 {
                        let tmp = val >> (64 - 10);
                        print!("amp_{curr_thing}: {tmp}, ");
                        calculated_total += tmp;
                        val <<= 10;
                        curr_thing += 1;
                    }

                    let ff = buf.read_u8()?;
                    let num = buf.read_u8()? as i16;
                    let num2 = buf.read_u8()? as i16;
                    println!("pdp shiz: 0xff: {:02X}, num: {}, {}", ff, num, num2);
                    println!("calculated total amps: {}", calculated_total);
                }
                9 => {
                    // use modular_bitfield_msb::*;
                    // use modular_bitfield_msb::specifiers::*;
                    // #[bitfield(bits = 72)]
                    // #[derive(Debug)]
                    // pub struct PackedData {
                    //     norm_0x00: B8,
                    //     norm_0x20: B8,
                    //     a1: B10,
                    //     idk: B6,
                    //     a2: B10,
                    //     total_a: B30
                    // }

                    // let vals = buf.read_const_amount::<9>()?;
                    // println!("{vals:?}");
                    // for val in vals{
                    //     print!("{:08b}, ", val);
                    // }
                    // println!("{:#?}", PackedData::from_bytes(*vals));
                    // buf.assert_empty()?;

                    let vals = buf.read_const_amount::<9>()?;
                    // let vals = [        0,
                    // 20,
                    // 3,
                    // 176,
                    // 44,
                    // 17,
                    // 58,
                    // 254,
                    // 223,];//buf.read_const_amount::<9>()?;
                    let data = PdpPowerReportInner(*vals);

                    // println!("{data:#?}");
                    println!("pdp shiz 2 eletric boogaloo");
                    println!("norm 0 (pdp can id?): {}", data.can_id());
                    println!("norm 20 (pdp # ports?): {}", data.pdp_ports());
                    println!("current pdp amp draw?: {}", data.current_pdp_amps());
                    println!("current pdp watt draw?: {}", data.current_pdp_wats());
                    println!("total pdp ???(unit) draw?: {}", data.total_unknown());
                    println!();
                    println!(
                        "calculated voltage: {}",
                        data.current_pdp_wats() as f32 / data.current_pdp_amps() as f32
                    );

                    let mut map = HashMap::new();

                    let mut last = 0;

                    let mut bru = |len: usize, color: (u8, u8, u8)| {
                        for i in last..(len + last) {
                            map.insert(i, color);
                        }
                        last += len;
                    };

                    bru(8, (255, 0, 0));
                    bru(8, (0, 0, 255));

                    bru(12, (0, 255, 0));
                    // bru(4, (255, 255, 255));
                    bru(16, (0, 255, 255));

                    bru(28, (255, 255, 255));

                    println!("MSB 0 1 2 3 4 5 6 7");
                    for (index, val) in vals.iter().enumerate() {
                        let mut val = *val;
                        print!("{:02X?}: ", index * 8);
                        for i in 0..8 {
                            let bit_index = i + index * 8;
                            let bit = (val & 0b1000_0000) / 128 == 1;
                            if let Some((r, g, b)) = map.get(&bit_index) {
                                print!("\x1b[38;2;{r};{g};{b}m")
                            }
                            if bit {
                                print!("#");
                            } else {
                                print!(".",);
                            }
                            if i != 7 {
                                val <<= 1;
                                print!("\x1b[0m ")
                            } else {
                                println!("\x1b[0m")
                            }
                        }
                    }

                    // println!("{:#?}", PackedData([0,0,0,0,0,0,0,1,0]));
                    buf.assert_empty()?;
                    // PackedData::(vals);

                    //idk something to do with amps
                    // the last 24 bit number keeps counting up and never goes down whenever
                    // let v1 = buf.read_u8()?;
                    // let val = buf.read_u8()?;
                    // if val != 0x20{

                    // }
                    // // let vals = buf.read_const_amount::<4>()?;
                    // let val_2 = buf.read_u32()?;
                    // let a1 = (val_2 >> 4) & 0b1111111111;
                    // let a2 = (val_2 >> 20) & 0b1111111111;
                    // let val_fb = buf.read_u24()?;
                    // buf.assert_empty()?;
                    // println!("9 usage: {val}, {v1:#02X?}, a1: {a1}, a2: {a2}, val: {val_fb}, a_full: {val_2:032b}");
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
        Ok(self)
    }
}

impl<'a> CreateFromBuf<'a> for RobotToDriverstationPacket {
    fn create_from_buf(
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<Self, Self::Error> {
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
                    let start = buf.read_u8()?;
                    let vals = buf.read_const_amount::<21>()?;

                    let other = buf.read_const_amount::<3>()?;
                    println!("8 usage: start{start:02X?}, vals: {vals:02X?}, other: {other:02X?}");
                }
                9 => {
                    let v1 = buf.read_u8()?;
                    let val = buf.read_u8()?;
                    if val != 0x20 {}
                    let vals = buf.read_const_amount::<5>()?;
                    let val = buf.read_u16()?;
                    buf.assert_empty()?;
                    println!("9 usage: {v1:#02X?}, vals: {vals:#02X?}, val: {val}");
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

pub mod reader;
pub mod writter;
