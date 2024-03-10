use std::{
    io::{self, stdout},
    time::Duration,
};

use tokio::{sync::mpsc::{
    error::TryRecvError,
    UnboundedReceiver, UnboundedSender,
}, time::sleep};

use crossterm::{
    event::{self, KeyCode},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen
    },
    ExecutableCommand,
};

use ratatui::{
    layout::*,
    prelude::{
        Backend, 
        Buffer, 
        CrosstermBackend, 
        Rect, 
        Terminal
    },
    symbols::border,
    text::Line,
    widgets::{block::*, *},
};

use super::parser::Command;
use crate::common::message::{Request, Response};

type Source = UnboundedReceiver<Response>;
type Sink = UnboundedSender<Request>;

#[derive(Debug)]
struct App {
    messages: Vec<String>,
    input: String,
    quit: bool,
    source: Source,
    sink: Sink,
}

const EVENT_TIMEOUT: Duration = Duration::from_millis(10);

impl App {
    fn new(source: Source, sink: Sink) -> Self {
        let messages = Vec::<String>::new();
        let input = String::new();
        let quit = false;

        App {
            messages,
            input,
            quit,
            source,
            sink,
        }
    }

    async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        while !self.quit {
            self.handle_events().await?;
            self.check_for_messages()?;

            sleep(Duration::from_millis(10)).await;

            terminal.draw(|frame| frame.render_widget(
                &*self, 
                frame.size()
            ))?;
        }

        Ok(())
    }

    async fn handle_events(&mut self) -> io::Result<()> {
        if !async { event::poll(EVENT_TIMEOUT) }.await? {
            return Ok(());
        }

        if let event::Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                return Ok(());
            }

            match key.code {
                KeyCode::Char(ch) => {
                    self.input.push(ch);
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Enter => {
                    let input = self.input.clone();
                    self.input.clear();

                    if let Some(command) = Command::from(&input) {
                        if command == Command::Quit {
                            self.quit = true;
                        }

                        self.send(command.to_request())?;
                    }
                }
                _ => {
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    fn send(&mut self, request: Request) -> io::Result<()> {
        self.sink
            .send(request)
            .map_err(|reason| io::Error::new(
                io::ErrorKind::BrokenPipe,
                reason
            ))
    }

    fn check_for_messages(&mut self) -> io::Result<()> {
        match self.source.try_recv() {
            Ok(message) => {
                self.messages.push(message.to_string());
            }
            Err(TryRecvError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "Unexpected disconnect",
                ))
            }
            Err(TryRecvError::Empty) => { }
        }

        Ok(())
    }
}

fn chat_history<'a>(messages: &Vec<String>) -> Paragraph<'a> {
    let messages = messages
        .iter()
        .map(|msg| Line::from(msg.to_owned()))
        .collect::<Vec<_>>();

    Paragraph::new(messages)
        .wrap(Wrap { trim: true })
        .left_aligned()
}

fn input_view<'a>(text: &str) -> Title<'a> {
    let prompt = format!("Message: {text}");

    Title::from(Line::from(prompt))
        .alignment(Alignment::Center)
        .position(block::Position::Bottom)
}

fn compose<'a>(title: Title<'a>, input: Title<'a>, history: Paragraph<'a>) -> Paragraph<'a> {
    let block = Block::default()
        .title(title)
        .title(input)
        .borders(Borders::ALL)
        .border_set(border::ROUNDED);

    history.block(block)
}

impl Widget for &App {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let title = Title::from("Chat")
            .alignment(Alignment::Center)
            .position(block::Position::Top);

        let input = input_view(&self.input);
        let history = chat_history(&self.messages);

        compose(title, input, history)
            .render(area, buffer);
    }
}

pub async fn run(source: Source, sink: Sink) -> io::Result<()> {
    enable_raw_mode()?;

    stdout()
        .execute(Clear(ClearType::All))?
        .execute(EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = App::new(source, sink);

    app.run(&mut terminal).await?;

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
