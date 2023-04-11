use crossterm::style::Color;
use roborio::RoborioCom;
use std::{
    collections::{HashMap, HashSet},
    hash::Hasher,
    sync::Arc,
};

use crate::{
    etui::{self, math_util::VecI2, Context, Style, StyledText},
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
            test_layout_text(ui);

            if true {
                return;
            }

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
                        if ui.button("こんにちは世界!").pressed() {
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

                                ui.tabbed_area(
                                    etui::id::Id::new("TABBS"),
                                    ["Bruh1", "Bruh2", "Bruh3"],
                                    |tab, ui| ui.label(format!("tab: {}", tab)),
                                );
                            });
                        });
                    });
                });
            });
        });
    }
}

fn test_layout_text(ui: &mut etui::Ui) {
    use etui::Layout::*;

    ui.tabbed_area(
        etui::id::Id::new("TABS"),
        ["Vertical", "Horizontal"],
        |tab, ui| {
            ui.bordered(|ui| {
                ui.with_size(ui.get_max().size(), |ui| {
                    if tab == 1 {
                        
                        let max = ui.get_max();
                        
                        ui.layout(TopLeftHorizontal, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopLeft\nHorizontal");
                                ui.label("TopLeftHorizontal");
                            });
                        });

                        ui.layout(BottomLeftHorizontal, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopLeft\nHorizontal");
                                ui.label("TopLeftHorizontal");
                            });
                        });

                        ui.set_max(max);

                        ui.layout(TopRightHorizontal, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopRight\nHorizontal");
                                ui.label("TopRightHorizontal");
                            });
                        });

                        ui.set_max(max);

                        ui.layout(BottomRightHorizontal, |ui| {
                            ui.bordered(|ui| {
                                ui.label("BottomRight\nHorizontal");
                                ui.label("BottomRightHorizontal");
                            });
                        });
                    } else {

                        let max = ui.get_max();

                        ui.layout(TopLeftVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopLeft\nVertical");
                                ui.label("TopLeftVertical");
                            });
                        });

                        ui.layout(BottomLeftVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("BottomLeft\nVertical");
                                ui.label("BottomLeftVertical");
                            });
                        });

                        ui.set_max(max);

                        ui.layout(TopRightVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopRight\nVertical");
                                ui.label("TopRightVertical");
                            });
                        });

                        ui.set_max(max);

                        ui.layout(BottomRightVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("BottomRight\nVertical");
                                ui.label("BottomRightVertical");
                            });
                        });
                    }
                });
            });
        },
    );
}

fn test_layout_tabs(ui: &mut etui::Ui) {
    use etui::Layout::*;

    ui.tabbed_area(
        etui::id::Id::new("TABS"),
        ["Vertical", "Horizontal"],
        |tab, ui| {
            ui.bordered(|ui| {
                ui.with_size(ui.get_max().size(), |ui| {
                    if tab == 1 {
                        ui.layout(TopLeftHorizontal, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopLeft\nHorizontal");
                                ui.label("TopLeftHorizontal");
                            });
                        });

                        ui.layout(BottomLeftHorizontal, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopLeft\nHorizontal");
                                ui.label("TopLeftHorizontal");
                            });
                        });

                        ui.layout(TopRightHorizontal, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopRight\nHorizontal");
                                ui.label("TopRightHorizontal");
                            });
                        });

                        ui.layout(BottomRightHorizontal, |ui| {
                            ui.bordered(|ui| {
                                ui.label("BottomRightHorizontal");
                                ui.label("BottomRightHorizontal");
                            });
                        });
                    } else {
                        ui.layout(TopLeftVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopLeft\nVertical");
                                ui.label("TopLeftVertical");
                            });
                        });

                        ui.layout(BottomLeftVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("BottomLeft\nVertical");
                                ui.label("BottomLeftVertical");
                            });
                        });

                        ui.layout(TopRightVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopRight\nVertical");
                                ui.label("TopRightVertical");
                            });
                        });

                        ui.layout(BottomRightVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("BottomRight\nVertical");
                                ui.label("BottomRightVertical");
                            });
                        });
                    }
                });
            });
        },
    );
}
