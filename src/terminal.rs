/* //! Demonstrates how to block read events.
//!
//! cargo run --example event-read

use std::io::stdout;

use crossterm::event::{poll, KeyEvent, KeyModifiers};
use crossterm::ExecutableCommand;
use crossterm::{
    cursor::position,
    event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
    Result,
};
use std::time::Duration;

const HELP: &str = r#"Blocking read()
 - Keyboard, mouse and terminal resize events enabled
 - Hit "c" to print current cursor position
 - Use Esc to quit
"#;

struct Application {
    musics: Vec<UIMusic>,
    app_status: AppStatus,
    current_time: u32,
    total_time: u32,
    volume: f32,
}
*/
pub struct UIMusic {
    pub status: MusicStatus,
    pub title: String,
    pub author: String,
}

impl UIMusic {
    pub fn new(video: &Video, status: MusicStatus) -> UIMusic {
        UIMusic {
            status,
            title: video.title.clone(),
            author: video.author.clone(),
        }
    }
}

impl UIMusic {
    fn text(&self) -> String {
        format!(
            " {} {} | {}",
            self.status.character(),
            self.author,
            self.title
        )
    }
}

#[derive(PartialEq)]
pub enum AppStatus {
    Paused,
    Playing,
    NoMusic,
}

impl AppStatus {
    pub fn new(sink: &Player) -> Self {
        if sink.is_finished() {
            Self::NoMusic
        } else if sink.is_paused() {
            Self::Paused
        } else {
            Self::Playing
        }
    }
}

impl AppStatus {
    fn colors(&self) -> (Color, Color) {
        match self {
            AppStatus::Paused => (Color::Yellow, Color::Black),
            AppStatus::Playing => (Color::Green, Color::Black),
            AppStatus::NoMusic => (Color::White, Color::Black),
        }
    }
}

#[derive(PartialEq)]
pub enum MusicStatus {
    Playing,
    Paused,
    Previous,
    Next,
    Downloading,
}

impl MusicStatus {
    fn character(&self) -> char {
        match self {
            MusicStatus::Playing => '▶',
            MusicStatus::Paused => '⋈',
            MusicStatus::Previous => ' ',
            MusicStatus::Next => ' ',
            MusicStatus::Downloading => '⮁',
        }
    }

    fn colors(&self) -> (Color, Color) {
        match self {
            MusicStatus::Playing => (Color::Green, Color::Black),
            MusicStatus::Paused => (Color::Yellow, Color::Black),
            MusicStatus::Previous => (Color::White, Color::Black),
            MusicStatus::Next => (Color::White, Color::Black),
            MusicStatus::Downloading => (Color::Blue, Color::Black),
        }
    }
}
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use flume::{Receiver, Sender};
use player::Player;
use std::{
    error::Error,
    io,
    sync::Arc,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState},
    Frame, Terminal,
};
use ytpapi::Video;

use crate::SoundAction;

pub struct App {
    pub musics: Vec<UIMusic>,
    pub app_status: AppStatus,
    pub current_time: u32,
    pub total_time: u32,
    pub volume: f32,
}

impl App {
    pub fn new(sink: &Player, musics: Vec<UIMusic>) -> Self {
        let app_status = AppStatus::new(sink);
        let current_time = sink.elapsed().as_secs() as u32;
        let total_time = sink.duration().map(|x| x as u32).unwrap_or(1);
        let volume = sink.volume_percent() as f32 / 100.;
        App {
            musics,
            app_status,
            current_time,
            total_time,
            volume,
        }
    }
}

impl App {}

pub fn main(
    app_updater: flume::Receiver<App>,
    action_sender: Arc<Sender<SoundAction>>,
) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App {
        musics: vec![],
        app_status: AppStatus::NoMusic,
        current_time: 0,
        total_time: 0,
        volume: 0.5,
    };
    let res = run_app(action_sender, app_updater, &mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    action_sender: Arc<Sender<SoundAction>>,
    updater: Receiver<App>,
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        while let Ok(e) = updater.try_recv() {
            app = e;
        }
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if KeyCode::Esc == key.code
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    return Ok(());
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('<') | KeyCode::Left => {
                            action_sender.send(SoundAction::Previous).unwrap();
                        }
                        KeyCode::Char('>') | KeyCode::Right => {
                            action_sender.send(SoundAction::Next).unwrap();
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char(' ') => {
                            action_sender.send(SoundAction::PlayPause).unwrap();
                        }
                        KeyCode::Char('<') | KeyCode::Left => {
                            action_sender.send(SoundAction::Backward).unwrap();
                        }
                        KeyCode::Char('>') | KeyCode::Right => {
                            action_sender.send(SoundAction::Forward).unwrap();
                        }
                        KeyCode::Char('+') | KeyCode::Up => {
                            action_sender.send(SoundAction::Plus).unwrap();
                        }
                        KeyCode::Char('-') | KeyCode::Down => {
                            action_sender.send(SoundAction::Minus).unwrap();
                        }
                        e => {
                            std::fs::write("log.txt", format!("{:?}", e))?;
                        }
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn split_y(f: Rect, end_size: u16) -> [Rect; 2] {
    let mut rectlistvol = f;
    rectlistvol.height -= end_size;
    let mut rectprogress = f;
    rectprogress.y += rectprogress.height - end_size;
    rectprogress.height = end_size;
    [rectlistvol, rectprogress]
}
fn split_x(f: Rect, end_size: u16) -> [Rect; 2] {
    let mut rectlistvol = f;
    rectlistvol.width -= end_size;
    let mut rectprogress = f;
    rectprogress.x +=  rectprogress.width - end_size;
    rectprogress.width = end_size;
    [rectlistvol, rectprogress]
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let [top_rect, progress_rect] = split_y(f.size(), 3);
    let [list_rect, volume_rect] = split_x(top_rect, 10);

    let label = format!(
        "{}:{:02} / {}:{:02}",
        app.current_time / 60,
        app.current_time % 60,
        app.total_time / 60,
        app.total_time % 60
    );
    let colors = app.app_status.colors();
    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(
                    app.musics
                        .iter()
                        .find(|x| {
                            x.status == MusicStatus::Playing || x.status == MusicStatus::Paused
                        })
                        .map(|x| format!(" {} | {} ", x.author, x.title))
                        .unwrap_or_default(),
                )
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(colors.0).bg(colors.1))
        .ratio(
            if app.app_status == AppStatus::NoMusic {
                0.5
            } else {
                app.current_time as f64 / app.total_time as f64
            }
            .min(1.0)
            .max(0.0),
        )
        .label(label);

    let gauge1 = Gauge::default()
        .block(Block::default().title(" Volume ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(colors.0).bg(colors.1))
        .ratio((app.volume as f64).min(1.0).max(0.0));
    f.render_widget(gauge1, volume_rect);
    f.render_widget(gauge, progress_rect);
    let items: Vec<ListItem> = app
        .musics
        .iter()
        .map(|i| {
            ListItem::new(i.text()).style(
                Style::default()
                    .fg(i.status.colors().0)
                    .bg(i.status.colors().1),
            )
        })
        .collect();
    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items).block(Block::default().borders(Borders::ALL).title(" Playlist "));
    f.render_stateful_widget(items, list_rect, &mut ListState::default());
}
