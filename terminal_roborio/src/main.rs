use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    style::{Attribute},
    terminal::{
        disable_raw_mode, enable_raw_mode, DisableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    QueueableCommand,
};
use etui::{Context, StyledText, math_util::{VecI2, Rect}};
use roborio::RoborioCom;
use std::{
    io::{self, Stdout, Write},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::app::App;

mod app;
pub mod etui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Message,
    Warning,
    Error,
}

#[derive(Debug, Default)]
pub struct Log {
    log: std::sync::Mutex<Vec<(LogLevel, String)>>,
}

impl Log {
    pub fn message(&self, msg: impl Into<String>) {
        self.log(msg, LogLevel::Message)
    }
    pub fn warning(&self, msg: impl Into<String>) {
        self.log(msg, LogLevel::Warning)
    }
    pub fn error(&self, msg: impl Into<String>) {
        self.log(msg, LogLevel::Error)
    }
    pub fn log(&self, msg: impl Into<String>, level: LogLevel) {
        self.log.lock().unwrap().push((level, msg.into()))
    }
    pub fn get_last_n(&self, amount: usize) -> Vec<(LogLevel, String)> {
        let mut bruh = Vec::with_capacity(amount);
        let lock = self.log.lock().unwrap();
        let start = lock.len().saturating_sub(amount);
        for log in &lock[start..] {
            bruh.push(log.clone())
        }
        bruh
    }
}

fn main() -> Result<(), io::Error> {
    let driverstation = Arc::new(RoborioCom::default());
    let log = Arc::new(Log::default());
    let send = log.clone();
    _ = driverstation.set_error_handler(move |_com, error| send.error(format!("{:?}", error)));
    let send = log.clone();
    _ = driverstation.set_restart_rio_hook(move || {
        send.message("Hook: Restart Rio");
    });
    let send = log.clone();
    _ = driverstation.set_restart_code_hook(move || {
        send.message("Hook: Restart Rio Code");
    });
    let send = log.clone();
    _ = driverstation.set_estop_hook(move || {
        send.message("Hook: Estop");
    });
    let send = log.clone();
    _ = driverstation.set_disable_hook(move || {
        send.message("Hook: Disable");
    });
    let send = log.clone();
    _ = driverstation.set_teleop_hook(move || {
        send.message("Hook: Teleop");
    });
    let send = log.clone();
    _ = driverstation.set_auton_hook(move || {
        send.message("Hook: Auton");
    });
    let send = log.clone();
    _ = driverstation.set_test_hook(move || {
        send.message("Hook: Test");
    });
    RoborioCom::start_daemon(driverstation.clone());
    // driverstation

    let child = std::process::Command::new("avahi-publish-service")
        .args(["roboRIO-1114-FRC", "_ni-rt._tcp", "1110", "\"ROBORIO\""])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let child = KillOnDrop(child);
    struct KillOnDrop(std::process::Child);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            _ = self.0.kill();
        }
    }

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    execute!(stdout, DisableLineWrap)?;
    execute!(stdout, crossterm::cursor::Hide)?;

    let app = App::new(driverstation, log);
    let res = run_app(&mut stdout, app, std::time::Duration::from_millis(33));

    // restore terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    execute!(stdout, crossterm::cursor::Show)?;

    drop(child);

    res
}

fn run_app(stdout: &mut Stdout, mut app: App, tick_rate: Duration) -> io::Result<()> {
    let mut last_tick = Instant::now();

    let (x, y) = crossterm::terminal::size()?;

    let mut ctx = Context::new(Rect::new_pos_size(VecI2::new(0, 0), VecI2::new(x, y)));

    let mut data: Vec<u8> = Vec::new();
    loop {
        data.clear();
        app.ui(&ctx);

        data.queue(crossterm::terminal::Clear(
            crossterm::terminal::ClearType::All,
        ))?;

        let mut draws = Vec::new();
        ctx.take_draw_commands(&mut draws);

        let mut last_fg = None;
        let mut last_bg = None;
        let mut last_attr = None;
        let mut last_position = None;

        for items in draws {
            match items {
                etui::Draw::ClearAll(_) => todo!(),
                etui::Draw::Clear(_, _) => todo!(),
                etui::Draw::Text(text, start) => {
                    let StyledText { text, style } = text;

                    if last_position == Some(start) {
                        let mut next = start;
                        next.x += unicode_width::UnicodeWidthStr::width(text.as_str()) as u16;
                        last_position = Some(next)
                    } else {
                        if let Some(old) = last_position {
                            if old.x == start.x {
                                data.queue(crossterm::cursor::MoveToRow(start.y))?;
                            } else if old.y == start.y {
                                data.queue(crossterm::cursor::MoveToColumn(start.x))?;
                            } else {
                                data.queue(crossterm::cursor::MoveTo(start.x, start.y))?;
                            }
                        } else {
                            data.queue(crossterm::cursor::MoveTo(start.x, start.y))?;
                        }
                        let mut next = start;
                        next.x += unicode_width::UnicodeWidthStr::width(text.as_str()) as u16;
                        last_position = Some(next);
                        // continue;
                    }
                    if last_fg != Some(style.fg) {
                        data.queue(crossterm::style::SetForegroundColor(style.fg))?;
                        last_fg = Some(style.fg);
                    }
                    if last_bg != Some(style.bg) {
                        data.queue(crossterm::style::SetBackgroundColor(style.bg))?;
                        last_bg = Some(style.bg);
                    }

                    if last_attr != Some(style.attributes) {
                        let mut attr = style.attributes;
                        attr.set(Attribute::Reset);
                        data.queue(crossterm::style::SetAttributes(attr))?;
                        last_attr = Some(style.attributes);
                    }

                    data.queue(crossterm::style::Print(text))?;
                }
            }
        }

        stdout.write_all(&data)?;
        stdout.flush()?;
        println!("{:#?}", data.len());
        // let chars:Vec<char> =  data.iter().map(|c| (*c as char)).collect();
        // println!("{:?}",chars);

        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if crossterm::event::poll(timeout)? {
                let event = event::read()?;
                // if app.on_event(event) {
                //     return Ok(());
                // }
                if let Event::Key(key) = event {
                    if let event::KeyEvent {
                        code: KeyCode::Esc, ..
                    } = key
                    {
                        return Ok(());
                    }
                }
                ctx.handle_event(event);
            }
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
                break;
            }
        }
    }
}
