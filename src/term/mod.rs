pub mod device_lost;
pub mod item_list;
pub mod list_selector;
pub mod music_player;
pub mod playlist;
pub mod playlist_view;
pub mod search;
pub mod vertical_gauge;

use std::{
    io::{self},
    sync::Arc,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyEvent, KeyModifiers, MouseEvent,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use flume::{Receiver, Sender};
use ratatui::{backend::CrosstermBackend, layout::Rect, Frame, Terminal};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{structures::sound_action::SoundAction, systems::player::PlayerState, SIGNALING_STOP};

use self::{device_lost::DeviceLost, item_list::ListItem, playlist::Chooser, search::Search};

use crate::term::playlist_view::PlaylistView;

// A trait to handle the different screens
pub trait Screen {
    fn on_mouse_press(&mut self, mouse_event: MouseEvent, frame_data: &Rect) -> EventResponse;
    fn on_key_press(&mut self, mouse_event: KeyEvent, frame_data: &Rect) -> EventResponse;
    fn render(&mut self, frame: &mut Frame);
    fn handle_global_message(&mut self, message: ManagerMessage) -> EventResponse;
    fn close(&mut self, new_screen: Screens) -> EventResponse;
    fn open(&mut self) -> EventResponse;
}

#[derive(Debug, Clone)]
pub enum EventResponse {
    Message(Vec<ManagerMessage>),
    None,
}

// A message that can be sent to the manager
#[derive(Debug, Clone)]
pub enum ManagerMessage {
    Error(String, Box<Option<ManagerMessage>>),
    PassTo(Screens, Box<ManagerMessage>),
    Inspect(String, Screens, Vec<YoutubeMusicVideoRef>),
    ChangeState(Screens),
    SearchFrom(Screens),
    PlayerFrom(Screens),
    #[allow(dead_code)]
    PlaylistFrom(Screens),
    RestartPlayer,
    Quit,
    AddElementToChooser((String, Vec<YoutubeMusicVideoRef>)),
}

impl ManagerMessage {
    pub fn pass_to(self, screen: Screens) -> Self {
        Self::PassTo(screen, Box::new(self))
    }
    pub fn event(self) -> EventResponse {
        EventResponse::Message(vec![self])
    }
}

// The different screens
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Screens {
    MusicPlayer = 0x0,
    Playlist = 0x1,
    Search = 0x2,
    DeviceLost = 0x3,
    PlaylistViewer = 0x4,
}

// The screen manager that handles the different screens
pub struct Manager {
    music_player: PlayerState,
    chooser: Chooser,
    search: Search,
    device_lost: DeviceLost,
    current_screen: Screens,
    playlist_viewer: PlaylistView,
}

impl Manager {
    pub async fn new(action_sender: Arc<Sender<SoundAction>>, music_player: PlayerState) -> Self {
        Self {
            music_player,
            chooser: Chooser {
                action_sender: action_sender.clone(),
                goto: Screens::MusicPlayer,
                item_list: ListItem::new(" Choose a playlist ".to_owned()),
            },
            playlist_viewer: PlaylistView {
                sender: action_sender.clone(),
                items: ListItem::new(" Playlist ".to_owned()),
                goto: Screens::Playlist,
                videos: Vec::new(),
            },
            search: Search::new(action_sender).await,
            current_screen: Screens::Playlist,
            device_lost: DeviceLost(Vec::new(), None),
        }
    }
    pub fn current_screen(&mut self) -> &mut dyn Screen {
        self.get_screen(self.current_screen)
    }
    pub fn get_screen(&mut self, screen: Screens) -> &mut dyn Screen {
        match screen {
            Screens::MusicPlayer => &mut self.music_player,
            Screens::Playlist => &mut self.chooser,
            Screens::Search => &mut self.search,
            Screens::DeviceLost => &mut self.device_lost,
            Screens::PlaylistViewer => &mut self.playlist_viewer,
        }
    }
    pub fn set_current_screen(&mut self, screen: Screens) {
        self.current_screen = screen;
        let k = self.current_screen().open();
        self.handle_event(k);
    }
    pub fn handle_event(&mut self, event: EventResponse) -> bool {
        match event {
            EventResponse::Message(messages) => {
                for message in messages {
                    if self.handle_manager_message(message) {
                        return true;
                    }
                }
            }
            EventResponse::None => {}
        }
        false
    }
    pub fn handle_manager_message(&mut self, e: ManagerMessage) -> bool {
        match e {
            ManagerMessage::PassTo(e, a) => {
                let rs = self.get_screen(e).handle_global_message(*a);
                self.handle_event(rs);
            }
            ManagerMessage::Quit => {
                let c = self.current_screen;
                self.current_screen().close(c);
                return true;
            }
            ManagerMessage::ChangeState(e) => {
                self.current_screen().close(e);
                self.set_current_screen(e);
            }
            ManagerMessage::SearchFrom(e) => {
                self.current_screen().close(Screens::Search);
                self.search.goto = e;
                self.set_current_screen(Screens::Search);
            }
            ManagerMessage::PlayerFrom(e) => {
                self.current_screen().close(Screens::MusicPlayer);
                self.music_player.goto = e;
                self.set_current_screen(Screens::MusicPlayer);
            }
            ManagerMessage::PlaylistFrom(e) => {
                self.current_screen().close(Screens::Playlist);
                self.chooser.goto = e;
                self.set_current_screen(Screens::Playlist);
            }
            e => {
                return self.handle_manager_message(ManagerMessage::PassTo(
                    Screens::DeviceLost,
                    Box::new(ManagerMessage::Error(
                        format!(
                        "Invalid manager message (Forward the message to a screen maybe):\n{e:?}"
                    ),
                        Box::new(None),
                    )),
                ));
            }
        }
        false
    }

    /// The main loop of the manager
    pub fn run(&mut self, updater: &Receiver<ManagerMessage>) -> Result<(), io::Error> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // create app and run it
        let tick_rate = Duration::from_millis(250);

        let mut last_tick = Instant::now();
        'a: loop {
            if matches!(SIGNALING_STOP.1.try_recv(), Ok(())) {
                break;
            }
            while let Ok(e) = updater.try_recv() {
                if self.handle_manager_message(e) {
                    break 'a;
                }
            }
            let rectsize = terminal.size()?;
            terminal.draw(|f| {
                self.music_player.update();
                self.current_screen().render(f);
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if crossterm::event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        if (key.code == event::KeyCode::Char('c')
                            || key.code == event::KeyCode::Char('d'))
                            && key.modifiers == KeyModifiers::CONTROL
                        {
                            break;
                        }
                        let k = self.current_screen().on_key_press(key, &rectsize);
                        if self.handle_event(k) {
                            break;
                        }
                    }
                    Event::Mouse(mouse) => {
                        let k = self.current_screen().on_mouse_press(mouse, &rectsize);
                        if self.handle_event(k) {
                            break;
                        }
                    }
                    _ => (),
                }
            }
            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }
}

