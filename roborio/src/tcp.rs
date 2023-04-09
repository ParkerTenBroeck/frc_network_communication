use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicU16, AtomicU8, AtomicUsize},
};

use net_comm::robot_to_driverstation::Message;
use num_enum::FromPrimitive;
// use num_traits::FromPrimitive;
use util::{
    buffer_reader::{BufferReader, BufferReaderError, CreateFromBuf},
    super_small_vec::SuperSmallVec,
};

use crate::{ringbuffer::RingBuffer, PossibleRcSelf, RoborioCom, RoborioComError};

#[derive(Debug)]
pub(super) struct RoborioTcp {
    reset_con: AtomicU8,

    ds_tcp_connected: AtomicBool,

    bytes_received: AtomicUsize,
    packets_received: AtomicUsize,

    message_number: AtomicU16,

    send_buffer: std::sync::Mutex<RingBuffer>,

    game_data: spin::Mutex<Option<String>>,
    match_info: spin::Mutex<Option<MatchInfo>>,
    controller_info: spin::Mutex<[Option<ControllerInfo>; 6]>,
}

impl Default for RoborioTcp {
    fn default() -> Self {
        Self {
            reset_con: Default::default(),
            ds_tcp_connected: Default::default(),
            bytes_received: Default::default(),
            packets_received: Default::default(),
            message_number: Default::default(),
            send_buffer: std::sync::Mutex::new(RingBuffer::with_maximum_capacity(0x20000)),
            game_data: Default::default(),
            match_info: Default::default(),
            controller_info: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum MatchType {
    None = 0,
    Practis = 1,
    Qualifications = 2,
    Eliminations = 3,
    #[num_enum(catch_all)]
    Unknown(u8),
}

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub name: String,
    pub match_type: MatchType,
    pub match_number: u16,
    pub replay: u8,
}

#[derive(Debug, Clone)]
pub struct ControllerInfo {
    pub js_type: JoystickType,
    pub is_xbox: bool,
    pub name: String,
    pub axis: SuperSmallVec<AxisType, 11>,
    pub buttons: u8,
    pub povs: u8,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash, FromPrimitive)]
#[repr(u8)]
pub enum AxisType {
    XAxis = 0,
    YAxis = 1,
    ZAxis = 2,
    TwistAxis = 3,
    ThrottleAxis = 4,
    #[num_enum(catch_all)]
    Unknown(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash, FromPrimitive)]
#[repr(u8)]
pub enum JoystickType {
    Unknown = 0xFF,
    XInputUnknwon = 0,
    XInputGamepad = 1,
    XInputWheel = 2,
    XInputArcade = 3,
    XInputFlightStick = 4,
    XInputDancePad = 5,
    XInputGuitar = 6,
    XInputGitar2 = 7,
    XInputDrumKit = 8,
    XInputGuitar3 = 11,
    XInputArcadePad = 19,
    HIDJoystick = 20,
    HIDGamepad = 21,
    HIDDriving = 22,
    HIDFlight = 23,
    HID1stPerson = 24,
    #[num_enum(catch_all)]
    UnknownVarient(u8),
}

impl RoborioCom {
    pub(super) fn run_tcp_daemon<
        T: 'static + Send + Sync + PossibleRcSelf + Deref<Target = Self>,
    >(
        myself: &T,
    ) {
        let connection_wait_timeout_ms = 100;

        while (*myself).exists_elsewhere() {
            let listener = match TcpListener::bind("0.0.0.0:1740") {
                Ok(ok) => ok,
                Err(err) => {
                    myself.report_error(crate::RoborioComError::TcpIoInitError(err));
                    std::thread::sleep(std::time::Duration::from_millis(
                        connection_wait_timeout_ms,
                    ));
                    continue;
                }
            };

            let connections = std::sync::Mutex::new(Vec::<TcpStream>::new());
            std::thread::scope(|s| {
                s.spawn(|| {
                    let mut buf = [0u8; 0x2000];

                    while (*myself).exists_elsewhere() {
                        if myself.tcp.ds_tcp_connected.load(atomic::Ordering::Relaxed) {
                            // TODO: read in data

                            let res = myself
                                .tcp
                                .send_buffer
                                .lock()
                                .unwrap()
                                .take_tracked(&mut buf);
                            match res {
                                Ok(size) if size > 0 => {
                                    let buf = &buf[..size];

                                    for connection in &mut *connections.lock().unwrap() {
                                        // so uh idrk what the best chunk size to send is?? but a;dssf;lkjatlkj
                                        connection.write_all(buf).unwrap();
                                    }
                                }
                                Ok(_) => {}
                                Err(err) => {
                                    todo!("{:#?}", err)
                                }
                            }
                        }

                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                });

                // tcp packet recieving from ONLY the currently connected driverstation device.
                s.spawn(|| {
                    while (*myself).exists_elsewhere() {
                        let mut lock = connections.lock().unwrap();
                        let mut ds_stream = None;

                        lock.retain(|stream: &TcpStream| {
                            match stream.peer_addr().map(|ok| Some(ok.ip())) {
                                Ok(ok) => {
                                    if ok == *myself.common.driverstation_ip.lock() {
                                        match stream.try_clone() {
                                            Ok(ok) => {
                                                ds_stream = Some(ok);
                                                true
                                            }
                                            Err(err) => {
                                                myself.report_error(
                                                    crate::RoborioComError::TcpIoInitError(err),
                                                );
                                                false
                                            }
                                        }
                                    } else {
                                        true
                                    }
                                }
                                Err(err) => {
                                    myself.report_error(crate::RoborioComError::TcpIoGeneralError(
                                        err,
                                    ));
                                    false
                                }
                            }
                        });
                        drop(lock);

                        if let Some(mut stream) = ds_stream {
                            match myself.handle_stream_read(&mut stream, myself) {
                                Ok(_) => {
                                    myself
                                        .tcp
                                        .ds_tcp_connected
                                        .store(false, atomic::Ordering::Release);
                                }
                                Err(err) => {
                                    myself
                                        .tcp
                                        .ds_tcp_connected
                                        .store(false, atomic::Ordering::Release);
                                    connections.lock().unwrap().retain(|s2| {
                                        match (stream.peer_addr(), s2.peer_addr()) {
                                            (Ok(addr), Ok(addr2)) => addr != addr2,
                                            _ => false,
                                        }
                                    });
                                    myself.report_error(err);
                                }
                            }
                            let reset_strength =
                                myself.tcp.reset_con.swap(0, atomic::Ordering::Relaxed);
                            if reset_strength > 0 {
                                // drop all connections
                                connections.lock().unwrap().clear()
                            }
                            if reset_strength > 1 {}
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(
                                connection_wait_timeout_ms,
                            ));
                        }
                    }
                });

                // TODO: make this non blocking or timeout
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            connections.lock().unwrap().push(stream);
                        }
                        Err(err) => {
                            myself.report_error(crate::RoborioComError::TcpIoInitError(err))
                        }
                    }
                }
            });
        }
    }

    fn handle_stream_read<T: 'static + Send + Sync + PossibleRcSelf + Deref<Target = Self>>(
        &self,
        stream: &mut TcpStream,
        myself: &T,
    ) -> Result<(), RoborioComError> {
        if let Err(err) = stream.set_nonblocking(false) {
            return Err(crate::RoborioComError::TcpIoInitError(err));
        }
        if let Err(err) = stream.set_read_timeout(Some(std::time::Duration::from_millis(100))) {
            return Err(crate::RoborioComError::TcpIoInitError(err));
        }

        macro_rules! return_if_not_driverstation {
            () => {
            // ignore incomming information unless its from the address our udp socket is currently receiving data from
            match stream.peer_addr().map(|ok|Some(ok.ip())){
                Ok(ok) => {
                    if ok != *self.common.driverstation_ip.lock(){
                        return Ok(());
                    }
                },
                Err(err) => {
                    return Err(crate::RoborioComError::TcpIoGeneralError(err))
                },
            }
            };
        }

        let mut buf = [0u8; 0x10000];
        while myself.exists_elsewhere() && self.tcp.reset_con.load(atomic::Ordering::Relaxed) == 0 {
            if let Err(err) = stream.read_exact(&mut buf[..2]) {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    return_if_not_driverstation!();
                    continue;
                } else {
                    return Err(crate::RoborioComError::TcpIoReceiveError(err));
                }
            }

            // we can unwrap because this will never panic
            let size = u16::from_be_bytes(buf[..2].try_into().unwrap());

            if size == 0 {
                continue;
            }

            let buf = &mut buf[..size as usize];
            while myself.exists_elsewhere()
                && self.tcp.reset_con.load(atomic::Ordering::Relaxed) == 0
            {
                if let Err(err) = stream.read_exact(buf) {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        return_if_not_driverstation!();
                        continue;
                    } else {
                        return Err(crate::RoborioComError::TcpIoReceiveError(err));
                    }
                } else {
                    break;
                }
            }

            self.tcp
                .bytes_received
                .fetch_add(2 + size as usize, atomic::Ordering::Relaxed);

            return_if_not_driverstation!();

            self.tcp
                .ds_tcp_connected
                .store(true, atomic::Ordering::Release);

            let buf = BufferReader::new(buf);

            if let Err(err) = self.read_data(buf) {
                myself.report_error(crate::RoborioComError::TcpPacketReadError(err))
            } else {
                self.tcp
                    .packets_received
                    .fetch_add(1, atomic::Ordering::Relaxed);
            }
        }
        Ok(())
    }

    fn read_data(&self, mut buf: BufferReader<'_>) -> Result<(), BufferReaderError> {
        match buf.read_u8()? {
            0x02 => {
                let index = buf.read_u8()?;
                let is_xbox = buf.read_u8()? == 1;

                let c = ControllerInfo {
                    is_xbox,
                    js_type: JoystickType::from_primitive(buf.read_u8()?),
                    name: buf.read_short_str()?.to_owned(),
                    axis: {
                        let mut axis = SuperSmallVec::new();
                        for _ in 0..buf.read_u8()? {
                            axis.push(AxisType::from_primitive(buf.read_u8()?))
                        }
                        axis
                    },
                    buttons: buf.read_u8()?,
                    povs: buf.read_u8()?,
                };

                if let Some(t) = self.tcp.controller_info.lock().get_mut(index as usize) {
                    *t = Some(c);
                } else {
                    todo!("out of bounds error")
                }
            }
            0x07 => {
                let match_info = MatchInfo {
                    name: buf.read_short_str()?.to_owned(),
                    match_type: MatchType::from_primitive(buf.read_u8()?),
                    match_number: buf.read_u16()?,
                    replay: buf.read_u8()?,
                };
                *self.tcp.match_info.lock() = Some(match_info);
            }
            0x0E => {
                //Game Data
                *self.tcp.game_data.lock() =
                    Some(buf.read_str(buf.remaining_buf_len())?.to_owned());
            }
            val => {
                println!("Unknown data tag: {val:02X}")
            }
        }
        Ok(())
    }
}

