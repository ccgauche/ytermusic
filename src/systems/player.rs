use std::{collections::VecDeque, path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use flume::Sender;
use player::{Guard, Player};
use ytpapi::Video;

use crate::{
    terminal::{App, AppMessage, MusicStatus, UIMusic},
    SoundAction,
};

use super::download::IN_DOWNLOAD;

pub fn player_system(updater: Arc<Sender<AppMessage>>) -> Sender<SoundAction> {
    let (tx, rx) = flume::unbounded::<SoundAction>();
    std::thread::spawn(move || {
        let (mut sink, guard) = Player::new();
        let mut queue: VecDeque<Video> = VecDeque::new();
        let mut previous: Vec<Video> = Vec::new();
        let mut current: Option<Video> = None;
        loop {
            updater
                .send(AppMessage::UpdateApp(App::new(
                    &sink,
                    generate_music(&queue, &previous, &current, &sink),
                )))
                .unwrap();
            std::thread::sleep(Duration::from_millis(100));
            while let Ok(e) = rx.try_recv() {
                apply_sound_action(
                    e,
                    &mut sink,
                    &guard,
                    &mut queue,
                    &mut previous,
                    &mut current,
                );
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
                        if let Some(e) = current.take() {
                            previous.push(e);
                        }
                        while let Ok(e) = rx.try_recv() {
                            apply_sound_action(
                                e.clone(),
                                &mut sink,
                                &guard,
                                &mut queue,
                                &mut previous,
                                &mut current,
                            );
                            if matches!(e, SoundAction::PlayVideo(_)) {
                                continue 'a;
                            }
                        }
                        std::thread::sleep(Duration::from_millis(200));

                        updater
                            .send(AppMessage::UpdateApp(App::new(
                                &sink,
                                generate_music(&queue, &previous, &current, &sink),
                            )))
                            .unwrap();
                    }
                }
            }
        }
    });
    tx
}

fn apply_sound_action(
    e: SoundAction,
    sink: &mut Player,
    guard: &Guard,
    queue: &mut VecDeque<Video>,
    previous: &mut Vec<Video>,
    current: &mut Option<Video>,
) {
    match e {
        SoundAction::Backward => sink.seek_bw(),
        SoundAction::Forward => sink.seek_fw(),
        SoundAction::PlayPause => sink.toggle_playback(),
        SoundAction::Plus => sink.volume_up(),
        SoundAction::Minus => sink.volume_down(),
        SoundAction::Next => {
            /* if sink.is_finished() {
                if let Some(e) = queue.pop_front() {
                    previous.push(e);
                }
            } */
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
