#[derive(Default)]
pub struct RingBuffer {
    head: usize,
    tail: usize,
    len: usize,
    max_capacity: usize,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct ExceededMaximumCapacity;
impl std::fmt::Display for ExceededMaximumCapacity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExceededMaximumCapacity").finish()
    }
}
impl std::error::Error for ExceededMaximumCapacity {}

impl RingBuffer {
    pub const fn new() -> Self {
        Self {
            head: 0,
            tail: 0,
            len: 0,
            max_capacity: usize::MAX,
            data: Vec::new(),
        }
    }

    pub const fn with_maximum_capacity(max: usize) -> Self {
        assert!(max.is_power_of_two());
        Self {
            head: 0,
            tail: 0,
            len: 0,
            max_capacity: max,
            data: Vec::new(),
        }
    }

    pub fn write_combined_tracked(
        &mut self,
        combied: &[&[u8]],
    ) -> Result<(), ExceededMaximumCapacity> {
        let size: usize = combied.iter().map(|e| e.len()).sum();
        let total_size = size + 2;
        let size: u16 = match size.try_into() {
            Ok(ok) => ok,
            Err(_err) => return Err(ExceededMaximumCapacity),
        };

        self.resize_or_erase(total_size)?;
        self.write(size.to_be_bytes().as_slice()).unwrap();
        for data in combied {
            // this should't fail
            self.write(data).unwrap();
        }
        Ok(())
    }

    pub fn write_combined(&mut self, combied: &[&[u8]]) -> Result<(), ExceededMaximumCapacity> {
        let total_size: usize = combied.iter().map(|e| e.len()).sum();
        self.resize_or_erase(total_size)?;
        for data in combied {
            // this should't fail
            self.write(data).unwrap();
        }
        Ok(())
    }

    #[inline(always)]
    fn wrap_in_buffer(&self, val: usize) -> usize {
        val & (self.current_capacity() - 1)
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), ExceededMaximumCapacity> {
        if self.current_capacity() < self.len + data.len() {
            self.resize_or_erase(data.len())?
        }
        self.len += data.len();
        let start = self.tail;
        self.tail += data.len();
        self.tail = self.wrap_in_buffer(self.tail);
        if self.tail <= start {
            let len = self.current_capacity();
            self.data[start..].copy_from_slice(&data[..(len - start)]);
            self.data[..self.tail].copy_from_slice(&data[(len - start)..]);
        } else {
            self.data[start..self.tail].copy_from_slice(data)
        }
        Ok(())
    }

    pub fn write_tracked(&mut self, data: &[u8]) -> Result<(), ExceededMaximumCapacity> {
        // pub struct TrackedRingBuffer<'a>{
        //     tb: &'a mut RingBuffer,
        //     amount_written: u16,
        //     error: bool
        // }

        // impl<'a> TrackedRingBuffer<'a>{
        //     pub fn write(&mut self, data: &[u8]) -> Result<(), ExceededMaximumCapacity>{
        //         if data.len() > u16::MAX as usize{
        //             return Err(ExceededMaximumCapacity)
        //         }
        //         if let Some(written) = self.amount_written.checked_add(data.len() as u16){
        //             self.amount_written = written;
        //             Ok(())
        //         }else{
        //             Err(ExceededMaximumCapacity)
        //         }
        //     }
        // }

        let size: usize = data.len();
        let total_size = size + 2;
        let size: u16 = match size.try_into() {
            Ok(ok) => ok,
            Err(_err) => return Err(ExceededMaximumCapacity),
        };

        self.resize_or_erase(total_size)?;
        self.write(size.to_be_bytes().as_slice()).unwrap();
        self.write(data).unwrap();
        Ok(())
    }

