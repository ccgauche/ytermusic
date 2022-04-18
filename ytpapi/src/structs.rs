use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Error;

/// Will parse a path and a json value to a Value
/// # Example
/// `test.tabs.#0.value`
///

fn extract_meaninfull<'a>(value: &'a Value, path: &str) -> Result<&'a Value, Error> {
    let mut current_value = value;
    for path_part in path.split('.') {
        if let Ok(a) = path_part.parse::<usize>() {
            let array = current_value.as_array().ok_or_else(|| {
                Error::InvalidJsonCantFind(path_part.to_string(), current_value.to_string())
            })?;
            if array.len() < a {
                return Err(Error::InvalidJsonCantFind(
                    path_part.to_string(),
                    current_value.to_string(),
                ));
            }
            current_value = &array[a];
        } else if let Some(v) = current_value.get(path_part) {
            current_value = v;
        } else {
            return Err(Error::InvalidJsonCantFind(
                path_part.to_string(),
                current_value.to_string(),
            ));
        }
    }
    Ok(current_value)
}
const PATH: &str = "contents.singleColumnBrowseResultsRenderer.tabs.0.tabRenderer.content.sectionListRenderer.contents.0.musicCarouselShelfRenderer.contents";
const PLAYLIST_PATH: &str = "contents.singleColumnBrowseResultsRenderer.tabs.0.tabRenderer.content.sectionListRenderer.contents.0.musicPlaylistShelfRenderer.contents";
// contents.singleColumnBrowseResultsRenderer.tabs.0.tabRenderer.content.sectionListRenderer.contents
pub(crate) fn playlists_from_json(string: &str) -> Result<Vec<Playlist>, Error> {
    let jason = serde_json::from_str(string).map_err(Error::SerdeJson)?;
    as_array(extract_meaninfull(&jason, PATH)?)?
        .iter()
        .map(get_playlist)
        .collect()
}

fn as_array(value: &Value) -> Result<&Vec<Value>, Error> {
    value
        .as_array()
        .ok_or_else(|| Error::InvalidJsonCantFind("Not an array".to_owned(), value.to_string()))
}

fn as_str(value: &Value) -> Result<String, Error> {
    value
        .as_str()
        .ok_or_else(|| Error::InvalidJsonCantFind("Not a string".to_owned(), value.to_string()))
        .map(|x| x.to_owned())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Video {
    pub title: String,
    pub author: String,
    pub album: String,
    pub video_id: String,
    pub thumbnail: Vec<Thumbnail>,
    pub duration: String,
}

impl Display for Video {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} - {} ({}): {}",
            self.title, self.author, self.album, self.duration
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

fn get_video(value: &Value) -> Result<Video, Error> {
    let value = extract_meaninfull(value, "musicResponsiveListItemRenderer")?;
    let texts: Vec<String> = as_array(extract_meaninfull(value, "flexColumns")?)?
        .iter()
        .map(get_text_from_flexcolumn)
        .collect();
    let k = extract_meaninfull(
        value,
        "thumbnail.musicThumbnailRenderer.thumbnail.thumbnails",
    )?;
    Ok(Video {
        title: texts[0].to_owned(),
        author: texts[1].to_owned(),
        album: texts[2].to_owned(),
        video_id: as_str(extract_meaninfull(value, "playlistItemData.videoId")?)?,
        thumbnail: serde_json::from_value(k.clone()).map_err(Error::SerdeJson)?,
        duration: as_str(extract_meaninfull(
            value,
            "fixedColumns.0.musicResponsiveListItemFixedColumnRenderer.text.runs.0.text",
        )?)?,
    })
}

fn get_text_from_flexcolumn(value: &Value) -> String {
    let k: Option<String> = (|| {
        let value = value.as_object()?.values().next()?;

        Some(
            if let Ok(e) = extract_meaninfull(value, "text.runs.0.text") {
                e.as_str()?.to_string()
            } else {
                String::new()
            },
        )
    })();
    k.unwrap_or_default()
}

pub fn videos_from_playlist(string: &str) -> Result<Vec<Video>, Error> {
    let jason = serde_json::from_str(string).map_err(Error::SerdeJson)?;
    let out = extract_meaninfull(&jason, PLAYLIST_PATH)?;

    as_array(out)?.iter().map(get_video).collect()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
    pub subtitle: String,
    pub browse_id: String,
    pub thumbnail: String,
}

pub fn get_playlist(value: &Value) -> Result<Playlist, Error> {
    let music_two_item_renderer = &extract_meaninfull(value, "musicTwoRowItemRenderer")?;
    let subtitle = as_array(extract_meaninfull(
        music_two_item_renderer,
        "subtitle.runs",
    )?)?
    .iter()
    .map(|x| as_str(extract_meaninfull(x, "text")?))
    .collect::<Result<Vec<_>, _>>()?
    .join("");
    let title_div = extract_meaninfull(music_two_item_renderer, "title.runs.0")?;
    let browse_id = as_str(extract_meaninfull(
        title_div,
        "navigationEndpoint.browseEndpoint.browseId",
    )?)?;
    let name = as_str(extract_meaninfull(title_div, "text")?)?;
    let thumbnail = as_str(extract_meaninfull(
        music_two_item_renderer,
        "thumbnailRenderer.musicThumbnailRenderer.thumbnail.thumbnails.0.url",
    )?)?;
    Ok(Playlist {
        name,
        subtitle,
        browse_id,
        thumbnail,
    })
}
