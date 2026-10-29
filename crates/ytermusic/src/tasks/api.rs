use std::sync::{Arc, Mutex, OnceLock};

use flume::Sender;
use log::{error, info};
use once_cell::sync::Lazy;
use tokio::{sync::Semaphore, task::JoinSet};
use ytpapi2::{Endpoint, HeaderMap, HeaderValue, YoutubeMusicInstance, YoutubeMusicPlaylistRef};

use crate::{
    consts::CONFIG,
    get_header_file, run_service,
    structures::performance,
    term::{ManagerMessage, Screens},
    try_get_cookies,
};

/// Shared API client, set once the API task has connected.
/// Reused by the search screen so only one HTTP client exists.
pub static API: OnceLock<Arc<YoutubeMusicInstance>> = OnceLock::new();

/// Limits how many playlists are fetched at once, each response is parsed fully in memory.
pub static FETCH_LIMIT: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(CONFIG.global.fetch_limit()));

async fn create_instance() -> ytpapi2::Result<YoutubeMusicInstance> {
    if let Some(cookies) = try_get_cookies() {
        let mut headermap = HeaderMap::new();
        headermap.insert("cookie", HeaderValue::from_str(&cookies).unwrap());
        headermap.insert(
            "user-agent",
            HeaderValue::from_static(
                "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
            ),
        );
        YoutubeMusicInstance::new(headermap, None).await
    } else {
        YoutubeMusicInstance::from_header_file(get_header_file().unwrap().1.as_path()).await
    }
}

pub fn get_text_cookies_expired_or_invalid() -> String {
    let (Ok((_, path)) | Err((_, path))) = get_header_file();
    format!(
        "The `{}` file is not configured correctly. \nThe cookies are expired or invalid.",
        path.display()
    )
}

pub fn spawn_api_task(updater_s: Sender<ManagerMessage>) {
    run_service(async move {
        info!("API task on");
        let guard = performance::guard("API task");
        let client = create_instance().await;
        match client {
            Ok(api) => {
                let api = Arc::new(api);
                let _ = API.set(api.clone());
                let mut set = JoinSet::new();
                let api_ = api.clone();
                let updater_s_ = updater_s.clone();
                set.spawn(async move {
                    let search_results = api_.get_home(2).await;
                    match search_results {
                        Ok(e) => {
                            for playlist in e.playlists {
                                spawn_browse_playlist_task(
                                    playlist.clone(),
                                    api_.clone(),
                                    updater_s_.clone(),
                                )
                            }
                        }
                        Err(e) => {
                            error!("get_home {e:?}")
                        }
                    }
                });
                let api_ = api.clone();
                let updater_s_ = updater_s.clone();
                set.spawn(async move {
                    let search_results = api_.get_library(&Endpoint::MusicLikedPlaylists, 2).await;
                    match search_results {
                        Ok(e) => {
                            for playlist in e {
                                spawn_browse_playlist_task(
                                    playlist.clone(),
                                    api_.clone(),
                                    updater_s_.clone(),
                                )
                            }
                        }
                        Err(e) => {
                            error!("MusicLikedPlaylists -> {e:?}");
                        }
                    }
                });
                let api_ = api.clone();
                let updater_s_ = updater_s.clone();
                set.spawn(async move {
                    let search_results = api_.get_library(&Endpoint::MusicLibraryLanding, 2).await;
                    match search_results {
                        Ok(e) => {
                            for playlist in e {
                                spawn_browse_playlist_task(
                                    playlist.clone(),
                                    api_.clone(),
                                    updater_s_.clone(),
                                )
                            }
                        }
                        Err(e) => {
                            error!("MusicLibraryLanding -> {e:?}");
                        }
                    }
                });
                while let Some(e) = set.join_next().await {
                    e.unwrap();
                }
            }
            Err(e) => match &e {
                ytpapi2::YoutubeMusicError::NoCookieAttribute
                | ytpapi2::YoutubeMusicError::NoSapsidInCookie
                | ytpapi2::YoutubeMusicError::InvalidCookie(_)
                | ytpapi2::YoutubeMusicError::NeedToLogin
                | ytpapi2::YoutubeMusicError::CantFindInnerTubeApiKey(_)
                | ytpapi2::YoutubeMusicError::CantFindInnerTubeClientVersion(_)
                | ytpapi2::YoutubeMusicError::CantFindVisitorData(_)
                | ytpapi2::YoutubeMusicError::IoError(_) => {
                    error!("{}", get_text_cookies_expired_or_invalid());
                    error!("{e:?}");
                    updater_s
                        .send(
                            ManagerMessage::Error(
                                get_text_cookies_expired_or_invalid(),
                                Box::new(Some(ManagerMessage::Quit)),
                            )
                            .pass_to(Screens::DeviceLost),
                        )
                        .unwrap();
                }
                e => {
                    error!("{e:?}");
                }
            },
        }
        drop(guard);
    });
}

static BROWSED_PLAYLISTS: Lazy<Mutex<Vec<(String, String)>>> = Lazy::new(|| Mutex::new(vec![]));

fn spawn_browse_playlist_task(
    playlist: YoutubeMusicPlaylistRef,
    api: Arc<YoutubeMusicInstance>,
    updater_s: Sender<ManagerMessage>,
) {
    if playlist.browse_id.starts_with("UC") && CONFIG.player.hide_channels_on_homepage {
        log::info!(
            "Skipping channel (CONFIG) {} {}",
            playlist.name,
            playlist.browse_id
        );
        return;
    }
    if playlist.browse_id.starts_with("MPREb_") && CONFIG.player.hide_albums_on_homepage {
        log::info!(
            "Skipping album (CONFIG) {} {}",
            playlist.name,
            playlist.browse_id
        );
        return;
    }
    {
        let mut k = BROWSED_PLAYLISTS.lock().unwrap();
        if k.iter()
            .any(|(name, id)| name == &playlist.name && id == &playlist.browse_id)
        {
            return;
        }
        k.push((playlist.name.clone(), playlist.browse_id.clone()));
    }

    run_service(async move {
        let guard = format!("Browse playlist {} {}", playlist.name, playlist.browse_id);
        let guard = performance::guard(&guard);
        let permit = FETCH_LIMIT.acquire().await.unwrap();
        let videos = api.get_playlist(&playlist, 5).await;
        drop(permit);
        match videos {
            Ok(videos) => {
                if videos.len() < 2 {
                    info!("Playlist {} is too small so skipped", playlist.name);
                    return;
                }
                let _ = updater_s.send(
                    ManagerMessage::AddElementToChooser((
                        format!("{} ({})", playlist.name, playlist.subtitle),
                        videos,
                    ))
                    .pass_to(Screens::Playlist),
                );
            }
            Err(e) => {
                error!("{e:?}");
            }
        }

        drop(guard);
    });
}
