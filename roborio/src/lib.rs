use std::{
    default,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket},
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize},
};

use net_comm::robot_voltage::RobotVoltage;
use rclite::Arc;
use robot_comm::{
    common::{
        alliance_station::AllianceStation,
        control_code::ControlCode,
        joystick::{Joystick, NonNegU16},
        request_code::{DriverstationRequestCode, RobotRequestCode},
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
use util::{buffer_reader::BufferReader, buffer_writter::SliceBufferWritter};

#[derive(Default, Debug)]
pub struct RoborioCom {
    udp: RoborioUdp,
    tcp: RoborioTcp,
    common: RoborioCommon,
}

#[derive(Default, Debug)]
struct RoborioTcp {
    reset_con: AtomicBool,
}

#[derive(Debug)]
struct RoborioUdp {
    recv: Mutex<DriverstationToRobotCorePacketDate>,
    joystick_values: Mutex<[Option<Joystick>; 6]>,
    countdown: Mutex<Option<f32>>,
    time: Mutex<TimeData>,
    observed_information: Mutex<RobotToDriverstationPacket>,

    tag_data: Mutex<RoborioUdpTags>,
    tag_frequences: RoborioUdpTagFrequencies,

    reset_con: AtomicBool,
    connected: AtomicBool,
    bytes_sent: AtomicUsize,
    packets_sent: AtomicUsize,
    bytes_received: AtomicUsize,
    packets_received: AtomicUsize,
    /// The number of packets being -received- that have been "dropped"
    /// (if the sequence skips a value)
    packets_dropped: AtomicUsize,

    connection_timeout_ms: AtomicU32,
}

impl Default for RoborioUdp {
    fn default() -> Self {
        Self {
            recv: Default::default(),
            joystick_values: Default::default(),
            countdown: Default::default(),
            time: Default::default(),
            observed_information: Default::default(),
            tag_data: Default::default(),
            tag_frequences: Default::default(),
            reset_con: Default::default(),
            connected: Default::default(),
            bytes_sent: Default::default(),
            packets_sent: Default::default(),
            bytes_received: Default::default(),
            packets_received: Default::default(),
            packets_dropped: Default::default(),
            //mid
            connection_timeout_ms: AtomicU32::new(10000),
        }
    }
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

#[derive(Debug)]
struct RoborioUdpTagFrequencies {
    disk_usage_m: AtomicU8,
    cpu_usage_m: AtomicU8,
    ram_usage_m: AtomicU8,
    pdp_port_report_m: AtomicU8,
    pdp_power_report_m: AtomicU8,
    can_usage_m: AtomicU8,
}

impl Default for RoborioUdpTagFrequencies {
    fn default() -> Self {
        Self {
            disk_usage_m: AtomicU8::new(51),
            cpu_usage_m: AtomicU8::new(51),
            ram_usage_m: AtomicU8::new(51),
            pdp_port_report_m: AtomicU8::new(3),
            pdp_power_report_m: AtomicU8::new(3),
            can_usage_m: AtomicU8::new(3),
        }
    }
}

#[derive(Default, Debug)]
struct RoborioCommon {
    request_info: AtomicBool,
}

impl RoborioCom {
    pub fn start_daemon<
        T: 'static + Clone + Sync + Send + PossibleRcSelf + Deref<Target = Self>,
    >(
        myself: T,
    ) {
        // ya so this is weird i promis it makes sense
        // the PossiblyRcSelf will keep the threads alive if we clone it (possibly)
        // so we pass them as a reference instead
        std::thread::spawn(move || {
            let myself = &myself;
            std::thread::scope(move |scope| {
                scope.spawn(|| {
                    Self::run_udp_daemon(myself);
                });
                Self::run_tcp_daemon(myself)
            });
        });
    }

    fn run_tcp_daemon<T: 'static + Send + PossibleRcSelf + Deref<Target = Self>>(myself: &T) {
        use std::sync::atomic::Ordering::Relaxed;
        if true {
            return;
        }
        while myself.exists_elsewhere() {
            let buf = [0u8; 2096];
            let listener = TcpListener::bind("0.0.0.0:1740");
            let listener = match listener {
                Ok(listener) => listener,
                Err(err) => {
                    println!("Failed to start roborio TCP daemon: {err:?}");
                    continue;
                }
            };
            // for stream in listener.incoming() {
            //     let stream = match stream{
            //         Ok(stream) => stream,
            //         Err(_) => todo!(),
            //     }
            // }
        }
    }

    fn run_udp_daemon<T: 'static + Send + PossibleRcSelf + Deref<Target = Self>>(myself: &T) {
        use std::sync::atomic::Ordering::Relaxed;
        while myself.exists_elsewhere() {
            myself.udp.packets_dropped.store(0, Relaxed);
            myself.udp.connected.store(false, Relaxed);
            myself.udp.bytes_received.store(0, Relaxed);
            myself.udp.packets_received.store(0, Relaxed);
            myself.udp.packets_sent.store(0, Relaxed);
            myself.udp.bytes_sent.store(0, Relaxed);
            *myself.udp.time.lock() = TimeData::default();
            *myself.udp.joystick_values.lock() = [None; 6];
            *myself.udp.observed_information.lock() = RobotToDriverstationPacket {
                tag_comm_version: 1,
                ..Default::default()
            };
            *myself.udp.countdown.lock() = None;

            let socket =
                UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1110))
                    .expect("Failed to bind to socket? udp port in use?");
            socket
                .set_read_timeout(Some(std::time::Duration::from_millis(50)))
                .expect("Failed to set UDP roborio daemon socket timeout");

            // let mut socket = Socket::new_target_unknown(1110, 1150);
            // socket.set_input_nonblocking(false);
            // socket.set_read_timout(Some(std::time::Duration::from_millis(100)));

            // we should treat a new connection as a sucsess
            let mut last_sucsess = std::time::Instant::now();

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
                            // break;
                            if last_sucsess.elapsed()
                                > std::time::Duration::from_millis(
                                    myself.udp.connection_timeout_ms.load(Relaxed) as u64,
                                )
                            {
                                break;
                            } else {
                                continue;
                            }
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
                                    let lower = packet.sequence.wrapping_add(1);
                                    let higher = recv_packet.sequence;
                                    if lower != higher && myself.udp.connected.load(Relaxed) {
                                        let calc_dif = match lower.cmp(&higher) {
                                            std::cmp::Ordering::Less => higher.wrapping_sub(lower),
                                            std::cmp::Ordering::Equal => 0,
                                            std::cmp::Ordering::Greater => {
                                                lower.wrapping_sub(higher)
                                            }
                                        };
                                        myself
                                            .udp
                                            .packets_dropped
                                            .fetch_add(calc_dif as usize, Relaxed);
                                    }

                                    packet.sequence = recv_packet.sequence;

                                    let mut writter = SliceBufferWritter::new(&mut send_buf);
                                    let idk = robot_to_driver::writter::RobotToDriverstaionPacketWritter::new(
                                            &mut writter,
                                            *packet,
                                        );

                                    if recv_packet.control_code.is_disabled() {
                                        packet.request.set_request_disable(false);
                                    }
                                    drop(packet);

                                    let mut packet_writter = if let Ok(val) = idk {
                                        val
                                    } else {
                                        //failed to write 6 bytes?? ya idk something seems fishy to me but whatever
                                        myself.udp.connected.store(false, Relaxed);
                                        continue;
                                    };

                                    'tags: {
                                        let lock = myself.udp.tag_data.lock();
                                        let packets_sent = myself.udp.packets_sent.load(Relaxed);

                                        // TODO: report errros better
                                        macro_rules! tag_thing {
                                            ($tag:ident) => {
                                                if let Some($tag) = lock.$tag{
                                                    if packet_writter.$tag($tag).is_err(){
                                                        break 'tags;
                                                    }
                                                }
                                            };
                                            ($tag:ident, $tag_m:ident $(, $offset:expr)?) => {
                                                if let Some($tag) = lock.$tag{
                                                    let $tag_m = myself.udp.tag_frequences.$tag_m.load(Relaxed);
                                                    if $tag_m != 0{
                                                        if ((packets_sent $(+ $offset)?) as usize) % ($tag_m as usize) == 0{
                                                            if packet_writter.$tag($tag).is_err(){
                                                                break 'tags;
                                                            }
                                                        }
                                                    }
                                                }
                                            };
                                            (&$tag:ident, $tag_m:ident $(, $offset:expr)?) => {
                                                if let Some($tag) = &lock.$tag{
                                                    let $tag_m = myself.udp.tag_frequences.$tag_m.load(Relaxed);
                                                    if $tag_m != 0{
                                                        if ((packets_sent $(+ $offset)?) as usize) % ($tag_m as usize) == 0{
                                                            if packet_writter.$tag($tag).is_err(){
                                                                break 'tags;
                                                            }
                                                        }
                                                    }
                                                }
                                            };
                                        }

                                        tag_thing!(rumble);
                                        tag_thing!(disk_usage, disk_usage_m, 1);
                                        tag_thing!(&cpu_usage, cpu_usage_m, 2);
                                        tag_thing!(ram_usage, ram_usage_m, 3);
                                        tag_thing!(&pdp_port_report, pdp_port_report_m, 4);
                                        tag_thing!(pdp_power_report, pdp_power_report_m, 5);
                                        tag_thing!(can_usage, can_usage_m, 6);
                                    }
                                    match socket.send_to(
                                        packet_writter.into_buf(),
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
                                    #[inline(always)]
                                    fn accept_joystick(
                                        &mut self,
                                        index: usize,
                                        joystick: Option<Joystick>,
                                    ) {
                                        if let Some(joy) =
                                            self.daemon.udp.joystick_values.lock().get_mut(index)
                                        {
                                            *joy = joystick;
                                        }
                                    }

                                    #[inline(always)]
                                    fn accept_countdown(&mut self, countdown: Option<f32>) {
                                        *self.daemon.udp.countdown.lock() = countdown;
                                    }
                                    #[inline(always)]
                                    fn accept_time_data(&mut self, timedata: TimeData) {
                                        self.daemon.udp.time.lock().update_existing_from(&timedata);
                                        self.daemon
                                            .udp
                                            .observed_information
                                            .lock()
                                            .request
                                            .set_request_time(false);
                                    }
                                }

                                match reader.read_tags(Acceptor { daemon: myself }) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        eprintln!("Error while reading roborio UDP tags: {:?}", err)
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

                        last_sucsess = std::time::Instant::now()
                    }
                    Err(err) => {
                        if last_sucsess.elapsed()
                            > std::time::Duration::from_millis(
                                myself.udp.connection_timeout_ms.load(Relaxed) as u64,
                            )
                        {
                            // this time is arbitrary but if we haven't been able to
                            // received or send a packet in over 500 ms we should reset the entire connection
                            eprintln!(
                                "No sucsessful communication in over {}ms, resetting connection",
                                myself.udp.connection_timeout_ms.load(Relaxed)
                            );
                            break;
                        }
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
    }

    // pub fn
}

