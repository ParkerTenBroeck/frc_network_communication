use std::num::NonZeroU16;

use util::{buffer_reader::{BufferReader, ReadFromBuff}, buffer_writter::{WriteToBuff, BufferWritterError}};

use std::fmt::Debug;

use super::error::RobotPacketParseError;

#[derive(Default)]
pub struct Joysticks {
    length: usize,
    data: [Joystick; 6],
}

impl<'a> WriteToBuff<'a> for Joysticks{
    type Error = BufferWritterError;

    fn write_to_buff(&self, buf: &mut util::buffer_writter::BufferWritter<'a>) -> Result<(), Self::Error> {
        for i in 0..self.length{
            self.get_joystick(i).unwrap().write_to_buff(buf)?;
        }
        Ok(())
    }
}

impl Debug for Joysticks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Joysticks")
            .field("joysticks", &&self.data[..self.length])
            .finish()
    }
}

impl Joysticks {
    pub fn push_joystick(&mut self, joystick: Joystick) {
        if self.length < 6 {
            self.data[self.length] = joystick;
            self.length += 1;
        }
    }

    pub fn get_num_joysticks(&self) -> usize {
        self.length
    }

    pub fn get_joystick(&self, index: usize) -> Option<&Joystick> {
        self.data.get(index)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Joystick {
    buttons: JoystickButtonData,
    axises: JoystickAxisData,
    povs: JoystickPovData,
}

impl<'a> WriteToBuff<'a> for Joystick{
    type Error = BufferWritterError;

    fn write_to_buff(&self, buf: &mut util::buffer_writter::BufferWritter<'a>) -> Result<(), Self::Error> {
        
        let size = 
        (self.buttons.get_num_buttons() + 7)  / 8 + 1
        + self.axises.get_num_axis() + 1
        + self.povs.get_num_pov() * 2 + 1;

        buf.write_u8(size as u8)?;
        buf.write_u8(12)?;

        buf.write_u8(self.axises.get_num_axis() as u8)?;
        for i in 0..self.axises.get_num_axis(){
            buf.write_u8(self.axises.get_axis(i) as u8)?;
        }

        buf.write_u8(self.buttons.get_num_buttons() as u8)?;
        for p in (0..((self.buttons.get_num_buttons() + 7) / 8)).rev(){
            buf.write_u8((self.buttons.data >> (p * 8)) as u8)?;
        }


        buf.write_u8(self.povs.get_num_pov() as u8)?;
        for i in 0..self.povs.get_num_pov(){
            buf.write_u16(self.povs.get_pov(i).unwrap_or(0xFFFF))?;
        }


        Ok(())
    }
}

impl<'a> ReadFromBuff<'a> for Joystick {
    type Error = RobotPacketParseError;

    fn read_from_buff(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        let mut axises = JoystickAxisData::default();
        for axis in buf.read_short_u8_arr()? {
            axises.push_axis(*axis as i8);
        }

        let buttons = buf.read_u8()?;
        let mut button_data = 0;
        for _ in 0..((buttons + 7) / 8) {
            button_data = (button_data << 8) | buf.read_u8()? as u64;
        }
        let buttons = JoystickButtonData {
            length: buttons,
            data: button_data,
        };

        let mut povs = JoystickPovData::default();
        for _ in 0..buf.read_u8()? {
            let pov = buf.read_u16()?;
            povs.push_raw_pov(pov)
        }

        if buf.has_more() {
            Err(RobotPacketParseError::InvalidDataLength(
                buf.raw_buff().len(),
            ))?
        }

        Ok(Joystick {
            buttons,
            axises,
            povs,
        })
    }
}

impl Joystick {
    pub fn get_buttons(&self) -> &JoystickButtonData {
        &self.buttons
    }

    pub fn get_axises(&self) -> &JoystickAxisData {
        &self.axises
    }

    pub fn get_povs(&self) -> &JoystickPovData {
        &self.povs
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JoystickButtonData {
    length: u8,
    data: u64,
}

impl Debug for JoystickButtonData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut t = f.debug_struct("JoystickButtonData");
        t.field("length", &self.length);

        for i in 0..self.length {
            t.field(&format!("button_{i}"), &self.get_button(i as usize));
        }
        t.finish()
    }
}

impl JoystickButtonData {
    pub fn get_num_buttons(&self) -> usize {
        self.length as usize
    }

    pub fn get_button(&self, button: usize) -> bool {
        (self.data >> button) & 1 == 1
    }
}

#[derive(Clone)]
pub enum JoystickAxisData<const L_SIZE: usize = 23> {
    Local(u8, [i8; L_SIZE]),
    Heap(Vec<i8>),
}

impl Debug for JoystickAxisData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut t = f.debug_struct("JoystickAxisData");
        t.field("length", &self.get_num_axis());

        for i in 0..self.get_num_axis() {
            t.field(&format!("axis_{i}"), &self.get_axis(i));
        }
        t.finish()
    }
}

