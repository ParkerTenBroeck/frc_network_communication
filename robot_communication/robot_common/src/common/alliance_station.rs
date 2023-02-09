use super::error::RobotPacketParseError;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AllianceStation {
    #[default]
    Red1 = 0,
    Red2 = 1,
    Red3 = 2,
    Blue1 = 3,
    Blue2 = 4,
    Blue3 = 5,
}

impl TryFrom<u8> for AllianceStation {
    type Error = RobotPacketParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Red1),
            1 => Ok(Self::Red2),
            2 => Ok(Self::Red3),
            3 => Ok(Self::Blue1),
            4 => Ok(Self::Blue2),
            5 => Ok(Self::Blue3),
            val => Err(RobotPacketParseError::InvalidStationCode(val)),
        }
    }
}
