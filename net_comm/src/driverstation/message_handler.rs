use std::{
    io::{Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    sync::atomic::AtomicBool,
};

use util::buffer_reader::{BufferReader, ReadFromBuff};

use crate::robot_to_driverstation::{Message, MessageReadError};

pub trait MessageHandler {
    fn receive_message(&mut self, message: Message<'_>);
    fn parse_error(&mut self, err: MessageReadError);
}

pub struct MessageConsole<T: MessageHandler> {
    reciever: T,
    exit: AtomicBool,
}

impl<T: MessageHandler> MessageConsole<T> {
    pub fn new(reciever: T) -> Self {
        Self {
            reciever,
            exit: false.into(),
        }
    }

    fn run_blocking_ret(&mut self, ipaddr: IpAddr) -> Result<(), std::io::Error> {
        let mut conn = TcpStream::connect(SocketAddr::new(ipaddr, 1740))?;

        let mut buf = Vec::with_capacity(4096);
        while !self.exit.load(std::sync::atomic::Ordering::Relaxed) {
            // to reduce the number of pad packets read before we find the "start" of packets
            // we look for a valid control code (0x0B 0x0C 0x00) but we need to keep track of the
            // size just before the packet.

            // we need to read 3 bytes into the buffer before we can actually check if the code is correct
            buf.resize(3, 0);
            conn.read_exact(&mut buf[..3])?;
            let mut shift_buf: u32 = 0;
            shift_buf |= buf[0] as u32;
            shift_buf <<= 8;
            shift_buf |= buf[1] as u32;
            shift_buf <<= 8;
            shift_buf |= buf[2] as u32;

            // while we dont have a valid control code shift our stored buffer over and read a new byte
            // while {
            //     let code = shift_buf & 0xFF;
            //     !(code == 0xB || code == 0xC/*|| code == 0x00 */)
            // } {
            //     // read another byte and shift it into our buffer
            //     shift_buf <<= 8;
            //     let mut byte = [0; 1];
            //     conn.read_exact(&mut byte)?;
            //     shift_buf |= byte[0] as u32;
            // }

            // now that we know we're probably looking at a valid comm packet we take the size
            // from the 2 bytes before our msg code.
            let size = ((shift_buf >> 8) & 0xFFFF) as usize;
            buf.resize(size, 0);
            let buff_exact = &mut buf[..size];

            if size > 0 {
                // we insert our message control code into the buffer so the message can be read form a buffer
                buff_exact[0] = (shift_buf & 0xFF) as u8;

                //since we've already read our msg code we read exactly our buff size starting 1 byte from the beginning
                conn.read_exact(&mut buff_exact[1..])?;
            }

            match crate::robot_to_driverstation::Message::read_from_buff(&mut BufferReader::new(
                buff_exact,
            )) {
                Ok(packet) => self.reciever.receive_message(packet),
                Err(err) => self.reciever.parse_error(err),
            }

            conn.write_all(&[0, 0]).unwrap();
        }

        Ok(())
    }

    pub fn run_blocking(mut self, ipaddr: IpAddr) {
        loop {
            match self.run_blocking_ret(ipaddr) {
                Ok(_) => return,
                Err(err) => {
                    eprintln!("Error: {}", err)
                }
            }
        }
    }

    pub fn create_blocking(mr: T, ipaddr: IpAddr) {
        MessageConsole::new(mr).run_blocking(ipaddr)
    }
}

impl<T: MessageHandler + Send + 'static> MessageConsole<T> {
    pub fn create_new_thread(mr: T, ipaddr: IpAddr) {
        std::thread::Builder::new()
            .name("Net Comm".into())
            .spawn(move || MessageConsole::new(mr).run_blocking(ipaddr))
            .unwrap();
    }
}
