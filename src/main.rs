// let mut idk_socket = Socket::new(1130, 1140);
// idk_socket.set_input_nonblocking(true);
// let mut fms_socket = Socket::new(1160, 1120);
// fms_socket.set_input_nonblocking(true);
// let mut netconsole_socket = Socket::new("0.0.0.0:6668", "0.0.0.0:6666");
// netconsole_socket.set_input_nonblocking(true);

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

pub fn simulate_roborio() {
    let driverstation: Arc<DriverstationComm> = DriverstationComm::start_comm();

    let ds = driverstation.clone();

    std::thread::spawn(move || {
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
                                        js_type: todo!(),
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

                    if let Some(joystick) = ds.get_joystick(0) {
                        // send_msg(Message::warn(
                        //     format!("{:#?}", joystick),
                        //     Warnings::Unknown(0x12345678),
                        //     "defg",
                        //     "hijklmnop",
                        // ));
                        // send_msg(Message::info(format!("{:#?}", joystick)));
                        // joystick.get_axises().clone()
                    }

                    if send_info {
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

                        send_msg(Message {
                            kind: net_comm::robot_to_driverstation::MessageKind::UnderlineAnd5VDisable {
                                disable_5v: 123,
                                second_top_signal: 2,
                                third_top_signal: 2,
                                top_signal: 2,
                            },
                        });

                        send_msg(Message {
                            kind: net_comm::robot_to_driverstation::MessageKind::DisableFaults {
                                comms: 69,
                                fault_12v: 55,
                            },
                        });

                        send_msg(Message {
                            kind: MessageKind::RailFaults {
                                short_3_3v: 12,
                                short_5v: 5,
                                short_6v: 6,
                            },
                        })
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

                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            };
            println!("{:#?}", res());
        }
    });

    loop {
        let last_core = driverstation.get_last_core_data();
        driverstation.observe_robot_code();
        driverstation.request_time();

        let control_code = last_core.control_code;
        if control_code.is_autonomus() {
            driverstation.observe_robot_autonomus()
        } else if control_code.is_teleop() {
            driverstation.observe_robot_teleop()
        } else if control_code.is_test() {
            driverstation.observe_robot_test()
        }

        if control_code.is_enabled() {
            driverstation.observe_robot_enabled();
        } else {
            driverstation.observe_robot_disabled();
        }

        if let Some(joystick) = driverstation.get_joystick(0) {
            let int = 69; //(((127.0 - joystick.get_axis(1).unwrap() as f32) / 255.0) * 30.0) as u8;
            let dec = (127 - joystick.get_axis(5).unwrap() as i32) as u8;

            driverstation.observe_robot_voltage(RobotVoltage { int, dec })
        }

        driverstation.request_time();

        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

use std::{
    borrow::Cow,
    error::Error,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, TcpListener},
    pin::Pin,
    sync::Arc,
};

use eframe::egui::{self, Context};
use gilrs::Gilrs;
use net_comm::{
    driverstation::{
        console_message::{Ignore, SystemConsoleOutput},
        message_handler::MessageConsole,
    },
    robot_to_driverstation::{error::Warnings, Message, MessageKind},
    robot_voltage::RobotVoltage,
};
use robot_comm::{
    common::{joystick::{Joystick, self, NonNegU16}, request_code::RobotRequestCode},
    driverstation::{RobotComm, self},
    robot::DriverstationComm,
    util::{
        buffer_reader::BufferReader,
        buffer_writter::{BufferWritter, SliceBufferWritter, WriteToBuff},
        robot_discovery::find_robot_ip,
        super_small_vec::SuperSmallVec,
    },
};
use stick::Listener;

type Girls = Gilrs;

use pasts::Loop;
use std::task::Poll::{self, Pending, Ready};
use stick::{Controller, Event};

type Exit = usize;

#[derive(Default)]
struct Pov{
    val: u8,
}
impl Pov{
    pub fn to_val(&self) -> NonNegU16{
        match self.val{
            0b0001 => NonNegU16::new(0),
            0b0011 => NonNegU16::new(45),
            0b0010 => NonNegU16::new(90),
            0b0110 => NonNegU16::new(90 + 45),
            0b0100 => NonNegU16::new(180),
            0b1100 => NonNegU16::new(180 + 45),
            0b1000 => NonNegU16::new(180 + 90),
            0b1001 => NonNegU16::new(180 + 90 + 45),
            _ => NonNegU16::none()
        }
    }

