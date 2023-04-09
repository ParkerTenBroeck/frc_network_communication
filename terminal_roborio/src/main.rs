use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    style::StyledContent,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use etui::{Context, StyledText};
use roborio::RoborioCom;
use std::{
    io::{self, Stdout, Write},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::app::App;

mod app;
pub mod etui;

fn main() -> Result<(), io::Error> {
    let driverstation = Arc::new(RoborioCom::default());
    RoborioCom::start_daemon(driverstation.clone());

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

    // driverstation
    driverstation.observe_robot_code(true);

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    execute!(stdout, crossterm::cursor::Hide)?;

    let app = App::new(driverstation);
    let res = run_app(&mut stdout, app, std::time::Duration::from_millis(16));

    // restore terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    execute!(stdout, crossterm::cursor::Show)?;

    drop(child);

    res
}

fn run_app(stdout: &mut Stdout, mut app: App, tick_rate: Duration) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let mut ctx = Context::default();

    loop {
        app.ui(&ctx);

        stdout.queue(crossterm::terminal::Clear(
            crossterm::terminal::ClearType::All,
        ))?;
        let mut draws = Vec::new();
        ctx.take_draw_commands(&mut draws);
        for items in draws {
            match items {
                etui::Draw::ClearAll(_) => todo!(),
                etui::Draw::Clear(_, _) => todo!(),
                etui::Draw::Text(text, start) => {
                    let StyledText { text, style } = text;

                    io::stdout().queue(crossterm::cursor::MoveTo(start.x, start.y))?;
                    io::stdout().queue(crossterm::style::PrintStyledContent(
                        StyledContent::new(
                            crossterm::style::ContentStyle {
                                foreground_color: Some(style.fg),
                                background_color: Some(style.bg),
                                underline_color: Some(style.fg),
                                attributes: style.attributes,
                            },
                            text.as_str(),
                        ),
                    ))?;
                }
            }
        }

        stdout.flush()?;

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
            ctx.new_event(event);
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
