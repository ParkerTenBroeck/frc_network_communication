use std::{
    cell::UnsafeCell,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc, Mutex, MutexGuard,
    },
    time::Duration,
};

use util::{buffer_writter::SliceBufferWritter, robot_voltage::RobotVoltage, socket::Socket};

use crate::{
    common::{
        control_code::ControlCode,
        joystick::{Joystick, Joysticks},
        request_code::DriverstationRequestCode,
        roborio_status_code::RobotStatusCode,
        time_data::TimeData,
    },
    driver_to_robot::{DriverstationToRobotCorePacketDate, DriverstationToRobotPacket},
    robot_to_driver::RobotToDriverstationPacket,
};

#[derive(Default)]
pub struct DriverstationComm<const OUT_PORT: u16 = 1110, const IN_PORT: u16 = 1150> {
    last_core_data: Mutex<DriverstationToRobotCorePacketDate>,
    last_joystick_data: Mutex<Joysticks>,
    last_time_data: Mutex<TimeData>,
    observed_control_state: UnsafeCell<ControlCode>,
    observed_status: UnsafeCell<RobotStatusCode>,
    observed_voltage: UnsafeCell<RobotVoltage>,
    request_code: UnsafeCell<DriverstationRequestCode>,
    connected: AtomicBool,
}

unsafe impl<const OUT_PORT: u16, const IN_PORT: u16> Sync for DriverstationComm<OUT_PORT, IN_PORT> {}
unsafe impl<const OUT_PORT: u16, const IN_PORT: u16> Send for DriverstationComm<OUT_PORT, IN_PORT> {}
impl<const OUT_PORT: u16, const IN_PORT: u16> std::panic::UnwindSafe
    for DriverstationComm<OUT_PORT, IN_PORT>
{
}
impl<const OUT_PORT: u16, const IN_PORT: u16> std::panic::RefUnwindSafe
    for DriverstationComm<OUT_PORT, IN_PORT>
{
}

impl<const OUT_PORT: u16, const IN_PORT: u16> DriverstationComm<OUT_PORT, IN_PORT> {
    pub fn start_comm() -> Arc<Self> {
        let arc: Arc<Self> = Arc::new(Self::default());
        arc.clone().spawn_on_thread();
        arc
    }

