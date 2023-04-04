use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicU16},
};

use num_enum::FromPrimitive;
// use num_traits::FromPrimitive;
use util::{
    buffer_reader::{BufferReader, BufferReaderError},
    super_small_vec::SuperSmallVec,
};

use crate::{PossibleRcSelf, RoborioCom};

#[derive(Default, Debug)]
pub(super) struct RoborioTcp {
    reset_con: AtomicBool,

    message_number: AtomicU16,

    game_data: spin::Mutex<Option<String>>,
    match_info: spin::Mutex<Option<MatchInfo>>,
    controller_info: spin::Mutex<[Option<ControllerInfo>; 6]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum MatchType{
    None = 0,
    Practis = 1,
    Qualifications = 2,
    Eliminations = 3,
    #[num_enum(catch_all)]
    Unknown(u8),
}


#[derive(Debug, Clone)]
pub struct MatchInfo{
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
    pub axis: SuperSmallVec<u8, 11>,
    pub buttons: u8,
    pub povs: u8,
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

        while (*myself).exists_elsewhere() {
            let mut buf = [0u8; 4096];
            let listener = match TcpListener::bind("0.0.0.0:1740") {
                Ok(ok) => ok,
                Err(err) => {
                    myself.report_error(crate::RoborioComError::TcpIoInitError(err));
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
            };
            

            let connections = std::sync::Mutex::new(Vec::new());
            std::thread::scope(|s| {
                s.spawn(||{
                    let mut buf = [0u8; 4096];
                    
                    while (*myself).exists_elsewhere(){

                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                });
                // TODO: make this non blocking or timeout
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            let stream2 = match stream.try_clone() {
                                Ok(ok) => ok,
                                Err(err) => {
                                    continue;
                                }
                            };
                            // s.spawn(move || myself.handle_stream_write(stream, myself));
                            s.spawn(move || myself.handle_stream_read(stream2, myself));
                            connections.lock().unwrap().push(stream);
                        }
                        Err(_) => todo!(),
                    }
                }
            });
            // for stream in listener.incoming() {
            //     let stream = match stream{
            //         Ok(stream) => stream,
            //         Err(_) => todo!(),
            //     }
            // }
        }
    }

    fn handle_stream_read<T: 'static + Send + Sync + PossibleRcSelf + Deref<Target = Self>>(
        &self,
        mut stream: TcpStream,
        myself: &T,
    ) {
        if let Err(err) = stream.set_nonblocking(false) {
            panic!("{:#?}", err)
        }
        let mut buf = [0u8; 4096];
        while myself.exists_elsewhere() {
            if let Err(err) = stream.read_exact(&mut buf[..2]) {
                todo!("{:#?}", err);
            }

            // ignore incomming information unless its from the address our udp socket is currently receiving data from 
            match stream.peer_addr().map(|ok|Some(ok.ip())){
                Ok(ok) => {
                    if ok != *self.common.driverstation_ip.lock(){
                        continue;
                    }
                },
                Err(_) => {
                    continue
                },
            } 

            // we can unwrap because this will never panic
            let size = u16::from_be_bytes(buf[..2].try_into().unwrap());
            if size == 0 {
                continue;
            }

            let buf = &mut buf[..size as usize];
            if let Err(err) = stream.read_exact(buf) {
                todo!("{:#?}", err);
            }

            let buf = BufferReader::new(buf);

            if let Err(err) = self.read_data(buf){
                todo!("{:#?}", err);
            }
        }
    }

    fn read_data(&self, mut buf: BufferReader<'_>) -> Result<(), BufferReaderError> {
        match buf.read_u8()? {
            0x02 => {
                let index = buf.read_u8()?;
                let is_xbox = buf.read_u8()? == 1;
                // let exists = buf.read_u8()? == 1;

                let c = ControllerInfo {
                    is_xbox,
                    js_type: JoystickType::from_primitive(buf.read_u8()?),
                    name: buf.read_short_str()?.to_owned(),
                    axis: {
                        let mut axis = SuperSmallVec::new();
                        for _ in 0..buf.read_u8()? {
                            axis.push(buf.read_u8()?)
                        }
                        axis
                    },
                    buttons: buf.read_u8()?,
                    povs: buf.read_u8()?,
                };

                if let Some(t) = self.tcp.controller_info.lock().get_mut(index as usize){
                    *t = Some(c);
                }else{
                    todo!("out of bounds error")
                }
            }
            0x07 => {
                let match_info = MatchInfo{
                    name: buf.read_short_str()?.to_owned(),
                    match_type: MatchType::from_primitive(buf.read_u8()?),
                    match_number: buf.read_u16()?,
                    replay: buf.read_u8()?,
                };
                *self.tcp.match_info.lock() = Some(match_info);
            }
            0x0E => {
                //Game Data
                *self.tcp.game_data.lock() = Some(buf.read_str(buf.remaining_buf_len())?.to_owned());
            }
            val => {
                println!("Unknown data tag: {val:02X}")
            }
        }
        Ok(())
    }

    fn handle_stream_write<T: 'static + Send + Sync + PossibleRcSelf + Deref<Target = Self>>(
        &self,
        mut stream: TcpStream,
        myself: &T,
    ) {
    }
}

