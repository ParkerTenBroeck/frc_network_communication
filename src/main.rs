// let mut idk_socket = Socket::new(1130, 1140);
// idk_socket.set_input_nonblocking(true);
// let mut fms_socket = Socket::new(1160, 1120);
// fms_socket.set_input_nonblocking(true);
// let mut netconsole_socket = Socket::new("0.0.0.0:6668", "0.0.0.0:6666");
// netconsole_socket.set_input_nonblocking(true);

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use crate::roborio::simulate_roborio;
use eframe::egui::{self};
use net_comm::driverstation::{
    console_message::{Ignore, SystemConsoleOutput},
    message_handler::MessageConsole,
};
use robot_comm::{
    common::{joystick::NonNegU16, request_code::RobotRequestCode},
    driverstation::RobotComm,
};

#[derive(Default)]
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
        self.val = self.val & 1 | pressed as u8;
    }
    pub fn set_right(&mut self, pressed: bool) {
        self.val = self.val & 0b10 | ((pressed as u8) << 1);
    }
    pub fn set_down(&mut self, pressed: bool) {
        self.val = self.val & 0b100 | ((pressed as u8) << 2);
    }
    pub fn set_left(&mut self, pressed: bool) {
        self.val = self.val & 0b1000 | ((pressed as u8) << 3);
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

pub mod bruh;
pub mod controllers;
pub mod roborio;

fn main() {
    // bruh::run_bruh()
    simulate_roborio()
    // run_driverstation()
}

pub fn run_driverstation() {
    // listener.
    // simulate_roborio();

    let ipaddr =
        robot_comm::util::robot_discovery::find_robot_ip(1114).expect("Failed to find roborio");
    // let ipaddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    // let ipaddr = IpAddr::V4(Ipv4Addr::new(10, 11, 14, 2));
    // println!("FOUND ROBORIO: {:?}", ipaddr);

    let driverstation = RobotComm::new(Some(ipaddr));
    driverstation.start_new_thread();
    driverstation.set_request_code(*RobotRequestCode::new().set_request_lib(true));

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

            ui.input(|i| {
                let speed =
                    i.key_down(egui::Key::W) as i8 * -10 + i.key_down(egui::Key::S) as i8 * 10;
                let turn =
                    i.key_down(egui::Key::A) as i8 * -10 + i.key_down(egui::Key::D) as i8 * 10;

                self.driverstation.modify_joystick(0, |joy| {
                    if let Some(joy) = joy {
                        if joy.get_axis(1).is_none() {
                            joy.set_axis(1, 0).unwrap();
                        }
                        if joy.get_axis(4).is_none() {
                            joy.set_axis(4, 0).unwrap();
                        }
                        joy.set_axis(1, joy.get_axis(1).unwrap().saturating_add(speed))
                            .unwrap();
                        joy.set_axis(4, joy.get_axis(4).unwrap().saturating_add(turn))
                            .unwrap();
                        if speed == 0 {
                            joy.set_axis(1, 0).unwrap();
                        }
                        if turn == 0 {
                            joy.set_axis(4, 0).unwrap();
                        }
                    } else {
                        *joy = Some(Default::default());
                    }
                    // println!("{:#?}", joy);
                });
            });

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
