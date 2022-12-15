use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Error;

pub(crate) fn from_json_string<T: PartialEq>(
    json: &str,
    transformer: impl Fn(&Value) -> Option<T>,
) -> Result<Vec<T>, Error> {
    from_json(
        &serde_json::from_str(json).map_err(Error::SerdeJson)?,
        transformer,
    )
}

/// Applies recursively the `transformer` function to the given json value
/// and returns the transformed values.
pub(crate) fn from_json<T: PartialEq>(
    json: &Value,
    transformer: impl Fn(&Value) -> Option<T>,
) -> Result<Vec<T>, Error> {
    /// Execute a function on each element of a json value recursively.
    /// When the function returns something, the value is added to the result.
    pub(crate) fn inner_crawl<T: PartialEq>(
        value: &Value,
        playlists: &mut Vec<T>,
        transformer: &impl Fn(&Value) -> Option<T>,
    ) {
        if let Some(e) = transformer(value) {
            // Maybe an hashset would be better
            if !playlists.contains(&e) {
                playlists.push(e);
            }
            return;
        }
        match value {
            Value::Array(a) => a
                .iter()
                .for_each(|x| inner_crawl(x, playlists, transformer)),
            Value::Object(a) => a
                .values()
                .for_each(|x| inner_crawl(x, playlists, transformer)),
            _ => (),
        }
    }
    let mut playlists = Vec::new();
    inner_crawl(&json, &mut playlists, &transformer);
    Ok(playlists)
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Video {
    pub title: String,
    pub author: String,
    pub album: String,
    pub video_id: String,
    pub duration: String,
}

impl Display for Video {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} | {}", self.author, self.title)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Eq, Ord, PartialEq, Hash)]
pub struct Playlist {
    pub name: String,
    pub subtitle: String,
    pub browse_id: String,
}

/// Tries to extract a playlist from a json value.
/// Quite flexible to reduce odds of API change breaking this.
pub(crate) fn get_playlist(value: &Value) -> Option<Playlist> {
    let object = value.as_object()?;
    let title_text = get_text(object.get("title")?, true, false)?;
    let subtitle = object
        .get("subtitle")
        .and_then(|x| get_text(x, false, false));
    let browse_id = &object
        .get("navigationEndpoint")
        .and_then(|x| x.get("browseEndpoint"))
        .and_then(|x| x.get("browseId"))
        .and_then(Value::as_str)?;
    Some(Playlist {
        name: title_text,
        subtitle: subtitle.unwrap_or_default(),
        browse_id: browse_id.strip_prefix("VL")?.to_string(),
    })
}

pub fn get_playlist_search(value: &Value) -> Option<Playlist> {
    let playlist_id = value
        .get("overlay")
        .and_then(|x| x.get("musicItemThumbnailOverlayRenderer"))
        .and_then(|x| x.get("content"))
        .and_then(|x| x.get("musicPlayButtonRenderer"))
        .and_then(|x| x.get("playNavigationEndpoint"))
        .and_then(|x| x.get("watchPlaylistEndpoint"))
        .and_then(|x| x.get("playlistId"))
        .and_then(Value::as_str)?;
    let titles: Vec<String> = value
        .get("flexColumns")?
        .as_array()?
        .iter()
        .flat_map(|x| {
            x.get("musicResponsiveListItemFlexColumnRenderer")
                .and_then(|x| x.get("text"))
                .and_then(|x| get_text(x, false, false))
        })
        .collect();
    Some(Playlist {
        name: titles.get(0)?.clone(),
        subtitle: titles.get(1)?.clone(),
        browse_id: playlist_id.to_string(),
    })
}

pub fn extract_playlist_info(value: &Value) -> Option<(String, String)> {
    let header = value.get("header")?.get("musicDetailHeaderRenderer")?;
    let title = get_text(header.get("title")?, false, false)?;
    let subtitles = header
        .get("subtitle")?
        .get("runs")?
        .as_array()?
        .iter()
        .flat_map(|x| get_text(x, false, false))
        .filter(|x| x != " • ")
        .collect::<Vec<String>>();
    Some((title, subtitles.get(1)?.clone()))
}

pub fn get_video_from_album(value: &Value) -> Option<Video> {
    let video_id = value
        .get("playlistItemData")
        .and_then(|x| x.get("videoId"))
        .and_then(Value::as_str)?;
    let title: Vec<String> = value
        .get("flexColumns")?
        .as_array()?
        .iter()
        .flat_map(|x| {
            x.get("musicResponsiveListItemFlexColumnRenderer")
                .and_then(|x| x.get("text"))
                .and_then(|x| get_text(x, false, false))
        })
        .collect();
    Some(Video {
        title: title.get(0)?.clone(),
        author: String::new(),
        album: String::new(),
        video_id: video_id.to_string(),
        duration: String::new(),
    })
}

/// Tries to extract the text from a json value.
/// text_clean: Weather to include singleton text.
/// dot: Weather to use the dotted text instead of the space
fn get_text(value: &Value, text_clean: bool, dot: bool) -> Option<String> {
    if let Some(e) = value.as_str() {
        Some(e.to_string())
    } else {
        let obj = value.as_object()?;
        if let Some(e) = obj.get("text") {
            if text_clean && obj.values().count() == 1 {
                return None;
            }
            get_text(e, text_clean, dot)
        } else if let Some(e) = obj.get("runs") {
            let k = e
                .as_array()?
                .iter()
                .flat_map(|x| get_text(x, text_clean, dot))
                .collect::<Vec<_>>();
            if k.is_empty() {
                None
            } else {
                Some(join_clean(&k, dot))
            }
        } else {
            None
        }
    }
}

fn join_clean(strings: &[String], dot: bool) -> String {
    strings
        .iter()
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join(if dot { " • " } else { " " })
}

/// Tries to find a video id in the json
pub fn get_videoid(value: &Value) -> Option<String> {
    match value {
        Value::Array(e) => e.iter().find_map(get_videoid),
        Value::Object(e) => e
            .get("videoId")
            .and_then(Value::as_str)
            .map(|x| x.to_string())
            .or_else(|| e.values().find_map(get_videoid)),
        _ => None,
    }
}

/// Tries to extract a video from a json value.
/// Quite flexible to reduce odds of API change breaking this.
pub(crate) fn get_video(value: &Value) -> Option<Video> {
    // Extract the text part (title, author, album) from a json value.
    let mut texts = value
        .as_object()?
        .get("flexColumns")?
        .as_array()?
        .iter()
        .flat_map(|x| {
            x.as_object()
                .and_then(|x| x.values().next())
                .and_then(|x| get_text(x, true, true))
        });

    Some(Video {
        video_id: get_videoid(value)?,
        title: texts.next()?,
        author: texts.next()?,
        album: texts.next().unwrap_or_default(),
        duration: String::new(),
    })
}
