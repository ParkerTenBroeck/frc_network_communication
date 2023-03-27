use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicUsize},
};

use net_comm::robot_voltage::RobotVoltage;
use rclite::Arc;
use robot_comm::{
    common::{
        alliance_station::AllianceStation,
        control_code::ControlCode,
        joystick::{Joystick, NonNegU16},
        request_code::RobotRequestCode,
        time_data::TimeData,
    },
    driver_to_robot::{
        reader::{DriverToRobotPacketReader, PacketTagAcceptor},
        DriverstationToRobotCorePacketDate,
    },
    robot_to_driver::{
        self, CpuUsage, PdpPortReport, PdpPowerReport, RobotToDriverCanUsage,
        RobotToDriverDiskUsage, RobotToDriverRamUsage, RobotToDriverRumble,
        RobotToDriverstationPacket,
    },
};
use spin::Mutex;
use util::{
    buffer_reader::BufferReader,
    buffer_writter::{BufferWritter, SliceBufferWritter},
    socket::Socket,
};

#[derive(Default, Debug)]
pub struct RoborioCom {
    udp: RoborioUdp,
    tcp: RoborioTcp,
}

#[derive(Default, Debug)]
struct RoborioTcp {}

#[derive(Default, Debug)]
struct RoborioUdp {
    recv: Mutex<DriverstationToRobotCorePacketDate>,
    joystick_values: Mutex<[Option<Joystick>; 6]>,
    countdown: Mutex<Option<f32>>,
    time: Mutex<TimeData>,
    observed_information: Mutex<RobotToDriverstationPacket>,

    tag_data: Mutex<RoborioUdpTags>,

    reset_con: AtomicBool,
    connected: AtomicBool,
    bytes_sent: AtomicUsize,
    packets_sent: AtomicUsize,
    bytes_received: AtomicUsize,
    packets_received: AtomicUsize,
    /// The number of packets being -received- that have been "dropped"
    /// (if the sequence skips a value)
    packets_dropped: AtomicUsize,
}

#[derive(Default, Debug)]
struct RoborioUdpTags {
    rumble: Option<RobotToDriverRumble>,
    disk_usage: Option<RobotToDriverDiskUsage>,
    cpu_usage: Option<Vec<CpuUsage>>,
    ram_usage: Option<RobotToDriverRamUsage>,
    pdp_port_report: Option<PdpPortReport>,
    pdp_power_report: Option<PdpPowerReport>,
    can_usage: Option<RobotToDriverCanUsage>,
}

struct RoborioCommon {}