    fn spawn_on_thread(self: Arc<Self>) {
        std::thread::spawn(move || {
            let res = std::panic::catch_unwind(|| {
                let mut socket = Socket::new_target_unknown(OUT_PORT, IN_PORT);
                //socket.set_input_nonblocking(true);
                let mut buf = [0u8; 4096];
                // let mut sequence = 0u16;

                // we should be getting packets ever 20 ms. if we wait any longer something will likely go wrong
                // we wait a little longer because we actually dont do any timings here
                // we simply reply (or try our best to) every time we get a packet
                socket.set_input_nonblocking(false);
                socket.set_read_timout(Some(std::time::Duration::from_millis(250)));

                let mut sequence: u16 = 0;
                // let mut last_sent = Instant::now();
                // let mut last_received = Instant::now();
                while Arc::strong_count(&self) > 1 {
                    // let start = std::time::Instant::now();
                    // let mut send_immeditly = false;
                    match socket.read::<DriverstationToRobotPacket>(&mut buf) {
                        Ok(Some(packet)) => {
                            use crate::common::request_code::*;

                            let robot_send = RobotToDriverstationPacket {
                                sequence: packet.core_data.sequence,
                                tag_comm_version: 1,
                                control_code: unsafe { *self.observed_control_state.get() },
                                status: unsafe { *self.observed_status.get() },
                                battery: unsafe { *self.observed_voltage.get() },
                                request: unsafe { *self.request_code.get() },
                            };

                            socket
                                .write(&robot_send, &mut SliceBufferWritter::new(&mut buf))
                                .unwrap();

                            // if sequence < packet.core_data.packet.wrapping_sub(1){
                            //     println!("sequence Behind: {sequence}, {}",  packet.core_data.packet);
                            // }else if sequence > packet.core_data.packet{
                            //     println!("sequence Ahead {sequence}, {}",  packet.core_data.packet);
                            //     continue;
                            // }

                            // send_immeditly = true;
                            // sequence = packet.core_data.packet;

                            std::thread::sleep(std::time::Duration::from_millis(1));

                            *self.last_core_data.spin_lock().unwrap() = packet.core_data;

                            if packet.time_data.has_data() {
                                self.last_time_data
                                    .spin_lock()
                                    .unwrap()
                                    .update_existing_from(&packet.time_data);

                                unsafe {
                                    self.request_code
                                        .get()
                                        .as_mut()
                                        .unwrap_unchecked()
                                        .set(DriverstationRequestCode::REQUEST_TIME, false);
                                }
                            }
                            *self.last_joystick_data.spin_lock().unwrap() = packet.joystick_data;

                            if sequence.wrapping_add(1) != packet.core_data.sequence {
                                println!("\u{001B}[31m{}\u{001b}[0m", packet.core_data.sequence);
                            }
                            sequence = packet.core_data.sequence;

                            self.connected
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        Ok(None) => {
                            self.connected
                                .store(false, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(error) => {
                            self.connected
                                .store(false, std::sync::atomic::Ordering::Relaxed);
                            eprintln!(
                                "Error while parsing packets: {error}\nNon fatial Continuing"
                            );
                        }
                    }
                    // println!("{:?}", last_received.elapsed());
                    // let timeout = last_sent.elapsed() >= Duration::from_millis(22)  && last_received.elapsed() < Duration::from_millis(40);
                    // if send_immeditly || (timeout) {

                    // sequence = sequence.wrapping_add(1);

                    // last_sent = Instant::now();
                    // }

                    // if timeout{
                    // println!("Packet Took too long")
                    // }
                    std::thread::sleep(Duration::from_millis(1));
                }
            });
            if let Err(err) = res {
                self.connected
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                std::panic::resume_unwind(err)
            }
        });
    }

    pub fn get_last_core_data(self: &Arc<Self>) -> DriverstationToRobotCorePacketDate {
        *self.last_core_data.lock().unwrap()
    }

    pub fn is_connected(self: &Arc<Self>) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn observe_robot_code(self: &Arc<Self>) {
        unsafe {
            self.observed_status
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set(RobotStatusCode::ROBOT_HAS_CODE, true);
        }
    }

    pub fn set_observe(self: &Arc<Self>, d: RobotStatusCode) {
        unsafe {
            *self.observed_status.get() = d;
        }
    }

    pub fn get_observe(&self) -> RobotStatusCode {
        unsafe { *self.observed_status.get() }
    }

    pub fn get_request(&self) -> DriverstationRequestCode {
        unsafe { *self.request_code.get() }
    }

    pub fn get_control(&self) -> ControlCode {
        unsafe { *self.observed_control_state.get() }
    }

    pub fn observe_robot_voltage(self: &Arc<Self>, robot_voltage: RobotVoltage) {
        unsafe {
            *self.observed_voltage.get().as_mut().unwrap_unchecked() = robot_voltage;
        }
    }

    pub fn request_time(self: &Arc<Self>) {
        unsafe {
            self.request_code
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set(DriverstationRequestCode::REQUEST_TIME, true);
        }
    }

    pub fn set_control(self: &Arc<Self>, control: ControlCode) {
        unsafe {
            *self.observed_control_state.get() = control;
        }
    }

    pub fn set_request(self: &Arc<Self>, request: DriverstationRequestCode) {
        unsafe { *self.request_code.get() = request }
    }

    pub fn observe_robot_enabled(self: &Arc<Self>) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set(ControlCode::ENABLED, true);
        }
    }

    pub fn observe_robot_disabled(self: &Arc<Self>) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set(ControlCode::ENABLED, false);
        }
    }

    pub fn observe_robot_test(self: &Arc<Self>) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set_test();
        }
    }

    pub fn observe_robot_autonomus(self: &Arc<Self>) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set_autonomus();
        }
    }

    pub fn observe_robot_teleop(self: &Arc<Self>) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set_teleop();
        }
    }

    pub fn observe_robot_estop(self: &Arc<Self>, value: bool) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set(ControlCode::ESTOP, value);
        }
    }

    pub fn observe_robot_brown_out_protection(self: &Arc<Self>, value: bool) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set(ControlCode::BROWN_OUT_PROTECTION, value);
        }
    }

    pub fn get_joystick(self: &Arc<Self>, index: usize) -> Option<Joystick> {
        self.last_joystick_data.lock().unwrap().get(index).copied()
    }
}

trait SpinLock<'a, T> {
    fn spin_lock(&'a self) -> T;
}

type SpinLockResult<'a, T> = Result<MutexGuard<'a, T>, std::sync::PoisonError<MutexGuard<'a, T>>>;

impl<'a, T> SpinLock<'a, SpinLockResult<'a, T>> for Mutex<T> {
    fn spin_lock(&'a self) -> SpinLockResult<'a, T> {
        loop {
            match self.try_lock() {
                Ok(ok) => return Ok(ok),
                Err(err) => match err {
                    std::sync::TryLockError::Poisoned(err) => return Err(err),
                    std::sync::TryLockError::WouldBlock => continue,
                },
            }
        }
    }
}
