use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Error;

/// Will parse a path and a json value to a Value
/// # Example
/// `test.tabs.#0.value`
///

pub fn extract_meaninfull<'a>(value: &'a Value, path: &str) -> Result<&'a Value, Error> {
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
fn get_video_titles(value: &Value) -> Result<Video, Error> {
    let value = extract_meaninfull(value, "musicResponsiveListItemRenderer")?;
    let texts: Vec<String> = as_array(extract_meaninfull(value, "flexColumns")?)?
        .iter()
        .map(get_text_from_flexcolumn)
        .collect();
    let author = texts.get(1).cloned().unwrap_or_default();
    let begin = author.find('•').map(|x| x + "•".len()).unwrap_or(0);
    let end = author.rfind('•').unwrap_or_else(|| author.len());
    let author = author[begin.min(end)..end.max(begin)].trim().to_owned();
    let k = author
        .rfind('•')
        .unwrap_or_else(|| author.len() - "•".len());
    let album = author[k + "•".len()..].trim().to_owned();
    let author = author[..k].trim().to_owned();
    Ok(Video {
        title: texts.get(0).cloned().unwrap_or_default(),
        author,
        album,
        video_id: as_str(extract_meaninfull(value, "playlistItemData.videoId")?)?,
        duration: String::new(),
    })
}
fn get_video(value: &Value) -> Result<Video, Error> {
    let value = extract_meaninfull(value, "musicResponsiveListItemRenderer")?;
    let texts: Vec<String> = as_array(extract_meaninfull(value, "flexColumns")?)?
        .iter()
        .map(get_text_from_flexcolumn)
        .collect();
    let k: Result<String, Error> = (|| {
        as_str(extract_meaninfull(
            value,
            "fixedColumns.0.musicResponsiveListItemFixedColumnRenderer.text.runs.0.text",
        )?)
    })();
    Ok(Video {
        title: texts.get(0).cloned().unwrap_or_default(),
        author: texts.get(1).cloned().unwrap_or_default(),
        album: texts.get(2).cloned().unwrap_or_default(),
        video_id: as_str(extract_meaninfull(value, "playlistItemData.videoId")?)?,
        duration: k.unwrap_or_default(),
    })
}

const ALLOWED: &[&str] = &[
    "Video", "Vidéo", "Title", "Song", "Titre", "Meilleur", "Best",
];

pub fn search_results(json: Value) -> Result<Vec<Video>, Error> {
    let json = extract_meaninfull(&json, "contents.tabbedSearchResultsRenderer.tabs.0.tabRenderer.content.sectionListRenderer.contents")?;
    let mut list: Vec<Video> = Vec::new();
    for i in as_array(json)?.iter() {
        let title = if let Ok(e) = extract_meaninfull(i, "musicShelfRenderer.title.runs.0.text") {
            e
        } else {
            continue;
        };
        let title = as_str(title)?;
        if ALLOWED.iter().any(|x| title.contains(x)) {
            for k in as_array(extract_meaninfull(i, "musicShelfRenderer.contents")?)?
                .iter()
                .flat_map(get_video_titles)
                .collect::<Vec<_>>()
            {
                if list.iter().any(|w| w.video_id == k.video_id) {
                    continue;
                }
                list.push(k);
            }
        }
    }
    Ok(list)
}

fn get_text_from_flexcolumn(value: &Value) -> String {
    let k: Option<String> = (|| {
        let value = value.as_object()?.values().next()?;

        Some(
            as_array(extract_meaninfull(value, "text.runs").ok()?)
                .ok()?
                .iter()
                .flat_map(|x| as_str(extract_meaninfull(x, "text").ok()?).ok())
                .collect::<Vec<_>>()
                .join(""),
        )
    })();
    k.unwrap_or_default()
}

pub fn videos_from_playlist(string: &str) -> Result<Vec<Video>, Error> {
    let jason = serde_json::from_str(string).map_err(Error::SerdeJson)?;
    let out = extract_meaninfull(&jason, PLAYLIST_PATH)?;

    as_array(out)?.iter().map(get_video).collect()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Playlist {
    pub name: String,
    pub subtitle: String,
    pub browse_id: String,
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
    Ok(Playlist {
        name,
        subtitle,
        browse_id,
    })
}
