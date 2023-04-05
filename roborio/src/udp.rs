use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    ops::Deref,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize},
};

use net_comm::robot_voltage::RobotVoltage;
use robot_comm::{
    common::{
        alliance_station::AllianceStation,
        control_code::ControlCode,
        joystick::{Joystick, NonNegU16},
        request_code::{DriverstationRequestCode, RobotRequestCode},
        time_data::TimeData, roborio_status_code::RobotStatusCode,
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
use spin::{Mutex, RwLock};
use util::{
    buffer_reader::BufferReader,
    buffer_writter::{BufferWritter, SliceBufferWritter},
};

use crate::{PossibleRcSelf, RoborioCom, RoborioComError};

#[derive(Debug)]
pub(super) struct RoborioUdp {
    recv: Mutex<DriverstationToRobotCorePacketDate>,
    joystick_values: Mutex<[Option<Joystick>; 6]>,
    countdown: Mutex<Option<f32>>,
    time: Mutex<TimeData>,
    observed_information: Mutex<RobotToDriverstationPacket>,
    clear_observed_status_on_send: AtomicBool,

    tag_data: Mutex<RoborioUdpTags>,
    tag_frequences: RoborioUdpTagFrequencies,

    reset_con: AtomicU8,
    connected: AtomicBool,
    bytes_sent: AtomicUsize,
    packets_sent: AtomicUsize,
    bytes_received: AtomicUsize,
    packets_received: AtomicUsize,
    /// The number of packets being -received- that have been "dropped"
    /// (if the sequence skips a value)
    packets_dropped: AtomicUsize,

    connection_timeout_ms: AtomicU32,
    read_block_timout_ms: AtomicU32,

    hooks: RwLock<Hooks>,
}

type HookDyn = dyn Fn() + Send + Sync + UnwindSafe + RefUnwindSafe + 'static;
type Hook = Option<Box<HookDyn>>;

#[derive(Default)]
struct Hooks {
    disable_hook: Hook,
    teleop_hook: Hook,
    auton_hook: Hook,
    test_hook: Hook,
    estop_hook: Hook,
    restart_code_hook: Hook,
    restart_rio_hook: Hook,
}

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hooks").finish()
    }
}