impl RoborioCom {
    pub fn reset_con(&self) {
        self.udp
            .reset_con
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn crash_driverstation(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.tag_comm_version = 0;
        lock.request = DriverstationRequestCode::from_bits(0xFF);
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

    pub fn get_observed_robot_voltage(&self) -> RobotVoltage {
        self.udp.observed_information.lock().battery
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

    pub fn observe_robot_estop(&self, estopped: bool) {
        let mut lock = self.udp.observed_information.lock();
        lock.status.set_disabled();
        lock.control_code.set_disabled().set_estop(estopped);
    }

    // pub fn observe_restart_roborio_code(&self) {
    //     todo!()// self.udp.observed_information.lock().status = RobotStatusCode::from_bits(0x31);
    // }

    pub fn observe_robot_disabled(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.control_code.set_disabled();
        lock.status.set_disabled();
    }

    pub fn is_brownout_protection(&self) -> bool {
        self.udp
            .observed_information
            .lock()
            .control_code
            .is_brown_out_protection()
    }

    pub fn is_estopped(&self) -> bool {
        self.udp.observed_information.lock().control_code.is_estop()
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

    pub fn get_request_time(&self) -> bool {
        self.udp.observed_information.lock().request.request_time()
    }

    pub fn get_request_disable(&self) -> bool {
        self.udp
            .observed_information
            .lock()
            .request
            .request_disabled()
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

    pub fn set_udp_connection_timeout(&self, connection_timeout_ms: u32) {
        self.udp
            .connection_timeout_ms
            .store(connection_timeout_ms, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_udp_connection_timeout(&self) -> u32 {
        self.udp
            .connection_timeout_ms
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    //------------------------------ tags

    pub fn set_rumble(&self, rumble: Option<RobotToDriverRumble>) {
        self.udp.tag_data.lock().rumble = rumble;
    }

    pub fn get_rumble(&self) -> Option<RobotToDriverRumble> {
        self.udp.tag_data.lock().rumble
    }

    pub fn set_disk_usage(&self, usage: Option<RobotToDriverDiskUsage>) {
        self.udp.tag_data.lock().disk_usage = usage;
    }

    pub fn get_disk_usage(&self) -> Option<RobotToDriverDiskUsage> {
        self.udp.tag_data.lock().disk_usage
    }

    pub fn set_cpu_usage(&self, usage: Option<&[CpuUsage]>) {
        if let Some(old_usage) = &mut self.udp.tag_data.lock().cpu_usage {
            old_usage.clear();
            if let Some(usage) = usage {
                old_usage.reserve(usage.len());
                old_usage.copy_from_slice(usage);
            }
        }
    }

    pub fn get_cpu_usage(&self) -> Option<Vec<CpuUsage>> {
        self.udp.tag_data.lock().cpu_usage.clone()
    }

    pub fn set_ram_usage(&self, usage: Option<RobotToDriverRamUsage>) {
        self.udp.tag_data.lock().ram_usage = usage;
    }

    pub fn get_ram_usage(&self) -> Option<RobotToDriverRamUsage> {
        self.udp.tag_data.lock().ram_usage
    }

    pub fn set_pdp_port_report(&self, report: Option<PdpPortReport>) {
        self.udp.tag_data.lock().pdp_port_report = report;
    }

    pub fn get_pdp_port_report(&self) -> Option<PdpPortReport> {
        self.udp.tag_data.lock().pdp_port_report
    }

    pub fn set_pdp_power_report(&self, report: Option<PdpPowerReport>) {
        self.udp.tag_data.lock().pdp_power_report = report
    }

    pub fn get_pdp_power_report(&self) -> Option<PdpPowerReport> {
        self.udp.tag_data.lock().pdp_power_report
    }

    pub fn set_can_usage(&self, usage: Option<RobotToDriverCanUsage>) {
        self.udp.tag_data.lock().can_usage = usage;
    }

    pub fn get_can_usage(&self) -> Option<RobotToDriverCanUsage> {
        self.udp.tag_data.lock().can_usage
    }
}

macro_rules! generate_udp_tag_data_frequencies_impl {
    ($($get_fn_name:ident, $set_fn_name:ident, $var_name:ident,)*) => {
        $(impl RoborioCom{
            pub fn $get_fn_name(&self) -> u8 {
                self.udp.tag_frequences.$var_name.load(std::sync::atomic::Ordering::Relaxed)
            }

            /// $set_fn_name
            pub fn $set_fn_name(&self, val: u8) {
                self.udp.tag_frequences.$var_name.store(val, std::sync::atomic::Ordering::Relaxed)
            }
        }  )*
    };
}

generate_udp_tag_data_frequencies_impl!(
    get_disk_usage_frequency,
    set_disk_usage_frequency,
    disk_usage_m,
    get_cpu_usage_frequency,
    set_cpu_usage_frequency,
    cpu_usage_m,
    get_ram_usage_frequency,
    set_ram_usage_frequency,
    ram_usage_m,
    get_pdp_port_report_frequency,
    set_pdp_port_report_frequency,
    pdp_port_report_m,
    get_pdp_power_report_frequency,
    set_pdp_power_report_frequency,
    pdp_power_report_m,
    get_can_usage_frequency,
    set_can_usage_frequency,
    can_usage_m,
);

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
