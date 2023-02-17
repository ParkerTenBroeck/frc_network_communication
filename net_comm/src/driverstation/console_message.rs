use crate::robot_to_driverstation::{Message, MessageKind, MessageReadError};

use super::message_handler::MessageHandler;

pub struct SystemConsoleOutput {}

impl MessageHandler for SystemConsoleOutput {
    fn receive_message(&mut self, message: Message<'_>) {
        match message.kind {
            MessageKind::Error(err) => {
                println!("\u{001B}[31mError {err}: {}\u{001b}[0m", message.message)
            }
            MessageKind::Warning(warn) => {
                println!("\u{001B}[33mWarning {warn}: {}\u{001b}[0m", message.message)
            }
            MessageKind::Message => {
                println!("{}", message.message)
            }
            MessageKind::ZeroCode => {
                println!("ZeroCode: {}", message.message)
            }
        }
    }

    fn parse_error(&mut self, err: MessageReadError) {
        eprintln!("{err:#?}")
    }
}
