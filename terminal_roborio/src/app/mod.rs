use crossterm::style::Color;
use roborio::{Joystick, RoborioCom};
use std::{
    collections::{HashMap, HashSet},
    hash::Hasher,
    sync::{Arc, Mutex},
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

    pub fn ui(&mut self, ctx: &etui::Context, len: usize) {
        self.driverstation.observe_robot_code(true);
        Context::frame(ctx, |ui| {
            test_layout_text(ui);
             if true {
                return;
            }

            ui.label(format!("{:?}", ui.get_max().size()));
            ui.label(format!("Draw call len: {}B", len));
           

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

                        // ui.label(format!("{:?}", ui.get_clip()));
                        // ui.label(format!("{:?}", ui.get_cursor()));
                        // ui.label(format!("{:?}", ui.get_max()));
                        // ui.label(format!("{:?}", ui.get_current()));
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
                                    ["Joy1", "Joy2", "Joy3", "Joy4", "Joy5", "Joy6"],
                                    |tab, ui| {
                                        ui.bordered(|ui| {
                                            if let Some(joy) = self.driverstation.get_joystick(tab)
                                            {
                                                ui.label("Buttons");
                                                ui.horizontal(|ui| {
                                                    for b in 0..joy.buttons_len() {
                                                        let style = Style {
                                                            fg: Color::Black,
                                                            bg: if joy
                                                                .get_button(b)
                                                                .unwrap_or(false)
                                                            {
                                                                Color::Green
                                                            } else {
                                                                Color::DarkGrey
                                                            },
                                                            ..Default::default()
                                                        };
                                                        ui.label(StyledText::styled(
                                                            "▎ ".to_owned(),
                                                            style,
                                                        ));
                                                    }
                                                });
                                                ui.label("Povs");

                                                ui.horizontal(|ui| {
                                                    for p in 0..joy.povs_len() {
                                                        let mut string = String::new();
                                                        let pov =
                                                            joy.get_pov(p).unwrap_or_default();
                                                        for y in 0..3 {
                                                            for x in 0..3 {
                                                                let (px, py) = match pov.get() {
                                                                    None => (1, 1),
                                                                    Some(val) => match val {
                                                                        0 => (1, 0),
                                                                        45 => (2, 0),
                                                                        90 => (2, 1),
                                                                        135 => (2, 2),
                                                                        180 => (1, 2),
                                                                        225 => (0, 2),
                                                                        270 => (0, 1),
                                                                        315 => (0, 0),
                                                                        _ => (99, 99),
                                                                    },
                                                                };
                                                                if x == px && y == py {
                                                                    string.push('█');
                                                                    string.push('█');
                                                                } else {
                                                                    string.push(' ');
                                                                    string.push(' ');
                                                                }
                                                            }
                                                            string.push('\n')
                                                        }
                                                        let style = Style {
                                                            fg: Color::Green,
                                                            bg: Color::DarkGrey,
                                                            ..Default::default()
                                                        };
                                                        ui.label(StyledText::styled(string, style));

                                                        ui.add_space_primary_direction(1);
                                                    }
                                                });

                                                ui.label("Axis");
                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        for a in 0..joy.axis_len() {
                                                            ui.horizontal(|ui| {
                                                                ui.label(format!(" {}:", a));
                                                                let val =
                                                                    joy.get_axis(a).unwrap_or(0)
                                                                        as i32
                                                                        + 128i32;
                                                                let val = val as f32 / 255.0;
                                                                let style = Style {
                                                                    fg: Color::Green,
                                                                    bg: Color::DarkGrey,
                                                                    ..Default::default()
                                                                };
                                                                ui.progress_bar(
                                                                    style,
                                                                    8,
                                                                    8,
                                                                    1,
                                                                    etui::Layout::TopLeftHorizontal,
                                                                    val,
                                                                )
                                                                // ui.label(format!("{}", val))
                                                            });
                                                        }
                                                    });
                                                    ui.add_space_primary_direction(1);
                                                    ui.vertical(|ui| {
                                                        let val = joy.get_axis(0).unwrap_or(0);
                                                        let val = val as i32;
                                                        let val = val + 128;

                                                        let val = val as f32 / 255.0;
                                                        let style = Style {
                                                            fg: Color::Green,
                                                            bg: Color::DarkGrey,
                                                            ..Default::default()
                                                        };
                                                        ui.progress_bar(
                                                            style,
                                                            6,
                                                            6,
                                                            2,
                                                            etui::Layout::TopLeftVertical,
                                                            val,
                                                        )
                                                    });
                                                });
                                            } else {
                                                ui.label("None")
                                            }
                                        });
                                    },
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
                            drop_down(ui, "4")
                        });

                        ui.layout(BottomLeftVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("BottomLeft\nVertical");
                                ui.label("BottomLeftVertical");
                            });
                            drop_down(ui, "3")
                        });

                        ui.set_max(max);

                        ui.layout(TopRightVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("TopRight\nVertical");
                                ui.label("TopRightVertical");
                            });
                            drop_down(ui, "2")
                        });

                        ui.set_max(max);

                        ui.layout(BottomRightVertical, |ui| {
                            ui.bordered(|ui| {
                                ui.label("BottomRight\nVertical");
                                ui.label("BottomRightVertical");
                            });
                            drop_down(ui, "1")
                        });
                    }
                });
            });
        },
    );
}

fn drop_down(ui: &mut etui::Ui, title: &str) {
    ui.drop_down(title, |ui| {
        ui.label("Bruh");
        if ui.button("bruh").pressed() {
            ui.label("asdasd")
        }
    });
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