impl RoborioCom {
    pub fn set_message(&self) {
        todo!()
    }

    pub fn send_error(&self) {
        todo!()
    }
}

impl RoborioCom{
    pub fn get_game_data(&self) -> Option<String>{
        self.tcp.game_data.lock().clone()
    }

    pub fn get_match_type(&self) -> Option<MatchType>{
        self.tcp.match_info.lock().as_ref().map(|f|f.match_type)
    }

    pub fn get_match_number(&self) -> Option<u16>{
        self.tcp.match_info.lock().as_ref().map(|f|f.match_number)
    }

    pub fn get_match_replay(&self) -> Option<u8>{
        self.tcp.match_info.lock().as_ref().map(|f|f.replay)
    }

    pub fn get_match_name(&self) -> Option<String>{
        self.tcp.match_info.lock().as_ref().map(|f|f.name.clone())
    }

    pub fn get_match_info(&self) -> Option<MatchInfo>{
        self.tcp.match_info.lock().clone()
    }

    pub fn get_controller_info(&self, controller: u8) -> Option<ControllerInfo>{
        self.tcp.controller_info.lock().get(controller as usize)?.clone()
    }
}

// println!("Connection established!");
// // stream.set_read_timeout(Some(std::time::Duration::from_micros(0))).unwrap();

// let mut res = || -> Result<(), Box<dyn std::error::Error>> {
//     while myself.exists_elsewhere() {
//         let mut send_info = false;

//         stream.set_nonblocking(true).unwrap();
//         while let Ok(size) = stream.peek(&mut buf) {
//             if size < 2 {
//                 break;
//             }
//             let packet_size = BufferReader::new(&buf).read_u16()? as usize;
//             if size - 2 < packet_size {
//                 break;
//             }
//             stream.read_exact(&mut buf[0..packet_size + 2])?;
//             if packet_size == 0 {
//                 send_info = true;
//                 break;
//             }

//             let mut buf = BufferReader::new(&buf);

//             let mut buf = buf.read_known_length_u16().unwrap();
//             match buf.read_u8()? {
//                 0x02 => {
//                     let index = buf.read_u8()?;
//                     let is_xbox = buf.read_u8()? == 1;

