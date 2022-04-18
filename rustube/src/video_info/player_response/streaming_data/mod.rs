use std::ops::Range;

use chrono::{DateTime, Utc};
use mime::Mime;
use serde::{Deserialize, Serialize};
use serde_with::{DefaultOnNull, json::JsonString};
use serde_with::serde_as;
use url::Url;

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamingData {
    // todo: remove the field adaptive_formats, and deserialize all formats into formats
    #[serde(default)]
    pub adaptive_formats: Vec<RawFormat>,
    #[serde_as(as = "JsonString")]
    pub expires_in_seconds: u64,
    #[serde(default)]
    pub formats: Vec<RawFormat>,
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RawFormat {
    #[serde(rename = "type")]
    pub format_type: Option<FormatType>,
    #[serde(default)]
    #[serde_as(as = "Option<JsonString>")]
    pub approx_duration_ms: Option<u64>,
    pub audio_channels: Option<u8>,
    pub audio_quality: Option<AudioQuality>,
    #[serde(default)]
    #[serde_as(as = "Option<DefaultOnNull<JsonString>>")]
    pub audio_sample_rate: Option<u64>,
    pub average_bitrate: Option<u64>,
    pub bitrate: Option<u64>,
    pub color_info: Option<ColorInfo>,
    #[serde(default)]
    #[serde_as(as = "Option<JsonString>")]
    pub content_length: Option<u64>,
    #[serde(default)]
    pub fps: u8,
    pub height: Option<u64>,
    pub high_replication: Option<bool>,
    #[serde(default)]
    #[serde_as(as = "Option<crate::serde_impl::range::Range>")]
    pub index_range: Option<Range<u64>>,
    #[serde(default)]
    #[serde_as(as = "Option<crate::serde_impl::range::Range>")]
    pub init_range: Option<Range<u64>>,
    pub itag: u64,
    #[serde(with = "crate::serde_impl::unix_timestamp_micro_secs")]
    pub last_modified: DateTime<Utc>,
    pub loudness_db: Option<f64>,
    #[serde(with = "crate::serde_impl::mime_type")]
    pub mime_type: MimeType,
    pub projection_type: ProjectionType,
    pub quality: Quality,
    pub quality_label: Option<QualityLabel>,
    #[serde(flatten, deserialize_with = "crate::serde_impl::signature_cipher::deserialize")]
    pub signature_cipher: SignatureCipher,
    pub width: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct SignatureCipher {
    pub url: Url,
    pub s: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum FormatType {
    #[serde(rename = "FORMAT_STREAM_TYPE_OTF")]
    Otf,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ColorInfo {
    pub primaries: Option<ColorInfoPrimary>,
    pub transfer_characteristics: TransferCharacteristics,
    pub matrix_coefficients: Option<MatrixCoefficients>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum ColorInfoPrimary {
    #[serde(rename = "COLOR_PRIMARIES_BT709")]
    BT709,
    #[serde(rename = "COLOR_PRIMARIES_BT2020")]
    BT2020,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum TransferCharacteristics {
    #[serde(rename = "COLOR_TRANSFER_CHARACTERISTICS_BT709")]
    BT709,
    #[serde(rename = "COLOR_TRANSFER_CHARACTERISTICS_SMPTEST2084")]
    SMPTEST2084,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum MatrixCoefficients {
    #[serde(rename = "COLOR_MATRIX_COEFFICIENTS_BT709")]
    BT709,
    #[serde(rename = "COLOR_MATRIX_COEFFICIENTS_BT2020_NCL")]
    BT2020NCL,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MimeType {
    pub mime: Mime,
    // todo: make codec an enum 
    pub codecs: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProjectionType {
    Rectangular,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AudioQuality {
    #[serde(rename = "AUDIO_QUALITY_LOW", alias = "low")]
    Low,
    #[serde(rename = "AUDIO_QUALITY_MEDIUM", alias = "medium")]
    Medium,
    #[serde(rename = "AUDIO_QUALITY_HIGH", alias = "high")]
    High,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Tiny,
    Small,
    Medium,
    Large,
    Highres,
    Hd720,
    Hd1080,
    Hd1440,
    Hd2160,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum QualityLabel {
    #[serde(rename = "144p")]
    P144,
    #[serde(rename = "144p HDR")]
    P144HDR,
    #[serde(rename = "144p60 HDR")]
    P144Hz60HDR,
    #[serde(rename = "240p")]
    P240,
    #[serde(rename = "240p HDR")]
    P240HDR,
    #[serde(rename = "240p60 HDR")]
    P240Hz60HDR,
    #[serde(rename = "360p")]
    P360,
    #[serde(rename = "360p HDR")]
    P360HDR,
    #[serde(rename = "360p60")]
    P360Hz60,
    #[serde(rename = "360p60 HDR")]
    P360Hz60HDR,
    #[serde(rename = "480p")]
    P480,
    #[serde(rename = "480p HDR")]
    P480HDR,
    #[serde(rename = "480p60")]
    P480Hz60,
    #[serde(rename = "480p60 HDR")]
    P480Hz60HDR,
    #[serde(rename = "720p")]
    P720,
    #[serde(rename = "720p50")]
    P720Hz50,
    #[serde(rename = "720p60")]
    P720Hz60,
    #[serde(rename = "720p60 HDR")]
    P720Hz60HDR,
    #[serde(rename = "1080p")]
    P1080,
    #[serde(rename = "1080p50")]
    P1080Hz50,
    #[serde(rename = "1080p60")]
    P1080Hz60,
    #[serde(rename = "1080p60 HDR")]
    P1080Hz60HDR,
    #[serde(rename = "1440p")]
    P1440,
    #[serde(rename = "1440p60")]
    P1440Hz60,
    #[serde(rename = "1440p60 HDR")]
    P1440Hz60HDR,
    #[serde(rename = "2160p")]
    P2160,
    #[serde(rename = "2160p60")]
    P2160Hz60,
    #[serde(rename = "2160p60 HDR")]
    P2160Hz60HDR,
    #[serde(rename = "4320p")]
    P4320,
    #[serde(rename = "4320p60")]
    P4320Hz60,
    #[serde(rename = "4320p60 HDR")]
    P4320Hz60HDR,
}
