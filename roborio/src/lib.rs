use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket},
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize}
};

use atomic::Atomic;
use net_comm::robot_voltage::RobotVoltage;
use rclite::Arc;
use robot_comm::{
    common::{
        alliance_station::AllianceStation,
        control_code::ControlCode,
        error::RobotPacketParseError,
        joystick::{Joystick, NonNegU16},
        request_code::{DriverstationRequestCode, RobotRequestCode},
        time_data::TimeData,
    },
    driver_to_robot::{
        reader::{DriverToRobotPacketReader, PacketTagAcceptor},
        DriverstationToRobotCorePacketDate,
    },
    robot_to_driver::{
        self, writter::RobotToDriverstaionPacketWritter, CpuUsage, PdpPortReport, PdpPowerReport,
        RobotToDriverCanUsage, RobotToDriverDiskUsage, RobotToDriverRamUsage, RobotToDriverRumble,
        RobotToDriverstationPacket,
    },
};
use spin::Mutex;
use util::{
    buffer_reader::BufferReader,
    buffer_writter::{BufferWritter, BufferWritterError, SliceBufferWritter},
};

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
    driverstation_ip: Mutex<Option<IpAddr>>,
    recv: Mutex<DriverstationToRobotCorePacketDate>,
    joystick_values: Mutex<[Option<Joystick>; 6]>,
    countdown: Mutex<Option<f32>>,
    time: Mutex<TimeData>,
    observed_information: Mutex<RobotToDriverstationPacket>,
    clear_observed_status_on_send: AtomicBool,

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

    disable_hook: Atomic<Option<fn()>>,
    teleop_hook: Atomic<Option<fn()>>,
    auton_hook: Atomic<Option<fn()>>,
    test_hook: Atomic<Option<fn()>>,
    estop_hook: Atomic<Option<fn()>>,
    restart_code_hook: Atomic<Option<fn()>>,
    restart_rio_hook: Atomic<Option<fn() -> !>>,
}

