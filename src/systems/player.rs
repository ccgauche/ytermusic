use std::{collections::VecDeque, path::PathBuf, str::FromStr, time::Duration};

use flume::Sender;
use player::Player;
use ytpapi::Video;

use crate::{
    terminal::{App, MusicStatus, UIMusic},
    SoundAction,
};

use super::download::IN_DOWNLOAD;

pub fn player_system(updater: Sender<App>) -> Sender<SoundAction> {
    let (tx, rx) = flume::unbounded::<SoundAction>();
    std::thread::spawn(move || {
        let (mut sink, guard) = Player::new();
        let mut queue: VecDeque<Video> = VecDeque::new();
        let mut previous: Vec<Video> = Vec::new();
        let mut current: Option<Video> = None;
        loop {
            updater
                .send(App::new(
                    &sink,
                    generate_music(&queue, &previous, &current, &sink),
                ))
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
            while let Ok(e) = rx.try_recv() {
                match e {
                    SoundAction::Backward => sink.seek_bw(),
                    SoundAction::Forward => sink.seek_fw(),
                    SoundAction::PlayPause => sink.toggle_playback(),
                    SoundAction::Plus => sink.volume_up(),
                    SoundAction::Minus => sink.volume_down(),
                    SoundAction::Next => {
                        if sink.is_finished() {
                            if let Some(e) = queue.pop_front() {
                                previous.push(e);
                            }
                        }
                        sink.stop(&guard);
                    }
                    SoundAction::PlayVideo(video) => {
                        queue.push_back(video);
                    }
                    SoundAction::Previous => {
                        if let Some(e) = previous.pop() {
                            if let Some(c) = current.take() {
                                queue.push_front(c);
                            }
                            queue.push_front(e);
                        }
                        sink.stop(&guard);
                    }
                }
            }
            if sink.is_finished() {
                'a: loop {
                    if let Some(video) = queue.pop_front() {
                        let k =
                            PathBuf::from_str(&format!("data/downloads/{}.mp4", video.video_id))
                                .unwrap();
                        if let Some(e) = current.replace(video) {
                            previous.push(e);
                        }
                        sink.play(k.as_path(), &guard);
                        break 'a;
                    } else {
                        while let Ok(e) = rx.try_recv() {
                            if let SoundAction::PlayVideo(video) = e {
                                queue.push_back(video);
                                continue 'a;
                            }
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
            }
        }
    });
    tx
}

fn generate_music(
    queue: &VecDeque<Video>,
    previous: &[Video],
    current: &Option<Video>,
    sink: &Player,
) -> Vec<UIMusic> {
    let mut music = Vec::new();
    {
        music.extend(
            IN_DOWNLOAD
                .lock()
                .unwrap()
                .iter()
                .map(|e| UIMusic::new(e, MusicStatus::Downloading)),
        );
        previous.iter().rev().take(3).rev().for_each(|e| {
            music.push(UIMusic::new(e, MusicStatus::Previous));
        });
        if let Some(e) = current {
            music.push(UIMusic::new(
                e,
                if sink.is_paused() {
                    MusicStatus::Paused
                } else {
                    MusicStatus::Playing
                },
            ));
        }
        music.extend(
            queue
                .iter()
                .take(40)
                .map(|e| UIMusic::new(e, MusicStatus::Next)),
        );
    }
    music
}
