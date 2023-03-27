use std::{
    error::Error,
    fmt::Display,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::Duration,
};

use crate::{
    buffer_reader::{BufferReader, CreateFromBuf, ReadFromBuf},
    buffer_writter::{BufferWritter, WriteToBuff},
};

pub struct Socket {
    socket: UdpSocket,
    send_target: SendTargetAddr,
    #[allow(unused)]
    recv_port: u16,
    send_port: u16,
    packets_sent: usize,
    packets_received: usize,
    bytes_sent: usize,
    bytes_recieved: usize,
}

enum SendTargetAddr {
    Known(SocketAddr),
    LastReceved(SocketAddr),
}

impl SendTargetAddr {
    pub fn get_addr(&self) -> &SocketAddr {
        match self {
            SendTargetAddr::Known(addr) => addr,
            SendTargetAddr::LastReceved(addr) => addr,
        }
    }
}

impl Socket {
    pub fn new_target_unknown(recv: u16, send: u16) -> Self {
        let empty = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let recv_addr = SocketAddr::new(empty, recv);
        Self {
            socket: UdpSocket::bind(recv_addr).expect("Failed to connect to input socket"),
            send_target: SendTargetAddr::LastReceved(SocketAddr::new(empty, send)),
            recv_port: recv,
            send_port: send,
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_recieved: 0,
        }
    }

    pub fn new_target_knonw(recv: u16, send: SocketAddr) -> Self {
        let empty = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let recv_addr = SocketAddr::new(empty, recv);
        Self {
            socket: UdpSocket::bind(recv_addr).expect("Failed to connect to input socket"),
            send_target: SendTargetAddr::Known(send),
            recv_port: recv,
            send_port: send.port(),
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_recieved: 0,
        }
    }

    pub fn get_packets_sent(&self) -> usize {
        self.packets_sent
    }

    pub fn get_packets_recv(&self) -> usize {
        self.packets_received
    }

    pub fn get_bytes_sent(&self) -> usize {
        self.bytes_sent
    }

    pub fn get_bytes_recv(&self) -> usize {
        self.bytes_recieved
    }

    pub fn set_input_nonblocking(&self, nonblocking: bool) {
        self.socket
            .set_nonblocking(nonblocking)
            .expect("Failed to set socket input to non blocking")
    }

    pub fn set_read_timout(&self, dur: Option<Duration>) {
        self.socket
            .set_read_timeout(dur)
            .expect("Failed to set socket read timeout");
    }

    pub fn set_write_timout(&self, dur: Option<Duration>) {
        self.socket
            .set_write_timeout(dur)
            .expect("Failed to set socket write timeout");
    }
}

#[derive(Debug)]
pub enum SocketReadError<T: std::fmt::Debug> {
    Io(std::io::Error),
    Buffer(T),
}

impl<T: Error> Error for SocketReadError<T> {}

impl<T: Display + std::fmt::Debug> Display for SocketReadError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<T: std::fmt::Debug> From<std::io::Error> for SocketReadError<T> {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug)]
pub enum SocketWriteError<T: std::fmt::Debug> {
    Io(std::io::Error),
    Buffer(T),
}

impl<T: Error> Error for SocketWriteError<T> {}

impl<T: Display + std::fmt::Debug> Display for SocketWriteError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<T: std::fmt::Debug> From<std::io::Error> for SocketWriteError<T> {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl Socket {
    pub fn read<'a, T>(&mut self, buf: &'a mut [u8]) -> Result<Option<T>, SocketReadError<T::Error>>
    where
        T: CreateFromBuf<'a>,
        <T as ReadFromBuf<'a>>::Error: std::error::Error + 'static,
    {
        let read = match self.socket.recv_from(buf) {
            Ok(read) => {
                if let SendTargetAddr::LastReceved(addr) = &mut self.send_target {
                    *addr = read.1;
                    addr.set_port(self.send_port)
                }
                read.0
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    // we dont treat this as a hard error just return none
                    0
                } else {
                    Err(err)?
                }
            }
        };
        if read == 0 {
            Ok(None)
        } else {
            let got = &buf[..read];
            let mut buff = BufferReader::new(got);

            let rec = match T::create_from_buf(&mut buff) {
                Ok(ok) => Some(ok),
                Err(err) => return Err(SocketReadError::Buffer(err)),
            };

            self.packets_received += 1;
            self.bytes_recieved += read;

            Ok(rec)
        }
    }

    pub fn read_with<'a, T, E: std::fmt::Debug>(
        &mut self,
        buf: &'a mut [u8],
        process: impl FnOnce(&'a [u8]) -> Result<T, E>,
    ) -> Result<Option<T>, SocketReadError<E>> {
        let read = match self.socket.recv_from(buf) {
            Ok(read) => {
                if let SendTargetAddr::LastReceved(addr) = &mut self.send_target {
                    *addr = read.1;
                    addr.set_port(self.send_port)
                }
                read.0
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    // we dont treat this as a hard error just return none
                    0
                } else {
                    Err(err)?
                }
            }
        };
        if read == 0 {
            Ok(None)
        } else {
            let got = &buf[..read];
            // let mut buff = BufferReader::new(got);

            let rec = match process(got) {
                Ok(ok) => Some(ok),
                Err(err) => return Err(SocketReadError::Buffer(err)),
            };

            self.packets_received += 1;
            self.bytes_recieved += read;

            Ok(rec)
        }
    }

    pub fn read_into<'a, T>(
        &mut self,
        val: &'a mut T,
        buf: &'a mut [u8],
    ) -> Result<Option<&'a mut T>, SocketReadError<T::Error>>
    where
        T: CreateFromBuf<'a>,
        <T as ReadFromBuf<'a>>::Error: std::error::Error + 'static,
    {
        let read = match self.socket.recv_from(buf) {
            Ok(read) => {
                if let SendTargetAddr::LastReceved(addr) = &mut self.send_target {
                    *addr = read.1;
                    addr.set_port(self.send_port)
                }
                read.0
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    // we dont treat this as a hard error just return none
                    0
                } else {
                    Err(err)?
                }
            }
        };
        if read == 0 {
            Ok(None)
        } else {
            let got = &buf[..read];
            let mut buff = BufferReader::new(got);

            let rec = match val.read_into_from_buf(&mut buff) {
                Ok(ok) => Some(val),
                Err(err) => return Err(SocketReadError::Buffer(err)),
            };

            self.packets_received += 1;
            self.bytes_recieved += read;

            Ok(rec)
        }
    }

    pub fn write<'a, T>(
        &mut self,
        val: &T,
        buf: &'a mut impl BufferWritter<'a>,
    ) -> Result<usize, SocketWriteError<T::Error>>
    where
        T: WriteToBuff<'a>,
        <T as WriteToBuff<'a>>::Error: std::error::Error + 'static,
    {
        match val.write_to_buf(buf) {
            Ok(_) => {
                let buf = buf.curr_buf();
                Ok(self.write_raw(buf)?)
            }
            Err(err) => Err(SocketWriteError::Buffer(err)),
        }
    }

    pub fn write_raw(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        let addr = self.send_target.get_addr();
        let written = self.socket.send_to(buf, addr)?;
        // println!("{:?}", buf);

        if written != buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Not all bytes written to packet expected: {} wrote :{written}",
                    buf.len()
                ),
            ));
        }

        self.packets_sent += 1;
        self.bytes_sent += buf.len();

        Ok(buf.len())
    }
}
