use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{json::JsonString, serde_as};

use crate::IdBuf;
use crate::video_info::player_response::video_details::Thumbnail;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::large_enum_variant)] // todo
pub enum PlayabilityStatus {
    #[serde(rename_all = "camelCase")]
    Ok {
        playable_in_embed: bool,
        miniplayer: Option<MiniPlayer>,
        #[serde(default)]
        messages: Vec<String>,
        context_params: String,
    },
    #[serde(rename_all = "camelCase")]
    Unplayable {
        #[serde(default)]
        messages: Vec<String>,
        reason: String,
        error_screen: Option<ErrorScreen>,
        playable_in_embed: Option<bool>,
        miniplayer: Option<MiniPlayer>,
        context_params: String,
    },
    #[serde(rename_all = "camelCase")]
    LoginRequired {
        #[serde(default)]
        messages: Vec<String>,
        error_screen: Option<ErrorScreen>,
        desktop_legacy_age_gate_reason: Option<i64>,
        context_params: String,
    },
    #[serde(rename_all = "camelCase")]
    LiveStreamOffline {
        reason: String,
        playable_in_embed: bool,
        live_streamability: LiveStreamAbility,
        miniplayer: Option<MiniPlayer>,
        context_params: String,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        reason: String,
        error_screen: Option<ErrorScreen>,
        context_params: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct MiniPlayer {
    pub miniplayer_renderer: Option<MiniplayerRenderer>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct MiniplayerRenderer {
    pub playback_mode: PlaybackMode,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackMode {
    #[serde(rename = "PLAYBACK_MODE_ALLOW")]
    Allow,
    #[serde(rename = "PLAYBACK_MODE_PAUSED_ONLY")]
    PausedOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ErrorScreen {
    pub player_error_message_renderer: PlayerErrorMessageRenderer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct PlayerErrorMessageRenderer {
    pub subreason: Option<Reason>,
    pub reason: Reason,
    pub proceed_button: Option<ProceedButton>,
    #[serde(rename = "thumbnail")]
    #[serde(serialize_with = "Thumbnail::serialize_vec")]
    #[serde(deserialize_with = "Thumbnail::deserialize_vec")]
    pub thumbnails: Vec<Thumbnail>,
    pub icon: Icon,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Reason {
    #[serde(alias = "simpleText")]
    pub text: Option<String>,
    #[serde(default)]
    pub runs: Vec<Reason>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ProceedButton {
    pub button_renderer: ButtonRenderer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ButtonRenderer {
    pub style: ButtonRendererStyle,
    pub size: ButtonRendererSize,
    pub is_disabled: bool,
    pub text: Reason,
    pub navigation_endpoint: NavigationEndpoint,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum ButtonRendererStyle {
    #[serde(rename = "STYLE_OVERLAY")]
    Overlay,
    #[serde(rename = "STYLE_PRIMARY")]
    Primary,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum ButtonRendererSize {
    #[serde(rename = "SIZE_DEFAULT")]
    Default
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct NavigationEndpoint {
    #[serde(flatten)]
    pub endpoint: Endpoint,
    pub sign_in_endpoint: Option<SignInEndpoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub click_tracking_params: String,
    pub command_metadata: CommandMetadata,

    // todo: there may be an extra field `url_endpoint: Option<UrlEndpoint>`
    // currently this field is only used in NextEndpoint and therefore not exposed in this struct
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct CommandMetadata {
    pub web_command_metadata: WebCommandMetadata,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct WebCommandMetadata {
    /// a relative url
    pub url: String,
    pub web_page_type: WebPageType,
    pub root_ve: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum WebPageType {
    #[serde(rename = "WEB_PAGE_TYPE_UNKNOWN")]
    Unknown
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct SignInEndpoint {
    pub next_endpoint: NextEndpoint,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct NextEndpoint {
    #[serde(flatten)]
    pub endpoint: Endpoint,
    pub url_endpoint: UrlEndpoint,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct UrlEndpoint {
    /// a relative url
    url: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    pub icon_type: IconType,
}

#[derive(Clone, Copy, Debug, derive_more::Display, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IconType {
    ErrorOutline
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamAbility {
    live_streamability_renderer: LiveStreamAbilityRenderer,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamAbilityRenderer {
    video_id: IdBuf,
    offline_slate: OfflineSlate,
    #[serde_as(as = "JsonString")]
    poll_delay_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct OfflineSlate {
    live_stream_offline_slate_renderer: LiveStreamOfflineSlateRenderer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamOfflineSlateRenderer {
    #[serde(with = "crate::serde_impl::unix_timestamp_secs")]
    scheduled_start_time: DateTime<Utc>,
    main_text: Reason,
    subtitle_text: Reason,
    #[serde(rename = "thumbnail")]
    #[serde(serialize_with = "Thumbnail::serialize_vec")]
    #[serde(deserialize_with = "Thumbnail::deserialize_vec")]
    pub thumbnails: Vec<Thumbnail>,
}
