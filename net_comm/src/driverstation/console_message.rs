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
            MessageKind::VersionInfo { kind } => {
                println!("Report: {kind:?}")
            }
            MessageKind::UnderlineAnd5VDisable { .. } => {}
            MessageKind::RailFaults {
                short_6v,
                short_5v,
                short_3_3v,
            } => {
                println!("Shorts: 6v: {short_6v}, 5v: {short_5v}, 3.3v: {short_3_3v}")
            }
            MessageKind::DisableFaults { comms, fault_12v } => {}
            MessageKind::UsageReport {
                team,
                unknwon,
                usage,
            } => {}
        }
    }

    fn parse_error(&mut self, err: MessageReadError) {
        eprintln!("{err:#?}")
    }
}

pub struct Ignore {}

impl MessageHandler for Ignore {
    fn receive_message(&mut self, message: Message<'_>) {}

    fn parse_error(&mut self, err: MessageReadError) {}
}
