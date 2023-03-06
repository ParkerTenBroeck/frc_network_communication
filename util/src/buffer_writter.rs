pub trait WriteToBuff<'a> {
    type Error;
    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum BufferWritterError {
    BufferTooSmall,
    FailedToGrowBuffer,
    SizeValueOverflow,
    InvalidData(&'static str),
}

impl std::error::Error for BufferWritterError {}

impl std::fmt::Display for BufferWritterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'a> WriteToBuff<'a> for &'a [u8] {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        buf.write_buf(self)
    }
}

impl<'a, const SIZE: usize> WriteToBuff<'a> for [u8; SIZE] {
    type Error = BufferWritterError;
    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), BufferWritterError> {
        buf.write_buf(self)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum RecordedSize {
    U8,
    U16,
    U32,
}

impl RecordedSize {
    fn max_allowed(&self) -> usize {
        match self {
            RecordedSize::U8 => u8::MAX as usize,
            RecordedSize::U16 => u16::MAX as usize,
            RecordedSize::U32 => u32::MAX as usize,
        }
    }

    fn bytes(&self) -> usize {
        match self {
            RecordedSize::U8 => 1,
            RecordedSize::U16 => 2,
            RecordedSize::U32 => 4,
        }
    }
}

pub struct BufferWritterSizeGuard<'a, T: BufferWritter<'a>> {
    inner: &'a mut T,
    recorded_size_start: usize,
    written: usize,
    recorded_size: RecordedSize,
}

impl<'a, T: BufferWritter<'a>> BufferWritterSizeGuard<'a, T> {
    pub fn new_u8(writter: &'a mut T) -> Result<Self, BufferWritterError> {
        Self::new(writter, RecordedSize::U8)
    }
    pub fn new_u16(writter: &'a mut T) -> Result<Self, BufferWritterError> {
        Self::new(writter, RecordedSize::U16)
    }
    pub fn new_u32(writter: &'a mut T) -> Result<Self, BufferWritterError> {
        Self::new(writter, RecordedSize::U32)
    }

    fn new(writter: &'a mut T, recorded_size: RecordedSize) -> Result<Self, BufferWritterError> {
        let myself = Self {
            recorded_size_start: writter.curr_buf_len(),
            inner: writter,
            written: 0,
            recorded_size,
        };
        // ignore the result because we will insert the data later
        myself.inner.write(recorded_size.bytes())?;
        Ok(myself)
    }
}

impl<'a, T: BufferWritter<'a>> BufferWritter<'a> for BufferWritterSizeGuard<'a, T> {
    fn reset(&mut self) {
        self.inner.reset()
    }

    fn curr_buf(&self) -> &[u8] {
        self.inner.curr_buf()
    }

    fn curr_buf_mut(&mut self) -> &mut [u8] {
        self.inner.curr_buf_mut()
    }

    fn write(&mut self, size: usize) -> Result<&mut [u8], BufferWritterError> {
        self.written += size;
        if self.written > self.recorded_size.max_allowed() {
            self.written -= size;
            Err(BufferWritterError::SizeValueOverflow)
        } else {
            self.inner.write(size)
        }
    }
}

impl<'a, T: BufferWritter<'a>> std::ops::Drop for BufferWritterSizeGuard<'a, T> {
    fn drop(&mut self) {
        let size_buf = &mut self.inner.curr_buf_mut()
            [self.recorded_size_start..self.recorded_size_start + self.recorded_size.bytes()];
        for byte in size_buf.iter_mut().rev() {
            *byte = self.written as u8;
            self.written >>= 8;
        }
    }
}

pub trait BufferWritter<'a>: Sized {
    fn reset(&mut self);
    fn curr_buf(&self) -> &[u8];
    fn curr_buf_mut(&mut self) -> &mut [u8];

    fn curr_buf_len(&self) -> usize {
        self.curr_buf().len()
    }

    #[must_use = "The data is never written to"]
    fn write(&mut self, size: usize) -> Result<&mut [u8], BufferWritterError>;

    #[must_use = "The data is never written to"]
    fn write_known<const SIZE: usize>(&mut self) -> Result<&mut [u8; SIZE], BufferWritterError> {
        Ok(self.write(SIZE)?.try_into().unwrap())
    }

    fn write_buf(&mut self, data: &[u8]) -> Result<(), BufferWritterError> {
        self.write(data.len())?.copy_from_slice(data);
        Ok(())
    }

    fn write_u8(&mut self, data: u8) -> Result<(), BufferWritterError> {
        self.write_known::<1>()?[0] = data;
        Ok(())
    }

    fn write_u16(&mut self, data: u16) -> Result<(), BufferWritterError> {
        let buf = self.write_known::<2>()?;
        buf[0] = (data >> 8) as u8;
        buf[1] = data as u8;
        Ok(())
    }

    fn write_u32(&mut self, data: u32) -> Result<(), BufferWritterError> {
        let buf = self.write_known::<4>()?;
        buf[0] = (data >> 24) as u8;
        buf[1] = (data >> 16) as u8;
        buf[2] = (data >> 8) as u8;
        buf[3] = data as u8;
        Ok(())
    }

    fn write_i8(&mut self, data: i8) -> Result<(), BufferWritterError> {
        self.write_u8(data as u8)
    }

    fn write_i16(&mut self, data: i16) -> Result<(), BufferWritterError> {
        self.write_u16(data as u16)
    }

    fn write_i32(&mut self, data: i32) -> Result<(), BufferWritterError> {
        self.write_u32(data as u32)
    }

    fn write_short_str(&mut self, data: &str) -> Result<(), BufferWritterError> {
        if data.len() > 255 {
            Err(BufferWritterError::SizeValueOverflow)
        } else {
            self.write_u8(data.len() as u8)?;
            let buf = self.write(data.len())?;
            buf.copy_from_slice(data.as_bytes());
            Ok(())
        }
    }

    fn create_u8_size_guard(
        &'a mut self,
    ) -> Result<BufferWritterSizeGuard<'_, Self>, BufferWritterError> {
        BufferWritterSizeGuard::new_u8(self)
    }

    fn create_u16_size_guard(
        &'a mut self,
    ) -> Result<BufferWritterSizeGuard<'_, Self>, BufferWritterError> {
        BufferWritterSizeGuard::new_u16(self)
    }
}

pub struct SliceBufferWritter<'a> {
    buff: &'a mut [u8],
    index: usize,
}

impl<'a> BufferWritter<'a> for SliceBufferWritter<'a> {
    fn reset(&mut self) {
        self.index = 0;
    }

    fn curr_buf(&self) -> &[u8] {
        &self.buff[..self.index]
    }

    fn curr_buf_mut(&mut self) -> &mut [u8] {
        &mut self.buff[..self.index]
    }

    fn write(&mut self, size: usize) -> Result<&mut [u8], BufferWritterError> {
        self.index += size;
        if self.index > self.buff.len() {
            self.index -= size;
            Err(BufferWritterError::BufferTooSmall)
        } else {
            Ok(&mut self.buff[self.index - size..self.index])
        }
    }
}

impl<'a> SliceBufferWritter<'a> {
    pub fn new(buff: &'a mut [u8]) -> Self {
        Self { buff, index: 0 }
    }
}
