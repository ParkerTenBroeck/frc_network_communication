use util::{
    buffer_reader::{BufferReader, BufferReaderError, CreateFromBuf, ReadFromBuf},
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
        for joy in &self.data {
            let mut buf = buf.create_u8_size_guard()?;
            buf.write_u8(12)?;

            if let Some(joy) = joy {
                joy.write_to_buf(&mut buf)?;
            } else {
                Joystick::default().write_to_buf(&mut buf)?;
            }
        }
        Ok(())
    }
}

impl Debug for Joysticks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Joysticks")
            .field("joysticks", &self.data)
            .finish()
    }
}

impl Joysticks {
    pub fn insert(&mut self, index: usize, joystick: Joystick) {
        self.data[index] = Some(joystick);
    }

    pub fn remove(&mut self, index: usize) -> Option<Joystick> {
        if let Some(joy) = self.data.get_mut(index) {
            joy.take()
        } else {
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

    pub fn get_o_mut(&mut self, index: usize) -> Option<&mut Option<Joystick>> {
        self.data.get_mut(index)
    }

    #[inline(always)]
    pub fn delete(&mut self, i: usize) {
        self.data[i] = None;
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

    pub fn none() -> Self {
        Self(0xFFFF)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ButtonLenOverflow;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PovLenOverflow;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AxisLenOverflow;

#[derive(Debug)]
pub enum JoystickParseError {
    ButtonLenOverflow(u8),
    PovLenOverflow(u8),
    AxisLenOverflow(u8),
    BufferReaderError(BufferReaderError),
}

impl From<BufferReaderError> for JoystickParseError {
    fn from(value: BufferReaderError) -> Self {
        Self::BufferReaderError(value)
    }
}
impl From<AxisLenOverflow> for JoystickParseError {
    fn from(_: AxisLenOverflow) -> Self {
        Self::AxisLenOverflow(11)
    }
}
impl From<PovLenOverflow> for JoystickParseError {
    fn from(_: PovLenOverflow) -> Self {
        Self::PovLenOverflow(3)
    }
}
impl From<ButtonLenOverflow> for JoystickParseError {
    fn from(_: ButtonLenOverflow) -> Self {
        Self::ButtonLenOverflow(33)
    }
}

#[derive(Clone, Copy)]
pub struct Joystick {
    axis_povs: NonZeroU8,
    buttons_len: u8,
    buttons: u32,
    povs: [NonNegU16; 2],
    axis: [i8; 10],
}

impl Debug for Joystick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buttons: [bool; 32] =
            std::array::from_fn(|i| self.get_button(i as u8).unwrap_or(false));
        f.debug_struct("Joystick")
            .field("buttons", &&buttons[..self.buttons_len() as usize])
            .field("axis", &&self.axis[..self.axis_len() as usize])
            .field("povs", &&self.povs[..self.povs_len() as usize])
            .finish()
    }
}

impl Joystick {
    pub fn new() -> Self {
        Self {
            axis_povs: unsafe { NonZeroU8::new_unchecked(0b1000_0000) },
            buttons_len: 0,
            buttons: 0,
            povs: [NonNegU16(0); 2],
            axis: [0; 10],
        }
    }

    pub fn axis_len(&self) -> u8 {
        self.axis_povs.get() & 0b1111
    }

    pub fn povs_len(&self) -> u8 {
        (self.axis_povs.get() >> 4) & 0b111
    }

    pub fn buttons_len(&self) -> u8 {
        self.buttons_len
    }

    pub fn push_button(&mut self, button: bool) -> Result<(), ButtonLenOverflow> {
        if self.buttons_len < 32 {
            self.buttons |= (button as u32) << self.buttons_len;
            self.buttons_len += 1;
            Ok(())
        } else {
            Err(ButtonLenOverflow)
        }
    }

    pub fn push_axis(&mut self, axis: i8) -> Result<(), AxisLenOverflow> {
        let len = self.axis_len();
        if len < 10 {
            unsafe {
                *self.axis.get_unchecked_mut(len as usize) = axis;
                self.axis_povs = NonZeroU8::new_unchecked(self.axis_povs.get() + 1);
            }
            Ok(())
        } else {
            Err(AxisLenOverflow)
        }
    }

    pub fn push_pov(&mut self, pov: NonNegU16) -> Result<(), PovLenOverflow> {
        let len = self.povs_len();
        if len < 2 {
            unsafe {
                *self.povs.get_unchecked_mut(len as usize) = pov;
                self.axis_povs = NonZeroU8::new_unchecked(self.axis_povs.get() + (1 << 4));
            }
            Ok(())
        } else {
            Err(PovLenOverflow)
        }
    }

    pub fn get_button(&self, button: u8) -> Option<bool> {
        if button < self.buttons_len() {
            Some(self.buttons >> button & 1 == 1)
        } else {
            None
        }
    }

    pub fn get_pov(&self, pov: u8) -> Option<NonNegU16> {
        if pov < self.povs_len() {
            unsafe { Some(*self.povs.get_unchecked(pov as usize)) }
        } else {
            None
        }
    }

    pub fn get_axis(&self, axis: u8) -> Option<i8> {
        if axis < self.axis_len() {
            unsafe { Some(*self.axis.get_unchecked(axis as usize)) }
        } else {
            None
        }
    }

    pub fn clear_axis(&mut self) {
        unsafe { self.axis_povs = NonZeroU8::new_unchecked(self.axis_povs.get() & !0b1111) }
    }

    pub fn clear_povs(&mut self) {
        unsafe { self.axis_povs = NonZeroU8::new_unchecked(self.axis_povs.get() & !0b1110000) }
    }

    pub fn clear_buttons(&mut self) {
        self.buttons = 0;
    }

    pub fn set_button(&mut self, index: u8, bool: bool) -> Result<(), ButtonLenOverflow> {
        if index < 10 {
            self.buttons = self.buttons & !(1 << index as u32) | ((bool as u32) << (index as u32));
            self.buttons_len = self.buttons_len.max(index + 1);
            Ok(())
        } else {
            Err(ButtonLenOverflow)
        }
    }

    pub fn set_pov(&mut self, pov: u8, val: NonNegU16) -> Result<(), PovLenOverflow> {
        if pov < 2 {
            unsafe {
                *self.povs.get_unchecked_mut(pov as usize) = val;
                if pov >= self.povs_len() {
                    self.axis_povs = NonZeroU8::new_unchecked(
                        (self.axis_povs.get() & 0b1000_1111) + ((pov + 1) << 4),
                    );
                }
            }
            Ok(())
        } else {
            Err(PovLenOverflow)
        }
    }

    pub fn set_axis(&mut self, axis: u8, val: i8) -> Result<(), AxisLenOverflow> {
        if axis < 11 {
            unsafe {
                *self.axis.get_unchecked_mut(axis as usize) = val;
                if axis >= self.axis_len() {
                    self.axis_povs =
                        NonZeroU8::new_unchecked((self.axis_povs.get() & 0b1111_0000) + (axis + 1));
                }
            }
            Ok(())
        } else {
            Err(AxisLenOverflow)
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
        buf.write_u8(self.axis_len())?;
        for i in 0..self.axis_len() {
            buf.write_u8(self.get_axis(i).unwrap() as u8)?;
        }

        buf.write_u8(self.buttons_len())?;
        for i in (0..((self.buttons_len() + 7) / 8)).rev() {
            buf.write_u8((self.buttons >> (i * 8)) as u8)?;
        }

        buf.write_u8(self.povs.len() as u8)?;
        for i in 0..self.povs.len() {
            buf.write_u16(self.povs[i].raw())?;
        }

        Ok(())
    }
}

impl<'a> ReadFromBuf<'a> for Joystick {
    type Error = JoystickParseError;

    fn read_into_from_buf(
        &mut self,
        buf: &mut util::buffer_reader::BufferReader<'a>,
    ) -> Result<&mut Self, Self::Error> {
        self.clear_axis();

        for axis in buf.read_short_u8_arr()? {
            self.push_axis(*axis as i8)?;
        }

        self.clear_buttons();
        let buttons_len = buf.read_u8()?;
        if buttons_len > 32 {
            Err(JoystickParseError::ButtonLenOverflow(buttons_len))?
        }

        for _ in 0..((buttons_len + 7) / 8) {
            self.buttons = (self.buttons << 8) | buf.read_u8()? as u32;
        }
        self.buttons_len = buttons_len;

        self.clear_povs();
        for _ in 0..buf.read_u8()? {
            self.push_pov(NonNegU16::new(buf.read_u16()?))?;
        }

        buf.assert_empty()?;

        Ok(self)
    }
}

impl<'a> CreateFromBuf<'a> for Joystick {
    fn create_from_buf(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        let mut joy = Joystick::default();

        for axis in buf.read_short_u8_arr()? {
            joy.push_axis(*axis as i8)?;
        }

        let buttons_len = buf.read_u8()?;
        if buttons_len > 32 {
            Err(JoystickParseError::ButtonLenOverflow(buttons_len))?
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
