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
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
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

pub enum AppMessage {
    UpdateApp(App),
    AddElementToChooser((String, Vec<Video>)),
}

enum View {
    App,
    Chooser,
}
pub struct Chooser {
    pub selected: usize,
    pub items: Vec<(String, Vec<Video>)>,
}
impl Chooser {
    fn render<B: Backend>(&self, f: &mut Frame<B>) {
        f.render_stateful_widget(
            List::new(
                self.items
                    .iter()
                    .enumerate()
                    .skip(self.selected.saturating_sub(1))
                    .map(|(index, i)| {
                        ListItem::new(i.0.as_str()).style(
                            Style::default()
                                .fg(if index == self.selected {
                                    Color::Black
                                } else {
                                    Color::White
                                })
                                .bg(if index != self.selected {
                                    Color::Black
                                } else {
                                    Color::White
                                }),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Select the playlist to play "),
            ),
            f.size(),
            &mut ListState::default(),
        );
    }
    fn selected(&mut self, selected: isize) {
        if selected < 0 {
            self.selected = self.items.len() - 1;
        } else if selected >= self.items.len() as isize {
            self.selected = 0;
        } else {
            self.selected = selected as usize;
        }
    }
    fn add_element(&mut self, element: (String, Vec<Video>)) {
        self.items.push(element);
    }
    fn keyboard_input(&mut self, key: &KeyEvent, sender: &Sender<Video>) -> (View, bool) {
        if KeyCode::Esc == key.code
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            return (View::Chooser, true);
        }
        match key.code {
            KeyCode::Enter => {
                for video in self.items.get(self.selected).unwrap().1.iter() {
                    sender.send(video.clone()).unwrap();
                }
                return (View::App, false);
            }
            KeyCode::Char('+') | KeyCode::Up => self.selected(self.selected as isize - 1),
            KeyCode::Char('-') | KeyCode::Down => self.selected(self.selected as isize + 1),
            _ => {}
        }

        return (View::Chooser, false);
    }
}
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

    fn keyboard_input(&self, key: &KeyEvent, sender: &Sender<SoundAction>) -> bool {
        if KeyCode::Esc == key.code
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            return true;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('<') | KeyCode::Left => {
                    sender.send(SoundAction::Previous).unwrap();
                }
                KeyCode::Char('>') | KeyCode::Right => {
                    sender.send(SoundAction::Next).unwrap();
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char(' ') => {
                    sender.send(SoundAction::PlayPause).unwrap();
                }
                KeyCode::Char('<') | KeyCode::Left => {
                    sender.send(SoundAction::Backward).unwrap();
                }
                KeyCode::Char('>') | KeyCode::Right => {
                    sender.send(SoundAction::Forward).unwrap();
                }
                KeyCode::Char('+') | KeyCode::Up => {
                    sender.send(SoundAction::Plus).unwrap();
                }
                KeyCode::Char('-') | KeyCode::Down => {
                    sender.send(SoundAction::Minus).unwrap();
                }
                _ => {}
            }
        }
        return false;
    }
    fn render<B: Backend>(&self, f: &mut Frame<B>) {
        let [top_rect, progress_rect] = split_y(f.size(), 3);
        let [list_rect, volume_rect] = split_x(top_rect, 10);
        let colors = self.app_status.colors();
        f.render_widget(
            Gauge::default()
                .block(Block::default().title(" Volume ").borders(Borders::ALL))
                .gauge_style(Style::default().fg(colors.0).bg(colors.1))
                .ratio((self.volume as f64).min(1.0).max(0.0)),
            volume_rect,
        );
        f.render_widget(
            Gauge::default()
                .block(
                    Block::default()
                        .title(
                            self.musics
                                .iter()
                                .find(|x| {
                                    x.status == MusicStatus::Playing
                                        || x.status == MusicStatus::Paused
                                })
                                .map(|x| format!(" {} | {} ", x.author, x.title))
                                .unwrap_or_else(|| " No music playing ".to_owned()),
                        )
                        .borders(Borders::ALL),
                )
                .gauge_style(Style::default().fg(colors.0).bg(colors.1))
                .ratio(
                    if self.app_status == AppStatus::NoMusic {
                        0.5
                    } else {
                        self.current_time as f64 / self.total_time as f64
                    }
                    .min(1.0)
                    .max(0.0),
                )
                .label(format!(
                    "{}:{:02} / {}:{:02}",
                    self.current_time / 60,
                    self.current_time % 60,
                    self.total_time / 60,
                    self.total_time % 60
                )),
            progress_rect,
        );
        // Create a List from all list items and highlight the currently selected one
        f.render_stateful_widget(
            List::new(
                self.musics
                    .iter()
                    .map(|i| {
                        ListItem::new(i.text()).style(
                            Style::default()
                                .fg(i.status.colors().0)
                                .bg(i.status.colors().1),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title(" Playlist ")),
            list_rect,
            &mut ListState::default(),
        );
    }
}

pub fn main(
    video_sender: Arc<Sender<Video>>,
    app_updater: flume::Receiver<AppMessage>,
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
    let res = run_app(
        action_sender,
        video_sender,
        app_updater,
        &mut terminal,
        app,
        tick_rate,
    );

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
    video_sender: Arc<Sender<Video>>,
    updater: Receiver<AppMessage>,
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut chooser = Chooser {
        selected: 0,
        items: Vec::new(),
    };
    let mut view = View::Chooser;
    let mut last_tick = Instant::now();
    loop {
        while let Ok(e) = updater.try_recv() {
            match e {
                AppMessage::UpdateApp(e) => {
                    app = e;
                }
                AppMessage::AddElementToChooser(e) => {
                    chooser.add_element(e);
                }
            }
        }
        match &view {
            View::App => {
                terminal.draw(|f| app.render(f))?;
            }
            View::Chooser => {
                terminal.draw(|f| chooser.render(f))?;
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match &view {
                    View::App => {
                        if app.keyboard_input(&key, &action_sender) {
                            return Ok(());
                        }
                    }
                    View::Chooser => {
                        let (a, b) = chooser.keyboard_input(&key, &video_sender);
                        view = a;
                        if b {
                            return Ok(());
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
    rectprogress.x += rectprogress.width - end_size;
    rectprogress.width = end_size;
    [rectlistvol, rectprogress]
}
