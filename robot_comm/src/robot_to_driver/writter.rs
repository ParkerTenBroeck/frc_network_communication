use std::marker::PhantomData;

use util::buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff};

use super::{
    CpuUsage, PdpPortReport, PdpPowerReport, RobotToDriverCanUsage, RobotToDriverDiskUsage,
    RobotToDriverRamUsage, RobotToDriverRumble, RobotToDriverstationPacket,
};

pub struct RobotToDriverstaionPacketWritter<'a, 'b, T: BufferWritter<'a>> {
    writter: &'b mut T,
    _phantom: PhantomData<&'a mut [u8]>,
}

impl<'a, 'b, T: BufferWritter<'a>> RobotToDriverstaionPacketWritter<'a, 'b, T> {
    pub fn new(
        writter: &'b mut T,
        packet: RobotToDriverstationPacket,
    ) -> Result<Self, BufferWritterError> {
        packet.write_to_buf(writter)?;
        Ok(Self {
            writter,
            _phantom: PhantomData,
        })
    }

    pub fn rumble(&mut self, rumble: RobotToDriverRumble) -> Result<&mut Self, BufferWritterError> {
        let mut buf = self.writter.create_u8_size_guard()?;
        buf.write_u8(0x01)?;
        buf.write_u32(rumble.no_idea)?;
        buf.write_u16(rumble.left)?;
        buf.write_u16(rumble.right)?;
        drop(buf);
        Ok(self)
    }

    pub fn disk_usage(
        &mut self,
        usage: RobotToDriverDiskUsage,
    ) -> Result<&mut Self, BufferWritterError> {
        self.writter.write_u8(9)?; //size(we know ahead of time)
        self.writter.write_u8(0x04)?; //tag
        self.writter.write_u64(usage.bytes_free)?;
        Ok(self)
    }

    pub fn cpu_usage(&mut self, usage: &[CpuUsage]) -> Result<&mut Self, BufferWritterError> {
        let mut buf = self.writter.create_u8_size_guard()?;
        buf.write_u8(0x05)?;
        for usage in usage {
            buf.write_buf_const(&usage.user)?;
            buf.write_buf_const(&usage._unknown1)?;
            buf.write_buf_const(&usage._unknown2)?;
            buf.write_buf_const(&usage.system)?;
        }
        drop(buf);
        Ok(self)
    }

    pub fn ram_usage(
        &mut self,
        usage: RobotToDriverRamUsage,
    ) -> Result<&mut Self, BufferWritterError> {
        self.writter.write_u8(9)?; //size(we know ahead of time)
        self.writter.write_u8(0x06)?; //tag
        self.writter.write_u64(usage.bytes_free)?;
        Ok(self)
    }

    pub fn pdp_port_report(
        &mut self,
        report: &PdpPortReport,
    ) -> Result<&mut Self, BufferWritterError> {
        self.writter.write_u8(26)?; //size(we know ahead of time)
        self.writter.write(0x08)?;
        self.writter.write_u8(report.unknown_0)?;
        self.writter.write_buf_const(&report.port_amp_report.0)?;
        self.writter.write_buf_const(&report.unknown_1)?;
        Ok(self)
    }

    pub fn pdp_power_report(
        &mut self,
        report: PdpPowerReport,
    ) -> Result<&mut Self, BufferWritterError> {
        self.writter.write_u8(10)?; //size(we know ahead of time)
        self.writter.write(0x08)?;
        self.writter.write_buf_const(&report.inner.0)?;
        Ok(self)
    }

    pub fn can_usage(
        &mut self,
        usage: RobotToDriverCanUsage,
    ) -> Result<&mut Self, BufferWritterError> {
        self.writter.write_u8(15)?; //size(we know ahead of time)
        self.writter.write_u8(0x0e)?; //tag
        self.writter.write_f32(usage.utilization)?;
        self.writter.write_u32(usage.bus_off)?;
        self.writter.write_u32(usage.tx_full)?;
        self.writter.write_u8(usage.rx)?;
        self.writter.write_u8(usage.tx)?;
        Ok(self)
    }

    pub fn into_buf(self) -> &'b [u8] {
        self.writter.curr_buf()
    }
}