    pub fn take_tracked(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        if self.len() < 2 {
            return Ok(0);
        }
        let len = u16::from_be_bytes([
            self.data[self.head],
            self.data[self.wrap_in_buffer(self.head + 1)],
        ]);
        let start = self.wrap_in_buffer(self.head);
        let end = self.wrap_in_buffer(self.head + 2 + len as usize);
        if end <= start {
            let v = self.current_capacity() - end;
            buf[..v].copy_from_slice(&self.data[start..]);
            buf[v..].copy_from_slice(&self.data[..end]);
        } else {
            buf[..(len as usize + 2)].copy_from_slice(&self.data[start..end])
        }
        self.erase(len as usize + 2);
        Ok(len as usize + 2)
    }

    fn take(&mut self, amount: usize) {
        self.len -= amount;
        self.head += amount;
        self.tail = self.wrap_in_buffer(self.tail);
    }

    pub fn current_capacity(&self) -> usize {
        self.data.len()
    }

    pub fn max_capacity(&self) -> usize {
        self.max_capacity
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn resize_or_erase(&mut self, amount: usize) -> Result<(), ExceededMaximumCapacity> {
        let old_capacity = self.current_capacity();
        let mut preposed_capacity = (amount + old_capacity).next_power_of_two();

        if preposed_capacity >= self.max_capacity {
            preposed_capacity = self.max_capacity;
            if preposed_capacity < amount {
                return Err(ExceededMaximumCapacity);
            }
            if let Some(min_erase) = amount.checked_sub(preposed_capacity - self.len) {
                self.erase(min_erase);
            }
        }

        if old_capacity != preposed_capacity {
            self.data.resize(preposed_capacity, 0);
            if self.tail <= self.head {
                let (head, tail) = self.data.split_at_mut(self.head);
                let st = old_capacity - head.len();
                tail[st..st + self.tail].copy_from_slice(&head[..self.tail]);
                self.tail += old_capacity;
            }
        }
        Ok(())
    }

    fn erase(&mut self, min_erase: usize) {
        self.len -= min_erase;
        self.head += min_erase;
        self.head = self.wrap_in_buffer(self.head);
        if self.is_empty() {
            self.head = 0;
            self.tail = 0;
        }
    }

    fn erase_tracked(&mut self, min_erase: usize) {
        self.len -= min_erase;
        self.head += min_erase;
        self.head = self.wrap_in_buffer(self.head);
        if self.is_empty() {
            self.head = 0;
            self.tail = 0;
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.len = 0;
        self.head = 0;
        self.tail = 0;
    }
}

impl std::fmt::Debug for RingBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();

        for i in 0..self.len {
            list.entry(&self.data[(i + self.head) % self.data.len()]);
        }

        list.finish()
    }
}

mod test {
    use crate::ringbuffer::RingBuffer;

    #[test]
    pub fn test() {
        let mut bruh = RingBuffer::with_maximum_capacity(32);

        bruh.write_tracked(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        println!("{:02?}", bruh);
        bruh.write_tracked(&[11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
        println!("{:02?}", bruh);
        bruh.write_tracked(&[21, 22, 23, 24, 25, 26, 27, 28, 29, 30]);
        println!("{:02?}", bruh);
        bruh.write_combined_tracked(&[[1, 2, 3, 4, 5, 6, 7, 8].as_slice(); 3])
            .unwrap();
        println!("{:02?}", bruh);
        bruh.write_tracked(&[0xFF; 33]);
        println!("{:02?}", bruh);
        bruh.write_tracked(&[31, 32, 33, 34, 35, 36, 37, 38, 39, 40]);
        println!("{:02?}", bruh);
        bruh.write_tracked(&[41, 42, 43, 44, 45, 46, 47, 48, 49, 50]);
        println!("{:02?}", bruh);
        bruh.write_tracked(&[51, 52, 53, 54, 55, 56, 57, 58, 59, 60]);
        println!("{:02?}", bruh);
        bruh.write_tracked(&[61, 62]);
        println!("{:02?}", bruh);

        bruh.take(12);
        println!("{:?}", bruh);

        bruh.write_tracked(&[63, 64, 255]);
        println!("{:?}", bruh);
    }
}
