use std::{
    cell::UnsafeCell,
    sync::{atomic::AtomicU32, Arc, Mutex, MutexGuard},
};

use util::{buffer_writter::BufferWritter, robot_voltage::RobotVoltage, socket::Socket};

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
    connected: AtomicU32,
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

                while Arc::strong_count(&self) > 1 {
                    match socket.read::<DriverstationToRobotPacket>(&mut buf) {
                        Ok(Some(packet)) => {
                            use crate::common::request_code::*;
                            let robot_send = RobotToDriverstationPacket {
                                packet: packet.core_data.packet,
                                tag_comm_version: 1,
                                control_code: unsafe { *self.observed_control_state.get() },
                                status: unsafe { *self.observed_status.get() },
                                battery: unsafe { *self.observed_voltage.get() },
                                request: unsafe { *self.request_code.get() },
                            };

                            socket
                                .write(&robot_send, &mut BufferWritter::new(&mut buf))
                                .unwrap();

                            *self.last_core_data.spin_lock().unwrap() = packet.core_data;

                            if packet.time_data.has_data() {
                                self.last_time_data
                                    .spin_lock()
                                    .unwrap()
                                    .copy_existing_from(&packet.time_data);

                                unsafe {
                                    self.request_code
                                        .get()
                                        .as_mut()
                                        .unwrap_unchecked()
                                        .set(DriverstationRequestCode::REQUEST_TIME, false);
                                }
                            }
                            *self.last_joystick_data.spin_lock().unwrap() = packet.joystick_data;


                            println!("{:#?}", packet.core_data);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            eprintln!(
                                "Error while parsing packets: {error}\nNon fatial Continuing"
                            );
                        }
                    }
                }
            });
            if let Err(err) = res {
                self.connected
                    .store(0, std::sync::atomic::Ordering::Relaxed);
                std::panic::resume_unwind(err)
            }
        });
    }

    pub fn get_last_core_data(self: &Arc<Self>) -> DriverstationToRobotCorePacketDate {
        *self.last_core_data.lock().unwrap()
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
                .set(ControlCode::TEST, true)
                .set(ControlCode::AUTONOMUS, false);
        }
    }

    pub fn observe_robot_autonomus(self: &Arc<Self>) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set(ControlCode::TEST, false)
                .set(ControlCode::AUTONOMUS, true);
        }
    }

    pub fn observe_robot_teleop(self: &Arc<Self>) {
        unsafe {
            self.observed_control_state
                .get()
                .as_mut()
                .unwrap_unchecked()
                .set(ControlCode::TEST, false)
                .set(ControlCode::AUTONOMUS, false);
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
        self.last_joystick_data
            .lock()
            .unwrap()
            .get_joystick(index)
            .cloned()
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
