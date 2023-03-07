use crate::robot_to_driverstation::{Message, MessageKind, MessageReadError};

use super::message_handler::MessageHandler;

pub struct SystemConsoleOutput {}

impl MessageHandler for SystemConsoleOutput {
    fn receive_message(&mut self, message: Message<'_>) {
        match message.kind {
            MessageKind::Error {
                err,
                msg,
                loc,
                stack,
                ..
            } => {
                if stack.len() > 0 {
                    println!("\u{001B}[31mError {err}: {msg} at {loc} stack \n{stack}\u{001b}[0m")
                } else {
                    println!("\u{001B}[31mError {err}: {msg} at {loc}\u{001b}[0m")
                }
            }
            MessageKind::Warning {
                warn,
                msg,
                loc,
                stack,
                ..
            } => {
                if stack.len() > 0 {
                    println!(
                        "\u{001B}[33mWarning {warn}: {msg} at {loc} stack \n{stack}\u{001b}[0m"
                    )
                } else {
                    println!("\u{001B}[33mWarning {warn}: {msg} at {loc}\u{001b}[0m")
                }
            }
            MessageKind::Message { msg, .. } => {
                println!("{}", msg)
            }
            MessageKind::ZeroCode { msg } => {
                println!("ZeroCode: {}", msg)
            }
            MessageKind::Report { kind } => {
                println!("Report: {kind:?}")
            }
            MessageKind::Unknown0x0D { .. } => {
                // println!("Unknown0x0D: {_data:?}")
            }
        }
    }

    fn parse_error(&mut self, err: MessageReadError) {
        eprintln!("{err:#?}")
    }
}