// UTILS SECTION TO SPLIT THE TERMINAL INTO DIFFERENT PARTS

pub fn split_y_start(f: Rect, start_size: u16) -> [Rect; 2] {
    let mut rectlistvol = f;
    rectlistvol.height = start_size;
    let mut rectprogress = f;
    rectprogress.y += start_size;
    rectprogress.height = rectprogress.height.saturating_sub(start_size);
    [rectlistvol, rectprogress]
}
pub fn split_y(f: Rect, end_size: u16) -> [Rect; 2] {
    let mut rectlistvol = f;
    rectlistvol.height = rectlistvol.height.saturating_sub(end_size);
    let mut rectprogress = f;
    rectprogress.y += rectprogress.height.saturating_sub(end_size);
    rectprogress.height = end_size;
    [rectlistvol, rectprogress]
}
pub fn split_x(f: Rect, end_size: u16) -> [Rect; 2] {
    let mut rectlistvol = f;
    rectlistvol.width = rectlistvol.width.saturating_sub(end_size);
    let mut rectprogress = f;
    rectprogress.x += rectprogress.width.saturating_sub(end_size);
    rectprogress.width = end_size;
    [rectlistvol, rectprogress]
}

pub fn rect_contains(rect: &Rect, x: u16, y: u16, margin: u16) -> bool {
    rect.x + margin <= x
        && x <= rect.x + rect.width.saturating_sub(margin)
        && rect.y + margin <= y
        && y <= rect.y + rect.height.saturating_sub(margin)
}

pub fn relative_pos(rect: &Rect, x: u16, y: u16, margin: u16) -> (u16, u16) {
    (
        x.saturating_sub(rect.x + margin),
        y.saturating_sub(rect.y + margin),
    )
}
