use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr, fmt::Display,
};

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    Client, ClientBuilder,
};

use string_utils::StringUtils;

use structs::{get_playlist, from_json, get_video};
pub use structs::{Playlist, Video};

const YTM_DOMAIN: &str = "https://music.youtube.com";

mod string_utils;
mod structs;

fn unescape(inp: &str) -> Result<String, Error> {
    let mut string = String::with_capacity(inp.len());
    let mut iter = inp.chars();
    while let Some(e) = iter.next() {
        if e == '\\' {
            match iter
                .next()
                .ok_or_else(|| Error::InvalidEscapedSequence(inp.to_owned()))?
            {
                'n' => string.push('\n'),
                'r' => string.push('\r'),
                't' => string.push('\t'),
                'x' => {
                    let mut hex = String::with_capacity(2);
                    hex.push(
                        iter.next()
                            .ok_or_else(|| Error::InvalidEscapedSequence(inp.to_owned()))?,
                    );
                    hex.push(
                        iter.next()
                            .ok_or_else(|| Error::InvalidEscapedSequence(inp.to_owned()))?,
                    );
                    string.push(
                        u8::from_str_radix(&hex, 16)
                            .map_err(|_| Error::InvalidEscapedSequence(inp.to_owned()))?
                            as char,
                    );
                }
                'u' => {
                    let mut hex = String::with_capacity(4);
                    for _ in 0..4 {
                        hex.push(
                            iter.next()
                                .ok_or_else(|| Error::InvalidEscapedSequence(inp.to_owned()))?,
                        );
                    }
                    let hex = u32::from_str_radix(&hex, 16)
                        .map_err(|_| Error::InvalidEscapedSequence(inp.to_owned()))?;
                    string.push(
                        std::char::from_u32(hex)
                            .ok_or_else(|| Error::InvalidEscapedSequence(inp.to_owned()))?,
                    );
                }
                e => string.push(e),
            }
        } else {
            string.push(e);
        }
    }
    Ok(string)
}

async fn get_visitor_id(
    request_func: &reqwest::Client,
    headers: &HeaderMap,
) -> Result<(String, Vec<Playlist>), Error> {
    let response: String = request_func
        .get(YTM_DOMAIN)
        .headers(headers.clone())
        .send()
        .await
        .map_err(Error::Reqwest)?
        .text()
        .await
        .map_err(Error::Reqwest)?;
    let playlist = from_json(&extract_json(&response)?, get_playlist)?;
    response
        .between("VISITOR_DATA\":\"", "\"")
        .to_owned_()
        .map(|x| (x, playlist))
        .ok_or_else(|| Error::InvalidHTMLFile(0,response.to_string()))
}

async fn get_user_playlists(
    request_func: &reqwest::Client,
    headers: &HeaderMap,
) -> Result<Vec<Playlist>, Error> {
    let response: String = request_func
        .get(format!("{YTM_DOMAIN}/library/playlists"))
        .headers(headers.clone())
        .send()
        .await
        .map_err(Error::Reqwest)?
        .text()
        .await
        .map_err(Error::Reqwest)?;
    from_json(&extract_json(&response)?, get_playlist)
}

fn extract_json(string: &str) -> Result<String, Error> {
    let json = string
        .between(
            "initialData.push({path: '\\/browse', params: J",
            "'});ytcfg.set({",
        )
        .after("data: '")
        .to_owned_()
        .ok_or_else(|| Error::InvalidHTMLFile(1,string.to_string()))?;
    unescape(&json)
}
fn extract_json_search(string: &str) -> Result<String, Error> {
    let json = string
        .between(
            "initialData.push({path: '\\/search', params: J",
            "'});ytcfg.set({",
        )
        .after("data: '")
        .to_owned_()
        .ok_or_else(|| Error::InvalidHTMLFile(2,string.to_string()))?;
    unescape(&json)
}

pub struct YTApi {
    client: Client,
    playlists: Vec<Playlist>,
}

