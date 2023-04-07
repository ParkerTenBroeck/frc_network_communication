use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use roborio::RoborioCom;
use std::{io, sync::Arc, thread, time::Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Widget},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

fn main() -> Result<(), io::Error> {
    let driverstation = Arc::new(RoborioCom::default());
    RoborioCom::start_daemon(driverstation.clone());
    let child = std::process::Command::new("avahi-publish-service")
        .args(["roboRIO-1114-FRC", "_ni-rt._tcp", "1110", "\"ROBORIO\""])
        // .stdout(std::process::Stdio::null())
        // .stderr(std::process::Stdio::null())
        // .stdin(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let child = KillOnDrop(child);
    struct KillOnDrop(std::process::Child);
    impl Drop for KillOnDrop{
        fn drop(&mut self) {
            _ = self.0.kill();
        }
    }

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(driverstation);
    let res = app.run_app(&mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    drop(child);

    res
}

enum InputMode {
    Normal,
    Editing,
}

enum SelectedTab {
    Common,
    Udp,
    Tcp,
}

struct App {
    driverstation: Arc<RoborioCom>,
    tab: SelectedTab,
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
}

struct Common {}

impl Common {
    pub fn ui<B: Backend>(&self, f: &mut Frame<B>) {}
}

struct Udp {}

impl Udp {
    pub fn ui<B: Backend>(&self, f: &mut Frame<B>) {}
}

struct Tcp {}

impl Tcp {
    pub fn ui<B: Backend>(&self, f: &mut Frame<B>) {}
}

impl App {
    pub fn new(driverstation: Arc<RoborioCom>) -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            tab: SelectedTab::Common,
            driverstation,
        }
    }

    pub fn run_app<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            let event = event::read()?;

            if let Event::Key(key) = event {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            self.messages.push(self.input.drain(..).collect());
                        }
                        KeyCode::Char(c) => {
                            self.input.push(c);
                        }
                        KeyCode::Backspace => {
                            self.input.pop();
                        }
                        KeyCode::Esc => {
                            self.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    pub fn ui<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(f.size());

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    Span::raw("Press "),
                    Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to exit, "),
                    Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to start editing."),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    Span::raw("Press "),
                    Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to stop editing, "),
                    Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to record the message"),
                ],
                Style::default(),
            ),
        };
        let mut text = Text::from(Spans::from(msg));
        text.patch_style(style);
        let help_message = Paragraph::new(text);
        f.render_widget(help_message, chunks[0]);

        let input = Paragraph::new(self.input.as_ref())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[1]);
        match self.input_mode {
            InputMode::Normal =>
                // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                {}

            InputMode::Editing => {
                // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
                f.set_cursor(
                    // Put cursor past the end of the input text
                    chunks[1].x + self.input.width() as u16 + 1,
                    // Move one line down, from the border to the input line
                    chunks[1].y + 1,
                )
            }
        }

        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
                ListItem::new(content)
            })
            .collect();
        let messages =
            List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));
        f.render_widget(messages, chunks[2]);
    }
}
