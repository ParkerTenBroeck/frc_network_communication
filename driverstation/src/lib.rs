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

use std::{
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, AtomicU8},
        Arc, Mutex,
    },
};

use gilrs::Gilrs;
use robot_comm::common::{
    alliance_station::AllianceStation, joystick::Joystick, request_code::RobotRequestCode,
};
use util::robot_discovery::RobotDiscovery;

pub struct Driverstation {
    robot_udp: Mutex<RobotCommUdp>,
    robot_tcp: Mutex<RobotCommTcp>,
    fms_udp: Mutex<FmsUdp>,
    fms_tcp: Mutex<FmsTcp>,
    daemon_status: AtomicU8,
    disconect_daemon: AtomicBool,
}

impl Driverstation {
    fn signal_daemon_thread_stop(&self) {
        // if self.daemon_status.fetch_sub(1, order) == 0{
        //     self.disconect_daemon
        // }
    }
}

struct RobotCommUdp {}

struct RobotCommTcp {}

struct FmsUdp {}

struct FmsTcp {}

pub fn get_driverstation() -> &'static Driverstation {
    static DRIVERSTATION: Driverstation = Driverstation::new();
    &DRIVERSTATION
}

// macro_rules! write_ptr {
//     ($start:ident = $val:expr) => {
//         $start.write($val);
//     };
//     ($start:ident->$next:ident $(->$remaining:ident)* = $val:expr) => {
//         {
//             let tmp = std::ptr::addr_of_mut!((*$start).$next);
//             write_ptr!(tmp$(->$remaining)* = $val);
//         }
//     };
// }

macro_rules! ptr_field {
    ({$start:expr}) => {
        {$start}
    };
    ($start:ident) => {
        {$start}
    };
    ({$start:expr}->$next:ident $(->$remaining:ident)*) => {
        ptr_field!({std::ptr::addr_of_mut!((*$start).$next)}$(->$remaining)*)
    };
    ($start:ident->$next:ident $(->$remaining:ident)*) => {
        ptr_field!({std::ptr::addr_of_mut!((*$start).$next)}$(->$remaining)*)
    };
}

macro_rules! write_ptr {
    ({$start:expr} = $val:expr) => {
        $start.write($val);
    };
    ($start:ident = $val:expr) => {
        $start.write($val);
    };
    ({$start:expr}->$next:ident $(->$remaining:ident)* = $val:expr) => {
        write_ptr!({std::ptr::addr_of_mut!((*$start).$next)}$(->$remaining)* = $val);
    };
    ($start:ident->$next:ident $(->$remaining:ident)* = $val:expr) => {
        write_ptr!({std::ptr::addr_of_mut!((*$start).$next)}$(->$remaining)* = $val);
    };
}

const DRIVERSTATION_DEFAULT_STATE: u8 = 0;

impl Driverstation {
    pub const fn new() -> Self {
        Self {
            robot_udp: Mutex::new(RobotCommUdp {}),
            robot_tcp: Mutex::new(RobotCommTcp {}),
            fms_udp: Mutex::new(FmsUdp {}),
            fms_tcp: Mutex::new(FmsTcp {}),
            daemon_status: AtomicU8::new(0),
            disconect_daemon: AtomicBool::new(false),
        }
    }

    // pub fn start_daemon(&'static self){
    //     if self.daemon_status.swap(true, std::sync::atomic::Ordering::Acquire){
    //         let udp_res = std::thread::Builder::new().name("DRIVERSTATION UDP SERVER".to_owned()).spawn(||{
    //             std::env::set_var("RUST_BACKTRACE", "1");
    //             loop {
    //                 let res = std::panic::catch_unwind(||{
    //                     self.run_udp_server()
    //                 });
    //                 // err is already handled by our panic_hook
    //                 if let Ok(ok) = res {
    //                     eprintln!("Error while running driverstation udp server: {:?}", ok)
    //                 }
    //             }
    //         });

    //         let tcp_res = std::thread::spawn(||{
    //             std::env::set_var("RUST_BACKTRACE", "1");
    //             while {
    //                 let res = std::panic::catch_unwind(||{
    //                     self.run_tcp_server()
    //                 });
    //                 if res.is_err(){

    //                 }
    //             }{}
    //         });

    //     }
    // }

    fn run_udp_server(&self) {}

    fn run_tcp_server(&self) {}

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