impl RoborioCom {
    pub fn start_daemon<T: 'static + Clone + Send + PossibleRcSelf + Deref<Target = Self>>(
        myself: T,
    ) {
        Self::start_udp_daemon(myself.clone());
        Self::start_tcp_daemon(myself)
    }

    fn start_tcp_daemon<T: 'static + Send + PossibleRcSelf + Deref<Target = Self>>(myself: T) {}

    fn start_udp_daemon<T: 'static + Send + PossibleRcSelf + Deref<Target = Self>>(myself: T) {
        use std::sync::atomic::Ordering::Relaxed;
        std::thread::spawn(move || {
            while myself.exists_elsewhere() {
                let socket =
                    UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1110))
                        .expect("Failed to bind to socket? udp port in use?");
                socket
                    .set_read_timeout(Some(std::time::Duration::from_millis(100)))
                    .expect("Failed to set UDP roborio daemon socket timeout");

                // let mut socket = Socket::new_target_unknown(1110, 1150);
                // socket.set_input_nonblocking(false);
                // socket.set_read_timout(Some(std::time::Duration::from_millis(100)));

                myself.udp.packets_dropped.store(0, Relaxed);
                myself.udp.connected.store(false, Relaxed);
                myself.udp.bytes_received.store(0, Relaxed);
                myself.udp.packets_received.store(0, Relaxed);
                myself.udp.packets_sent.store(0, Relaxed);
                myself.udp.bytes_sent.store(0, Relaxed);
                *myself.udp.time.lock() = TimeData::default();
                *myself.udp.joystick_values.lock() = [None; 6];
                *myself.udp.countdown.lock() = None;

                let mut send_addr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
                let mut send_buf = [0u8; 1024];
                let mut recv_buf = [0u8; 1024];
                while myself.exists_elsewhere() {
                    if myself.udp.reset_con.swap(false, Relaxed) {
                        break;
                    }
                    match socket.recv_from(&mut recv_buf) {
                        Ok((read, rec_addr)) => {
                            myself.udp.bytes_received.fetch_add(read, Relaxed);
                            let recv_buf = &recv_buf[..read];
                            if send_addr != rec_addr.ip() && myself.udp.connected.load(Relaxed) {
                                //reconnect if theres a new address to send to
                                break;
                            }
                            send_addr = rec_addr.ip();

                            let mut reader = BufferReader::new(recv_buf);
                            match DriverToRobotPacketReader::new(&mut reader) {
                                Ok((recv_packet, reader)) => {
                                    {
                                        *myself.udp.recv.lock() = recv_packet;
                                    }
                                    {
                                        //scope to make sure our mutexes aren't locked 0w0
                                        let mut packet = myself.udp.observed_information.lock();

                                        // todo in the case of packets dropping
                                        // while this value wraps this value is incremented
                                        // much more than the actual number of packets dropped...
                                        // idk dont do that or something
                                        if packet.sequence != recv_packet.sequence.wrapping_sub(1)
                                            && myself.udp.connected.load(Relaxed)
                                        {
                                            let calc_dif = packet
                                                .sequence
                                                .wrapping_sub(recv_packet.sequence)
                                                .wrapping_sub(1)
                                                as usize;
                                            myself.udp.packets_dropped.fetch_add(calc_dif, Relaxed);
                                        }
                                        if recv_packet.control_code.is_disabled() {
                                            // packet.request.set_request_disable(false);
                                        }
                                        packet.sequence = recv_packet.sequence;
                                        let packet = {
                                            let tmp = *packet;
                                            drop(packet);
                                            tmp
                                        };
                                        let mut writter = SliceBufferWritter::new(&mut send_buf);
                                        let idk = robot_to_driver::writter::RobotToDriverstaionPacketWritter::new(
                                            &mut writter,
                                            packet,
                                        );

                                        let mut idk = if let Ok(val) = idk {
                                            val
                                        } else {
                                            myself.udp.connected.store(false, Relaxed);
                                            continue;
                                        };

                                        {
                                            let lock = myself.udp.tag_data.lock();
                                        }

                                        match socket.send_to(
                                            idk.into_buf(),
                                            SocketAddr::new(send_addr, 1150),
                                        ) {
                                            Ok(wrote) => {
                                                myself.udp.bytes_sent.fetch_add(wrote, Relaxed);
                                                myself.udp.packets_sent.fetch_add(1, Relaxed);
                                                myself.udp.connected.store(true, Relaxed);
                                            }
                                            Err(err) => {
                                                eprintln!(
                                                    "Roborio UDP daemon encountered error: {:?}",
                                                    err
                                                );
                                                myself.udp.connected.store(false, Relaxed);
                                            }
                                        }
                                    }

                                    myself.udp.packets_received.fetch_add(1, Relaxed);

                                    struct Acceptor<'a> {
                                        daemon: &'a RoborioCom,
                                    }
                                    impl<'a> PacketTagAcceptor for Acceptor<'a> {
                                        fn accept_joystick(
                                            &mut self,
                                            index: usize,
                                            joystick: Option<Joystick>,
                                        ) {
                                            if let Some(joy) = self
                                                .daemon
                                                .udp
                                                .joystick_values
                                                .lock()
                                                .get_mut(index)
                                            {
                                                *joy = joystick;
                                            }
                                        }

                                        fn accept_countdown(&mut self, countdown: Option<f32>) {
                                            *self.daemon.udp.countdown.lock() = countdown;
                                        }
                                        #[inline(always)]
                                        fn accept_time_data(&mut self, timedata: TimeData) {
                                            self.daemon
                                                .udp
                                                .time
                                                .lock()
                                                .update_existing_from(&timedata);
                                            self.daemon
                                                .udp
                                                .observed_information
                                                .lock()
                                                .request
                                                .set_request_time(false);
                                        }
                                    }
                                    match reader.read_tags(Acceptor { daemon: &myself }) {
                                        Ok(_) => {}
                                        Err(err) => {
                                            eprintln!(
                                                "Error while reading roborio UDP tags: {:?}",
                                                err
                                            )
                                        }
                                    }
                                }
                                Err(err) => {
                                    myself.udp.connected.store(false, Relaxed);
                                    eprintln!(
                                        "Error while reading roborio UDP daemon packet: {:?}",
                                        err
                                    )
                                }
                            }
                        }
                        Err(err) => {
                            if err.kind() == std::io::ErrorKind::WouldBlock {
                                myself.udp.connected.store(false, Relaxed);
                                continue;
                            } else {
                                // reset connection
                                eprintln!("Error reading UDP socket in roborio daemon: {:?}", err);
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    // pub fn
}

impl RoborioCom{
    pub fn reset_con(&self){
        self.udp.reset_con.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

//udp tings
impl RoborioCom {
    pub fn request_time(&self) {
        self.udp
            .observed_information
            .lock()
            .request
            .set_request_time(true);
    }

    pub fn request_disable(&self) {
        self.udp
            .observed_information
            .lock()
            .request
            .set_request_disable(true);
    }

    pub fn observe_robot_code(&self, has_code: bool) {
        self.udp
            .observed_information
            .lock()
            .status
            .set_has_robot_code(has_code);
    }

    pub fn observe_robot_brownout(&self, brownout_protection: bool) {
        self.udp
            .observed_information
            .lock()
            .control_code
            .set_brownout_protection(brownout_protection);
    }

    pub fn observe_robot_voltage(&self, voltage: RobotVoltage) {
        self.udp.observed_information.lock().battery = voltage;
    }

    pub fn observe_robot_teleop(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.control_code
            .set_teleop()
            .set_enabled()
            .set_estop(false);
        lock.status.set_teleop();
    }

    pub fn observe_robot_autonomus(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.control_code
            .set_autonomus()
            .set_enabled()
            .set_estop(false);
        lock.status.observe_robot_autonomus();
    }

    pub fn observe_robot_test(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.control_code.set_test().set_enabled().set_estop(false);
        lock.status.set_test();
    }

    pub fn observe_robot_estop(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.status.set_disabled();
        lock.control_code.set_disabled().set_estop(true);
    }

    pub fn observe_robot_disabled(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.control_code.set_disabled().set_estop(false);
        lock.status.set_disabled();
    }

    pub fn get_control_code(&self) -> ControlCode {
        self.udp.recv.lock().control_code
    }

    pub fn get_alliance_station(&self) -> AllianceStation {
        self.udp.recv.lock().station
    }

    pub fn get_request_code(&self) -> RobotRequestCode {
        self.udp.recv.lock().request_code
    }

    pub fn get_countdown(&self) -> Option<f32> {
        *self.udp.countdown.lock()
    }

    pub fn get_time(&self) -> TimeData {
        *self.udp.time.lock()
    }


    pub fn get_request_time(&self) -> bool{
        self.udp.observed_information.lock().request.request_time()
    }

    pub fn get_request_disable(&self) -> bool{
        self.udp.observed_information.lock().request.request_disabled()
    }

    pub fn get_udp_packets_dropped(&self) -> usize {
        self.udp
            .packets_dropped
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn get_udp_packets_sent(&self) -> usize {
        self.udp
            .packets_sent
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn get_udp_packets_received(&self) -> usize {
        self.udp
            .packets_received
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn get_udp_bytes_sent(&self) -> usize {
        self.udp
            .bytes_sent
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn get_udp_bytes_received(&self) -> usize {
        self.udp
            .bytes_received
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn is_connected(&self) -> bool {
        self.udp
            .connected
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn get_axis(&self, controller: usize, axis: u8) -> Option<i8> {
        let lock = self.udp.joystick_values.lock();
        if let Some(Some(joy)) = lock.get(controller) {
            joy.get_axis(axis)
        } else {
            None
        }
    }

    pub fn get_pov(&self, controller: usize, pov: u8) -> Option<NonNegU16> {
        let lock = self.udp.joystick_values.lock();
        if let Some(Some(joy)) = lock.get(controller) {
            joy.get_pov(pov)
        } else {
            None
        }
    }

    pub fn get_button(&self, controller: usize, button: u8) -> Option<bool> {
        let lock = self.udp.joystick_values.lock();
        if let Some(Some(joy)) = lock.get(controller) {
            joy.get_button(button)
        } else {
            None
        }
    }

    pub fn get_joystick(&self, controller: usize) -> Option<Joystick> {
        *self
            .udp
            .joystick_values
            .lock()
            .get(controller)
            .unwrap_or(&None)
    }
}

pub trait PossibleRcSelf {
    fn exists_elsewhere(&self) -> bool;
}

impl<T> PossibleRcSelf for &T {
    fn exists_elsewhere(&self) -> bool {
        true
    }
}

impl<T> PossibleRcSelf for Arc<T> {
    fn exists_elsewhere(&self) -> bool {
        self.strong_count() > 1
    }
}

impl<T> PossibleRcSelf for std::sync::Arc<T> {
    fn exists_elsewhere(&self) -> bool {
        std::sync::Arc::strong_count(self) > 1
    }
}