    pub fn set_up(&mut self, pressed: bool){
        self.val = self.val & 1 | pressed as u8;
    }
    pub fn set_right(&mut self, pressed: bool){
        self.val = self.val & 0b10 | ((pressed as u8) << 1);
    }
    pub fn set_down(&mut self, pressed: bool){
        self.val = self.val & 0b100 | ((pressed as u8) << 2);
    }
    pub fn set_left(&mut self, pressed: bool){
        self.val = self.val & 0b1000 | ((pressed as u8) << 3);
    }
}

struct State {
    listener: Listener,
    controllers: Vec<Controller>,
    povs: Vec<Pov>,
    rumble: (f32, f32),
    comm: Arc<RobotComm>
}

impl State {
    fn connect(&mut self, controller: Controller) -> Poll<Exit> {

        if controller.name() != "input-remapper gamepad"{
            println!(
                "Connected p{}, id: {:016X}, name: {}",
                self.controllers.len() + 1,
                controller.id(),
                controller.name(),
            );
            let mut joy = Joystick::default();
            joy.set_button(9, false).unwrap();
            joy.set_pov(0, NonNegU16::none()).unwrap();
            joy.set_axis(5, 0).unwrap();
            self.comm.update_joystick(self.controllers.len(), joy);
            self.controllers.push(controller);
            self.povs.push(Default::default());
        }
        Pending
    }

    fn event(&mut self, id: usize, event: Event) -> Poll<Exit> {

        let player = id + 1;
        println!("p{}: {}", player, event);
        
        
        self.comm.modify_joystick(id, |joy|{
            if let Event::Disconnect = event {
                self.controllers.swap_remove(id);
                *joy = None;
            }else if let Some(joy) = joy{
                match event{
                    Event::ActionA(bool) => joy.set_button(0, bool).unwrap(),
                    Event::ActionB(bool) => joy.set_button(1, bool).unwrap(),
                    Event::ActionV(bool) => joy.set_button(2, bool).unwrap(),
                    Event::ActionH(bool) => joy.set_button(3, bool).unwrap(),

                    Event::MenuL(bool) => joy.set_button(4, bool).unwrap(),
                    Event::MenuR(bool) => joy.set_button(5, bool).unwrap(),

                    Event::BumperL(bool) => joy.set_button(6, bool).unwrap(),
                    Event::BumperR(bool) => joy.set_button(7, bool).unwrap(),
                    Event::Joy(bool) => joy.set_button(8, bool).unwrap(),
                    Event::Cam(bool) => joy.set_button(9, bool).unwrap(),


                    Event::ActionC(bool) => joy.set_button(2, bool).unwrap(),
                    Event::ActionD(bool) => joy.set_button(3, bool).unwrap(),
                    Event::ActionL(bool) => joy.set_button(0, bool).unwrap(),
                    Event::ActionM(bool) => joy.set_button(0, bool).unwrap(),
                    Event::ActionR(bool) => joy.set_button(0, bool).unwrap(),


                    Event::JoyX(val) => joy.set_axis(0, (val * 255.0) as i8).unwrap(),
                    Event::JoyY(val) => joy.set_axis(1, (val * 255.0) as i8).unwrap(),
                    Event::JoyZ(val) => joy.set_axis(2, (val * 255.0 * 255.0) as i8).unwrap(),
                    Event::CamZ(val) => joy.set_axis(3, (val * 255.0 * 255.0) as i8).unwrap(),
                    Event::CamX(val) => joy.set_axis(4, (val * 255.0) as i8).unwrap(),
                    Event::CamY(val) => joy.set_axis(5, (val * 255.0) as i8).unwrap(),

                    Event::PovUp(pressed) => {
                        self.povs[id].set_up(pressed);
                        joy.set_pov(0, self.povs[id].to_val()).unwrap();
                    }
                    Event::PovRight(pressed) => {
                        self.povs[id].set_right(pressed);
                        joy.set_pov(0, self.povs[id].to_val()).unwrap();
                    }
                    Event::PovDown(pressed) => {
                        self.povs[id].set_down(pressed);
                        joy.set_pov(0, self.povs[id].to_val()).unwrap();
                    }
                    Event::PovLeft(pressed) => {
                        self.povs[id].set_left(pressed);
                        joy.set_pov(0, self.povs[id].to_val()).unwrap();
                    }
                    _ => {}
                }
                println!("{joy:#?}")
            }
        });
        Pending
    }
}

