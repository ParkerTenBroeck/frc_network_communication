use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    style::{Attribute, Color},
    terminal::{
        disable_raw_mode, enable_raw_mode, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    QueueableCommand,
};
use etui::{
    math_util::{Rect, VecI2},
    screen::Screen,
    Context, Style, StyledText,
};
use roborio::RoborioCom;
use std::{
    io::{self, Stdout, Write},
    num::NonZeroU8,
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
    std::env::set_var("RUST_BACKTRACE", "1");

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

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut stdout = io::stdout();

        // restore terminal
        disable_raw_mode().unwrap();
        execute!(stdout, LeaveAlternateScreen, DisableMouseCapture).unwrap();
        execute!(stdout, EnableLineWrap).unwrap();
        execute!(stdout, crossterm::cursor::Show).unwrap();

        hook(info);

        // setup terminal
        enable_raw_mode().unwrap();
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
        execute!(stdout, DisableLineWrap).unwrap();
        execute!(stdout, crossterm::cursor::Hide).unwrap();
    }));

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    execute!(stdout, DisableLineWrap)?;
    execute!(stdout, crossterm::cursor::Hide)?;

    let app = App::new(driverstation, log);
    let res = run_app(stdout, app, std::time::Duration::from_millis(30));

    let mut stdout = io::stdout();
    // restore terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    execute!(stdout, EnableLineWrap)?;
    execute!(stdout, crossterm::cursor::Show)?;

    drop(child);

    match res {
        AppResult::Ok(ok) => Ok(ok),
        AppResult::Err(err) => Err(err),
        AppResult::Panic(panic) => std::panic::resume_unwind(panic),
    }
}

