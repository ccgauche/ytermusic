use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    Client, ClientBuilder,
};

use sha1::{Digest, Sha1};
use string_utils::StringUtils;

use structs::{
    extract_playlist_info, from_json, from_json_string, get_playlist, get_playlist_search,
    get_video, get_video_from_album,
};
pub use structs::{Playlist, Video};

const YTM_DOMAIN: &str = "https://music.youtube.com";

mod string_utils;
pub mod structs;

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
                    let c = iter
                        .next()
                        .ok_or_else(|| Error::InvalidEscapedSequence(inp.to_owned()))?;
                    if c == '{' {
                        while let Some(e) = iter.next() {
                            if e == '}' {
                                break;
                            }
                            hex.push(e);
                        }
                    } else {
                        hex.push(c);
                        for _ in 0..3 {
                            hex.push(
                                iter.next()
                                    .ok_or_else(|| Error::InvalidEscapedSequence(inp.to_owned()))?,
                            );
                        }
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
    let playlist = from_json_string(&extract_json(&response, YTM_DOMAIN)?, get_playlist)?;
    response
        .between("VISITOR_DATA\":\"", "\"")
        .to_owned_()
        .map(|x| (x, playlist))
        .ok_or_else(|| Error::InvalidHTMLFile(0, YTM_DOMAIN.to_string(), response.to_string()))
}

/*
async function getSApiSidHash(SAPISID, origin) {
    function sha1(str) {
      return window.crypto.subtle.digest("SHA-1", new TextEncoder("utf-8").encode(str)).then(buf => {
        return Array.prototype.map.call(new Uint8Array(buf), x=>(('00'+x.toString(16)).slice(-2))).join('');
      });
    }

    const TIMESTAMP_MS = Date.now();
    const digest = await sha1(`${TIMESTAMP_MS} ${SAPISID} ${origin}`);

    return `${TIMESTAMP_MS}_${digest}`;
}

const SAPISIDHASH = await getSApiSidHash(document.cookie.split('SAPISID=')[1].split('; ')[0], 'https://photos.google.com');
console.log(SAPISIDHASH);
 */

#[test]
fn test_compute_sapi_hash() {
    assert_eq!(
        compute_sapi_hash(
            1677060301,
            "6hRvW7xAyUr8l4D3/A23oPPN82HMBbEPAF",
            "https://music.youtube.com"
        ),
        "49162ac753efab46e0956085dfc3e9fc5fa6178c"
    );
}

fn timestamp() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs()
}