#[derive(Debug)]
pub enum Error {
    InvalidHTMLFile(u32,String),
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
    InvalidHeaderValue(InvalidHeaderValue),
    InvalidHeaderName(InvalidHeaderName),
    InvalidJsonCantFind(String, String),
    InvalidHeaderFormat(PathBuf, String),
    Io(std::io::Error),
    InvalidEscapedSequence(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
			Error::InvalidHTMLFile(e,s) => write!(f, "Invalid HTML file: {} {}", e,s),
			Error::Reqwest(e) => write!(f, "Reqwest error: {}", e),
			Error::SerdeJson(e) => write!(f, "SerdeJson error: {}", e),
			Error::InvalidHeaderValue(e) => write!(f, "Invalid header value: {}", e),
			Error::InvalidHeaderName(e) => write!(f, "Invalid header name: {}", e),
			Error::InvalidJsonCantFind(e, s) => write!(f, "Invalid json: {} {}", e, s),
			Error::InvalidHeaderFormat(e, s) => write!(f, "Invalid header format: {} {}", e.display(), s),
			Error::Io(e) => write!(f, "IO error: {}", e),
			Error::InvalidEscapedSequence(e) => write!(f, "Invalid escaped sequence: {}", e),
		}
		
    }
}

impl YTApi {
    pub async fn from_header_file(filepath: &Path) -> Result<Self, Error> {
        let mut headers = HashMap::new();
        let k = std::fs::read_to_string(filepath).map_err(Error::Io)?;
        for line in k.lines() {
            let mut parts = line.splitn(2, ':');
            let key = parts.next().ok_or_else(|| {
                Error::InvalidHeaderFormat(
                    filepath.to_owned(),
                    "HeaderFormat:\nHEADER_NAME: HEADER_VALUE".to_string(),
                )
            })?;
            let value = parts.next().ok_or_else(|| {
                Error::InvalidHeaderFormat(
                    filepath.to_owned(),
                    "HeaderFormat:\nHEADER_NAME: HEADER_VALUE".to_string(),
                )
            })?;
            headers.insert(key.to_owned(), value.to_owned());
        }
        headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:98.0) Gecko/20100101 Firefox/98.0".to_string());
        headers.insert("Accept".to_string(), "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".to_string());
        headers.insert("Accept-Language".to_string(), "en-US,en;q=0.5".to_string());
		headers.insert("Accept-Encoding".to_string(), "gzip, deflate".to_string());
        Self::from_headers(&headers).await
    }
    pub async fn search(&self, search: &str) -> Result<Vec<Video>, Error> {
        let k = extract_json_search(
            &self
                .client
                .get(&format!("https://music.youtube.com/search?q={}", search))
                .send()
                .await
                .map_err(Error::Reqwest)?
                .text()
                .await
                .map_err(Error::Reqwest)?,
        )?;
        from_json(&k, get_video)
    }
    pub fn playlists(&self) -> &Vec<Playlist> {
        &self.playlists
    }
    pub async fn from_headers_map(mut headers: HeaderMap) -> Result<Self, Error> {
        let (xgoo, mut playlists) = get_visitor_id(&Client::new(), &headers).await?;
        headers.insert(
            "x-goog-visitor-id",
            HeaderValue::from_str(&xgoo).map_err(Error::InvalidHeaderValue)?,
        );
        let k = ClientBuilder::default()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .map_err(Error::Reqwest)?;
        playlists.append(&mut get_user_playlists(&k, &HeaderMap::new()).await?);
        playlists.sort();
        playlists.dedup();
        Ok(Self {
            client: k,
            playlists,
        })
    }
    pub async fn from_headers(map: &HashMap<String, String>) -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        for (x, y) in map {
            headers.insert(
                HeaderName::from_str(x.trim()).map_err(Error::InvalidHeaderName)?,
                HeaderValue::from_str(y.trim()).map_err(Error::InvalidHeaderValue)?,
            );
        }
        Self::from_headers_map(headers).await
    }
    pub async fn browse_playlist(&self, playlistid: &str) -> Result<Vec<Video>, Error> {
        let playlist = extract_json(
            &self
                .client
                .get(&format!(
                    "https://music.youtube.com/playlist?list={}",
                    playlistid.strip_prefix("VL").unwrap_or(playlistid)
                ))
                .send()
                .await
                .map_err(Error::Reqwest)?
                .text()
                .await
                .map_err(Error::Reqwest)?,
        )?;
        from_json(&playlist, get_video)
    }
}
