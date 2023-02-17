use std::{
    sync::Arc,
};

use net_comm::{
    driverstation::{console_message::SystemConsoleOutput, message_handler::MessageConsole},
    find_robot_ip,
    robot_voltage::RobotVoltage,
};
use robot_comm::{robot::DriverstationComm, driverstation::Driverstation};


// let mut idk_socket = Socket::new(1130, 1140);
// idk_socket.set_input_nonblocking(true);
// let mut fms_socket = Socket::new(1160, 1120);
// fms_socket.set_input_nonblocking(true);
// let mut netconsole_socket = Socket::new("0.0.0.0:6668", "0.0.0.0:6666");
// netconsole_socket.set_input_nonblocking(true);


pub fn simulate_roborio() {
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
}




use eframe::egui;

fn main() -> Result<(), eframe::Error> {

    let ipaddr = find_robot_ip(1114).expect("Failed to find roborio");
    println!("FOUND ROBORIO: {:?}", ipaddr);

    MessageConsole::create_new_thread(SystemConsoleOutput {}, ipaddr);
    let driverstation = Driverstation::new(ipaddr);
    driverstation.start_new_thread();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Driver Station",
        options,
        Box::new(|_cc| Box::new(MyApp{
            driverstation,
        })),
    )
}

struct MyApp {
    driverstation: Arc<Driverstation>
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            
            
            let status = self.driverstation.get_observed_status();
            if status.has_robot_code(){
                ui.label("Has Robot Code");
            }else{
                ui.label("No Robot Code");
            }

            let control = self.driverstation.get_observed_control();

            if control.is_brown_out_protection(){
                ui.label("BROWN OUT PROTECTION");
            }

            if control.is_estop(){
                ui.label("ESTOP");
            }

            if control.is_driverstation_attached(){
                ui.label("NO IDEA");
            }

            ui.horizontal(|ui|{

                ui.vertical(|ui|{

                    if ui.toggle_value(&mut control.is_teleop(), "Teleop").clicked(){
                        self.driverstation.set_disabled();
                        self.driverstation.set_teleop();
                    }
                    if ui.toggle_value(&mut control.is_autonomus(), "Auton").clicked(){
                        self.driverstation.set_disabled();
                        self.driverstation.set_autonomus();
                    }
                    if ui.toggle_value(&mut false, "Practis").clicked(){
                        self.driverstation.set_disabled();
                        //TODO: add practis mode support
                    }
                    if ui.toggle_value(&mut control.is_test(), "Test").clicked(){
                        self.driverstation.set_disabled();
                        self.driverstation.set_test()
                    }
                });

                ui.vertical(|ui|{

                    ui.label(format!("{:.2}", self.driverstation.get_observed_voltage()));

                    ui.horizontal(|ui|{
                        let en_res = ui.toggle_value(&mut control.is_enabled(), "Enable");
        
                        let dis_res = ui.toggle_value(&mut !control.is_enabled(), "Dissable");
        
                        if en_res.clicked(){
                            self.driverstation.set_enabled();
                        }
                        if dis_res.clicked(){
                            self.driverstation.set_disabled();
                        }
                    });
                });
            });


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
