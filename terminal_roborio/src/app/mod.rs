use crossterm::style::Color;
use roborio::RoborioCom;
use std::{collections::{HashMap, HashSet}, sync::Arc, hash::Hasher};

use crate::{
    etui::{self, Context, Style, StyledText, math_util::VecI2},
    Log,
};

enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    driverstation: Arc<RoborioCom>,
    log: Arc<Log>,
    tab: usize,
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
}

struct Common {}

impl Common {}

struct Udp {}

impl Udp {}

struct Tcp {}

impl Tcp {}

impl App {
    pub fn new(driverstation: Arc<RoborioCom>, log: Arc<Log>) -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            tab: 0,
            driverstation,
            log,
        }
    }

    pub fn ui(&mut self, ctx: &etui::Context) {
        self.driverstation.observe_robot_code(true);
        Context::frame(ctx, |ui| {
            ui.bordered(|ui| {
                let mut msg = StyledText::new("Press esc to exit");
                msg.rapid_blink(true);
                ui.label(msg);
            });

            ui.bordered(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(format!(
                            "Driverstation:    {:?}",
                            self.driverstation.get_driverstation_ip()
                        ));

                        let control = self.driverstation.get_control_code();
                        let mut style = Style::default();
                        let msg = if control.is_estop() {
                            style.fg = Color::White;
                            style.bg = Color::Red;
                            "ESTOP"
                        } else if control.is_disabled() {
                            "Disabled"
                        } else if control.is_teleop() {
                            style.fg = Color::Blue;
                            "Teleop"
                        } else if control.is_autonomus() {
                            style.fg = Color::Green;
                            "Auton"
                        } else if control.is_test() {
                            style.fg = Color::DarkYellow;
                            "Test"
                        } else {
                            "Bruh"
                        };

                        ui.label(StyledText {
                            text: msg.into(),
                            style,
                        });

                        ui.label(format!(
                            "Packets Dropped:  {}",
                            self.driverstation.get_udp_packets_dropped()
                        ));
                        ui.label(format!(
                            "Packets Sent:     {}",
                            self.driverstation.get_udp_packets_sent()
                        ));
                        ui.label(format!(
                            "Bytes Sent:       {}",
                            self.driverstation.get_udp_bytes_sent()
                        ));
                        ui.label(format!(
                            "Packets Received: {}",
                            self.driverstation.get_udp_packets_received()
                        ));
                        ui.label(format!(
                            "Bytes Received:   {}",
                            self.driverstation.get_udp_bytes_received()
                        ));

                        ui.horizontal(|ui| {
                            ui.label("Alliance: ");
                            let mut style = Style::default();
                            if self.driverstation.get_alliance_station().is_red() {
                                style.fg = Color::Red;
                            } else {
                                style.fg = Color::DarkBlue;
                            }
                            ui.label(StyledText::styled(
                                format!("{:?}", self.driverstation.get_alliance_station()),
                                style,
                            ))
                        });
                        if ui.button("こんにちは世界!") {
                            ui.label("UNICODEEEEE")
                        }
                        ui.label(format!("{:#?}", ui.ctx().get_event()));

                        ui.label(format!("{:?}", ui.get_clip()));
                        ui.label(format!("{:?}", ui.get_cursor()));
                        ui.label(format!("{:?}", ui.get_max()));
                        ui.label(format!("{:?}", ui.get_current()));
                    });
                    ui.with_size(ui.get_max().size(), |ui| {
                        ui.seperator();
                        ui.vertical(|ui| {
                            ui.bordered(|ui| {
                                ui.vertical(|ui| {
                                    ui.label("Events:");
                                    ui.set_minimum_size(VecI2::new(40, 10));
                                    for (level, msg) in self.log.get_last_n(10) {
                                        let mut style = Style::default();
                                        match level {
                                            crate::LogLevel::Message => {}
                                            crate::LogLevel::Warning => style.bg = Color::Yellow,
                                            crate::LogLevel::Error => style.fg = Color::Red,
                                        }
                                        ui.horizontal(|ui| {
                                            ui.label("->");
                                            ui.label(StyledText::styled(msg, style))
                                        });
                                    }
                                });
                            });
                            ui.vertical(|ui| {
                                ui.drop_down("Luigi", |ui| {
                                    ui.label("luigi!!!");
                                    ui.label("wahoo");
                                });
                                ui.add_vertical_space(1);
                                ui.drop_down("Luigi2", |ui| {
                                    ui.label("luigi!!!");
                                    ui.label("wahoo");
                                });
                                ui.add_vertical_space(1);
                                ui.drop_down("Luigi3!!", |ui| {
                                    ui.label("luigi!!!");
                                    ui.label("wahoo");
                                });
                            });
                        });
                    });
                });
            });
        });
    }
}
