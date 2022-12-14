use ytpapi::Video;

use crate::{
    errors::{handle_error, handle_error_option},
    systems::player::PlayerState,
};

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
    PlayVideo(Video),
    PlayVideoUnary(Video),
    ReplaceQueue(Vec<Video>),
}

impl SoundAction {
    pub fn apply_sound_action(self, player: &mut PlayerState) {
        match self {
            Self::Backward => player.sink.seek_bw(),
            Self::Forward => player.sink.seek_fw(),
            Self::PlayPause => player.sink.toggle_playback(),
            Self::Cleanup => {
                player.queue.clear();
                player.previous.clear();
                player.current = None;
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
            Self::PlayVideo(video) => {
                player.queue.push_back(video);
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
                    Self::PlayVideo(e).apply_sound_action(player);
                }
            }
            Self::PlayVideoUnary(video) => {
                player.queue.push_front(video);
            }
            Self::ReplaceQueue(videos) => {
                player.queue.clear();
                player.queue.extend(videos.into_iter());
            }
        }
    }
}
