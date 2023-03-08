pub trait ReadFromBuff<'a>: Sized {
    type Error;
    fn read_from_buff(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error>;
}

#[derive(Debug)]
pub enum BufferReaderError {
    BufferReadOverflow {
        actual_buffer_length: usize,
        tried_index: usize,
    },
    ParseUft8Error(std::str::Utf8Error),
    GeneralError(Box<dyn std::error::Error + 'static + Send>),
}

impl From<std::str::Utf8Error> for BufferReaderError {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::ParseUft8Error(value)
    }
}

impl std::error::Error for BufferReaderError {}

impl std::fmt::Display for BufferReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl<'a> ReadFromBuff<'a> for &'a [u8] {
    type Error = BufferReaderError;

    fn read_from_buff(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        Ok(buf.raw_buff())
    }
}

pub struct BufferReader<'a> {
    buff: &'a [u8],
    index: usize,
}

impl<'a> BufferReader<'a> {
    pub fn new(buff: &'a [u8]) -> Self {
        Self { buff, index: 0 }
    }

    pub fn raw_buff(&self) -> &'a [u8] {
        self.buff
    }

    pub fn total_packet_size(&self) -> usize {
        self.buff.len()
    }

    pub fn remaining_packet_data(&self) -> usize {
        self.buff.len() - self.index
    }

    pub fn peek_u8(&mut self) -> Result<u8, BufferReaderError> {
        let buf = self.peek_amount(1)?;
        Ok(buf[0])
    }

    pub fn read_u8(&mut self) -> Result<u8, BufferReaderError> {
        let buf = self.read_amount(1)?;
        Ok(buf[0])
    }

    pub fn read_known_length_u16(&mut self) -> Result<Self, BufferReaderError> {
        let size = self.read_u16()? as usize;
        let buf = self.read_amount(size)?;
        Ok(Self::new(buf))
    }

    pub fn read_u16(&mut self) -> Result<u16, BufferReaderError> {
        let buf = self.read_amount(2)?;
        Ok((buf[0] as u16) << 8 | buf[1] as u16)
    }

    pub fn read_u24(&mut self) -> Result<u32, BufferReaderError> {
        let buf = self.read_amount(3)?;
        Ok((buf[0] as u32) << 16 | (buf[1] as u32) << 8 | buf[2] as u32)
    }

    pub fn read_u32(&mut self) -> Result<u32, BufferReaderError> {
        let buf = self.read_amount(4)?;
        Ok(((buf[0] as u32) << 24)
            | ((buf[1] as u32) << 16)
            | ((buf[2] as u32) << 8)
            | buf[3] as u32)
    }

    pub fn read_u64(&mut self) -> Result<u64, BufferReaderError> {
        let buf = self.read_amount(8)?;
        Ok(   ((buf[0] as u64) << 56)
            | ((buf[1] as u64) << 48)
            | ((buf[2] as u64) << 40)
            | ((buf[3] as u64) << 32)
            | ((buf[4] as u64) << 24)
            | ((buf[5] as u64) << 16)
            | ((buf[6] as u64) << 8)
            | buf[7] as u64)
    }

    pub fn read_f32(&mut self) -> Result<f32, BufferReaderError> {
        let u32 = self.read_u32()?;
        Ok(f32::from_bits(u32))
    }

    pub fn skip(&mut self, amount: usize) {
        self.index += amount;
    }

    /// Returns a slice with length \[0, 255\]
    /// Will return an error if the buffer is empty or not long enough for the length read
    pub fn read_short_u8_arr(&mut self) -> Result<&'a [u8], BufferReaderError> {
        let len = self.read_u8()?;
        self.read_amount(len as usize)
    }

    /// Returns a string with length \[0, 255\] (in bytes not chars)
    /// Will return an error if the string is not valid utf8
    /// or if the buffer is empty or not long enough for the length read
    pub fn read_short_str(&mut self) -> Result<&'a str, BufferReaderError> {
        Ok(std::str::from_utf8(self.read_short_u8_arr()?)?)
    }

    pub fn read_str(&mut self, length: usize) -> Result<&'a str, BufferReaderError> {
        Ok(std::str::from_utf8(self.read_amount(length)?)?)
    }

    /// Reads a buffer of length `amount` returning an error
    /// if the length reads over the buffer size
    /// this will not modify the current index of the reader
    pub fn peek_amount(&mut self, amount: usize) -> Result<&'a [u8], BufferReaderError> {
        let tmp = self.read_amount(amount);
        self.index -= amount;
        tmp
    }

    pub fn read_amount(&mut self, amount: usize) -> Result<&'a [u8], BufferReaderError> {
        self.index += amount;
        if self.index > self.buff.len() {
            Err(BufferReaderError::BufferReadOverflow {
                actual_buffer_length: self.buff.len(),
                tried_index: self.index - 1,
            })
        } else {
            Ok(&self.buff[self.index - amount..self.index])
        }
    }

    pub fn read_const_amount<const AMOUNT: usize>(
        &mut self,
    ) -> Result<&'a [u8; AMOUNT], BufferReaderError> {
        Ok(self.read_amount(AMOUNT)?.try_into().unwrap())
    }

    pub fn has_more(&self) -> bool {
        self.remaining_packet_data() != 0
    }
}
