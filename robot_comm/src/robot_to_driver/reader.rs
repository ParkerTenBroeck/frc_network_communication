use std::marker::PhantomData;

use util::buffer_reader::{BufferReader, CreateFromBuf, ReadFromBuf};

use crate::{
    common::error::RobotPacketParseError,
    robot_to_driver::{PdpPowerReportInner, RobotToDriverCanUsage},
};

use super::{
    CpuUsage, PdpPortAmpReport, PdpPortReport, RobotToDriverDiskUsage, RobotToDriverRamUsage,
    RobotToDriverRumble, RobotToDriverstationPacket,
};

pub struct RobotToDriverPacketReader<'a, T> {
    reader: BufferReader<'a>,
    phantom: PhantomData<T>,
}

struct Core;
struct Tags;

impl<'a> RobotToDriverPacketReader<'a, Core> {
    pub fn new(reader: BufferReader<'a>) -> Self {
        Self {
            reader,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn read_core(
        mut self,
    ) -> Result<
        (
            RobotToDriverstationPacket,
            RobotToDriverPacketReader<'a, Tags>,
        ),
        RobotPacketParseError,
    > {
        let mut default = RobotToDriverstationPacket::default();
        default.read_into_from_buf(&mut self.reader)?;
        // TODO!! RobotToDriverstationPacket currently reads all values form tags
        // we need it to uh not do that :3
        Ok((
            default,
            RobotToDriverPacketReader::<'a, Tags> {
                reader: self.reader,
                phantom: PhantomData,
            },
        ))
    }
}

pub fn print_packet(buf: &[u8]) -> Result<(), RobotPacketParseError> {
    struct Bruh {
        core: RobotToDriverstationPacket,
    }

    impl PacketTagAcceptor for Bruh {
        #[inline(always)]
        fn accept_rumble(&mut self, rumble: RobotToDriverRumble) {
            println!("{:#?}", rumble);
        }

        #[inline(always)]
        fn accept_ram_usage(&mut self, bytes_free: RobotToDriverRamUsage) {
            println!("{:#?}", bytes_free);
        }

        #[inline(always)]
        fn accept_disk_usage(&mut self, bytes_free: RobotToDriverDiskUsage) {
            println!("{:#?}", bytes_free);
        }

        #[inline(always)]
        fn accept_cpu_usage(&mut self, cpu_usage: &[CpuUsage]) {
            println!("{:#?}", cpu_usage);
        }

        #[inline(always)]
        fn accept_can_usage(&mut self, can_usafe: RobotToDriverCanUsage) {
            println!("{:#?}", can_usafe);
        }

        #[inline(always)]
        fn accept_pdp_port_report(&mut self, pdp_port_report: PdpPortReport) {
            println!("{:#?}", pdp_port_report);
        }

        #[inline(always)]
        fn accept_pdp_power_report(&mut self, pdp_power_report: PdpPowerReportInner<[u8; 9]>) {
            println!("{:#?}", pdp_power_report);
        }
    }

    let reader = BufferReader::new(buf);

    let reader = RobotToDriverPacketReader::new(reader);

    let (packet, reader) = reader.read_core().unwrap();
    println!("{:#?}", packet);

    reader.read_tags(&mut Bruh { core: packet })?;
    Ok(())
}

impl<'a> RobotToDriverPacketReader<'a, Tags> {
    #[inline(always)]
    pub fn read_tags<T: PacketTagAcceptor>(
        self,
        acceptor: &mut T,
    ) -> Result<(), RobotPacketParseError> {
        let mut buf = self.reader;

        while buf.has_more() {
            let mut buf = buf.sized_u8_reader()?;
            // concerning but not the end of the world
            if buf.remaining_buf_len() == 0 {
                continue;
            }
            let extra_id = buf.read_u8()?;
            match extra_id {
                1 => {
                    todo!("Actually implement the joystick receiver");
                    // let bruh = buf.read_u32()?;
                    // let left = buf.read_u16()?;
                    // let right = buf.read_u16()?;
                    // acceptor.accept_rumble(RobotToDriverRumble { left, right });
                }
                4 => {
                    let bytes_free = buf.read_u64()?;
                    acceptor.accept_ram_usage(RobotToDriverRamUsage { bytes_free });
                }
                5 => {
                    let cpus = buf.read_u8()? as usize;
                    let buf = buf.read_amount(cpus * std::mem::size_of::<CpuUsage>())?;
                    let slice: &'a [CpuUsage] =
                        unsafe { std::slice::from_raw_parts(buf.as_ptr().cast(), cpus) };
                    acceptor.accept_cpu_usage(slice);
                }
                6 => {
                    let bytes_free = buf.read_u64()?;
                    acceptor.accept_disk_usage(RobotToDriverDiskUsage { bytes_free });
                }
                8 => {
                    let val = PdpPortReport {
                        unknown_0: buf.read_u8()?,
                        port_amp_report: PdpPortAmpReport(*buf.read_const_amount::<21>()?),
                        unknown_1: *buf.read_const_amount::<3>()?,
                    };
                    acceptor.accept_pdp_port_report(val);
                }
                9 => {
                    let vals = buf.read_const_amount::<9>()?;
                    let data = PdpPowerReportInner(*vals);
                    acceptor.accept_pdp_power_report(data);
                }
                14 => {
                    let utilization = RobotToDriverCanUsage {
                        utilization: buf.read_f32()?,
                        bus_off: buf.read_u32()?,
                        tx_full: buf.read_u32()?,
                        rx: buf.read_u8()?,
                        tx: buf.read_u8()?,
                    };
                    acceptor.accept_can_usage(utilization);
                }
                invalid => Err(RobotPacketParseError::RobotToDriverInvalidUsageTag(invalid))?,
            }
            buf.assert_empty()?;
        }
        Ok(())
    }
}

pub trait PacketTagAcceptor {
    fn accept_rumble(&mut self, rumble: RobotToDriverRumble);
    fn accept_ram_usage(&mut self, bytes_free: RobotToDriverRamUsage);
    fn accept_disk_usage(&mut self, bytes_free: RobotToDriverDiskUsage);
    fn accept_cpu_usage(&mut self, cpu_usage: &[CpuUsage]);
    fn accept_can_usage(&mut self, can_usafe: RobotToDriverCanUsage);
    fn accept_pdp_port_report(&mut self, pdp_port_report: PdpPortReport);
    fn accept_pdp_power_report(&mut self, pdp_power_report: PdpPowerReportInner<[u8; 9]>);
}