async fn event_loop(comm: Arc<RobotComm>) {
    let mut state = State {
        listener: Listener::default(),
        povs: Vec::new(),
        controllers: Vec::new(),
        rumble: (0.0, 0.0),
        comm,
    };

    let player_id = Loop::new(&mut state)
        .when(|s| &mut s.listener, State::connect)
        .poll(|s| &mut s.controllers[..], State::event)
        .await;

    println!("p{} ended the session", player_id);
}

fn main() {
    // stick::focus();
    // stick::Controller::

    // listener.
    // simulate_roborio();

    let ipaddr = find_robot_ip(1114).expect("Failed to find roborio");
    // let ipaddr = IpAddr::V4(Ipv4Addr::new(10, 11, 14, 2));
    println!("FOUND ROBORIO: {:?}", ipaddr);

    let driverstation = RobotComm::new(Some(ipaddr));
    driverstation.start_new_thread();
    driverstation.set_request_code(*RobotRequestCode::new().set_normal(true));

    MessageConsole::create_new_thread(SystemConsoleOutput {}, ipaddr);
    // MessageConsole::new(SystemConsoleOutput {}).run_blocking(ipaddr);
    {
        let driverstation = driverstation.clone();
        std::thread::spawn(move ||{
            pasts::block_on(event_loop(driverstation));
        });
    }

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Driver Station",
        options,
        Box::new(|_cc| {
            Box::new(MyApp {
                driverstation,
            })
        }),
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

            if control.is_estop() {
                ui.label("ESTOP");
            }

            if control.is_driverstation_attached() {
                ui.label("NO IDEA");
            }

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

            ctx.request_repaint();
        });
    }
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub enum ResourceType {
    kResourceType_Controller,
    kResourceType_Module,
    kResourceType_Language,
    kResourceType_CANPlugin,
    kResourceType_Accelerometer,
    kResourceType_ADXL345,
    kResourceType_AnalogChannel,
    kResourceType_AnalogTrigger,
    kResourceType_AnalogTriggerOutput,
    kResourceType_CANJaguar,
    kResourceType_Compressor, // 10
    kResourceType_Counter,
    kResourceType_Dashboard,
    kResourceType_DigitalInput,
    kResourceType_DigitalOutput,
    kResourceType_DriverStationCIO,
    kResourceType_DriverStationEIO,
    kResourceType_DriverStationLCD,
    kResourceType_Encoder,
    kResourceType_GearTooth,
    kResourceType_Gyro, // 20
    kResourceType_I2C,
    kResourceType_Framework,
    kResourceType_Jaguar,
    kResourceType_Joystick,
    kResourceType_Kinect,
    kResourceType_KinectStick,
    kResourceType_PIDController,
    kResourceType_Preferences,
    kResourceType_PWM,
    kResourceType_Relay, // 30
    kResourceType_RobotDrive,
    kResourceType_SerialPort,
    kResourceType_Servo,
    kResourceType_Solenoid,
    kResourceType_SPI,
    kResourceType_Task,
    kResourceType_Ultrasonic,
    kResourceType_Victor,
    kResourceType_Button,
    kResourceType_Command, // 40
    kResourceType_AxisCamera,
    kResourceType_PCVideoServer,
    kResourceType_SmartDashboard,
    kResourceType_Talon,
    kResourceType_HiTechnicColorSensor,
    kResourceType_HiTechnicAccel,
    kResourceType_HiTechnicCompass,
    kResourceType_SRF08,
    kResourceType_AnalogOutput,
    kResourceType_VictorSP, // 50
    kResourceType_PWMTalonSRX,
    kResourceType_CANTalonSRX,
    kResourceType_ADXL362,
    kResourceType_ADXRS450,
    kResourceType_RevSPARK,
    kResourceType_MindsensorsSD540,
    kResourceType_DigitalGlitchFilter,
    kResourceType_ADIS16448,
    kResourceType_PDP,
    kResourceType_PCM, // 60
    kResourceType_PigeonIMU,
    kResourceType_NidecBrushless,
    kResourceType_CANifier,
    kResourceType_TalonFX,
    kResourceType_CTRE_future1,
    kResourceType_CTRE_future2,
    kResourceType_CTRE_future3,
    kResourceType_CTRE_future4,
    kResourceType_CTRE_future5,
    kResourceType_CTRE_future6, // 70
    kResourceType_LinearFilter,
    kResourceType_XboxController,
    kResourceType_UsbCamera,
    kResourceType_NavX,
    kResourceType_Pixy,
    kResourceType_Pixy2,
    kResourceType_ScanseSweep,
    kResourceType_Shuffleboard,
    kResourceType_CAN,
    kResourceType_DigilentDMC60, // 80
    kResourceType_PWMVictorSPX,
    kResourceType_RevSparkMaxPWM,
    kResourceType_RevSparkMaxCAN,
    kResourceType_ADIS16470,
    kResourceType_PIDController2,
    kResourceType_ProfiledPIDController,
    kResourceType_Kinematics,
    kResourceType_Odometry,
    kResourceType_Units,
    kResourceType_TrapezoidProfile, // 90
    kResourceType_DutyCycle,
    kResourceType_AddressableLEDs,
    kResourceType_FusionVenom,
    kResourceType_CTRE_future7,
    kResourceType_CTRE_future8,
    kResourceType_CTRE_future9,
    kResourceType_CTRE_future10,
    kResourceType_CTRE_future11,
    kResourceType_CTRE_future12,
    kResourceType_CTRE_future13, // 100
    kResourceType_CTRE_future14,
}

