use ytpapi2::YoutubeMusicVideoRef;

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
    AddVideosToQueue(Vec<YoutubeMusicVideoRef>),
    AddVideoUnary(YoutubeMusicVideoRef),
    ReplaceQueue(Vec<YoutubeMusicVideoRef>),
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
                player.list.clear();
                player.current = 0;
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

                player.set_relative_current(a as _);
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
                    player.list.push(v)
                }
            }
            Self::Previous(a) => {
                player.set_relative_current(- (a as isize));
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
                if let Some(e) = player.current().cloned() {
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
                player.list.insert(player.current + 1, video);
            }
            Self::ReplaceQueue(videos) => {
                player.list.truncate(player.current + 1);
                download::clean(&player.soundaction_sender);
                Self::AddVideosToQueue(videos).apply_sound_action(player);
                Self::Next(1).apply_sound_action(player);
            }
        }
    }
}