impl Default for RoborioUdp {
    fn default() -> Self {
        Self {
            driverstation_ip: Default::default(),
            recv: Default::default(),
            joystick_values: Default::default(),
            countdown: Default::default(),
            time: Default::default(),
            observed_information: Default::default(),
            clear_observed_status_on_send: Default::default(),
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

            disable_hook: Default::default(),
            teleop_hook: Default::default(),
            auton_hook: Default::default(),
            test_hook: Default::default(),
            estop_hook: Default::default(),
            restart_code_hook: Default::default(),
            restart_rio_hook: Default::default(),
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

#[derive(Debug)]
enum RoborioComError {
    UdpIoInitError(std::io::Error),
    UdpIoSendError(std::io::Error),
    UdpIoReceiveError(std::io::Error),
    UdpCorePacketWriteError(BufferWritterError),
    UdpPacketTagWritterError(BufferWritterError),
    UdpCorePacketReadError(RobotPacketParseError),
    UdpPacketTagReadError(RobotPacketParseError),
    UdpConnectionTimeoutError,
    ModeSwitchHookPanic(Box<dyn std::any::Any + Send>),
}

// #[derive(Debug)]
struct RoborioCommon {
    request_info: AtomicBool,
    error_handler: Atomic<fn(&RoborioCom, RoborioComError)>,
}
impl std::fmt::Debug for RoborioCommon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoborioCommon")
            .field("request_info", &self.request_info)
            .finish()
    }
}

impl Default for RoborioCommon {
    fn default() -> Self {
        Self {
            request_info: Default::default(),
            error_handler: Atomic::new(default_error_handler),
        }
    }
}

fn default_error_handler(_com: &RoborioCom, err: RoborioComError) {
    eprintln!("{:#?}", err)
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

    fn report_error(&self, err: RoborioComError) {
        self.common.error_handler.load(atomic::Ordering::Relaxed)(self, err)
    }
}

impl RoborioCom {
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
}

// all the UDP shit
impl RoborioCom {
    fn run_udp_daemon<T: 'static + Send + PossibleRcSelf + Deref<Target = Self>>(myself: &T) {
        use std::sync::atomic::Ordering::Relaxed;
        while myself.exists_elsewhere() {
            // reset out udp information every time reconnect
            myself.udp.driverstation_ip.lock().take();
            myself.udp.packets_dropped.store(0, Relaxed);
            myself.udp.connected.store(false, Relaxed);
            myself.udp.bytes_received.store(0, Relaxed);
            myself.udp.packets_received.store(0, Relaxed);
            myself.udp.packets_sent.store(0, Relaxed);
            myself.udp.bytes_sent.store(0, Relaxed);
            *myself.udp.time.lock() = TimeData::default();
            *myself.udp.joystick_values.lock() = [None; 6];
            {
                // these two states percist across reconnects
                let mut lock = myself.udp.observed_information.lock();
                *lock = RobotToDriverstationPacket {
                    tag_comm_version: 1,
                    control_code: *ControlCode::default()
                        .set_estop(lock.control_code.is_estop())
                        .set_brownout_protection(lock.control_code.is_brown_out_protection()),
                    ..Default::default()
                };
            }
            *myself.udp.countdown.lock() = None;

            // we should accept from any adress on port 1110
            let socket =
                match UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1110))
                {
                    Ok(ok) => ok,
                    Err(err) => {
                        myself.report_error(RoborioComError::UdpIoInitError(err));
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        continue;
                    }
                };
            // idk this time is sorta random :3
            match socket.set_read_timeout(Some(std::time::Duration::from_millis(50))) {
                Ok(_) => {}
                Err(err) => {
                    myself.report_error(RoborioComError::UdpIoInitError(err));
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    continue;
                }
            }

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
                            if last_sucsess.elapsed()
                                > std::time::Duration::from_millis(
                                    myself.udp.connection_timeout_ms.load(Relaxed) as u64,
                                )
                            {
                                break;
                            } else {
                                continue;
                            }
                        } else if !myself.udp.connected.load(Relaxed) {
                            // if we aren't already connected
                            send_addr = rec_addr.ip();
                            *myself.udp.driverstation_ip.lock() = Some(send_addr);
                        }

                        let mut reader = BufferReader::new(recv_buf);
                        match DriverToRobotPacketReader::new(&mut reader) {
                            Ok((recv_packet, reader)) => {
                                myself.respond_to_udp_packet(
                                    &mut send_buf,
                                    &socket,
                                    send_addr,
                                    recv_packet,
                                );

                                // read the additional tags and extra data after because it could possibly be slow
                                match reader.read_tags(UdpTagAcceptor { daemon: myself }) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        myself.report_error(RoborioComError::UdpPacketTagReadError(err))
                                    }
                                }

                                // if we've got this far yay!!
                                myself.udp.packets_received.fetch_add(1, Relaxed);

                                last_sucsess = std::time::Instant::now()
                            }
                            Err(err) => {
                                myself.udp.connected.store(false, Relaxed);
                                myself.report_error(RoborioComError::UdpCorePacketReadError(err));
                            }
                        }
                    }
                    Err(err) => {
                        if last_sucsess.elapsed()
                            > std::time::Duration::from_millis(
                                myself.udp.connection_timeout_ms.load(Relaxed) as u64,
                            )
                        {
                            myself.report_error(RoborioComError::UdpConnectionTimeoutError);
                            break;
                        }
                        if err.kind() == std::io::ErrorKind::WouldBlock {
                            myself.udp.connected.store(false, Relaxed);
                            continue;
                        } else {
                            // reset connection
                            myself.report_error(RoborioComError::UdpIoReceiveError(err));
                            break;
                        }
                    }
                }
            }
        }
    }

    #[cold]
    fn run_estop_hook(&self) {
        use atomic::Ordering::Relaxed;

        if let Some(hook) = self.udp.estop_hook.load(Relaxed) {
            let res = std::panic::catch_unwind(hook);

            if let Err(err) = res {
                std::mem::forget(err);
                self.udp.restart_rio_hook.load(Relaxed).unwrap()();
            }
        }
    }

    #[cold]
    fn run_hooks(&self, old: ControlCode, new: ControlCode, request: RobotRequestCode) {
        use atomic::Ordering::Relaxed;

        if !old.is_estop() && new.is_estop() {
            self.run_estop_hook()
        }

        if request.should_restart_roborio() {
            if let Some(hook) = self.udp.restart_rio_hook.load(Relaxed) {
                // if this panics were f**ked so very hard
                hook()
            }
        }

        if request.should_restart_roborio_code() {
            if let Some(hook) = self.udp.restart_code_hook.load(Relaxed) {
                let res = std::panic::catch_unwind(hook);
                // if we panic here do a -not so gracful- process abort~
                if let Err(err) = res {
                    std::mem::forget(err);
                    std::process::abort()
                }
            }
        }
        macro_rules! mode_switch_hook {
            ($hook:ident) => {
                let res = std::panic::catch_unwind(|| $hook());
                if let Err(err) = res {
                    self.report_error(RoborioComError::ModeSwitchHookPanic(err));
                    self.run_estop_hook();
                }
            };
        }

        if !old.is_disabled() && new.is_disabled() {
            if let Some(hook) = self.udp.disable_hook.load(Relaxed) {
                mode_switch_hook!(hook);
            }
        // the mode can be teleop/auton/test even when disabled
        // so the if else ensures that we dont run each respective hook
        // until we actually enable
        } else if !old.is_teleop() && new.is_teleop() {
            if let Some(hook) = self.udp.teleop_hook.load(Relaxed) {
                mode_switch_hook!(hook);
            }
        } else if !old.is_autonomus() && new.is_autonomus() {
            if let Some(hook) = self.udp.auton_hook.load(Relaxed) {
                mode_switch_hook!(hook);
            }
        } else if !old.is_test() && new.is_test() {
            if let Some(hook) = self.udp.test_hook.load(Relaxed) {
                mode_switch_hook!(hook);
            }
        }
    }

    #[inline(always)]
    fn respond_to_udp_packet(
        &self,
        send_buf: &mut [u8],
        socket: &UdpSocket,
        send_addr: IpAddr,
        recv_packet: DriverstationToRobotCorePacketDate,
    ) {
        use std::sync::atomic::Ordering::Relaxed;
        *self.udp.recv.lock() = recv_packet;

        let mut packet = self.udp.observed_information.lock();

        // we needs these for later and all the things we need are already locked
        let old_control_code = packet.control_code;
        let mut new_control_code = recv_packet.control_code;

        // we need to keep certian things like estop and brownout
        new_control_code.set_estop(old_control_code.is_estop() | new_control_code.is_estop());
        new_control_code.set_brownout_protection(
            old_control_code.is_brown_out_protection() | old_control_code.is_brown_out_protection(),
        );
        packet.control_code = new_control_code;

        // this calculates packet loss in the difference between sequence numbers
        let lower = packet.sequence.wrapping_add(1);
        let higher = recv_packet.sequence;
        if lower != higher && self.udp.connected.load(Relaxed) {
            let calc_dif = match lower.cmp(&higher) {
                std::cmp::Ordering::Less => higher.wrapping_sub(lower),
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => lower.wrapping_sub(higher),
            };
            self.udp
                .packets_dropped
                .fetch_add(calc_dif as usize, Relaxed);
        }
        // update the seruqnce after checking for dropped packets
        packet.sequence = recv_packet.sequence;

        let mut writter = SliceBufferWritter::new(send_buf);
        let writter =
            robot_to_driver::writter::RobotToDriverstaionPacketWritter::new(&mut writter, *packet);

        if recv_packet.control_code.is_disabled() {
            packet.request.set_request_disable(false);
        }
        drop(packet);

        'response: {
            let mut packet_writter = match writter {
                Ok(ok) => ok,
                Err(err) => {
                    // If we faild to write the core packet (this really should never fail but whatever)
                    // just stop tryint to respons.
                    // we cant return because we need to handle our hooks so we break
                    self.udp.connected.store(false, Relaxed);
                    self.report_error(RoborioComError::UdpCorePacketWriteError(err));
                    break 'response;
                }
            };

            self.write_udp_packet_tags(&mut packet_writter);

            // actually send our response
            match socket.send_to(packet_writter.into_buf(), SocketAddr::new(send_addr, 1150)) {
                Ok(wrote) => {
                    self.udp.bytes_sent.fetch_add(wrote, Relaxed);
                    self.udp.packets_sent.fetch_add(1, Relaxed);
                    self.udp.connected.store(true, Relaxed);
                }
                Err(err) => {
                    self.udp.connected.store(false, Relaxed);
                    self.report_error(RoborioComError::UdpIoSendError(err))
                }
            }
        }

        if recv_packet.request_code.should_restart_roborio()
            || recv_packet.request_code.should_restart_roborio()
            || old_control_code.get(ControlCode::MODE) != new_control_code.get(ControlCode::MODE)
            || old_control_code.is_disabled() != new_control_code.is_disabled()
        {
            self.run_hooks(old_control_code, new_control_code, recv_packet.request_code);
        }
    }

    /// Write the tags to the response packet writter acording the current settings/data in outself
    #[inline(always)]
    fn write_udp_packet_tags<'a, 'b, T: BufferWritter<'a>>(
        &self,
        packet_writter: &mut RobotToDriverstaionPacketWritter<'a, 'b, T>,
    ) {
        use std::sync::atomic::Ordering::Relaxed;
        'tags: {
            let lock = self.udp.tag_data.lock();
            let packets_sent = self.udp.packets_sent.load(Relaxed);

            // TODO: report errros better
            macro_rules! tag_thing {
                ($tag:ident) => {
                    if let Some($tag) = lock.$tag{
                        if let Err(err) = packet_writter.$tag($tag){
                            self.report_error(RoborioComError::UdpPacketTagWritterError(err));
                            break 'tags;
                        }
                    }
                };
                ($tag:ident, $tag_m:ident $(, $offset:expr)?) => {
                    if let Some($tag) = lock.$tag{
                        let $tag_m = self.udp.tag_frequences.$tag_m.load(Relaxed);
                        if $tag_m != 0{
                            if ((packets_sent $(+ $offset)?) as usize) % ($tag_m as usize) == 0{
                                if let Err(err) = packet_writter.$tag($tag){
                                    self.report_error(RoborioComError::UdpPacketTagWritterError(err));
                                    break 'tags;
                                }
                            }
                        }
                    }
                };
                (&$tag:ident, $tag_m:ident $(, $offset:expr)?) => {
                    if let Some($tag) = &lock.$tag{
                        let $tag_m = self.udp.tag_frequences.$tag_m.load(Relaxed);
                        if $tag_m != 0{
                            if ((packets_sent $(+ $offset)?) as usize) % ($tag_m as usize) == 0{
                                if let Err(err) = packet_writter.$tag($tag){
                                    self.report_error(RoborioComError::UdpPacketTagWritterError(err));
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
    }

    // pub fn
}

struct UdpTagAcceptor<'a> {
    daemon: &'a RoborioCom,
}
impl<'a> PacketTagAcceptor for UdpTagAcceptor<'a> {
    #[inline(always)]
    fn accept_joystick(&mut self, index: usize, joystick: Option<Joystick>) {
        if let Some(joy) = self.daemon.udp.joystick_values.lock().get_mut(index) {
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
        lock.status.set_teleop();
    }

    pub fn observe_robot_autonomus(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.status.observe_robot_autonomus();
    }

    pub fn observe_robot_test(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.status.set_test();
    }

    pub fn observe_robot_disabled(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.status.set_disabled();
    }

    pub fn is_brownout_protection(&self) -> bool {
        self.udp
            .observed_information
            .lock()
            .control_code
            .is_brown_out_protection()
    }

    pub fn set_estopped(&self) {
        self.udp
            .observed_information
            .lock()
            .control_code
            .set_estop(true);
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
