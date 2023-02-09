pub trait WriteToBuff<'a> {
    type Error;
    fn write_to_buff(&self, buf: &mut BufferWritter<'a>) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum BufferWritterError {
    BufferTooSmall,
    InvalidData(&'static str)
}

impl std::error::Error for BufferWritterError {}

impl std::fmt::Display for BufferWritterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BufferTooSmall")
    }
}

impl<'a> WriteToBuff<'a> for &'a [u8] {
    type Error = BufferWritterError;

    fn write_to_buff(&self, buf: &mut BufferWritter<'a>) -> Result<(), BufferWritterError> {
        buf.write_all(self)
    }
}

impl<'a, const SIZE: usize> WriteToBuff<'a> for [u8; SIZE] {
    type Error = BufferWritterError;
    fn write_to_buff(&self, buf: &mut BufferWritter<'a>) -> Result<(), BufferWritterError> {
        buf.write_all(self)
    }
}

pub struct BufferWritter<'a> {
    buff: &'a mut [u8],
    index: usize,
}

impl<'a> BufferWritter<'a> {
    pub fn new(buff: &'a mut [u8]) -> Self {
        Self { buff, index: 0 }
    }

    pub fn reset(&mut self) {
        self.index = 0;
    }

    pub fn get_curr_buff(&'a self) -> &'a [u8] {
        &self.buff[..self.index]
    }

    pub fn write_all(&mut self, vals: &[u8]) -> Result<(), BufferWritterError> {
        let old_index = self.index;
        self.increase_index(vals.len())?;
        let tmp = &mut self.buff[old_index..];
        let tmp = &mut tmp[..vals.len()];
        tmp.copy_from_slice(vals);
        Ok(())
    }

    pub fn write_u8(&mut self, val: u8) -> Result<(), BufferWritterError> {
        self.increase_index(1)?;
        self.buff[self.index - 1] = val;
        Ok(())
    }

    pub fn write_u16(&mut self, val: u16) -> Result<(), BufferWritterError> {
        self.increase_index(2)?;
        self.buff[self.index - 2] = (val >> 8) as u8;
        self.buff[self.index - 1] = val as u8;
        Ok(())
    }

    pub fn write_u32(&mut self, val: u32) -> Result<(), BufferWritterError> {
        self.increase_index(4)?;
        self.buff[self.index - 4] = (val >> 24) as u8;
        self.buff[self.index - 3] = (val >> 16) as u8;
        self.buff[self.index - 2] = (val >> 8) as u8;
        self.buff[self.index - 1] = val as u8;
        Ok(())
    }

    pub fn write_short_str(&mut self, str: &str) -> Result<(), BufferWritterError>{
        if str.len() >= 256{
            Err(BufferWritterError::InvalidData("Small string length longer than 255"))?
        }
        self.write_u8(str.len() as u8)?;
        self.write_all(str.as_bytes())?;
        Ok(())
    }

    fn increase_index(&mut self, amount: usize) -> Result<(), BufferWritterError> {
        self.index += amount;
        if self.index > self.buff.len() {
            Err(BufferWritterError::BufferTooSmall)
        } else {
            Ok(())
        }
    }
}