#[allow(non_upper_case_globals)]
pub mod instances {
    pub static kLanguage_LabVIEW: u32 = 1;
    pub static kLanguage_CPlusPlus: u32 = 2;
    pub static kLanguage_Java: u32 = 3;
    pub static kLanguage_Python: u32 = 4;
    pub static kLanguage_DotNet: u32 = 5;
    pub static kLanguage_Kotlin: u32 = 6;

    pub static kCANPlugin_BlackJagBridge: u32 = 1;
    pub static kCANPlugin_2CAN: u32 = 2;

    pub static kFramework_Iterative: u32 = 1;
    pub static kFramework_Simple: u32 = 2;
    pub static kFramework_CommandControl: u32 = 3;
    pub static kFramework_Timed: u32 = 4;
    pub static kFramework_ROS: u32 = 5;
    pub static kFramework_RobotBuilder: u32 = 6;

    pub static kRobotDrive_ArcadeStandard: u32 = 1;
    pub static kRobotDrive_ArcadeButtonSpin: u32 = 2;
    pub static kRobotDrive_ArcadeRatioCurve: u32 = 3;
    pub static kRobotDrive_Tank: u32 = 4;
    pub static kRobotDrive_MecanumPolar: u32 = 5;
    pub static kRobotDrive_MecanumCartesian: u32 = 6;
    pub static kRobotDrive2_DifferentialArcade: u32 = 7;
    pub static kRobotDrive2_DifferentialTank: u32 = 8;
    pub static kRobotDrive2_DifferentialCurvature: u32 = 9;
    pub static kRobotDrive2_MecanumCartesian: u32 = 10;
    pub static kRobotDrive2_MecanumPolar: u32 = 11;
    pub static kRobotDrive2_KilloughCartesian: u32 = 12;
    pub static kRobotDrive2_KilloughPolar: u32 = 13;

    pub static kDriverStationCIO_Analog: u32 = 1;
    pub static kDriverStationCIO_DigitalIn: u32 = 2;
    pub static kDriverStationCIO_DigitalOut: u32 = 3;

    pub static kDriverStationEIO_Acceleration: u32 = 1;
    pub static kDriverStationEIO_AnalogIn: u32 = 2;
    pub static kDriverStationEIO_AnalogOut: u32 = 3;
    pub static kDriverStationEIO_Button: u32 = 4;
    pub static kDriverStationEIO_LED: u32 = 5;
    pub static kDriverStationEIO_DigitalIn: u32 = 6;
    pub static kDriverStationEIO_DigitalOut: u32 = 7;
    pub static kDriverStationEIO_FixedDigitalOut: u32 = 8;
    pub static kDriverStationEIO_PWM: u32 = 9;
    pub static kDriverStationEIO_Encoder: u32 = 10;
    pub static kDriverStationEIO_TouchSlider: u32 = 11;

    pub static kADXL345_SPI: u32 = 1;
    pub static kADXL345_I2C: u32 = 2;

    pub static kCommand_Scheduler: u32 = 1;
    pub static kCommand2_Scheduler: u32 = 2;

    pub static kSmartDashboard_Instance: u32 = 1;

    pub static kKinematics_DifferentialDrive: u32 = 1;
    pub static kKinematics_MecanumDrive: u32 = 2;
    pub static kKinematics_SwerveDrive: u32 = 3;

    pub static kOdometry_DifferentialDrive: u32 = 1;
    pub static kOdometry_MecanumDrive: u32 = 2;
    pub static kOdometry_SwerveDrive: u32 = 3;
}
