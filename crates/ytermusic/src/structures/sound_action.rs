use common_structs::MusicDownloadStatus;
use download_manager::{DownloadManagerMessage, MessageHandler};
use flume::Sender;
use log::{error, trace};
use std::{fs, sync::Arc, time::Duration};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    DATABASE, ShutdownSignal, consts::CACHE_DIR, errors::handle_error_option, systems::{DOWNLOAD_MANAGER, player::PlayerState}
};

/// Actions that can be sent to the player from other services
#[derive(Debug, Clone)]
pub enum SoundAction {
    /// Set the volume of the player to the given value
    SetVolume(f32),
    Cleanup,
    PlayPause,
    RestartPlayer,
    Plus,
    Minus,
    /// Seek to a specific time in the current song in seconds
    SeekTo(Duration),
    Previous(usize),
    Forward,
    Backward,
    Next(usize),
    AddVideosToQueue(Vec<YoutubeMusicVideoRef>),
    AddVideoUnary(YoutubeMusicVideoRef),
    DeleteVideoUnary,
    ReplaceQueue(Vec<YoutubeMusicVideoRef>),
    VideoStatusUpdate(String, MusicDownloadStatus),
}

impl SoundAction {
    fn insert(player: &mut PlayerState, video: String, status: MusicDownloadStatus) {
        if matches!(
            player.music_status.get(&video),
            Some(&MusicDownloadStatus::DownloadFailed)
        ) {
            DOWNLOAD_MANAGER.remove_from_in_downloads(&video);
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
            Self::SetVolume(volume) => player.sink.set_volume((volume * 100.) as i32),
            Self::SeekTo(time) => player.sink.seek_to(time),
            Self::Backward => player.sink.seek_bw(),
            Self::Forward => player.sink.seek_fw(),
            Self::PlayPause => player.sink.toggle_playback(),
            Self::Cleanup => {
                player.list.clear();
                player.current = 0;
                player.music_status.clear();
                player.sink.stop();
            }
            Self::Plus => player.sink.volume_up(),
            Self::Minus => player.sink.volume_down(),
            Self::Next(a) => {
                player.sink.stop();

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
                player.set_relative_current(-(a as isize));
                player.sink.stop();
            }
            Self::RestartPlayer => {
                player.sink =
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
                if player.list.is_empty() {
                    player.list.push(video);
                } else {
                    player.list.insert(player.current + 1, video);
                }
            }
            Self::DeleteVideoUnary => {
                let index_list = player.list_selector.get_relative_position();
                let video = player.relative_current(index_list).cloned().unwrap();
                if matches!(
                    player.music_status.get(&video.video_id), // not sure abt conditions, needs testing
                    Some(
                        &MusicDownloadStatus::DownloadFailed
                            | &MusicDownloadStatus::Downloading(_)
                            | &MusicDownloadStatus::NotDownloaded
                    )
                ) {
                    DOWNLOAD_MANAGER.remove_from_in_downloads(&video.video_id);
                }
                player.music_status.remove(&video.video_id); // maybe not necessary to do it

                //manage deleting in the list
                player.list.retain(|vid| *vid != video);
                player.list_selector.list_size -= 1;
                if index_list < 0 {
                    player.set_relative_current(-1);
                }
                if index_list == 0 {
                    Self::Next(0).apply_sound_action(player);
                }

                // manage deleting physically
                DATABASE.remove_video(&video);

                let cache_folder = CACHE_DIR.join("downloads");
                let json_path = cache_folder.join(format!("{}.json", &video.video_id));
                match fs::remove_file(json_path) {
                    Ok(_) => trace!("Deleted JSON file"),
                    Err(e) => error!("Error deleting JSON video file: {}", e),
                }

                let mp4_path = cache_folder.join(format!("{}.mp4", &video.video_id));
                match fs::remove_file(mp4_path) {
                    Ok(_) => trace!("Deleted MP4 file"),
                    Err(e) => error!("Error deleting MP4 video file: {}", e),
                }
            }
            Self::ReplaceQueue(videos) => {
                player.list.truncate(player.current + 1);
                DOWNLOAD_MANAGER.clean(
                    ShutdownSignal,
                    download_manager_handler(player.soundaction_sender.clone()),
                );
                Self::AddVideosToQueue(videos).apply_sound_action(player);
                Self::Next(1).apply_sound_action(player);
            }
        }
    }
}

pub fn download_manager_handler(sender: Sender<SoundAction>) -> MessageHandler {
    Arc::new(move |message| match message {
        DownloadManagerMessage::VideoStatusUpdate(video, status) => {
            sender
                .send(SoundAction::VideoStatusUpdate(video, status))
                .unwrap();
        }
    })
}