impl RoborioCom {
    pub fn send_zero_code(&self, msg: &str) {
        //0x00
        if let Err(err) = self
            .tcp
            .send_buffer
            .lock()
            .unwrap()
            .write_combined_tracked(&[&[0x01], msg.as_bytes()])
        {
            print!("{:#?}", err);
        }
    }

    pub fn send_usage_report(&self) {
        //0x01

        // if let Err(err) = self
        //     .tcp
        //     .send_buffer
        //     .lock()
        //     .unwrap()
        //     .write_combined_tracked(&[&[0x01], msg.as_bytes()])
        // {
        //     print!("{:#?}", err);
        // }
    }

    pub fn send_disable_faults(&self, coms: u16, v12: u16) {
        // 0x04
        let coms = coms.to_be_bytes();
        let v12 = v12.to_be_bytes();
        let data = [0x04, coms[0], coms[1], v12[0], v12[1]];
        if let Err(err) = self
            .tcp
            .send_buffer
            .lock()
            .unwrap()
            .write_tracked(data.as_slice())
        {
            println!("{:#?}", err);
        }
    }

    pub fn send_rail_faults(&self, short_6v: u16, short_5v: u16, short_3_3v: u16) {
        // 0x05
        let short_6v = short_6v.to_be_bytes();
        let short_5v = short_5v.to_be_bytes();
        let short_3_3v = short_3_3v.to_be_bytes();
        let data = [
            0x05,
            short_6v[0],
            short_6v[1],
            short_5v[0],
            short_5v[1],
            short_3_3v[0],
            short_3_3v[1],
        ];
        if let Err(err) = self
            .tcp
            .send_buffer
            .lock()
            .unwrap()
            .write_tracked(data.as_slice())
        {
            println!("{:#?}", err);
        }
    }

