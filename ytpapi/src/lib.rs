#![feature(try_blocks)]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    Client, ClientBuilder,
};

use serde_json::Value;
use string_utils::StringUtils;
use structs::{playlists_from_json, search_results, videos_from_playlist};

pub use structs::{Playlist, Video};

const YTM_DOMAIN: &str = "https://music.youtube.com";

mod string_utils;
mod structs;

/* fn sapisid_from_cookie(string: &str) -> Option<String> {
    string.find("__Secure-3PAPISID=").map(|i| {
        let string = &string[i + "__Secure-3PAPISID=".len()..];
        let string = &string[..string.find(';').unwrap()];
        string.to_owned()
    })
} */

/* fn get_authorization(auth: &str) -> Option<String> {
    let mut hasher = Sha1::new();
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards!")
        .as_secs();

    hasher.update(format!("{unix} + ' ' + {auth}").as_bytes());

    Some(format!("SAPISIDHASH {unix}_{:x}", hasher.finalize()))
} */

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
    let playlist = playlists_from_json(&extract_json(&response)?)?;
    response
        .between("VISITOR_DATA\":\"", "\"")
        .to_owned_()
        .map(|x| (x, playlist))
        .ok_or_else(|| Error::InvalidHTMLFile(response.to_string()))
}

fn extract_json(string: &str) -> Result<String, Error> {
    let json = string
        .between(
            "initialData.push({path: '\\/browse', params: J",
            "'});ytcfg.set({",
        )
        .after("data: '")
        .to_owned_()
        .ok_or_else(|| Error::InvalidHTMLFile(string.to_string()))?;
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
        .ok_or_else(|| Error::InvalidHTMLFile(string.to_string()))?;
    unescape(&json)
}

/* const YTM_BASE_API: &'static str = "https://music.youtube.com/youtubei/v1/";
const YTM_PARAMS: &'static str = "?alt=json&key=AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30"; */

pub struct YTApi {
    /* headers: HeaderMap, */
    /* sapi: String, */
    client: Client,
    playlists: Vec<Playlist>,
}

#[derive(Debug)]
pub enum Error {
    InvalidHTMLFile(String),
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
    InvalidHeaderValue(InvalidHeaderValue),
    InvalidHeaderName(InvalidHeaderName),
    InvalidJsonCantFind(String, String),
    InvalidHeaderFormat(PathBuf, String),
    Io(std::io::Error),
    InvalidEscapedSequence(String),
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
        Self::from_headers(&headers).await
    }
    pub async fn update_playlists(&mut self) -> Result<(), Error> {
        self.playlists = get_visitor_id(&self.client, &HeaderMap::new()).await?.1;
        Ok(())
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
        let json: Value = serde_json::from_str(&k).map_err(Error::SerdeJson)?;
        std::fs::write("search.json", k).map_err(Error::Io)?;
        search_results(json)
    }
    pub fn playlists(&self) -> &Vec<Playlist> {
        &self.playlists
    }
    pub async fn from_headers_map(mut headers: HeaderMap) -> Result<Self, Error> {
        let (xgoo, playlists) = get_visitor_id(&Client::new(), &headers).await?;
        headers.insert(
            "x-goog-visitor-id",
            HeaderValue::from_str(&xgoo).map_err(Error::InvalidHeaderValue)?,
        );
        /* let sapi = sapisid_from_cookie(headers.get("cookie").unwrap().to_str().unwrap()).unwrap(); */
        Ok(Self {
            /* sapi, */
            client: ClientBuilder::default()
                .cookie_store(true)
                .default_headers(headers)
                .build()
                .map_err(Error::Reqwest)?,
            /* headers: headers, */
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
    /* fn browse_home(&self) {
        self.send_request(
            "browse",
            serde_json::from_str("{\"browseId\":\"FEmusic_home\"}").unwrap(),
        )
    } */
    pub async fn browse_playlist(&self, playlistid: &str) -> Result<Vec<Video>, Error> {
        videos_from_playlist(&extract_json(
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
        )?)
    }
    /* fn send_request(&self, endpoint: &str, body: serde_json::Map<String, Value>) {
        let mut context: serde_json::Map<String, Value> = serde_json::from_str(
            r#"{"context":{"client":{"hl":"en","clientName":"WEB_REMIX","clientVersion":"0.1"},"user":{}}}"#,
        ).unwrap();
        context.extend(body.into_iter());
        let body = serde_json::to_string(&context).unwrap();
        let origin = self
            .headers
            .get("origin")
            .or_else(|| self.headers.get("x-origin"))
            .unwrap()
            .to_str()
            .unwrap();
        let reponse = self
            .client
            .post(format!("{YTM_BASE_API}{endpoint}{YTM_PARAMS}"))
            .body(body)
            .header("content-type", "application/json")
            .header(
                "Authorization",
                HeaderValue::from_str(&format!("{} {origin}", self.sapi)).unwrap(),
            )
            .send()
            .unwrap()
            .text()
            .unwrap();

        std::fs::write("reponse.html", reponse).unwrap();
    } */
}
