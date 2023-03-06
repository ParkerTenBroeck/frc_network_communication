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

use std::sync::Arc;

use robot_comm::{driverstation::RobotComm, common::{joystick::Joystick, request_code::{RobotRequestCode}, alliance_station::{AllianceStation}}};
use util::robot_discovery::{RobotDiscovery};

#[derive(Clone)]
pub struct Driverstation {
    robot: Arc<RobotComm>,
    console: (),
    fms: ()
}

impl Driverstation {

    pub fn new(robot: impl Into<RobotDiscovery>) -> Self{
        let new = Self { 
            robot: RobotComm::new(None), 
            console: (), 
            fms: () 
        };
        new.robot.start_new_thread();

        new.connect_to(robot);
        new
    }

    pub fn connect_to(&self, robot: impl Into<RobotDiscovery>){
        let discovery = robot.into();
        let c = self.clone();
        std::thread::spawn(move ||{
            //let ip = DiscoveryMethod::connect(discovery);
            c.robot.connect_to(todo!());
        });
    }

    pub fn disconnect(&self){
        self.robot.connect_to(None);
    }

    pub fn reconnect(&self){
        self.robot.reconnect()
    }

    pub fn stop_comm(&self){
        self.robot.kill_comm();
    }

    pub fn start_comm(&self){
        self.robot.start_new_thread();
    }

    pub fn is_robot_connected(&self) -> bool{
        self.robot.is_connected()
    }

    pub fn get_observed_control(&self) -> robot_comm::common::control_code::ControlCode{
        self.robot.get_observed_control()
    }

    pub fn get_observed_status(&self) -> robot_comm::common::roborio_status_code::RobotStatusCode{
        self.robot.get_observed_status()
    }

    pub fn get_observed_voltage(&self) -> net_comm::robot_voltage::RobotVoltage{
        self.robot.get_observed_voltage()
    }

    pub fn set_alliance_station(&self, alliance_station: AllianceStation){
        self.set_alliance_station(alliance_station)
    }

    pub fn set_autonomus(&self){
        self.robot.set_autonomus()
    }

    pub fn set_brownout_protection(&self, brownout_protection: bool){
        self.robot.set_brownout_protection(brownout_protection)
    }

    pub fn set_disabled(&self){
        self.robot.set_disabled()
    }

    pub fn set_estop(&self, estop: bool){
        self.robot.set_estop(estop)
    }

    pub fn set_request_code(&self, request_code: RobotRequestCode){
        self.robot.set_request_code(request_code)
    }

    pub fn set_teleop(&self){
        self.robot.set_teleop()
    }

    pub fn set_test(&self){
        self.robot.set_test()
    }

    pub fn update_joystick(&self, index: usize, joystick: Joystick){
        self.robot.update_joystick(index, joystick)
    }
}