    pub fn send_version_info(&self, version_info: &[()]) {
        // 0x0a
    }

    pub fn send_warning(&self) {
        let msg_num = self
            .tcp
            .message_number
            .fetch_add(1, atomic::Ordering::Relaxed);

        let ms = msg_num as u32;
    }

    pub fn send_error(&self) {
        let msg_num = self
            .tcp
            .message_number
            .fetch_add(1, atomic::Ordering::Relaxed);

        let ms = msg_num as u32;
    }

    pub fn send_message(&self, msg: &str) {
        let msg_num = self
            .tcp
            .message_number
            .fetch_add(1, atomic::Ordering::Relaxed);

        let ms = msg_num as u32;

        if let Err(err) = self
            .tcp
            .send_buffer
            .lock()
            .unwrap()
            .write_combined_tracked(&[
                &[0x0C],
                &ms.to_be_bytes(),
                &msg_num.to_be_bytes(),
                msg.as_bytes(),
            ])
        {
            print!("{:#?}", err);
        }
    }

    pub fn send_underline_5v_disabled(&self, disable_5v: u16, underline: [u8; 3]) {
        // 0x0D
        let disable_5v = disable_5v.to_be_bytes();
        let data = [
            0x0D,
            disable_5v[0],
            disable_5v[1],
            underline[0],
            underline[1],
            underline[2],
        ];
        if let Err(err) = self
            .tcp
            .send_buffer
            .lock()
            .unwrap()
            .write_tracked(data.as_slice())
        {
            println!("{:#?}", err);
        }
    }
}