impl<const L_SIZE: usize> Default for JoystickAxisData<L_SIZE> {
    fn default() -> Self {
        Self::Local(0, [0; L_SIZE])
    }
}

impl<const L_SIZE: usize> JoystickAxisData<L_SIZE> {
    pub fn get_num_axis(&self) -> usize {
        match self {
            JoystickAxisData::Local(size, _) => *size as usize,
            JoystickAxisData::Heap(buf) => buf.len(),
        }
    }

    pub fn get_axis(&self, axis: usize) -> i8 {
        match self {
            JoystickAxisData::Local(_, buf) => buf[axis],
            JoystickAxisData::Heap(buf) => buf[axis],
        }
    }

    pub fn push_axis(&mut self, axis: i8) {
        match self {
            JoystickAxisData::Local(len, buf) => {
                if (*len as usize) >= L_SIZE {
                    let mut vec = buf.to_vec();
                    vec.push(axis);
                    *self = JoystickAxisData::Heap(vec);
                    return;
                }
                buf[*len as usize] = axis;
                *len += 1;
            }
            JoystickAxisData::Heap(buf) => {
                buf.push(axis);
            }
        }
    }
}

#[derive(Clone)]
pub enum JoystickPovData<const L_SIZE: usize = 8> {
    Local(u8, [Option<NonZeroU16>; L_SIZE]),
    Heap(Vec<Option<NonZeroU16>>),
}

impl Debug for JoystickPovData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut t = f.debug_struct("JoystickPovData");
        t.field("length", &self.get_num_pov());

        for i in 0..self.get_num_pov() {
            t.field(&format!("axis_{i}"), &self.get_pov(i));
        }
        t.finish()
    }
}

impl<const L_SIZE: usize> Default for JoystickPovData<L_SIZE> {
    fn default() -> Self {
        Self::Local(0, [None; L_SIZE])
    }
}

impl<const L_SIZE: usize> JoystickPovData<L_SIZE> {
    pub fn get_num_pov(&self) -> usize {
        match self {
            JoystickPovData::Local(size, _) => *size as usize,
            JoystickPovData::Heap(buf) => buf.len(),
        }
    }

    pub fn get_pov(&self, index: usize) -> Option<u16> {
        match self {
            JoystickPovData::Local(_, buf) => buf[index].map(|val| val.get().wrapping_sub(1)),
            JoystickPovData::Heap(buf) => buf[index].map(|val| val.get().wrapping_sub(1)),
        }
    }

    pub fn push_raw_pov(&mut self, pov: u16) {
        let pov = NonZeroU16::new(pov.wrapping_add(1));
        match self {
            JoystickPovData::Local(len, buf) => {
                if (*len as usize) >= L_SIZE {
                    let mut vec = buf.to_vec();
                    vec.push(pov);
                    *self = JoystickPovData::Heap(vec);
                    return;
                }
                buf[*len as usize] = pov;
                *len += 1;
            }
            JoystickPovData::Heap(buf) => {
                buf.push(pov);
            }
        }
    }
}
