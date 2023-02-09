use std::{sync::Arc, net::{SocketAddr, ToSocketAddrs, SocketAddrV4, Ipv4Addr}};

use driverstation_comm::DriverstationComm;
use robot_common::{common::{robot_voltage::RobotVoltage, control_code::ControlCode, request_code::RobotRequestCode, alliance_station::AllianceStation, time_data::TimeData, joystick::Joysticks}, util::{socket::Socket, buffer_writter::BufferWritter, buffer_reader::BufferReader}, driver_to_robot::{DriverstationToRobotPacket, DriverstationToRobotCorePacketDate}, robot_to_driver::RobotToDriverstationPacket};

    // let mut idk_socket = Socket::new(1130, 1140);
    // idk_socket.set_input_nonblocking(true);

    // std::thread::spawn(|| {
    //     let socket = TcpListener::bind("0.0.0.0:1735").unwrap();
    //     let mut buf = Vec::new();

    //     while let Ok((mut socket, _addr)) = socket.accept() {
    //         buf.clear();
    //         let read = socket.read_to_end(&mut buf).unwrap();
    //         println!("{:?}", &buf[..read]);
    //     }
    // });

fn main() {
    // simulate_driverstation();

    let driverstation: Arc<DriverstationComm> = DriverstationComm::start_comm();

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
            let int = (((127.0 - joystick.get_axises().get_axis(1) as f32) / 255.0) * 30.0) as u8;
            let dec = (127 - joystick.get_axises().get_axis(5) as i32) as u8;

            driverstation.observe_robot_voltage(RobotVoltage { int, dec })
        }

        driverstation.request_time();

        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // let mut netconsole_socket = Socket::new("0.0.0.0:6668", "0.0.0.0:6666");
    // netconsole_socket.set_input_nonblocking(true);

    // let mut buf = [0u8; 4096];
    // loop {
    // if let Some(fms_packet) = fms_socket.read::<&[u8]>(&mut buf).unwrap() {
    //     println!("fms: {fms_packet:#?}");
    // }

    // if let Some(idk_packet) = idk_socket.read::<&[u8]>(&mut buf).unwrap() {
    //     println!("fms: {idk_packet:#?}");
    // }

    //     if let Some(mut robot_packet) = robot_socket
    //         .read::<DriverstationToRobotPacket>(&mut buf)
    //         .unwrap()
    //     {
    //         //print!("robot: {robot_packet:#?}");
    //         if robot_packet.joystick_data.is_some(){
    //             let joystick = &robot_packet.joystick_data.unwrap();
    //             let a1 = (((127.0 - joystick.get_axises().get_axis(1) as f32) / 255.0) * 30.0) as u8;
    //             let a2 = (127 - joystick.get_axises().get_axis(5) as i32) as u8;

    //             let reset = robot_packet.core_data.request_code.get(RobotRequestCode::RESTART_ROBORIO) | robot_packet.core_data.request_code.get(RobotRequestCode::RESTART_ROBORIO_CODE);

    //             use robot_common::common::roborio_status_code::*;
    //             use robot_common::common::robot_voltage::*;
    //             use robot_common::common::request_code::*;
    //             let robot_send = RobotToDriverstationPacket {
    //                 packet: robot_packet.core_data.packet,
    //                 tag_comm_version: 1,
    //                 control_code: *robot_packet.core_data.control_code.set(ControlCode::_RESERVED, 0b0),
    //                 status: *RobotStatusCode::new()
    //                     .set(RobotStatusCode::ROBOT_HAS_CODE, true),
    //                 battery: RobotVoltage { int: a1, dec: a2 },
    //                 request: DriverstationRequestCode::from_bits(0x1),
    //                 // extended: robot::send::RobotOutExtended::None,
    //             };

    //             robot_socket
    //             .write(&robot_send, &mut BufferWritter::new(&mut buf))
    //             .unwrap();
    //         }
    //     }

    //     // if let Some(netconsole_packet) = netconsole_socket.read::<&[u8]>(&mut buf).unwrap() {
    //     //     println!("netconsole: {netconsole_packet:#?}");
    //     // }

    //     std::thread::sleep(std::time::Duration::from_millis(1));
    // }
}


pub fn simulate_driverstation(){
       // let mut fms_socket = Socket::new(1160, 1120);
    // fms_socket.set_input_nonblocking(true);

    let mut robot_socket = Socket::new_target_knonw(1150, SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 11, 14, 21), 1110)));
    robot_socket.set_input_timout(Some(std::time::Duration::from_millis(20)));
    // robot_socket.set_input_nonblocking(true);

    let mut idk_socket = Socket::new_target_knonw(1130, SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 11, 14, 21), 1140)));
    //robot_socket.set_input_timout(Some(std::time::Duration::from_millis(20)));
    

    let mut packet_count = 0;
    // let mut packet_loss = 0;
    let mut buf = [0u8; 4096];
    let mut request_time = false;
    loop{
        let ds_to_rb = DriverstationToRobotPacket{
            core_data: DriverstationToRobotCorePacketDate{
                packet: packet_count,
                tag_comm_version: 1,
                control_code: ControlCode::new(),
                request_code: *RobotRequestCode::new().set_normal(true),
                station: AllianceStation::Red1,
            },
            time_data: if request_time {TimeData::from_system()} else {TimeData::default()},
            joystick_data: Joysticks::default(),
        };
        let res = robot_socket.write(&ds_to_rb, &mut BufferWritter::new(&mut buf));
        if let Err(err) = res{
            eprint!("error: {err:#?}");
        }else{
            //println!("wrote: {}", res.unwrap());
        }
        packet_count = packet_count.wrapping_add(1);

        for _ in 0..1{
            let res = robot_socket.read::<RobotToDriverstationPacket>(&mut buf);
            if let Ok(Some(packet)) = res{
                //println!("asdljasd: {packet:#?}");
                request_time = packet.request.request_time();
                // if packet_count == packet.packet{
                // }
            }else if let Err(err) = res{
                eprintln!("bruh: {err}")
            }
        }
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
