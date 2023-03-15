// use robot_comm::driverstation::RobotComm;

// pub enum RobotAddr{
//     None,
//     KnownTeamNumber(TeamNumber),
//     KnownIp(IpAddr),
// }

// impl RobotAddr{
//     fn is_none(&self) -> bool{
//         matches!(self, RobotAddr::None)
//     }
// }

// impl From<IpAddr> for RobotAddr{
//     fn from(value: IpAddr) -> Self {
//         Self::KnownIp(value)
//     }
// }

// impl From<TeamNumber> for RobotAddr{
//     fn from(value: TeamNumber) -> Self {
//         Self::KnownTeamNumber(value)
//     }
// }

// pub struct Driverstation{
//     robot_addr: Mutex<RobotAddr>,
//     robot_comm: Arc<RobotComm>,
//     net_console: (),
// }

// impl Driverstation{
//     pub fn new(robot_addr: impl Into<RobotAddr>) -> Self {

//         Arc::new(Self {
//             robot_comm: todo!(),
//             net_console: todo!(),
//             robot_addr,
//         })
//     }
// }

pub type Girls = Gilrs;

use std::sync::{atomic::AtomicBool, Arc, Mutex};

use gilrs::Gilrs;
use robot_comm::{
    common::{
        alliance_station::AllianceStation, joystick::Joystick, request_code::RobotRequestCode,
    },
    driverstation::RobotComm,
};
use util::robot_discovery::RobotDiscovery;

pub struct Driverstation {
    robot_udp: Mutex<RobotCommUdp>,
    robot_tcp: Mutex<RobotCommTcp>,
    fms_udp: Mutex<FmsUdp>,
    fms_tcp: Mutex<FmsTcp>,
}

struct RobotCommUdp {}

struct RobotCommTcp {}

struct FmsUdp {}

struct FmsTcp {}

pub fn get_driverstation() -> &'static Driverstation {
    static DAEMON_STARTED: AtomicBool = AtomicBool::new(false);
    static DRIVERSTATION: Driverstation = Driverstation::new();

    if !DAEMON_STARTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        _ = std::thread::spawn(|| {
            DRIVERSTATION.run_blocking();
        });
    }

    &DRIVERSTATION
}

impl Driverstation {
    pub const fn new() -> Self {
        Self {
            robot_udp: Mutex::new(RobotCommUdp {}),
            robot_tcp: Mutex::new(RobotCommTcp {}),
            fms_udp: Mutex::new(FmsUdp {}),
            fms_tcp: Mutex::new(FmsTcp {}),
        }
    }

    pub fn run_blocking(&self) -> ! {
        loop {
            self.run_blocking_inner()
        }
    }

    pub fn run_blocking_inner(&self) {
        todo!()
    }

    pub fn connect_to(&self, robot: impl Into<RobotDiscovery>) {
        todo!()
    }

    pub fn disconnect(&self) {
        todo!()
    }

    pub fn reconnect(&self) {
        todo!()
    }

    pub fn stop_comm(&self) {
        todo!()
    }

    pub fn is_robot_connected(&self) -> bool {
        todo!()
    }

    pub fn get_observed_control(&self) -> robot_comm::common::control_code::ControlCode {
        todo!()
    }

    pub fn get_observed_status(&self) -> robot_comm::common::roborio_status_code::RobotStatusCode {
        todo!()
    }

    pub fn get_observed_voltage(&self) -> net_comm::robot_voltage::RobotVoltage {
        todo!()
    }

    pub fn set_alliance_station(&self, alliance_station: AllianceStation) {
        todo!()
    }

    pub fn set_autonomus(&self) {
        todo!()
    }

    pub fn set_brownout_protection(&self, brownout_protection: bool) {
        todo!()
    }

    pub fn set_disabled(&self) {
        todo!()
    }

    pub fn set_estop(&self, estop: bool) {
        todo!()
    }

    pub fn set_request_code(&self, request_code: RobotRequestCode) {
        todo!()
    }

    pub fn set_teleop(&self) {
        todo!()
    }

    pub fn set_test(&self) {
        todo!()
    }

    pub fn update_joystick(&self, index: usize, joystick: Joystick) {
        todo!()
    }
}