fn compute_sapi_hash(timestamp: u64, sapisid: &str, origin: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("{} {} {}", timestamp, sapisid, origin));
    let result = hasher.finalize();
    let mut hex = String::with_capacity(40);
    for byte in result {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

async fn get_user_playlists(
    request_func: &reqwest::Client,
    headers: &HeaderMap,
    cookies: &str
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
    let innertube_api_key = response
        .between("INNERTUBE_API_KEY\":\"", "\"")
        .to_owned_()
        .ok_or_else(|| Error::InvalidHTMLFile(0, YTM_DOMAIN.to_string(), response.to_string()))?;
    let timestamp = timestamp();
    let sapi = format!(
        "SAPISIDHASH {timestamp}_{}",
        compute_sapi_hash(
            timestamp,
            cookies
                .between("SAPISID=", ";").unwrap(),
            "https://music.youtube.com"
        )
    );
    /*
    
    https://music.youtube.com/youtubei/v1/browse?key=AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30&prettyPrint=false
    
    {
      "context": {
        "client": {
          "clientName": "WEB_REMIX",
          "clientVersion": "1.20230215.01.00"
        }
      },
      "browseId": "FEmusic_liked_playlists"
    }
    
    'Content-Type'=> 'application/json',
    'Authorization'=> 'SAPISIDHASH 1677060301_49162ac753efab46e0956085dfc3e9fc5fa6178c',
    'X-Origin'=> 'https://music.youtube.com',
     */
    let client_version = response
        .between("INNERTUBE_CLIENT_VERSION\":\"", "\"")
        .to_owned_()
        .ok_or_else(|| Error::InvalidHTMLFile(7, YTM_DOMAIN.to_string(), response.to_string()))?;
    let request = reqwest::Client::new().post(&format!("https://music.youtube.com/youtubei/v1/browse?key={innertube_api_key}"))
        .header("Content-Type", "application/json")
        .header("Authorization", sapi)
        .header("X-Origin", "https://music.youtube.com")
        .body(format!(r#"{{"context":{{"client":{{"clientName":"WEB_REMIX","clientVersion":"{client_version}"}}}},"browseId":"FEmusic_liked_playlists"}}"#))
        .send().await.map_err(Error::Reqwest)?.text().await.map_err(Error::Reqwest)?;
    //from_json_string(request_func., transformer)
    from_json_string(
        &request,
        get_playlist,
    )
}

fn extract_json(string: &str, url: &str) -> Result<String, Error> {
    let json = string
        .between(
            "initialData.push({path: '\\/browse', params: J",
            "'});ytcfg.set({",
        )
        .after("data: '")
        .to_owned_()
        .ok_or_else(|| Error::InvalidHTMLFile(1, url.to_string(), string.to_string()))?;
    unescape(&json)
}
fn extract_json_search(string: &str, url: &str) -> Result<String, Error> {
    let json = string
        .between(
            "initialData.push({path: '\\/search', params: J",
            "'});ytcfg.set({",
        )
        .after("data: '")
        .to_owned_()
        .ok_or_else(|| Error::InvalidHTMLFile(2, url.to_string(), string.to_string()))?;
    unescape(&json)
}

pub struct YTApi {
    client: Client,
    playlists: Vec<Playlist>,
}

#[derive(Debug)]
pub enum Error {
    InvalidHTMLFile(u32, String, String),
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
            Error::InvalidHTMLFile(e, a, s) => write!(f, "Invalid HTML file: {} {} {}", e, a, s),
            Error::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            Error::SerdeJson(e) => write!(f, "SerdeJson error: {}", e),
            Error::InvalidHeaderValue(e) => write!(f, "Invalid header value: {}", e),
            Error::InvalidHeaderName(e) => write!(f, "Invalid header name: {}", e),
            Error::InvalidJsonCantFind(e, s) => write!(f, "Invalid json: {} {}", e, s),
            Error::InvalidHeaderFormat(e, s) => {
                write!(f, "Invalid header format: {} {}", e.display(), s)
            }
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
        headers.insert(
            "User-Agent".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:98.0) Gecko/20100101 Firefox/98.0"
                .to_string(),
        );
        headers.insert(
            "Accept".to_string(),
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"
                .to_string(),
        );
        headers.insert("Accept-Language".to_string(), "en-US,en;q=0.5".to_string());
        headers.insert("Accept-Encoding".to_string(), "gzip, deflate".to_string());
        Self::from_headers(&headers).await
    }
    pub async fn search(&self, search: &str) -> Result<(Vec<Video>, Vec<Playlist>), Error> {
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
            &format!("https://music.youtube.com/search?q={}", search),
        )?;
        let json = serde_json::from_str::<serde_json::Value>(&k).map_err(Error::SerdeJson)?;
        Ok((
            from_json(&json, get_video)?,
            from_json(&json, get_playlist_search)?,
        ))
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
            .default_headers(headers.clone())
            .build()
            .map_err(Error::Reqwest)?;
        playlists.append(&mut get_user_playlists(&k, &HeaderMap::new(),headers.get("Cookie").unwrap().to_str().unwrap()).await?);
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
            &format!(
                "https://music.youtube.com/playlist?list={}",
                playlistid.strip_prefix("VL").unwrap_or(playlistid)
            ),
        )?;
        let json =
            serde_json::from_str::<serde_json::Value>(&playlist).map_err(Error::SerdeJson)?;
        let mut videos = from_json(&json, get_video)?;
        let info = extract_playlist_info(&json);
        for mut video in from_json(&json, get_video_from_album)? {
            if videos.iter().any(|x| x.video_id == video.video_id) {
                continue;
            }
            if let Some((title, artist)) = info.as_ref() {
                if video.album.is_empty() {
                    video.album = title.to_string();
                }
                if video.author.is_empty() {
                    video.author = artist.to_string();
                }
            }
            videos.push(video);
        }
        Ok(videos)
    }
}
