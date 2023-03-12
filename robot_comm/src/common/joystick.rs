use util::{
    buffer_reader::{BufferReader, ReadFromBuff, BufferReaderError},
    buffer_writter::{BufferWritter, BufferWritterError, WriteToBuff},
};

use std::{fmt::Debug, num::NonZeroU8};

#[derive(Default, Clone, Copy)]
pub struct Joysticks {
    data: [Option<Joystick>; 6],
}

impl<'a> WriteToBuff<'a> for Joysticks {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        for joy in self.data.iter() {
            if let Some(joy) = joy {
                joy.write_to_buf(buf)?;
            } else {
                Joystick::default().write_to_buf(buf)?;
            }
        }
        Ok(())
    }
}

impl Debug for Joysticks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Joysticks")
            //.field("joysticks", &&self.data[..self.length])
            .finish()
    }
}

impl Joysticks {
    pub fn insert(&mut self, index: usize, joystick: Joystick) {
        self.data[index] = Some(joystick);
    }

    pub fn remove(&mut self, index: usize) -> Option<Joystick> {
        if let Some(joy) = self.data.get_mut(index){
            joy.take()
        }else{
            None
        }
    }

    pub fn count(&self) -> usize {
        self.data.iter().filter(|p| p.is_some()).count()
    }

    pub fn get(&self, index: usize) -> Option<&Joystick> {
        if let Some(Some(joy)) = self.data.get(index) {
            Some(joy)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Joystick> {
        if let Some(Some(joy)) = self.data.get_mut(index) {
            Some(joy)
        } else {
            None
        }
    }
}


#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonNegU16(u16);

impl Debug for NonNegU16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NonNegU16").field(&self.get()).finish()
    }
}

impl NonNegU16 {
    pub fn new(val: u16) -> Self {
        Self(val)
    }

    pub fn get(&self) -> Option<u16> {
        if self.0 == 0xFFFF {
            None
        } else {
            Some(self.0)
        }
    }

    pub fn raw(&self) -> u16 {
        self.0
    }
}



pub struct ButtonLenOverflow;
pub struct PovLenOverflow;
pub struct AxisLenOverflow;

#[derive(Debug)]
pub enum JoystickParseError{
    ButtonLenOverflow(u8),
    PovLenOverflow(u8),
    AxisLenOverflow(u8),
    BufferReaderError(BufferReaderError)
}


impl From<BufferReaderError> for JoystickParseError{
    fn from(value: BufferReaderError) -> Self {
        Self::BufferReaderError(value)
    }
}
impl From<AxisLenOverflow> for JoystickParseError{
    fn from(_: AxisLenOverflow) -> Self {
        Self::AxisLenOverflow(11)
    }
}
impl From<PovLenOverflow> for JoystickParseError{
    fn from(_: PovLenOverflow) -> Self {
        Self::PovLenOverflow(3)
    }
}
impl From<ButtonLenOverflow> for JoystickParseError{
    fn from(_: ButtonLenOverflow) -> Self {
        Self::ButtonLenOverflow(33)
    }
}

#[derive(Clone, Copy)]
pub struct Joystick{
    axis_povs: NonZeroU8,
    buttons_len: u8,
    buttons: u32,
    povs: [NonNegU16; 2],
    axis: [i8; 10]
}

impl Debug for Joystick{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buttons: [bool; 32] = std::array::from_fn(|i|{
            self.get_button(i as u8).unwrap_or(false)
        });
        f.debug_struct("Joystick")
        .field("buttons", &&buttons[..self.buttons_len() as usize])
        .field("axis", &&self.axis[..self.axis_len() as usize])
        .field("povs", &&self.povs[..self.povs_len() as usize])
        .finish()
    }
}


impl Joystick{
    pub fn new() -> Self{
        Self { 
            axis_povs: unsafe { NonZeroU8::new_unchecked(0b1000_0000)},
            buttons_len: 0, 
            buttons: 0, 
            povs: [NonNegU16(0);2], 
            axis: [0;10] 
        }
    }