//                     // let num_axis;
//                     let controller = if buf.read_u8()? == 1 {
//                         ControllerInfo::Some {
//                             id: index,
//                             is_xbox,
//                             js_type: JoystickType::HIDGamepad,
//                             name: Cow::Borrowed(buf.read_short_str()?),
//                             axis: {
//                                 let mut axis = SuperSmallVec::new();
//                                 for _ in 0..buf.read_u8()? {
//                                     axis.push(buf.read_u8()?)
//                                 }
//                                 axis
//                             },
//                             buttons: buf.read_u8()?,
//                             povs: buf.read_u8()?,
//                         }
//                     } else {
//                         ControllerInfo::None { id: index }
//                     };
//                     println!("{controller:#?}");
//                 }
//                 0x07 => {
//                     // match info
//                     let event_name = buf.read_short_str()?;
//                     // 0 None, 1 Practis, 2 quals, 3 elims
//                     let match_type = buf.read_u8()?;
//                     let match_number = buf.read_u16()?;
//                     let replay_number = buf.read_u8()?;
//                     println!("0x07 => event name: {event_name}, match: {match_type}, match#: {match_number}, replay#: {replay_number}");
//                 }
//                 0x0E => {
//                     //Game Data
//                     println!("GameData => {:?}", buf.read_str(buf.remaining_buf_len())?);
//                 }
//                 val => {
//                     println!("Unknown data tag: {val:02X}")
//                 }
//             }
//         }
//         let mut bufw = SliceBufferWritter::new(&mut buf);

//         stream.set_nonblocking(false).unwrap();
//         let mut send_msg = |mut msg: Message| {
//             let mut bufws = bufw.create_u16_size_guard().unwrap();
//             msg.set_msg_num(message_num);
//             message_num = message_num.wrapping_add(1);
//             msg.set_ms(
//                 std::time::SystemTime::now()
//                     .duration_since(std::time::SystemTime::UNIX_EPOCH)
//                     .unwrap()
//                     .as_millis() as u32,
//             );
//             msg.write_to_buf(&mut bufws).unwrap();
//             // bufws.write((8 -(bufws.curr_buf_len() + 2) % 8) %8).unwrap();
//             drop(bufws);

//             // stream.write_all(bufw.curr_buf()).unwrap();
//             // bufw.reset();
//         };
//         // println!("{send_info}");
//         if true {
//             send_msg(Message {
//                 kind: net_comm::robot_to_driverstation::MessageKind::VersionInfo {
//                     kind: net_comm::robot_to_driverstation::VersionInfo::ImageVersion(
//                         "Holy Cow It's Rust".into(),
//                     ),
//                 },
//             });

//             send_msg(Message {
//                 kind: net_comm::robot_to_driverstation::MessageKind::VersionInfo {
//                     kind: net_comm::robot_to_driverstation::VersionInfo::LibCVersion(
//                         "Lib :3 Rust".into(),
//                     ),
//                 },
//             });

//             send_msg(Message {
//                 kind: net_comm::robot_to_driverstation::MessageKind::VersionInfo {
//                     kind: net_comm::robot_to_driverstation::VersionInfo::Empty(Cow::Borrowed("")),
//                 },
//             });

//             // send_msg(Message {
//             //     kind: net_comm::robot_to_driverstation::MessageKind::UnderlineAnd5VDisable {
//             //         disable_5v: 123,
//             //         second_top_signal: 2,
//             //         third_top_signal: 2,
//             //         top_signal: 2,
//             //     },
//             // });

//             // send_msg(Message {
//             //     kind: net_comm::robot_to_driverstation::MessageKind::DisableFaults {
//             //         comms: 69,
//             //         fault_12v: 55,
//             //     },
//             // });

//             // send_msg(Message {
//             //     kind: MessageKind::RailFaults {
//             //         short_3_3v: 12,
//             //         short_5v: 5,
//             //         short_6v: 6,
//             //     },
//             // })
//         }
//         // for _ in 0..20{

//         // send_msg(Message::info("Hello!"));
//         //}
//         // send_msg(Message::warn(
//         //     "abc",
//         //     Warnings::Unknown(0x12345678),
//         //         "defg", "hijklmnop"
//         // ));
//         // send_msg(Message::error("This is a Error :0", Errors::Error, "Bruh", ""));

//         stream.write_all(bufw.curr_buf())?;

//         // println!("Sent Message!");

//         std::thread::sleep(std::time::Duration::from_millis(10));
//     }
//     Ok(())
// };
// println!("{:#?}", res());
