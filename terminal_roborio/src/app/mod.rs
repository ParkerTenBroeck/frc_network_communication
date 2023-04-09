use roborio::RoborioCom;
use std::{
    sync::Arc,
};


use crate::etui::{self, Context};

enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    driverstation: Arc<RoborioCom>,
    tab: usize,
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
}

struct Common {}

impl Common {
}

struct Udp {}

impl Udp {
}

struct Tcp {}

impl Tcp {
}

impl App {
    pub fn new(driverstation: Arc<RoborioCom>) -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            tab: 0,
            driverstation,
        }
    }

    pub fn ui(&mut self, ctx: &etui::Context) {
        Context::frame(ctx, |ui| {
            ui.label(format!("Driverstation:    {:?}", self.driverstation.get_driverstation_ip()));
            ui.label(format!("Packets Dropped:  {}", self.driverstation.get_udp_packets_dropped()));
            ui.label(format!("Packets Sent:     {}", self.driverstation.get_udp_packets_sent()));
            ui.label(format!("Bytes Sent:       {}", self.driverstation.get_udp_bytes_sent()));
            ui.label(format!("Packets Received: {}", self.driverstation.get_udp_packets_received()));
            ui.label(format!("Bytes Received:   {}", self.driverstation.get_udp_bytes_received()));

            ui.label("Bruh\n\tHey the newline and tab works");
            if ui.button("こんにちは世界!") {
                ui.label("UNICODEEEEE")
            }
            ui.label(format!("{:#?}", ui.ctx().get_event()))
        });
    }
}
