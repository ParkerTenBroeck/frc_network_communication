use std::{
    error::Error,
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::{atomic::AtomicUsize, Arc, Mutex},
    time::Instant,
};

use crate::team_number::TeamNumber;
use mdns_sd::{ServiceDaemon, ServiceEvent};

struct Daemon(ServiceDaemon);
impl Drop for Daemon {
    fn drop(&mut self) {
        while Err(mdns_sd::Error::Again) == self.0.shutdown() {}
    }
}

impl Deref for Daemon {
    type Target = ServiceDaemon;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Daemon {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn find_robot_ip(team_number: impl Into<TeamNumber>) -> Result<IpAddr, Box<dyn Error>> {
    let team_number = team_number.into();

    let mdns = Daemon(ServiceDaemon::new()?);

    // no idea what this service is but it works
    let service = "_ni-rt._tcp.local.";
    let name = format!("roboRIO-{team_number}-FRC.{service}");

    let receiver = mdns.browse(service)?;

    let timeout = std::time::Duration::from_secs(5);
    let start = Instant::now();
    let deadline = start.checked_add(timeout).unwrap();
    while Instant::now() < deadline {
        let res = receiver.recv_deadline(deadline);
        let event = match res {
            Ok(ok) => ok,
            Err(_err) => {
                break;
            }
        };
        println!("{:?}", event);
        if let ServiceEvent::ServiceResolved(service) = event {
            if service.get_fullname().eq_ignore_ascii_case(&name) {
                if let Some(found_ip) = service.get_addresses().iter().next() {
                    return Ok(IpAddr::V4(*found_ip));
                }
            }
        }
    }

    // stop the mdns service deamon
    drop(mdns);

    if team_number.0 <= 9999 {
        println!("Trying static");
        let good_addr: Mutex<Option<Ipv4Addr>> = Mutex::new(Option::None);

        let upper = (team_number.0 / 100) as u8;
        let lower = (team_number.0 % 100) as u8;

        std::thread::scope(|s| {
            let good_addr = &good_addr;
            for i in 0..=255 {
                s.spawn(move || {
                    let addr = Ipv4Addr::new(10, upper, lower, i);
                    //TODO Determine if ip address is actually a valid rio or not
                    let is_valid = true;
                    if is_valid {
                        let mut good_addr = good_addr.lock().unwrap();
                        if let Some(good_addr) = &mut *good_addr {
                            // prioratize lower addresses in the (rare) case where multiple good roborios are found
                            if good_addr.octets()[3] > i {
                                *good_addr = addr;
                            }
                        }
                    }
                });
            }
        });

        if let Some(addr) = Mutex::into_inner(good_addr).unwrap() {
            return Ok(addr.into());
        }
    }

    Err("Cannot find roborio".into())
}

pub trait DiscoveryMethod {
    fn connect(robot_discovery: RobotDiscovery) -> Option<IpAddr> {
        match robot_discovery {
            RobotDiscovery::TeamNumber(team_number) => find_robot_ip(team_number).ok(),
            RobotDiscovery::HostName(_host) => todo!(),
            RobotDiscovery::Ip(ip) => Some(ip),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RobotConn<T: DiscoveryMethod> {
    inner: Arc<RobotConnInner>,
    reconnect: usize,
    reset_conn: usize,
    _phantom: PhantomData<T>,
}

#[derive(Debug)]
struct RobotConnInner {
    state: Mutex<RobotConnState>,
    reconnect: AtomicUsize,
    reset_conn: AtomicUsize,
}

impl RobotConnInner {
    fn get_connection_blocking<T: DiscoveryMethod>(&self) -> (Option<IpAddr>, usize) {
        let mut lock = self.state.lock().unwrap();
        let reconnect = self.reconnect.load(std::sync::atomic::Ordering::Acquire);

        // temporarily take ownership of the data
        match std::mem::replace(&mut *lock, RobotConnState::Disconnected) {
            // if we actually have a new connection imbound
            // connect to it with our discovery method and set our state accordingly
            RobotConnState::YetToConnect(discovery) => match T::connect(discovery) {
                Some(addr) => {
                    *lock = RobotConnState::Connected(addr);
                    (Some(addr), reconnect)
                }
                None => (None, reconnect),
            },
            // if were disconnected we can just return nothing
            RobotConnState::Disconnected => (None, reconnect),
            // if were connected we must reset our state then return the addr
            RobotConnState::Connected(addr) => {
                *lock = RobotConnState::Connected(addr);
                (Some(addr), reconnect)
            }
        }
    }
}

pub enum Reconnect {
    Yes(Option<IpAddr>),
    No,
}

impl<T: DiscoveryMethod> RobotConn<T> {
    pub fn new(conn: Option<impl Into<RobotDiscovery>>) -> Self {
        Self {
            inner: Arc::new(RobotConnInner {
                state: Mutex::new(
                    conn.map(|c| RobotConnState::YetToConnect(c.into()))
                        .unwrap_or(RobotConnState::Disconnected),
                ),
                reconnect: 0.into(),
                reset_conn: 0.into(),
            }),
            reconnect: 0,
            reset_conn: 0,
            _phantom: PhantomData::default(),
        }
    }

    pub fn reconnect(&mut self, conn: Option<impl Into<RobotDiscovery>>) {
        *self.inner.state.lock().unwrap() = conn
            .map(|c| RobotConnState::YetToConnect(c.into()))
            .unwrap_or(RobotConnState::Disconnected);
        self.inner
            .reconnect
            .fetch_add(1, std::sync::atomic::Ordering::Release);
    }

    pub fn check_reconnect(&mut self) -> Reconnect {
        let reset_conn = self
            .inner
            .reset_conn
            .load(std::sync::atomic::Ordering::Acquire);
        if self.reset_conn != reset_conn {
            Reconnect::Yes(self.connect())
        } else {
            Reconnect::No
        }
    }

    pub fn should_restart_conn(&mut self) -> bool {
        let reset_conn = self
            .inner
            .reset_conn
            .load(std::sync::atomic::Ordering::Relaxed);
        if self.reset_conn != reset_conn {
            self.reset_conn = reset_conn;
            true
        } else {
            false
        }
    }

    pub fn reset_conn(&mut self) {
        self.inner
            .reset_conn
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn connect(&mut self) -> Option<IpAddr> {
        let conn = self.inner.get_connection_blocking::<T>();
        self.reconnect = conn.1;
        conn.0
    }
}

#[derive(Debug)]
pub enum RobotDiscovery {
    TeamNumber(TeamNumber),
    HostName(String),
    Ip(IpAddr),
}

impl From<u16> for RobotDiscovery {
    fn from(value: u16) -> Self {
        Self::TeamNumber(value.into())
    }
}

impl From<IpAddr> for RobotDiscovery {
    fn from(value: IpAddr) -> Self {
        Self::Ip(value)
    }
}

impl From<Ipv4Addr> for RobotDiscovery {
    fn from(value: Ipv4Addr) -> Self {
        Self::Ip(IpAddr::V4(value))
    }
}

impl From<Ipv6Addr> for RobotDiscovery {
    fn from(value: Ipv6Addr) -> Self {
        Self::Ip(IpAddr::V6(value))
    }
}

impl From<&str> for RobotDiscovery {
    fn from(value: &str) -> Self {
        if let Ok(val) = TeamNumber::from_str(value) {
            Self::TeamNumber(val)
        } else if let Ok(val) = IpAddr::from_str(value) {
            Self::Ip(val)
        } else {
            Self::HostName(value.to_owned())
        }
    }
}

impl From<String> for RobotDiscovery {
    fn from(value: String) -> Self {
        if let Ok(val) = TeamNumber::from_str(&value) {
            Self::TeamNumber(val)
        } else if let Ok(val) = IpAddr::from_str(&value) {
            Self::Ip(val)
        } else {
            Self::HostName(value)
        }
    }
}

#[derive(Debug)]
enum RobotConnState {
    YetToConnect(RobotDiscovery),
    Disconnected,
    Connected(IpAddr),
}
