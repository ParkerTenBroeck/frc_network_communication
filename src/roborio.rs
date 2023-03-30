use std::{
    borrow::Cow,
    error::Error,
    io::{Read, Write},
    net::TcpListener,
    sync::Arc,
};

use eframe::{
    egui::{self, Margin, RichText, Slider, TextEdit, Widget},
    epaint::mutex::MutexGuard,
};
use net_comm::{robot_to_driverstation::Message, robot_voltage::RobotVoltage};
use roborio::RoborioCom;
use robot_comm::util::{
    buffer_reader::BufferReader,
    buffer_writter::{BufferWritter, SliceBufferWritter, WriteToBuff},
    super_small_vec::SuperSmallVec,
};
use sysinfo::{CpuExt, SystemExt};

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

struct RioUi {
    driverstation: Arc<RoborioCom>,
    joystick_selected: usize,
}

impl RioUi {
    fn new(driverstation: Arc<RoborioCom>) -> Self {
        Self {
            driverstation,
            joystick_selected: 0,
        }
    }
}

pub fn sysinfo<T>(uh: impl FnOnce(&sysinfo::System) -> T) -> T {
    use std::sync::RwLock;

    static SYSTEM_INFO: RwLock<Option<sysinfo::System>> = RwLock::new(None);
    let val = SYSTEM_INFO.read().unwrap();
    match &*val {
        Some(val) => uh(val),
        None => {
            drop(val);
            *SYSTEM_INFO.write().unwrap() = Some(sysinfo::System::new_all());
            std::thread::spawn(|| loop {
                {
                    if let Some(lock) = SYSTEM_INFO.write().unwrap().as_mut() {
                        lock.refresh_cpu();
                        lock.refresh_memory();
                        lock.refresh_networks();
                        lock.refresh_disks();
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            });
            let val = SYSTEM_INFO.read().unwrap();
            uh(val.as_ref().unwrap())
        }
    }
}

impl eframe::App for RioUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Bruh").show(ctx, |ui| {
            ui.horizontal(|ui| {
                sysinfo(|sysinfo| {
                    ui.label(format!(
                        "CPU: {:.2}%",
                        sysinfo.global_cpu_info().cpu_usage()
                    ));
                });
                ui.add_space(2.0);
                // for (name, info) in self.sysinfo.networks(){
                //     info.
                //     ui.label(format!("Network: {:.2}rx/{:.2}tx", info.));
                //     ui.add_space(2.0);
                // }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let control_code = self.driverstation.get_control_code();
            let request_code = self.driverstation.get_request_code();

            if control_code.is_disabled() {
                self.driverstation.observe_robot_disabled();
            } else if control_code.is_autonomus() {
                self.driverstation.observe_robot_autonomus()
            } else if control_code.is_teleop() {
                self.driverstation.observe_robot_teleop()
            } else if control_code.is_test() {
                self.driverstation.observe_robot_test()
            }

            if request_code.should_restart_roborio_code() {
                // self.driverstation.observe_restart_roborio_code();
            }
            // self.driverstation.request_disable();
            self.driverstation
                .observe_robot_code(!request_code.should_restart_roborio_code());

            // // Plot::new("Bruh").view_aspect(2.0).show(ui, |plot_ui| {});
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(if self.driverstation.is_connected() {
                        "Connected"
                    } else {
                        "Disconnected"
                    });

                    if ui.button("Reset Con").clicked() {
                        self.driverstation.reset_con();
                    }

                    if ui
                        .selectable_label(
                            self.driverstation.get_request_disable(),
                            "Request Disable",
                        )
                        .clicked()
                    {
                        self.driverstation.request_disable()
                    }

                    if ui
                        .selectable_label(self.driverstation.get_request_time(), "Request Time")
                        .clicked()
                    {
                        self.driverstation.request_time()
                    }

                    if ui
                        .selectable_label(self.driverstation.is_brownout_protection(), "Brownout")
                        .clicked()
                    {
                        self.driverstation
                            .observe_robot_brownout(!self.driverstation.is_brownout_protection())
                    }
                    if ui
                        .selectable_label(self.driverstation.is_estopped(), "ESTOP")
                        .clicked()
                    {
                        // self.driverstation
                        //     .observe_robot_estop(!self.driverstation.is_estopped())
                    }

                    if ui.button("Crash Driverstation").clicked() {
                        self.driverstation.crash_driverstation()
                    }

                    let mut battery_val = self.driverstation.get_observed_robot_voltage().to_f32();

                    if ui
                        .add(
                            Slider::new(&mut battery_val, 0.0..=14.0)
                                .smart_aim(false)
                                .text("Battery Voltage"),
                        )
                        .changed()
                    {
                        self.driverstation
                            .observe_robot_voltage(RobotVoltage::from_f32(battery_val))
                    }
                    egui::Frame {
                        stroke: ui.style().visuals.window_stroke,
                        inner_margin: Margin::same(2.0),
                        ..Default::default()
                    }
                    .show(ui, |ui| {
                        egui::Grid::new("my_grid")
                            .num_columns(2)
                            .spacing([40.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Packets Dropped");
                                ui.label(self.driverstation.get_udp_packets_dropped().to_string());
                                ui.end_row();

                                ui.label("Packets Send");
                                ui.label(self.driverstation.get_udp_packets_sent().to_string());
                                ui.end_row();

                                ui.label("Bytes Sent");
                                ui.label(self.driverstation.get_udp_bytes_sent().to_string());
                                ui.end_row();

                                ui.label("Packets Received");
                                ui.label(self.driverstation.get_udp_packets_received().to_string());
                                ui.end_row();

                                ui.label("Bytes Received");
                                ui.label(self.driverstation.get_udp_bytes_received().to_string());
                                ui.end_row();
                            });
                    });
                    // ui.separator();
                    // ui.separator();

                    // ui.label(format!(
                    //     "Packets Dropped: {}",
                    //     self.driverstation.get_udp_packets_dropped()
                    // ));
                    // ui.label(format!(
                    //     "Packets Sent: {}",
                    //     self.driverstation.get_udp_packets_sent()
                    // ));
                    // ui.label(format!(
                    //     "Bytes Send: {}",
                    //     self.driverstation.get_udp_bytes_sent()
                    // ));
                    // ui.label(format!(
                    //     "Packets Received: {}",
                    //     self.driverstation.get_udp_packets_received()
                    // ));
                    // ui.label(format!(
                    //     "Bytes Received: {}",
                    //     self.driverstation.get_udp_bytes_received()
                    // ));

                    ui.add_space(1.0);

                    if let Some(countdown) = self.driverstation.get_countdown() {
                        ui.label(format!("Countdown: {countdown}s"));
                    } else {
                        ui.label("Countdown: None");
                    }

                    let timedata = self.driverstation.get_time();
                    ui.label(format!("Timedata: {:#?}", timedata));

                    let control_code = self.driverstation.get_control_code();

                    if control_code.is_disabled() {
                        ui.label("mode: disabled");
                    } else if control_code.is_teleop() {
                        ui.label("mode: teleop");
                    } else if control_code.is_autonomus() {
                        ui.label("mode: autonomus");
                    } else if control_code.is_test() {
                        ui.label("mode: test");
                    } else {
                        ui.label("mode: unknwon?");
                    }

                    ui.label(format!(
                        "Alliance Station: {:#?}",
                        self.driverstation.get_alliance_station()
                    ));

                    ui.label(format!("{:#?}", self.driverstation.get_request_code()));
                    ui.label(format!("{:#?}", self.driverstation.get_control_code()));

                    // if ui
                    //     .selectable_label(control_code.is_brown_out_protection(), "ESTOPED")
                    //     .changed()
                    // {
                    //     driverstation
                    //         .observe_robot_brownout(!control_code.is_brown_out_protection());
                    // }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.collapsing("Disk Usage", |ui| {
                        ui.label("Disk Usage: Bytes free");
                        let mut disk = format!(
                            "{}",
                            self.driverstation
                                .get_disk_usage()
                                .map(|f| f.bytes_free)
                                .unwrap_or(0)
                        );
                        let response = TextEdit::singleline(&mut disk).desired_width(150.0).ui(ui);
                        if response.changed() {
                            if let Ok(val) = str::parse(&disk) {
                                self.driverstation.set_disk_usage(Some(
                                    robot_comm::robot_to_driver::RobotToDriverDiskUsage {
                                        bytes_free: val,
                                    },
                                ));
                            }
                        }
                    });

                    ui.collapsing("Cpu Usage", |ui| {});

                    ui.collapsing("Ram Usage", |ui| {
                        ui.label("Ram Usage: Bytes free");
                        let mut ram = format!(
                            "{}",
                            self.driverstation
                                .get_ram_usage()
                                .map(|f| f.bytes_free)
                                .unwrap_or(0)
                        );
                        let response = TextEdit::singleline(&mut ram).desired_width(150.0).ui(ui);
                        if response.changed() {
                            if let Ok(val) = str::parse(&ram) {
                                self.driverstation.set_ram_usage(Some(
                                    robot_comm::robot_to_driver::RobotToDriverRamUsage {
                                        bytes_free: val,
                                    },
                                ));
                            }
                        }
                    });

                    ui.collapsing("Pdp Port Report", |ui| {
                        // let mut ram = format!("{}", self.driverstation.get_ram_usage().map(|f|f.bytes_free).unwrap_or(0));
                        // let response = TextEdit::singleline(&mut ram).desired_width(150.0).ui(ui);
                        // if response.changed(){
                        //     if let Ok(val) = str::parse(&ram){
                        //             self.driverstation.set_ram_usage(Some(robot_comm::robot_to_driver::RobotToDriverRamUsage { bytes_free: val }));
                        //     }
                        // }
                    });

                    ui.collapsing("Pdp Power Report", |ui| {
                        let mut ram = format!(
                            "{}",
                            self.driverstation
                                .get_ram_usage()
                                .map(|f| f.bytes_free)
                                .unwrap_or(0)
                        );
                        let response = TextEdit::singleline(&mut ram).desired_width(150.0).ui(ui);
                        if response.changed() {
                            if let Ok(val) = str::parse(&ram) {
                                self.driverstation.set_ram_usage(Some(
                                    robot_comm::robot_to_driver::RobotToDriverRamUsage {
                                        bytes_free: val,
                                    },
                                ));
                            }
                        }
                    });

                    ui.collapsing("Can Usage", |ui| {
                        // let mut ram = format!("{}", self.driverstation.get_ram_usage().map(|f|f.bytes_free).unwrap_or(0));
                        // let response = TextEdit::singleline(&mut ram).desired_width(150.0).ui(ui);
                        // if response.changed(){
                        //     if let Ok(val) = str::parse(&ram){
                        //             self.driverstation.set_ram_usage(Some(robot_comm::robot_to_driver::RobotToDriverRamUsage { bytes_free: val }));
                        //     }
                        // }
                    });
                });

                ui.separator();

                // let last_core = self.driverstation.get_last_core_data();
                // ui.vertical(|ui| {
                //     ui.label(&format!("{:#?}", last_core.control_code));
                //     ui.label(&format!("{:#?}", last_core.station));
                //     ui.label(&format!("{:#?}", last_core.request_code));

                //     if ui
                //         .add(
                //             egui::Slider::new(&mut self.battery_voltage, 0.0..=13.5)
                //                 .show_value(true),
                //         )
                //         .changed()
                //     {
                //         self.driverstation
                //             .observe_robot_voltage(RobotVoltage::from_f32(self.battery_voltage));
                //     }
                //     {
                //         // self.ss = last_core.tag_comm_version;
                //         let mut status = self.driverstation.get_observe();
                //         let mut str = format!("{:08b}", status.to_bits());
                //         if ui.text_edit_singleline(&mut str).changed() {
                //             if let Ok(val) = u8::from_str_radix(&str, 2) {
                //                 status = RobotStatusCode::from_bits(val);
                //                 self.driverstation.set_observe(status);
                //             }
                //         }
                //         ui.label(&format!("{:#?}", status));
                //     }
                //     {
                //         let mut control = self.driverstation.get_control();
                //         let mut str = format!("{:08b}", control.to_bits());
                //         if ui.text_edit_singleline(&mut str).changed() {
                //             if let Ok(val) = u8::from_str_radix(&str, 2) {
                //                 control = ControlCode::from_bits(val);
                //                 self.driverstation.set_control(control);
                //             }
                //         }
                //         ui.label(&format!("{:#?}", control));
                //     }
                //     {
                //         let mut request = self.driverstation.get_request();
                //         let mut str = format!("{:08b}", request.to_bits());
                //         if ui.text_edit_singleline(&mut str).changed() {
                //             if let Ok(val) = u8::from_str_radix(&str, 2) {
                //                 request = DriverstationRequestCode::from_bits(val);
                //                 self.driverstation.set_request(request);
                //             }
                //         }
                //         ui.label(&format!("{:#?}", request));
                //     }
                // });
                ui.vertical(|ui| {
                    for i in 0..6 {
                        if ui
                            .selectable_label(
                                self.joystick_selected == i,
                                &format!("Joystick {}", i + 1),
                            )
                            .clicked()
                        {
                            self.joystick_selected = i;
                        }
                    }
                });

                ui.vertical(|ui| {
                    if let Some(joy) = self.driverstation.get_joystick(self.joystick_selected) {
                        if joy.axis_len() == 0 && joy.povs_len() == 0 && joy.buttons_len() == 0 {
                            ui.label("Empty? (reserved but not found)");
                        }
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                for i in 0..joy.buttons_len() {
                                    _ = ui.selectable_label(
                                        joy.get_button(i).unwrap(),
                                        format!("axis_{}", i),
                                    );
                                }
                                for i in 0..joy.povs_len() {
                                    ui.label(format!(
                                        "pov_{}: {}",
                                        i,
                                        joy.get_pov(i)
                                            .unwrap()
                                            .get()
                                            .map(|val| val.to_string())
                                            .unwrap_or("None".to_owned())
                                    ));
                                }
                            });
                            ui.vertical(|ui| {
                                for i in 0..joy.axis_len() {
                                    ui.add(
                                        egui::widgets::ProgressBar::new(
                                            (joy.get_axis(i).unwrap() as f32 / 128.0 * 0.5) + 0.5,
                                        )
                                        .text(
                                            RichText::new(format!(
                                                "axis_{:+.4} {}",
                                                joy.get_axis(i).unwrap(),
                                                i
                                            ))
                                            .monospace(),
                                        ),
                                    );
                                }
                            });
                        });
                    } else {
                        ui.label("Not connected");
                    }
                });
            });

            ctx.request_repaint();
        });
    }
}