impl RoborioCom {
    pub fn get_game_data(&self) -> Option<String> {
        self.tcp.game_data.lock().clone()
    }

    pub fn get_match_type(&self) -> Option<MatchType> {
        self.tcp.match_info.lock().as_ref().map(|f| f.match_type)
    }

    pub fn get_match_number(&self) -> Option<u16> {
        self.tcp.match_info.lock().as_ref().map(|f| f.match_number)
    }

    pub fn get_match_replay(&self) -> Option<u8> {
        self.tcp.match_info.lock().as_ref().map(|f| f.replay)
    }

    pub fn get_match_name(&self) -> Option<String> {
        self.tcp.match_info.lock().as_ref().map(|f| f.name.clone())
    }

    pub fn get_match_info(&self) -> Option<MatchInfo> {
        self.tcp.match_info.lock().clone()
    }

    pub fn get_controller_info(&self, controller: u8) -> Option<ControllerInfo> {
        self.tcp
            .controller_info
            .lock()
            .get(controller as usize)?
            .clone()
    }

    pub fn controller_is_xbox(&self, controller: u8) -> Option<bool> {
        self.tcp
            .controller_info
            .lock()
            .get(controller as usize)?
            .as_ref()
            .map(|c| c.is_xbox)
    }

    pub fn controller_povs(&self, controller: u8) -> Option<u8> {
        self.tcp
            .controller_info
            .lock()
            .get(controller as usize)?
            .as_ref()
            .map(|c| c.povs)
    }

    pub fn controller_buttons(&self, controller: u8) -> Option<u8> {
        self.tcp
            .controller_info
            .lock()
            .get(controller as usize)?
            .as_ref()
            .map(|c| c.buttons)
    }

    pub fn controller_axis(&self, controller: u8) -> Option<u8> {
        self.tcp
            .controller_info
            .lock()
            .get(controller as usize)?
            .as_ref()
            .map(|c| c.axis.len() as u8)
    }

    pub fn controller_axis_info(&self, controller: u8) -> Option<SuperSmallVec<AxisType, 11>> {
        self.tcp
            .controller_info
            .lock()
            .get(controller as usize)?
            .as_ref()
            .map(|c| c.axis.clone())
    }

    pub fn controller_type(&self, controller: u8) -> Option<JoystickType> {
        self.tcp
            .controller_info
            .lock()
            .get(controller as usize)?
            .as_ref()
            .map(|c| c.js_type)
    }

    pub fn controller_name(&self, controller: u8) -> Option<String> {
        self.tcp
            .controller_info
            .lock()
            .get(controller as usize)?
            .as_ref()
            .map(|c| c.name.clone())
    }
}