impl Default for RoborioUdp {
    fn default() -> Self {
        Self {
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
            read_block_timout_ms: AtomicU32::new(120),

            hooks: Default::default(),
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

// this impl block is everything related to running the UDP connection
impl RoborioCom {
    pub(super) fn run_udp_daemon<T: 'static + Send + PossibleRcSelf + Deref<Target = Self>>(
        myself: &T,
    ) {
        use std::sync::atomic::Ordering::Relaxed;

        myself.udp.reset_con.store(2, Relaxed);

        while (*myself).exists_elsewhere() {
            let reset_kind = myself.udp.reset_con.swap(0, Relaxed);
            if reset_kind >= 2 {
                myself.force_disable();
                // reset out udp information every time reconnect
                myself.common.driverstation_ip.lock().take();
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
                    lock.status.set_is_roborio(true);
                }
                *myself.udp.countdown.lock() = None;
            }

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
            match socket.set_read_timeout(Some(std::time::Duration::from_millis(
                myself.udp.read_block_timout_ms.load(Relaxed) as u64,
            ))) {
                Ok(_) => {}
                Err(err) => {
                    myself.report_error(RoborioComError::UdpIoInitError(err));
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    continue;
                }
            }

            // this runs until we need to reconnect or exit the daemon
            myself.run_udp_daemon_inner(myself, socket);
        }
    }

    fn run_udp_daemon_inner<T: 'static + Send + PossibleRcSelf + Deref<Target = Self>>(
        &self,
        myself_poss_ref: &T,
        socket: UdpSocket,
    ) {
        use std::sync::atomic::Ordering::Relaxed;
        // we should treat a new connection as a sucsess
        let mut last_sucsess = std::time::Instant::now();

        let mut send_addr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let mut send_buf = [0u8; 1024];
        let mut recv_buf = [0u8; 1024];
        while (*myself_poss_ref).exists_elsewhere() {
            if self.udp.reset_con.load(Relaxed) != 0 {
                return;
            }
            match socket.recv_from(&mut recv_buf) {
                Ok((read, rec_addr)) => {
                    self.udp.bytes_received.fetch_add(read, Relaxed);
                    let recv_buf = &recv_buf[..read];

                    if send_addr != rec_addr.ip() && self.udp.connected.load(Relaxed) {
                        //reconnect if theres a new address to send to
                        if last_sucsess.elapsed()
                            > std::time::Duration::from_millis(
                                self.udp.connection_timeout_ms.load(Relaxed) as u64,
                            )
                        {
                            // when the IP changes we want to treat this as a new connection
                            self.udp.reset_con.store(2, Relaxed);
                            break;
                        } else {
                            continue;
                        }
                    } else if !self.udp.connected.load(Relaxed) {
                        // if we aren't already connected
                        send_addr = rec_addr.ip();
                        *self.common.driverstation_ip.lock() = Some(send_addr);
                    }

                    let mut reader = BufferReader::new(recv_buf);
                    match DriverToRobotPacketReader::new(&mut reader) {
                        Ok((recv_packet, reader)) => {
                            self.respond_to_udp_packet(
                                &mut send_buf,
                                &socket,
                                send_addr,
                                recv_packet,
                            );

                            // if we've got this far yipee!!
                            self.udp.packets_received.fetch_add(1, Relaxed);

                            // read the additional tags and extra data after because it could possibly be slow
                            if let Err(err) = reader.read_tags(UdpTagAcceptor { daemon: self }) {
                                //waaaaa!
                                self.report_error(RoborioComError::UdpPacketTagReadError(err))
                            }

                            if recv_packet.request_code.is_requesting_lib_info() {
                                self.common.request_info.store(true, Relaxed);
                            }

                            last_sucsess = std::time::Instant::now()
                        }
                        Err(err) => {
                            self.udp.connected.store(false, Relaxed);
                            self.report_error(RoborioComError::UdpCorePacketReadError(err));
                        }
                    }
                }
                Err(err) => {
                    if last_sucsess.elapsed()
                        > std::time::Duration::from_millis(
                            self.udp.connection_timeout_ms.load(Relaxed) as u64,
                        )
                    {
                        // not sure if this should be 1 or 2, either way we should force disable
                        self.force_disable();
                        self.udp.reset_con.store(2, Relaxed);
                        self.udp.connected.store(false, Relaxed);
                        self.report_error(RoborioComError::UdpConnectionTimeoutError);
                        break;
                    }
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        // were still connected however we haven't received a packet in time so we should force disable
                        self.force_disable();
                    } else {
                        // if theres an IO error something weird happened so reset the connection
                        self.udp.reset_con.store(2, Relaxed);
                        self.udp.connected.store(false, Relaxed);
                        self.report_error(RoborioComError::UdpIoReceiveError(err));
                        return;
                    }
                }
            }
        }
    }

    #[cold]
    fn force_disable(&self) {
        let mut obv_lock = self.udp.observed_information.lock();
        let mut recv_lock = self.udp.recv.lock();
        let old = obv_lock.control_code;

        obv_lock.control_code.set_disabled();
        obv_lock.control_code.set_teleop();
        recv_lock.control_code.set_disabled();
        recv_lock.control_code.set_teleop();

        let new = obv_lock.control_code;
        drop(obv_lock);
        drop(recv_lock);
        self.run_hooks(
            old,
            new,
            RobotRequestCode::new(), /*Empty request code*/
        );
    }

    #[cold]
    fn run_estop_hook(&self) {
        if let Some(hook) = &self.udp.hooks.read().estop_hook {
            let res = std::panic::catch_unwind(hook);

            if let Err(err) = res {
                std::mem::forget(err);
                // oh god oh f**k HCF out of here aldfhlks
                self.udp.hooks.read().restart_rio_hook.as_ref().unwrap()();
            }
        }
    }

    #[cold]
    fn run_hooks(&self, old: ControlCode, new: ControlCode, request: RobotRequestCode) {
        if !old.is_estop() && new.is_estop() {
            self.run_estop_hook()
        }

        if request.should_restart_roborio() {
            if let Some(hook) = &self.udp.hooks.read().restart_rio_hook {
                // if this panics were f**ked so very hard
                hook()
            }
        }

        if request.should_restart_roborio_code() {
            if let Some(hook) = &self.udp.hooks.read().restart_code_hook {
                let res = std::panic::catch_unwind(hook);
                // if we panic here do a -not so gracful- process abort~
                if let Err(err) = res {
                    std::mem::forget(err);
                    std::process::abort()
                }
            }
        }
        macro_rules! mode_switch_hook {
            // ($hook:ident) => {
            //     let res = std::panic::catch_unwind(|| $hook());
            //     if let Err(err) = res {
            //         self.report_error(RoborioComError::ModeSwitchHookPanic(err));
            //         self.run_estop_hook();
            //     }
            // };
            ($hook:ident) => {
                if let Some($hook) = &self.udp.hooks.read().$hook {
                    let res = std::panic::catch_unwind($hook);
                    if let Err(err) = res {
                        self.report_error(RoborioComError::ModeSwitchHookPanic(err));
                        self.run_estop_hook();
                    }
                }
            };
        }

        if !old.is_disabled() && new.is_disabled() {
            mode_switch_hook!(disable_hook);
        // the mode can be teleop/auton/test even when disabled
        // so the if else ensures that we dont run each respective hook
        // until we actually enable
        } else if !old.is_teleop() && new.is_teleop() {
            mode_switch_hook!(teleop_hook);
        } else if !old.is_autonomus() && new.is_autonomus() {
            mode_switch_hook!(auton_hook);
        } else if !old.is_test() && new.is_test() {
            mode_switch_hook!(test_hook);
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

        if self.udp.clear_observed_status_on_send.load(Relaxed) {
            packet.status.set_disabled();
            packet.status.set_has_robot_code(false);
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

        // TODO: we could potentially speed up/optimize by checking for a change in the controlcode/request code before we call this?
        self.run_hooks(old_control_code, new_control_code, recv_packet.request_code);
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

            macro_rules! write_tag {
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

            write_tag!(rumble);
            write_tag!(disk_usage, disk_usage_m, 1);
            write_tag!(&cpu_usage, cpu_usage_m, 2);
            write_tag!(ram_usage, ram_usage_m, 3);
            write_tag!(&pdp_port_report, pdp_port_report_m, 4);
            write_tag!(pdp_power_report, pdp_power_report_m, 5);
            write_tag!(can_usage, can_usage_m, 6);
        }
    }

    // pub fn
}

// Public interface to UDP com ---------------------------------------------------

impl RoborioCom {
    
    pub fn reset_all_values(&self) {
        self.udp
            .reset_con
            .store(2, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn reconnect(&self) {
        // we dont want to override the reset con value if its stronger than 1
        // so we can use compare exchange to only set it when its zero
        // if it fails because the current value is 1 thats fine its what we want
        // and if it fails because the current value is greater than 1 it has a stronger reset and will do what we want
        let _ = self.udp.reset_con.compare_exchange(
            0,
            1,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// This will literally crash the offical driverstation 
    /// 
    /// # Safety
    /// 
    /// uh well.. this is sorta unsafe behavior so dont call it unless you actually want to... well... crash driverstation :)
    pub unsafe fn crash_driverstation(&self) {
        let mut lock = self.udp.observed_information.lock();
        lock.tag_comm_version = 0;
        lock.request = DriverstationRequestCode::from_bits(0xFF);
    }
}

//udp tings
impl RoborioCom {

    pub fn request_estop(&self){
        self.udp.observed_information.lock().control_code.set_estop(true);
        self.run_estop_hook();
    }

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

    pub fn get_observed_status(&self) -> RobotStatusCode {
        self.udp.observed_information.lock().status
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

    pub fn set_udp_read_blocking_timeout(&self, read_blocking_timeout_ms: u32) {
        self.udp.read_block_timout_ms.store(
            read_blocking_timeout_ms,
            std::sync::atomic::Ordering::Relaxed,
        );
        // we dont want to override the reset con value if its stronger than 1
        // so we can use compare exchange to only set it when its zero
        // if it fails because the current value is 1 thats fine its what we want
        // and if it fails because the current value is greater than 1 it has a stronger reset and will do what we want
        let _ = self.udp.reset_con.compare_exchange(
            0,
            1,
            std::sync::atomic::Ordering::Release,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
    pub fn get_udp_read_blocking_timeout(&self) -> u32 {
        self.udp
            .read_block_timout_ms
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

macro_rules! generate_hook_setters {
    ($($hook:ident, $fn_name:ident, $remove_fn_name:ident)*) => {
        impl RoborioCom{
            $(
                pub fn $fn_name(&self, func: impl Fn() + Send + Sync + RefUnwindSafe + UnwindSafe + 'static ) -> Hook{
                    let mut func: Hook = Some(Box::new(func));
                    let mut lock = self.udp.hooks.write();
                    std::mem::swap(&mut func, &mut lock.$hook);
                    drop(lock);
                    func
                }

                pub fn $remove_fn_name(&self) -> Hook{
                    let mut func: Hook = None;
                    let mut lock = self.udp.hooks.write();
                    std::mem::swap(&mut func, &mut lock.$hook);
                    drop(lock);
                    func
                }
            )*
        }
    };
}

generate_hook_setters!(
    disable_hook, set_disable_hook, take_disable_hook
    teleop_hook, set_teleop_hook, take_teleop_hook
    auton_hook, set_auton_hook, take_auton_hook
    test_hook, set_test_hook, take_test_hook
    estop_hook, set_estop_hook, take_estop_hook
    restart_code_hook, set_restart_code_hook, take_restart_code_hook
    restart_rio_hook, set_restart_rio_hook, take_restart_rio_hook
);

macro_rules! generate_udp_tag_data_frequencies_impl {
    ($($get_fn_name:ident, $set_fn_name:ident, $var_name:ident,)*) => {
        $(impl RoborioCom{

            /// Gets the frequency that this tag will be included in the packet
            /// a value of 1 will means the tag is included in every packet
            /// 2 will include it every other packet and so on.
            ///
            /// NOTE: The tag will not be included no matter the value of the frequency if it doesn't exist.
            ///
            /// A value of zero indicates this tag will never be sent even if it exists.
            pub fn $get_fn_name(&self) -> u8 {
                self.udp.tag_frequences.$var_name.load(std::sync::atomic::Ordering::Relaxed)
            }

            /// Sets the frequency that this tag will be included in the packet
            /// setting `val` to 1 will make make it so the tag is included in every packet
            /// 2 will include it every other packet and so on.
            ///
            /// NOTE: The tag will not be included no matter the value of the frequency if it doesn't exist.
            ///
            /// Setting `val` to zero will disable the tag from being included even if it exists
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