fn idk(){
    let mut buf = [0u8; 4096];
    let listener = TcpListener::bind("0.0.0.0:1740").unwrap();

    let mut message_num = 0;
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        println!("Connection established!");
        // stream.set_read_timeout(Some(std::time::Duration::from_micros(0))).unwrap();

        let mut res = || -> Result<(), Box<dyn Error>> {
            loop {
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
                            let comp = buf.read_short_str()?;
                            // 0 None, 1 Practis, 2 quals, 3 elims
                            let match_style = buf.read_u8()?;
                            let match_number = buf.read_u16()?;
                            let replay_number = buf.read_u8()?;
                            println!("0x07 => Comp: {comp}, match: {match_style}, match#: {match_number}, replay#: {replay_number}");
                        }
                        0x0E => {
                            //Game Data
                            println!(
                                "GameData => {:?}",
                                buf.read_str(buf.remaining_buf_len())?
                            );
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
                            kind: net_comm::robot_to_driverstation::VersionInfo::Empty,
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

                stream.write_all(bufw.curr_buf()).unwrap();

                // println!("Sent Message!");

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };
        println!("{:#?}", res());
    }
}

pub fn simulate_roborio() {
    // let com = DriverstationComm::start_comm();
    let com = Arc::new(roborio::RoborioCom::default());
    roborio::RoborioCom::start_daemon(com.clone());

    std::thread::spawn(||{
        loop{
            let res = std::panic::catch_unwind(idk);
            println!("{:#?}", res);
        }
    });

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(720.0, 480.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Roborio Station",
        options,
        Box::new(|_cc| Box::new(RioUi::new(com))),
    )
    .unwrap();
}
