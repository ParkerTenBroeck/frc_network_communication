use std::{fmt::Display, num::ParseIntError, str::FromStr};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TeamNumber(pub u16);

impl Display for TeamNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TeamNumber {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(u16::from_str(s)?))
    }
}

impl From<u16> for TeamNumber {
    fn from(value: u16) -> Self {
        Self(value)
    }
}
