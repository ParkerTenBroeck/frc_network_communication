use std::time::SystemTime;

use chrono::{Datelike, NaiveDateTime, Timelike};
use chrono_tz::Tz;
use std::fmt::Debug;
use util::{
    buffer_reader::BufferReader,
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
};

use super::error::RobotPacketParseError;

#[derive(Default)]
pub struct TimeData {
    time: Option<NaiveDateTime>,
    time_zone: Option<Tz>,
}

impl TimeData {
    pub fn read_time_data(
        &mut self,
        buf: &mut BufferReader<'_>,
    ) -> Result<(), RobotPacketParseError> {
        let micro = buf.read_u32()?;
        let sec = buf.read_u8()?;
        let min = buf.read_u8()?;
        let hour = buf.read_u8()?;
        let day = buf.read_u8()?;
        let month = buf.read_u8()? + 1;
        let year = buf.read_u8()? as u32 + 1900;

        let date = chrono::NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32);
        let date = match date {
            Some(date) => date,
            None => Err(RobotPacketParseError::InvalidTimeData)?,
        };
        let date = date.and_hms_micro_opt(hour as u32, min as u32, sec as u32, micro);

        let date = match date {
            Some(date) => date,
            None => Err(RobotPacketParseError::InvalidTimeData)?,
        };

        self.time = Some(date);
        Ok(())
    }

    pub fn empty(&mut self){
        self.time = None;
        self.time_zone = None;
    }

    pub fn read_time_zone_date(
        &mut self,
        buf: &mut BufferReader<'_>,
    ) -> Result<(), RobotPacketParseError> {
        let tz = buf.read_str(buf.total_packet_size())?;
        self.time_zone = Some(
            tz.parse()
                .map_err(|_| RobotPacketParseError::InvalidTimeZoneData)?,
        );
        Ok(())
    }

    pub fn get_system_time(&self) -> Option<SystemTime> {
        let date = self.time?;
        let t = std::time::Duration::new(date.timestamp() as u64, date.timestamp_subsec_nanos());
        std::time::SystemTime::UNIX_EPOCH.checked_add(t)
    }

    pub fn has_data(&self) -> bool {
        self.time.is_some() | self.time_zone.is_some()
    }

    pub fn copy_existing_from(&mut self, other: &Self) {
        if other.time.is_some() {
            self.time = other.time;
        }
        if other.time_zone.is_some() {
            self.time_zone = other.time_zone;
        }
    }

    pub fn from_system() -> Self {
        Self {
            time: Some(chrono::Utc::now().naive_utc()),
            time_zone: Some(Tz::UCT),
        }
    }
}

impl<'a> WriteToBuff<'a> for TimeData {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        if let Some(time) = self.time {
            buf.write_u8(11)?;
            buf.write_u8(15)?;
            buf.write_u32(time.timestamp_subsec_micros())?;
            buf.write_u8(time.second() as u8)?;
            buf.write_u8(time.minute() as u8)?;
            buf.write_u8(time.hour() as u8)?;
            buf.write_u8(time.day() as u8)?;
            buf.write_u8(time.month() as u8 - 1)?;
            buf.write_u8((time.year() - 1900) as u8)?;
        }
        if let Some(tz) = self.time_zone {
            let name = tz.name();
            buf.write_u8((1 + name.as_bytes().len()) as u8)?;
            buf.write_u8(16)?;
            buf.write_buf(name.as_bytes())?;
        }
        Ok(())
    }
}

impl Debug for TimeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatter = f.debug_struct("TimeData");
        formatter
            .field("time", &self.time)
            .field("time_zone", &self.time_zone);

        if let (Some(time), Some(time_zone)) = (self.time, self.time_zone) {
            let time = time.and_local_timezone(time_zone).unwrap();
            formatter.field(
                "FormattedDate",
                &time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            );
        }

        formatter.finish()
    }
}
