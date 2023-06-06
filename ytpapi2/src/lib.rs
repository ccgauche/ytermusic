use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use json_extractor::{
    extract_playlist_info, from_json, get_continuation, get_playlist, get_playlist_search,
    get_video, get_video_from_album, Continuation,
};
use log::{error, trace, debug};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use string_utils::StringUtils;

mod json_extractor;
mod string_utils;

pub use json_extractor::YoutubeMusicVideoRef;

pub type Result<T> = std::result::Result<T, YoutubeMusicError>;

const YTM_DOMAIN: &str = "https://music.youtube.com";

#[cfg(test)]
fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    let file = std::fs::read_to_string("../headers.txt").unwrap();
    for header in file.lines() {
        if header.trim().is_empty() {
            continue;
        }
        let (key, value) = header.split_once(": ").unwrap();
        headers.insert(
            match key {
                "Cookie" => reqwest::header::COOKIE,
                "User-Agent" => reqwest::header::USER_AGENT,
                _ => {
                    println!("Unknown header key: {}", key);
                    continue;
                }
            },
            value.parse().unwrap(),
        );
    }
    headers
}

#[test]
fn advanced_like() {
    use tokio::runtime::Runtime;
    Runtime::new().unwrap().block_on(async {
        let ytm = YoutubeMusicInstance::new(get_headers())
            .await
            .unwrap();
        println!("{}", ytm.compute_sapi_hash());
        let search = ytm.get_library(0, &Endpoint::MusicLibraryLanding).await.unwrap();
        assert_eq!(search.is_empty(), false);
        println!("{:?}", search[1]);
        println!("{:?}", ytm.get_playlist(&search[1], 0).await.unwrap());
    });
}

#[test]
fn advanced_test() {
    use tokio::runtime::Runtime;
    Runtime::new().unwrap().block_on(async {
        let ytm = YoutubeMusicInstance::new(get_headers())
            .await
            .unwrap();
        let search = ytm.search("j'ai la danse qui va avec", 0).await.unwrap();
        assert_eq!(search.videos.is_empty(), false);
        assert_eq!(search.playlists.is_empty(), false);
        let playlist_contents = ytm.get_playlist(&search.playlists[1], 0).await.unwrap();
        println!("{:?}", playlist_contents);
    });
}

#[test]
fn home_test() {
    use tokio::runtime::Runtime;
    Runtime::new().unwrap().block_on(async {
        let ytm = YoutubeMusicInstance::new(get_headers())
            .await
            .unwrap();
        let search = ytm.get_home(0).await.unwrap();
        println!("{:?}", search.playlists);
        assert_eq!(search.playlists.is_empty(), false);
        let playlist_contents = ytm.get_playlist(&search.playlists[0], 0).await.unwrap();
        println!("{:?}", playlist_contents);
    });
}

#[derive(Debug, Clone, PartialOrd, Eq, Ord, PartialEq, Hash, Serialize, Deserialize)]
pub struct YoutubeMusicPlaylistRef {
    pub name: String,
    pub subtitle: String,
    pub browse_id: String,
}

pub struct YoutubeMusicInstance {
    sapisid: String,
    innertube_api_key: String,
    client_version: String,
    cookies: String,
}

impl YoutubeMusicInstance {
    pub async fn from_header_file(path: &Path) -> Result<Self> {
        let mut headers = HeaderMap::new();
        for header in tokio::fs::read_to_string(path)
            .await
            .map_err(YoutubeMusicError::IoError)?
            .lines()
        {
            if let Some((key, value)) = header.split_once(": ") {
                headers.insert(
                    match key.to_lowercase().as_str() {
                        "cookie" => reqwest::header::COOKIE,
                        "user-agent" => reqwest::header::USER_AGENT,
                        _ => {
                            #[cfg(test)]
                            println!("Unknown header key: {key}");
                            continue;
                        }
                    },
                    value.parse().unwrap(),
                );
            }
        }
        if !headers.contains_key(reqwest::header::COOKIE) {
            return Err(YoutubeMusicError::InvalidHeaders);
        }
        if !headers.contains_key(reqwest::header::USER_AGENT) {
            headers.insert(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (X11; Linux x86_64; rv:108.0) Gecko/20100101 Firefox/108.0"
                    .parse()
                    .unwrap(),
            );
        }
        Self::new(headers).await
    }