fn run_app(
    mut stdout: Stdout,
    mut app: App,
    tick_rate: Duration,
) -> AppResult<(), io::Error, Box<dyn std::any::Any + Send + 'static>> {
    let res: Result<io::Result<()>, _> = std::panic::catch_unwind(move || {
        let mut last_tick = Instant::now();

        let (x, y) = crossterm::terminal::size()?;

        let mut ctx = Context::new(Rect::new_pos_size(VecI2::new(0, 0), VecI2::new(x, y)));

        // let mut screen_test = Screen::default();
        // screen_test.resize(VecI2::new(x, y));

        let mut data: Vec<u8> = Vec::new();
        let mut last_len = 0;

        loop {
            data.clear();
            app.ui(&ctx, last_len);

            // data.queue(crossterm::terminal::Clear(
            //     crossterm::terminal::ClearType::All,
            // ))?;

            let mut lock = ctx.inner().write().unwrap();
            let (mut current_frame, mut last_frame) = lock.finish_frame();

            let mut last_fg = None;
            let mut last_bg = None;
            let mut last_attr = None;
            let mut last_position = None;

            let mut update_prev = true;
            let mut update_now = true;
            let mut curr_prev = None;
            let mut curr_new = None;
            loop {
                if update_now {
                    update_now = false;
                    curr_new = current_frame.next();
                }
                if update_prev {
                    update_prev = false;
                    curr_prev = last_frame.next();
                }

                let (text, style, pos) = match (curr_new, curr_prev) {
                    (Some(curr), Some(prev)) => {
                        // thats very convienient
                        if curr == prev {
                            update_prev = true;
                            update_now = true;
                            continue;
                        }

                        if curr.2 == prev.2 {
                            update_prev = true;
                            update_now = true;
                            // same position different text/style
                            curr
                        } else if curr.2.y == prev.2.y{
                            if curr.2.x > prev.2.x{
                                update_prev = true;
                                (" ", Style::default(), prev.2)
                            }else if curr.2.x < prev.2.x{
                                update_now = true;
                                curr
                            }else{
                                panic!()
                            }
                        } else if curr.2.y < prev.2.y{
                            update_now = true;
                            curr
                        }else if curr.2.y > prev.2.y{
                            update_prev = true;
                            (" ", Style::default(), prev.2)
                        }else{
                            panic!()
                        }
                    }
                    (None, None) => break,
                    (None, Some((_bruh, _style, pos))) => {
                        update_prev = true;
                        // idk if a single space will do cells with a larger character width might do somethin funky
                        (" ", Style::default(), pos)
                    }
                    (Some(to_draw), None) => {
                        update_now = true;
                        to_draw
                    }
                };

                if last_position == Some(pos) {
                    let mut next = pos;
                    next.x += unicode_width::UnicodeWidthStr::width(text) as u16;
                    last_position = Some(next)
                } else {
                    if let Some(old) = last_position {
                        if old.x == pos.x {
                            data.queue(crossterm::cursor::MoveToRow(pos.y))?;
                        } else if old.y == pos.y {
                            data.queue(crossterm::cursor::MoveToColumn(pos.x))?;
                        } else {
                            data.queue(crossterm::cursor::MoveTo(pos.x, pos.y))?;
                        }
                    } else {
                        data.queue(crossterm::cursor::MoveTo(pos.x, pos.y))?;
                    }
                    let mut next = pos;
                    next.x += unicode_width::UnicodeWidthStr::width(text) as u16;
                    last_position = Some(next);
                }

                //todo make this better
                if last_attr != Some(style.attributes) {
                    let mut attr = style.attributes;
                    attr.set(Attribute::Reset);
                    data.queue(crossterm::style::SetAttributes(attr))?;
                    last_attr = Some(style.attributes);
                    data.queue(crossterm::style::SetForegroundColor(style.fg))?;
                    last_fg = Some(style.fg);
                    data.queue(crossterm::style::SetBackgroundColor(style.bg))?;
                    last_bg = Some(style.bg);
                }

                if last_fg != Some(style.fg) {
                    data.queue(crossterm::style::SetForegroundColor(style.fg))?;
                    last_fg = Some(style.fg);
                }
                if last_bg != Some(style.bg) {
                    data.queue(crossterm::style::SetBackgroundColor(style.bg))?;
                    last_bg = Some(style.bg);
                }

                data.queue(crossterm::style::Print(text))?;
            }

            // while let Some((text, style, pos)) = current_frame.next() {
            //     if last_position == Some(pos) {
            //         let mut next = pos;
            //         next.x += unicode_width::UnicodeWidthStr::width(text) as u16;
            //         last_position = Some(next)
            //     } else {
            //         if let Some(old) = last_position {
            //             if old.x == pos.x {
            //                 data.queue(crossterm::cursor::MoveToRow(pos.y))?;
            //             } else if old.y == pos.y {
            //                 data.queue(crossterm::cursor::MoveToColumn(pos.x))?;
            //             } else {
            //                 data.queue(crossterm::cursor::MoveTo(pos.x, pos.y))?;
            //             }
            //         } else {
            //             data.queue(crossterm::cursor::MoveTo(pos.x, pos.y))?;
            //         }
            //         let mut next = pos;
            //         next.x += unicode_width::UnicodeWidthStr::width(text) as u16;
            //         last_position = Some(next);
            //     }

            //     //todo make this better
            //     if last_attr != Some(style.attributes) {
            //         let mut attr = style.attributes;
            //         attr.set(Attribute::Reset);
            //         data.queue(crossterm::style::SetAttributes(attr))?;
            //         last_attr = Some(style.attributes);
            //         data.queue(crossterm::style::SetForegroundColor(style.fg))?;
            //         last_fg = Some(style.fg);
            //         data.queue(crossterm::style::SetBackgroundColor(style.bg))?;
            //         last_bg = Some(style.bg);
            //     }

            //     if last_fg != Some(style.fg) {
            //         data.queue(crossterm::style::SetForegroundColor(style.fg))?;
            //         last_fg = Some(style.fg);
            //     }
            //     if last_bg != Some(style.bg) {
            //         data.queue(crossterm::style::SetBackgroundColor(style.bg))?;
            //         last_bg = Some(style.bg);
            //     }

            //     data.queue(crossterm::style::Print(text))?;
            // }
            drop((current_frame, last_frame));
            drop(lock);

            last_len = data.len();
            stdout.write_all(&data)?;
            stdout.flush()?;

            ctx.clear_event();
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if crossterm::event::poll(timeout)? {
                let event = event::read()?;

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
            last_tick = Instant::now();
        }
    });
    match res {
        Ok(Ok(ok)) => AppResult::Ok(ok),
        Ok(Err(err)) => AppResult::Err(err),
        Err(panic) => AppResult::Panic(panic),
    }
}

enum AppResult<O, E, P> {
    Ok(O),
    Err(E),
    Panic(P),
}
