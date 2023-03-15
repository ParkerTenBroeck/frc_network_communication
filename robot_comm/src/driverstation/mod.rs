use std::{
    net::{IpAddr, SocketAddr},
    sync::{atomic::AtomicBool, Arc, Condvar, Mutex},
    time::Instant,
};

use util::{
    buffer_writter::{BufferWritter, SliceBufferWritter, WriteToBuff},
    robot_voltage::RobotVoltage,
    socket::Socket,
};

use crate::{
    common::{
        alliance_station::AllianceStation, control_code::ControlCode, joystick::Joystick,
        request_code::RobotRequestCode, roborio_status_code::RobotStatusCode, time_data::TimeData,
    },
    driver_to_robot::DriverstationToRobotPacket,
    robot_to_driver::RobotToDriverstationPacket,
};

pub struct RobotComm {
    robot_ip: Mutex<Option<IpAddr>>,
    robot_ip_condvar: Condvar,
    running: AtomicBool,
    exit: AtomicBool,
    connected: AtomicBool,
    reconnect: AtomicBool,
    packet_data: Mutex<DriverstationToRobotPacket>,
    other_data: Mutex<OtherData>,
}

#[derive(Debug, Default, Clone, Copy)]
struct OtherData {
    observed_voltage: RobotVoltage,
    observed_state: RobotStatusCode,
    observed_control: ControlCode,
}

impl RobotComm {
    pub fn new(robot_addr: Option<IpAddr>) -> Arc<Self> {
        Arc::new(Self {
            robot_ip: Mutex::new(robot_addr),
            robot_ip_condvar: Condvar::new(),
            running: false.into(),
            exit: false.into(),
            connected: false.into(),
            reconnect: false.into(),
            packet_data: Default::default(),
            other_data: Default::default(),
        })
    }

    pub fn start_new_thread(self: &Arc<Self>) {
        let arc = self.clone();
        std::thread::Builder::new()
            .name("Robot Comm".into())
            .spawn(|| {
                arc.run_blocking();
            })
            .unwrap();
    }

    fn create_socket(&self) -> Option<Socket> {
        let guard = self.robot_ip.lock().unwrap();

        let guard = self
            .robot_ip_condvar
            .wait_while(guard, |ip| {
                ip.is_none() && !self.exit.load(std::sync::atomic::Ordering::Relaxed)
            })
            .unwrap();

        if let Some(ip) = *guard {
            let socket = Socket::new_target_knonw(1150, SocketAddr::new(ip, 1110));
            socket.set_read_timout(Some(std::time::Duration::from_millis(100)));
            socket.set_write_timout(Some(std::time::Duration::from_millis(20)));
            Some(socket)
        } else {
            None
        }
    }

