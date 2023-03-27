use std::{error::Error, fmt::Display};

use super::util::{BufferReader, BufferReaderError, ReadFromBuff};

#[derive(Debug)]
pub struct FMSPacket {
    pub packet_count: u16,
    pub ds_version: u8,
    pub fms_control: u8,
    pub team_number: TeamNumber,
    pub robot_voltage: RobotVoltage,
}

#[derive(Debug)]
pub enum FMSPacketParseError {
    InvalidDataLength,
    InvalidData,
}

impl Display for FMSPacketParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<BufferReaderError> for FMSPacketParseError {
    fn from(_: BufferReaderError) -> Self {
        Self::InvalidDataLength
    }
}

impl Error for FMSPacketParseError {}

impl<'a> ReadFromBuff<'a> for FMSPacket {
    type Error = FMSPacketParseError;

    fn read_from_buff(buf: &mut BufferReader<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            packet_count: buf.read_u16()?,
            ds_version: buf.read_u8()?,
            fms_control: buf.read_u8()?,
            team_number: TeamNumber(buf.read_u16()?),
            robot_voltage: RobotVoltage {
                int: buf.read_u8()?,
                dec: buf.read_u8()?,
            },
        })
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
