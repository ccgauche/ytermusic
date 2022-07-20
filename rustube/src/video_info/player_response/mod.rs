use std::sync::Arc;

use serde::{Deserialize, Serialize};

use playability_status::PlayabilityStatus;
use streaming_data::StreamingData;
use video_details::VideoDetails;

pub(crate) mod playability_status;
pub(crate) mod streaming_data;
pub(crate) mod video_details;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResponse {
    pub assets: Option<Assets>,
    // todo:
    // attestation: _,
    // auxiliaryUi: _,
    // captions: _,
    // cards: _,
    // endscreen: _,
    // messages: _,
    #[cfg(feature = "microformat")]
    #[doc(cfg(feature = "microformat"))]
    pub microformat: Option<Microformat>,
    pub playability_status: PlayabilityStatus,
    // playbackTracking: _,
    // playerConfig: _,
    // response_context: ResponseContext,
    // storyboards: _,
    pub streaming_data: Option<StreamingData>,
    pub video_details: Arc<VideoDetails>,
    pub tracking_params: String,
}

#[derive(
    Clone,
    Default,
    Debug,
    derive_more::Display,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct Assets {
    pub js: String,
}
