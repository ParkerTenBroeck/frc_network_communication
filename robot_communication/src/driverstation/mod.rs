use std::{
    net::{IpAddr, SocketAddr},
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use util::{buffer_writter::{BufferWritter, WriteToBuff}, socket::Socket, robot_voltage::RobotVoltage};

use crate::{
    driver_to_robot::{DriverstationToRobotPacket},
    robot_to_driver::RobotToDriverstationPacket, common::{time_data::TimeData, roborio_status_code::RobotStatusCode, control_code::ControlCode, joystick::Joystick, request_code::RobotRequestCode, alliance_station::AllianceStation},
};

pub struct Driverstation {
    robot_ip: IpAddr,
    exit: AtomicBool,
    packet_data: Mutex<DriverstationToRobotPacket>,
    other_data: Mutex<OtherData>
}

#[derive(Debug, Default, Clone, Copy)]
struct OtherData{
    observed_voltage: RobotVoltage,
    observed_state: RobotStatusCode,
    observed_control: ControlCode,
}



impl Driverstation {
    pub fn new(robot_ip: IpAddr) -> Arc<Self> {
        Arc::new(Self {
            robot_ip,
            exit: false.into(),
            packet_data: Default::default(),
            other_data: Default::default(),
        })
    }

    pub fn start_new_thread(self: &Arc<Self>){
        let arc = self.clone();
        std::thread::spawn(||{
            arc.run_blocking();
        });
    }

    pub fn run_blocking(self: Arc<Self>) {
        let mut socket = Socket::new_target_knonw(1150, SocketAddr::new(self.robot_ip, 1110));
        socket.set_input_timout(Some(std::time::Duration::from_millis(10)));

        let mut buf = [0u8; 4069];
        let mut request_time = false;

        while !self.exit.load(std::sync::atomic::Ordering::Relaxed) {

            let start = std::time::Instant::now();

            let mut cb1_lock = self.packet_data.lock().unwrap();

            // update the time if its requested
            if request_time{
                cb1_lock.time_data = TimeData::from_system();
            }else{
                cb1_lock.time_data = TimeData::default();
            }

            // write our packet to the buffer
            let mut writter = BufferWritter::new(&mut buf);
            let res = cb1_lock.write_to_buff(&mut writter);
            
            // update the packet count (wrapping)
            cb1_lock.core_data.packet = cb1_lock.core_data.packet.wrapping_add(1);

            // copy our sent core data out
            // we should later use this to test if the received robot packet updates with our sent codes
            let _core_copy = cb1_lock.core_data;
            
            //drop our lock we dont need access to the packet anymore
            drop(cb1_lock);

            // actually write our packet buffer to the socket
            let sent = match res{
                Ok(_) => {
                    let buf_to_write = writter.get_curr_buff();
                    if let Err(err) = socket.write_raw(buf_to_write){
                        eprint!("error: {err:#?}");
                        false
                    }else{
                        true
                    }
                },
                Err(err) => {
                    eprint!("error: {err:#?}");
                    false
                },
            };

            if sent{
                let res = socket.read::<RobotToDriverstationPacket>(&mut buf);
                if let Ok(Some(packet)) = res {
                    request_time = packet.request.request_time();

                    let mut other_lock = self.other_data.lock().unwrap();

                    other_lock.observed_control = packet.control_code;
                    other_lock.observed_voltage = packet.battery;
                    other_lock.observed_state = packet.status;
                    
                } else if let Err(err) = res {
                    eprintln!("Error while reading robot packet: {err}")
                }
            }


            let elapsed = start.elapsed();
            if let Some(sleep) = std::time::Duration::from_millis(50).checked_sub(elapsed) {
                std::thread::sleep(sleep);
            }
        }
    }

    pub fn kill_comm(&self){
        self.exit.store(true, std::sync::atomic::Ordering::Release)
    }

    pub fn get_observed_status(&self) -> RobotStatusCode{
        self.other_data.lock().unwrap().observed_state
    }

    pub fn get_observed_control(&self) -> ControlCode{
        self.other_data.lock().unwrap().observed_control
    }

    pub fn get_observed_voltage(&self) -> RobotVoltage{
        self.other_data.lock().unwrap().observed_voltage
    }

    pub fn update_joystick(&self, index: usize, joystick: Joystick){
        self.packet_data.lock().unwrap().joystick_data.insert_joystick(index, joystick);
    }

    pub fn set_enabled(&self){
        self.packet_data.lock().unwrap().core_data.control_code.set_enabled();
    }

    pub fn set_teleop(&self){
        self.packet_data.lock().unwrap().core_data.control_code.set_teleop();
    }
    
    pub fn set_autonomus(&self){
        self.packet_data.lock().unwrap().core_data.control_code.set_autonomus();
    }

    pub fn set_test(&self){
        self.packet_data.lock().unwrap().core_data.control_code.set_test();
    }

    pub fn set_estop(&self, estop: bool){
        self.packet_data.lock().unwrap().core_data.control_code.set_estop(estop);
    }

    pub fn set_brownout_protection(&self, brownout_protection: bool){
        self.packet_data.lock().unwrap().core_data.control_code.set_brownout_protection(brownout_protection);
    }

    pub fn set_fms_attached(&self, fms_attached: bool){
        self.packet_data.lock().unwrap().core_data.control_code.set_fms_attached(fms_attached);
    }

    pub fn set_ds_attached(&self, ds_attached: bool){
        self.packet_data.lock().unwrap().core_data.control_code.set_ds_attached(ds_attached);
    }

    pub fn set_disabled(&self){
        self.packet_data.lock().unwrap().core_data.control_code.set_disabled();
    }

    pub fn set_request_code(&self, request_code: RobotRequestCode){
        self.packet_data.lock().unwrap().core_data.request_code = request_code;
    }

    pub fn set_alliance_station(&self, alliance_station: AllianceStation){
        self.packet_data.lock().unwrap().core_data.station = alliance_station;
    }
}
