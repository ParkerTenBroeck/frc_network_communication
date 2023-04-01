use std::{borrow::Cow, net::TcpListener, ops::Deref, sync::atomic::AtomicBool, io::{Read, Write}};

use net_comm::robot_to_driverstation::Message;
use util::{buffer_writter::{SliceBufferWritter, BufferWritter, WriteToBuff}, super_small_vec::SuperSmallVec, buffer_reader::BufferReader};

use crate::{PossibleRcSelf, RoborioCom};



#[derive(Default, Debug)]
pub(super) struct RoborioTcp {
    reset_con: AtomicBool,
}

#[derive(Debug)]
pub enum ControllerInfo<'a> {
    None {
        id: u8,
    },
    Some {
        id: u8,
        js_type: JoystickType,
        is_xbox: bool,
        name: Cow<'a, str>,
        axis: SuperSmallVec<u8, 11>,
        // axis: u8,
        // axis_ids: [u8; 12],
        buttons: u8,
        povs: u8,
    },
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash)]
pub enum JoystickType {
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
    Unknown(u8),
}


impl RoborioCom {
    pub(super) fn run_tcp_daemon<T: 'static + Send + PossibleRcSelf + Deref<Target = Self>>(myself: &T) {
        use std::sync::atomic::Ordering::Relaxed;

        while myself.exists_elsewhere() {
            // let buf = [0u8; 2096];
            // let listener = TcpListener::bind("0.0.0.0:1740");
            // let listener = match listener {
            //     Ok(listener) => listener,
            //     Err(err) => {
            //         println!("Failed to start roborio TCP daemon: {err:#?}");
            //         continue;
            //     }
            // };

            let mut buf = [0u8; 4096];
            let listener = TcpListener::bind("0.0.0.0:1740").unwrap();
        
            let mut message_num = 0;
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                println!("Connection established!");
                // stream.set_read_timeout(Some(std::time::Duration::from_micros(0))).unwrap();
        
                let mut res = || -> Result<(), Box<dyn std::error::Error>> {
                    while myself.exists_elsewhere() {
                        let mut send_info = false;
        
                        stream.set_nonblocking(true).unwrap();
                        while let Ok(size) = stream.peek(&mut buf) {
                            if size < 2 {
                                break;
                            }
                            let packet_size = BufferReader::new(&buf).read_u16()? as usize;
                            if size - 2 < packet_size {
                                break;
                            }
                            stream.read_exact(&mut buf[0..packet_size + 2])?;
                            if packet_size == 0 {
                                send_info = true;
                                break;
                            }
        
                            let mut buf = BufferReader::new(&buf);
        
                            let mut buf = buf.read_known_length_u16().unwrap();
                            match buf.read_u8()? {
                                0x02 => {
                                    let index = buf.read_u8()?;
                                    let is_xbox = buf.read_u8()? == 1;
        
                                    // let num_axis;
                                    let controller = if buf.read_u8()? == 1 {
                                        ControllerInfo::Some {
                                            id: index,
                                            is_xbox,
                                            js_type: JoystickType::HIDGamepad,
                                            name: Cow::Borrowed(buf.read_short_str()?),
                                            axis: {
                                                let mut axis = SuperSmallVec::new();
                                                for _ in 0..buf.read_u8()? {
                                                    axis.push(buf.read_u8()?)
                                                }
                                                axis
                                            },
                                            buttons: buf.read_u8()?,
                                            povs: buf.read_u8()?,
                                        }
                                    } else {
                                        ControllerInfo::None { id: index }
                                    };
                                    println!("{controller:#?}");
                                }
                                0x07 => {
                                    // match info
                                    let event_name = buf.read_short_str()?;
                                    // 0 None, 1 Practis, 2 quals, 3 elims
                                    let match_type = buf.read_u8()?;
                                    let match_number = buf.read_u16()?;
                                    let replay_number = buf.read_u8()?;
                                    println!("0x07 => event name: {event_name}, match: {match_type}, match#: {match_number}, replay#: {replay_number}");
                                }
                                0x0E => {
                                    //Game Data
                                    println!("GameData => {:?}", buf.read_str(buf.remaining_buf_len())?);
                                }
                                val => {
                                    println!("Unknown data tag: {val:02X}")
                                }
                            }
                        }
                        let mut bufw = SliceBufferWritter::new(&mut buf);
        
                        stream.set_nonblocking(false).unwrap();
                        let mut send_msg = |mut msg: Message| {
                            let mut bufws = bufw.create_u16_size_guard().unwrap();
                            msg.set_msg_num(message_num);
                            message_num = message_num.wrapping_add(1);
                            msg.set_ms(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u32,
                            );
                            msg.write_to_buf(&mut bufws).unwrap();
                            // bufws.write((8 -(bufws.curr_buf_len() + 2) % 8) %8).unwrap();
                            drop(bufws);
        
                            // stream.write_all(bufw.curr_buf()).unwrap();
                            // bufw.reset();
                        };
                        // println!("{send_info}");
                        if true {
                            send_msg(Message {
                                kind: net_comm::robot_to_driverstation::MessageKind::VersionInfo {
                                    kind: net_comm::robot_to_driverstation::VersionInfo::ImageVersion(
                                        "Holy Cow It's Rust".into(),
                                    ),
                                },
                            });
        
                            send_msg(Message {
                                kind: net_comm::robot_to_driverstation::MessageKind::VersionInfo {
                                    kind: net_comm::robot_to_driverstation::VersionInfo::LibCVersion(
                                        "Lib :3 Rust".into(),
                                    ),
                                },
                            });
        
                            send_msg(Message {
                                kind: net_comm::robot_to_driverstation::MessageKind::VersionInfo {
                                    kind: net_comm::robot_to_driverstation::VersionInfo::Empty(Cow::Borrowed("")),
                                },
                            });
        
                            // send_msg(Message {
                            //     kind: net_comm::robot_to_driverstation::MessageKind::UnderlineAnd5VDisable {
                            //         disable_5v: 123,
                            //         second_top_signal: 2,
                            //         third_top_signal: 2,
                            //         top_signal: 2,
                            //     },
                            // });
        
                            // send_msg(Message {
                            //     kind: net_comm::robot_to_driverstation::MessageKind::DisableFaults {
                            //         comms: 69,
                            //         fault_12v: 55,
                            //     },
                            // });
        
                            // send_msg(Message {
                            //     kind: MessageKind::RailFaults {
                            //         short_3_3v: 12,
                            //         short_5v: 5,
                            //         short_6v: 6,
                            //     },
                            // })
                        }
                        // for _ in 0..20{
        
                        // send_msg(Message::info("Hello!"));
                        //}
                        // send_msg(Message::warn(
                        //     "abc",
                        //     Warnings::Unknown(0x12345678),
                        //         "defg", "hijklmnop"
                        // ));
                        // send_msg(Message::error("This is a Error :0", Errors::Error, "Bruh", ""));
        
                        stream.write_all(bufw.curr_buf())?;
        
                        // println!("Sent Message!");
        
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Ok(())
                };
                println!("{:#?}", res());
            }
            // for stream in listener.incoming() {
            //     let stream = match stream{
            //         Ok(stream) => stream,
            //         Err(_) => todo!(),
            //     }
            // }
        }
    }
}