    pub async fn new(headers: HeaderMap) -> Result<Self> {
        trace!("Creating new YoutubeMusicInstance");
        let rest_client = reqwest::ClientBuilder::default()
            .default_headers(headers.clone())
            .build()
            .map_err(YoutubeMusicError::RequestError)?;
        trace!("Fetching YoutubeMusic homepage");
        let response: String = rest_client
            .get(YTM_DOMAIN)
            .headers(headers.clone())
            .send()
            .await
            .map_err(YoutubeMusicError::RequestError)?
            .text()
            .await
            .map_err(YoutubeMusicError::RequestError)?;
        trace!("Fetched");

        if response.contains("<base href=\"https://accounts.google.com/v3/signin/\">")
            || response.contains("<base href=\"https://consent.youtube.com/\">")
        {
            error!("Need to login");
            return Err(YoutubeMusicError::NeedToLogin);
        }
        trace!("Parsing cookies");
        let cookies = headers
            .get("Cookie")
            .ok_or(YoutubeMusicError::NoCookieAttribute)?
            .to_str()
            .map_err(|_| YoutubeMusicError::InvalidCookie)?
            .to_string();
        let sapisid = cookies
            .between("SAPISID=", ";")
            .ok_or_else(|| YoutubeMusicError::NoSapsidInCookie)?;
        trace!("Cookies parsed! SAPISID: {}", sapisid);
        let innertube_api_key = response
            .between("INNERTUBE_API_KEY\":\"", "\"")
            .ok_or_else(|| YoutubeMusicError::CantFindInnerTubeApiKey(response.to_string()))?;
        trace!("Innertube API key: {}", innertube_api_key);
        let client_version = response
            .between("INNERTUBE_CLIENT_VERSION\":\"", "\"")
            .ok_or_else(|| {
                YoutubeMusicError::CantFindInnerTubeClientVersion(response.to_string())
            })?;
        trace!("Innertube client version: {}", client_version);
        Ok(Self {
            sapisid: sapisid.to_string(),
            innertube_api_key: innertube_api_key.to_string(),
            client_version: client_version.to_string(),
            cookies,
        })
    }
    fn compute_sapi_hash(&self) -> String {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let timestamp = since_the_epoch.as_secs();
        let mut hasher = Sha1::new();
        hasher.update(format!("{timestamp} {} {YTM_DOMAIN}", self.sapisid));
        let result = hasher.finalize();
        let mut hex = String::with_capacity(40);
        for byte in result {
            hex.push_str(&format!("{byte:02x}"));
        }
        trace!("Computed SAPI Hash{timestamp}_{hex}");
        format!("{timestamp}_{hex}")
    }
    async fn browse_continuation(
        &self,
        continuation: &Continuation,
        continuations: bool,
    ) -> Result<(Value, Vec<Continuation>)> {
        let playlist_json: Value =
            serde_json::from_str(&self.browse_continuation_raw(continuation).await?)
                .map_err(YoutubeMusicError::SerdeJson)?;
        debug!("Browse continuation response: {playlist_json}");
        if playlist_json.get("error").is_some() {
            error!("Error in browse_continuation");
            error!("{:?}", playlist_json);
            return Err(YoutubeMusicError::YoutubeMusicError(playlist_json));
        }
        let continuation = if continuations {
            from_json(&playlist_json, get_continuation)?
        } else {
            Vec::new()
        };
        Ok((playlist_json, continuation))
    }
    async fn browse_continuation_raw(
        &self,
        Continuation {
            continuation,
            click_tracking_params,
        }: &Continuation,
    ) -> Result<String> {
        trace!("Browse continuation {continuation}");
        let url = format!(
            "https://music.youtube.com/youtubei/v1/browse?ctoken={continuation}&continuation={continuation}&type=next&itct={click_tracking_params}&key={}&prettyPrint=false",
            self.innertube_api_key
        );
        let body = format!(
            r#"{{"context":{{"client":{{"clientName":"WEB_REMIX","clientVersion":"{}"}}}}}}"#,
            self.client_version
        );
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("SAPISIDHASH {}", self.compute_sapi_hash()),
            )
            .header("X-Origin", "https://music.youtube.com")
            .header("Cookie", &self.cookies)
            .body(body)
            .send()
            .await
            .map_err(YoutubeMusicError::RequestError)?
            .text()
            .await
            .map_err(YoutubeMusicError::RequestError)
    }
    async fn browse_raw(
        &self,
        endpoint_route: &str,
        endpoint_key: &str,
        endpoint_param: &str,
    ) -> Result<String> {
        trace!("Browse {endpoint_route}");
        let url = format!(
            "https://music.youtube.com/youtubei/v1/{endpoint_route}?key={}&prettyPrint=false",
            self.innertube_api_key
        );
        let body = format!(
            r#"{{"context":{{"client":{{"clientName":"WEB_REMIX","clientVersion":"{}"}}}},"{endpoint_key}":"{endpoint_param}"}}"#,
            self.client_version
        );
        reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("SAPISIDHASH {}", self.compute_sapi_hash()),
            )
            .header("X-Origin", "https://music.youtube.com")
            .header("Cookie", &self.cookies)
            .body(body)
            .send()
            .await
            .map_err(YoutubeMusicError::RequestError)?
            .text()
            .await
            .map_err(YoutubeMusicError::RequestError)
    }
    async fn browse(
        &self,
        endpoint: &Endpoint,
        continuations: bool,
    ) -> Result<(serde_json::Value, Vec<Continuation>)> {
        let playlist_json: Value = serde_json::from_str(
            &self
                .browse_raw(
                    &endpoint.get_route(),
                    &endpoint.get_key(),
                    &endpoint.get_param(),
                )
                .await?,
        )
        .map_err(YoutubeMusicError::SerdeJson)?; 
        debug!("Browse response: {playlist_json}");
        if playlist_json.get("error").is_some() {
            error!("Error in browse");
            error!("{:?}", playlist_json);
            return Err(YoutubeMusicError::YoutubeMusicError(playlist_json));
        }
        let continuation = if continuations {
            from_json(&playlist_json, get_continuation)?
        } else {
            Vec::new()
        };
        Ok((playlist_json, continuation))
    }
    pub async fn get_library(
        &self,
        endpoint: &Endpoint,
        mut n_continuations: usize,
    ) -> Result<Vec<YoutubeMusicPlaylistRef>> {
        let (library_json, mut continuations) = self
            .browse(endpoint, n_continuations > 0)
            .await?;
        trace!("Fetched library");
        debug!("Library response: {library_json}");
        debug!("Continuations: {continuations:?}");
        let mut library = from_json(&library_json, get_playlist)?;
        debug!("Library: {library:?}");
        while let Some(continuation) = continuations.pop() {
            n_continuations -= 1;
            trace!("Fetching continuation {continuation:?}");
            let (library_json, new_continuations) = self
                .browse_continuation(&continuation, (n_continuations - 1) > 0)
                .await?;
            debug!("Library response: {library_json}");
            continuations.extend(new_continuations);
            let new_library = from_json(&library_json, get_playlist)?;
            trace!("Fetched {} playlists",new_library.len());
            debug!("Library response: {library_json}");
            library.extend(new_library);
            if n_continuations == 0 {
                break;
            }
        }

        Ok(library)
    }
    pub async fn get_playlist(
        &self,
        playlist: &YoutubeMusicPlaylistRef,
        n_continuations: usize,
    ) -> Result<Vec<YoutubeMusicVideoRef>> {
        self.get_playlist_raw(&playlist.browse_id, n_continuations)
            .await
    }
    pub async fn get_playlist_raw(
        &self,
        playlist_id: &str,
        mut n_continuations: usize,
    ) -> Result<Vec<YoutubeMusicVideoRef>> {
        let (playlist_json, mut continuations) = self
            .browse(
                &Endpoint::Playlist(playlist_id.to_string()),
                n_continuations > 0,
            )
            .await?;
        trace!("Fetched playlist");
        debug!("Playlist response: {playlist_json}");
        debug!("Continuations: {continuations:?}");
        let mut videos = parse_playlist(&playlist_json)?;

        debug!("Videos: {videos:?}");

        while let Some(continuation) = continuations.pop() {
            n_continuations -= 1;
            trace!("Fetching continuation {continuation:?}");
            let (playlist_json, new_continuations) = self
                .browse_continuation(&continuation, (n_continuations - 1) > 0)
                .await?;
            debug!("Playlist response: {playlist_json}");
            continuations.extend(new_continuations);
            let new_videos = parse_playlist(&playlist_json)?;
            trace!("Fetched {} videos",new_videos.len());
            debug!("Playlist response: {playlist_json}");
            videos.extend(new_videos);
            if n_continuations == 0 {
                break;
            }
        }

        Ok(videos)
    }
    pub async fn search(
        &self,
        search_query: &str,
        mut n_continuations: usize,
    ) -> Result<SearchResults> {
        let (search_json, mut continuations) = self
            .browse(&Endpoint::Search(search_query.to_string()), false)
            .await?;
        debug!("Search response: {search_json}");
        let mut videos = from_json(&search_json, get_video)?;
        debug!("Videos: {videos:?}");
        let mut playlists = from_json(&search_json, get_playlist_search)?;
        debug!("Playlists: {playlists:?}");

        while let Some(continuation) = continuations.pop() {
            n_continuations -= 1;
            trace!("Fetching continuation {continuation:?}");
            let (search_json, new_continuations) =
                self.browse_continuation(&continuation, false).await?;
            trace!("Search response: {search_json}");
            continuations.extend(new_continuations);
            let new_videos = from_json(&search_json, get_video)?;
            debug!("Videos: {videos:?}");
            let new_playlists = from_json(&search_json, get_playlist_search)?;
            debug!("Playlists: {playlists:?}");
            videos.extend(new_videos);
            playlists.extend(new_playlists);
            if n_continuations == 0 {
                break;
            }
        }

        Ok(SearchResults { videos, playlists })
    }

    pub async fn get_home(&self, mut n_continuations: usize) -> Result<SearchResults> {
        let (home_json, mut continuations) = self
            .browse(&Endpoint::MusicHome, n_continuations > 0)
            .await?;
        debug!("Home response: {home_json}");
        let mut videos = from_json(&home_json, get_video)?;
        debug!("Videos: {videos:?}");
        let mut playlists = from_json(&home_json, get_playlist)?;
        debug!("Playlists: {playlists:?}");

        while let Some(continuation) = continuations.pop() {
            n_continuations -= 1;
            trace!("Fetching continuation {continuation:?}");
            let (home_json, new_continuations) = self
                .browse_continuation(&continuation, n_continuations > 0)
                .await?;
            debug!("Home response: {home_json}");
            continuations.extend(new_continuations);
            let new_videos = from_json(&home_json, get_video)?;
            debug!("Videos: {videos:?}");
            let new_playlists = from_json(&home_json, get_playlist)?;
            debug!("Playlists: {playlists:?}");
            videos.extend(new_videos);
            playlists.extend(new_playlists);
            if n_continuations == 0 {
                break;
            }
        }
        Ok(SearchResults { playlists, videos })
    }
}

