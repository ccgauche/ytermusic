use ytpapi::Video;

use crate::{
    errors::{handle_error, handle_error_option},
    systems::{download, player::PlayerState},
    tasks::download::IN_DOWNLOAD,
    DATABASE,
};

use super::app_status::MusicDownloadStatus;
/// Actions that can be sent to the player from other services
#[derive(Debug, Clone)]
pub enum SoundAction {
    Cleanup,
    PlayPause,
    RestartPlayer,
    Plus,
    Minus,
    Previous(usize),
    Forward,
    Backward,
    Next(usize),
    AddVideosToQueue(Vec<Video>),
    AddVideoUnary(Video),
    ReplaceQueue(Vec<Video>),
    VideoStatusUpdate(String, MusicDownloadStatus),
}

impl SoundAction {
    fn insert(player: &mut PlayerState, video: String, status: MusicDownloadStatus) {
        if matches!(
            player.music_status.get(&video),
            Some(&MusicDownloadStatus::DownloadFailed)
        ) {
            IN_DOWNLOAD.lock().unwrap().remove(&video);
        }
        if matches!(
            player.music_status.get(&video),
            Some(&MusicDownloadStatus::Downloading(_) | &MusicDownloadStatus::Downloaded)
        ) && status == MusicDownloadStatus::NotDownloaded
        {
            return;
        }
        player.music_status.insert(video, status);
    }
    pub fn apply_sound_action(self, player: &mut PlayerState) {
        match self {
            Self::Backward => player.sink.seek_bw(),
            Self::Forward => player.sink.seek_fw(),
            Self::PlayPause => player.sink.toggle_playback(),
            Self::Cleanup => {
                player.queue.clear();
                player.previous.clear();
                player.current = None;
                player.music_status.clear();
                handle_error(
                    &player.updater,
                    "sink stop",
                    player.sink.stop(&player.guard),
                );
            }
            Self::Plus => player.sink.volume_up(),
            Self::Minus => player.sink.volume_down(),
            Self::Next(a) => {
                handle_error(
                    &player.updater,
                    "sink stop",
                    player.sink.stop(&player.guard),
                );

                if let Some(e) = player.current.take() {
                    player.previous.push(e);
                }
                for _ in 1..a {
                    player.previous.push(player.queue.pop_front().unwrap());
                }
            }
            Self::VideoStatusUpdate(video, status) => {
                player.music_status.insert(video, status);
            }
            Self::AddVideosToQueue(video) => {
                let db = DATABASE.read().unwrap();
                for v in video {
                    Self::insert(
                        player,
                        v.video_id.clone(),
                        if db.iter().any(|e| e.video_id == v.video_id) {
                            MusicDownloadStatus::Downloaded
                        } else {
                            MusicDownloadStatus::NotDownloaded
                        },
                    );
                    player.queue.push_back(v)
                }
            }
            Self::Previous(a) => {
                for _ in 0..a {
                    if let Some(e) = player.previous.pop() {
                        if let Some(c) = player.current.take() {
                            player.queue.push_front(c);
                        }
                        player.queue.push_front(e);
                    }
                }
                handle_error(
                    &player.updater,
                    "sink stop",
                    player.sink.stop(&player.guard),
                );
            }
            Self::RestartPlayer => {
                (player.sink, player.guard) =
                    handle_error_option(&player.updater, "update player", player.sink.update())
                        .unwrap();
                if let Some(e) = player.current.clone() {
                    Self::AddVideoUnary(e).apply_sound_action(player);
                }
            }
            Self::AddVideoUnary(video) => {
                Self::insert(
                    player,
                    video.video_id.clone(),
                    if DATABASE
                        .read()
                        .unwrap()
                        .iter()
                        .any(|e| e.video_id == video.video_id)
                    {
                        MusicDownloadStatus::Downloaded
                    } else {
                        MusicDownloadStatus::NotDownloaded
                    },
                );
                player.queue.push_front(video);
            }
            Self::ReplaceQueue(videos) => {
                player.queue.clear();
                download::clean(player.soundaction_sender.clone());
                Self::AddVideosToQueue(videos).apply_sound_action(player);
                Self::Next(1).apply_sound_action(player);
            }
        }
    }
}
