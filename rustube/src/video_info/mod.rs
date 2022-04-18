//! All the types, that hold video information.

use serde::{Deserialize, Serialize};
use serde_with::{json::JsonString, serde_as};

use player_response::PlayerResponse;

pub mod player_response;

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct VideoInfo {
    #[serde_as(deserialize_as = "JsonString")]
    pub player_response: PlayerResponse,
    #[serde(rename = "adaptive_fmts")]
    pub adaptive_fmts_raw: Option<String>,

    #[serde(skip)]
    pub is_age_restricted: bool,
}