fn parse_playlist(playlist_json: &Value) -> Result<Vec<YoutubeMusicVideoRef>> {
    let mut videos = from_json(playlist_json, get_video)?;
    let info = extract_playlist_info(playlist_json);
    for mut video in from_json(playlist_json, get_video_from_album)? {
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

#[derive(Debug, Clone, PartialOrd, Eq, Ord, PartialEq, Hash)]
pub struct SearchResults {
    pub videos: Vec<YoutubeMusicVideoRef>,
    pub playlists: Vec<YoutubeMusicPlaylistRef>,
}

#[derive(Debug, Clone, PartialOrd, Eq, Ord, PartialEq, Hash)]
pub enum Endpoint {
    MusicLikedPlaylists,
    MusicHome,
    MusicLibraryLanding,
    Playlist(String),
    Search(String),
}

impl Endpoint {
    fn get_key(&self) -> String {
        match self {
            Endpoint::MusicLikedPlaylists => "browseId".to_owned(),
            Endpoint::MusicLibraryLanding => "browseId".to_owned(),
            Endpoint::Playlist(_) => "browseId".to_owned(),
            Endpoint::MusicHome => "browseId".to_owned(),
            Endpoint::Search(_) => "query".to_owned(),
        }
    }
    fn get_param(&self) -> String {
        match self {
            Endpoint::MusicLikedPlaylists => "FEmusic_liked_playlists".to_owned(),
            Endpoint::MusicLibraryLanding => "FEmusic_library_landing".to_owned(),
            Endpoint::Playlist(id) => id.to_owned(),
            Endpoint::Search(query) => query.to_owned(),
            Endpoint::MusicHome => "FEmusic_home".to_owned(),
        }
    }
    fn get_route(&self) -> String {
        match self {
            Endpoint::MusicLikedPlaylists => "browse".to_owned(),
            Endpoint::MusicLibraryLanding => "browse".to_owned(),
            Endpoint::Playlist(_) => "browse".to_owned(),
            Endpoint::Search(_) => "search".to_owned(),
            Endpoint::MusicHome => "browse".to_owned(),
        }
    }
}

#[derive(Debug)]
pub enum YoutubeMusicError {
    RequestError(reqwest::Error),
    Other(String),
    NoCookieAttribute,
    NoSapsidInCookie,
    InvalidCookie,
    NeedToLogin,
    CantFindInnerTubeApiKey(String),
    CantFindInnerTubeClientVersion(String),
    CantFindVisitorData(String),
    SerdeJson(serde_json::Error),
    IoError(std::io::Error),
    YoutubeMusicError(Value),
    InvalidHeaders,
}