    pub fn run_blocking(self: Arc<Self>) {
        if self
            .running
            .swap(true, std::sync::atomic::Ordering::Acquire)
        {
            return;
        }

        let mut socket = if let Some(some) = self.create_socket() {
            some
        } else {
            return;
        };

        let mut buf = [0u8; 4069];
        let mut request_time = false;

        let mut drift = 0.0;
        let mut worst = std::time::Duration::from_millis(0);

        while !self.exit.load(std::sync::atomic::Ordering::Relaxed) {
            let start = std::time::Instant::now();

            // while we need to reconnect do it,,,
            while self.reconnect.load(std::sync::atomic::Ordering::Relaxed) {
                self.connected
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                drop(socket);
                socket = if let Some(some) = self.create_socket() {
                    some
                } else {
                    return;
                };
                self.reconnect
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }

            let mut cb1_lock = self.packet_data.lock().unwrap();

            // update the time if its requested
            if request_time {
                cb1_lock.time_data = TimeData::from_system();
            } else {
                cb1_lock.time_data = TimeData::default();
            }

            // write our packet to the buffer
            let mut writter = SliceBufferWritter::new(&mut buf);
            let packet_sent_sqeu = cb1_lock.core_data.packet;
            let res = cb1_lock.write_to_buf(&mut writter);

            // copy our sent core data out
            // we should later use this to test if the received robot packet updates with our sent codes
            let _core_copy = cb1_lock.core_data;

            // update the packet count (wrapping)
            cb1_lock.core_data.packet = cb1_lock.core_data.packet.wrapping_add(1);

            //drop our lock we dont need access to the packet anymore
            drop(cb1_lock);

            // actually write our packet buffer to the socket
            let sent = match res {
                Ok(_) => {
                    let buf_to_write = writter.curr_buf();

                    if let Err(err) = socket.write_raw(buf_to_write) {
                        self.reconnect();
                        eprint!("error: {err:#?}");
                        false
                    } else {
                        true
                    }
                }
                Err(err) => {
                    eprint!("error: {err:#?}");
                    false
                }
            };

            if sent {
                let mut packet_behind = true;
                while packet_behind {
                    packet_behind = false;
                    let now = std::time::Instant::now();

                    let start = Instant::now();
                    let res = socket.read::<RobotToDriverstationPacket>(&mut buf);
                    let time = start.elapsed();

                    if let Ok(Some(packet)) = res {
                        request_time = packet.request.request_time();

                        let mut other_lock = self.other_data.lock().unwrap();

                        other_lock.observed_control = packet.control_code;
                        other_lock.observed_voltage = packet.battery;
                        other_lock.observed_state = packet.status;

                        if packet.packet < packet_sent_sqeu && packet.packet != 0 {
                            println!(
                                "\u{001B}[31m{}\u{001b}[0m{}",
                                packet.packet, packet_sent_sqeu
                            );
                            packet_behind = true;
                        }

                        if time > std::time::Duration::from_millis(20) {
                            println!("\u{001B}[31mPacket late: {:?}\u{001b}[0m", time);
                        }

                        self.connected
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    } else if let Ok(None) = res {
                        // packet dropped
                        println!("\u{001B}[31mPacket dropped: {:?}\u{001b}[0m", time);
                        self.connected
                            .store(false, std::sync::atomic::Ordering::Relaxed);
                    } else if let Err(err) = res {
                        self.connected
                            .store(false, std::sync::atomic::Ordering::Relaxed);
                        match err {
                            util::socket::SocketReadError::Io(err) => {
                                match err.kind() {
                                    std::io::ErrorKind::WouldBlock => {}
                                    _ => self.reconnect(),
                                }
                                eprintln!("Error while reading robot packet: {err}");
                            }
                            util::socket::SocketReadError::Buffer(err) => {
                                eprintln!("Error while parsing robot packet: {err}")
                            }
                        }
                    }
                    if worst < now.elapsed() {
                        worst = now.elapsed();
                    }
                }
            }

            let elapsed = start.elapsed();
            if let Some(sleep) = std::time::Duration::from_millis(20).checked_sub(elapsed) {
                if drift < 0.0 {
                    if let Some(sleep) =
                        sleep.checked_sub(std::time::Duration::from_secs_f64(-drift))
                    {
                        std::thread::sleep(sleep);
                    }
                } else if let Some(sleep) =
                    sleep.checked_add(std::time::Duration::from_secs_f64(drift))
                {
                    std::thread::sleep(sleep);
                }
            }
            drift += 0.0200 - start.elapsed().as_secs_f64();
        }

        self.connected
            .store(false, std::sync::atomic::Ordering::Relaxed);
        self.running
            .store(false, std::sync::atomic::Ordering::Release)
    }

    pub fn kill_comm(&self) {
        self.exit.store(true, std::sync::atomic::Ordering::Relaxed);
        self.robot_ip_condvar.notify_all();
    }

    pub fn get_robot_ip(&self) -> Option<IpAddr> {
        *self.robot_ip.lock().unwrap()
    }

    pub fn get_observed_status(&self) -> RobotStatusCode {
        self.other_data.lock().unwrap().observed_state
    }

    pub fn get_observed_control(&self) -> ControlCode {
        self.other_data.lock().unwrap().observed_control
    }

    pub fn get_observed_voltage(&self) -> RobotVoltage {
        self.other_data.lock().unwrap().observed_voltage
    }

    pub fn update_joystick(&self, index: usize, joystick: Joystick) {
        self.packet_data
            .lock()
            .unwrap()
            .joystick_data
            .insert(index, joystick);
    }

    pub fn set_enabled(&self) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_enabled();
    }

    pub fn set_teleop(&self) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_teleop();
    }

    pub fn set_autonomus(&self) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_autonomus();
    }

    pub fn set_test(&self) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_test();
    }

    pub fn set_estop(&self, estop: bool) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_estop(estop);
    }

    pub fn set_brownout_protection(&self, brownout_protection: bool) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_brownout_protection(brownout_protection);
    }

    pub fn set_fms_attached(&self, fms_attached: bool) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_fms_attached(fms_attached);
    }

    pub fn set_ds_attached(&self, ds_attached: bool) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_ds_attached(ds_attached);
    }

    pub fn set_disabled(&self) {
        self.packet_data
            .lock()
            .unwrap()
            .core_data
            .control_code
            .set_disabled();
    }

    pub fn set_request_code(&self, request_code: RobotRequestCode) {
        self.packet_data.lock().unwrap().core_data.request_code = request_code;
    }

    pub fn set_alliance_station(&self, alliance_station: AllianceStation) {
        self.packet_data.lock().unwrap().core_data.station = alliance_station;
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn connect_to(&self, robot_addr: Option<IpAddr>) {
        let mut lock = self.robot_ip.lock().unwrap();
        *lock = robot_addr;
        self.reconnect
            .store(true, std::sync::atomic::Ordering::Release);
        self.robot_ip_condvar.notify_all();
    }

    pub fn reconnect(&self) {
        let mut other_lock = self.other_data.lock().unwrap();

        *other_lock = OtherData::default();

        self.reconnect
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.connected
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}
