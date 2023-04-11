use std::{
    net::IpAddr,
    ops::Deref,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{atomic::AtomicBool, Arc},
};

use robot_comm::common::error::RobotPacketParseError;
use spin::{Mutex, RwLock};
use tcp::RoborioTcp;
use udp::RoborioUdp;
use util::{buffer_reader::BufferReaderError, buffer_writter::BufferWritterError};

pub mod ringbuffer;
mod tcp;
mod udp;

#[derive(Default, Debug)]
pub struct RoborioCom {
    udp: RoborioUdp,
    tcp: RoborioTcp,
    common: RoborioCommon,
}

#[derive(Debug)]
pub enum RoborioComError {
    UdpIoInitError(std::io::Error),
    UdpIoSendError(std::io::Error),
    UdpIoReceiveError(std::io::Error),
    UdpCorePacketWriteError(BufferWritterError),
    UdpPacketTagWritterError(BufferWritterError),
    UdpCorePacketReadError(RobotPacketParseError),
    UdpPacketTagReadError(RobotPacketParseError),
    UdpConnectionTimeoutError,
    ModeSwitchHookPanic(Box<dyn std::any::Any + Send>),
    //tcp
    TcpIoInitError(std::io::Error),
    TcpIoSendError(std::io::Error),
    TcpIoReceiveError(std::io::Error),
    TcpIoGeneralError(std::io::Error),
    TcpPacketReadError(BufferReaderError),
}

type ErrorHandler =
    Box<dyn Fn(&RoborioCom, RoborioComError) + Send + Sync + UnwindSafe + RefUnwindSafe + 'static>;

struct RoborioCommon {
    request_info: AtomicBool,
    error_handler: RwLock<ErrorHandler>,
    driverstation_ip: Mutex<Option<IpAddr>>,
}

impl UnwindSafe for RoborioCommon {}
impl RefUnwindSafe for RoborioCommon {}

impl std::fmt::Debug for RoborioCommon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoborioCommon")
            .field("request_info", &self.request_info)
            .finish()
    }
}

impl Default for RoborioCommon {
    fn default() -> Self {
        fn default_error_handler(_com: &RoborioCom, err: RoborioComError) {
            eprintln!("{:#?}", err)
        }
        Self {
            request_info: Default::default(),
            error_handler: RwLock::new(Box::new(default_error_handler)),
            driverstation_ip: Default::default(),
        }
    }
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
        self.common.error_handler.read()(self, err)
    }
}

impl RoborioCom {
    pub fn set_error_handler(
        &self,
        handler: impl Fn(&RoborioCom, RoborioComError)
            + Send
            + Sync
            + UnwindSafe
            + RefUnwindSafe
            + 'static,
    ) -> ErrorHandler {
        let mut func: ErrorHandler = Box::new(handler);
        let mut lock = self.common.error_handler.write();
        std::mem::swap(&mut func, &mut *lock);
        drop(lock);
        func
    }

    pub fn get_driverstation_ip(&self) -> Option<IpAddr> {
        *self.common.driverstation_ip.lock()
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
        std::sync::Arc::strong_count(self) > 1
    }
}