    pub fn axis_len(&self) -> u8{
        self.axis_povs.get() & 0b1111
    }

    pub fn povs_len(&self) -> u8{
        (self.axis_povs.get() >> 4) & 0b111
    }

    pub fn buttons_len(&self) -> u8{
        self.buttons_len
    }

    pub fn push_button(&mut self, button: bool) -> Result<(), ButtonLenOverflow>{
        if self.buttons_len < 32{
            self.buttons |= (button as u32) << self.buttons_len;
            self.buttons_len += 1;
            Ok(())
        }else{
            Err(ButtonLenOverflow)
        }
    }

    pub fn push_axis(&mut self, axis: i8) -> Result<(), AxisLenOverflow>{
        let len = self.axis_len();
        if  len < 10{
            unsafe{
                *self.axis.get_unchecked_mut(len as usize) = axis;
                self.axis_povs = NonZeroU8::new_unchecked(self.axis_povs.get() + 1);
            }
            Ok(())
        }else{
            Err(AxisLenOverflow)
        }
    }

    pub fn push_pov(&mut self, pov: NonNegU16) -> Result<(), PovLenOverflow>{
        let len = self.povs_len();
        if len < 2{
            unsafe{
                *self.povs.get_unchecked_mut(len as usize) = pov;
                self.axis_povs = NonZeroU8::new_unchecked(self.axis_povs.get() + (1 << 4));
            }
            Ok(())
        }else{
            Err(PovLenOverflow)
        }
    }

    pub fn get_button(&self, button: u8) -> Option<bool>{
        if button < self.buttons_len(){
            Some(self.buttons >> button & 1 == 1)
        }else{
            None
        }
    }

    pub fn get_pov(&self, pov: u8) -> Option<NonNegU16>{
        if pov < self.povs_len(){
            unsafe {
                Some(*self.povs.get_unchecked(pov as usize))
            }
        }else{
            None
        }
    }

    pub fn get_axis(&self, axis: u8) -> Option<i8>{
        if axis < self.axis_len(){
            unsafe {
                Some(*self.axis.get_unchecked(axis as usize))
            }
        }else{
            None
        }
    }
}

impl Default for Joystick {
    fn default() -> Self {
        Self::new()
    }
}



impl<'a> WriteToBuff<'a> for Joystick {
    type Error = BufferWritterError;

    fn write_to_buf<T: BufferWritter<'a>>(&self, buf: &mut T) -> Result<(), Self::Error> {
        let mut buf = buf.create_u8_size_guard()?;

        buf.write_u8(12)?;

        buf.write_u8(self.axis_len())?;
        for i in 0..self.axis_len() {
            buf.write_u8(self.get_axis(i).unwrap() as u8)?;
        }

        buf.write_u8(self.buttons_len())?;
        for p in (0..((self.buttons_len() + 7) / 8)).rev() {
            buf.write_u8((self.buttons >> (p * 8)) as u8)?;
        }

        buf.write_u8(self.povs.len() as u8)?;
        for i in 0..self.povs.len() {
            buf.write_u16(self.povs[i].raw())?;
        }

        Ok(())
    }
}

impl<'a> ReadFromBuff<'a> for Joystick {
    type Error = JoystickParseError;

    fn read_from_buff(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        let mut joy = Joystick::default();
        
        for axis in buf.read_short_u8_arr()? {
            joy.push_axis(*axis as i8)?;
        }

        let buttons_len = buf.read_u8()?;
        if buttons_len > 32{
            //TODO: return an error
        }
        for _ in 0..((buttons_len + 7) / 8) {
            joy.buttons = (joy.buttons << 8) | buf.read_u8()? as u32;
        }
        joy.buttons_len = buttons_len;
        

        for _ in 0..buf.read_u8()? {
            joy.push_pov(NonNegU16::new(buf.read_u16()?))?;
        }

        buf.assert_empty()?;

        Ok(joy)
    }
}


