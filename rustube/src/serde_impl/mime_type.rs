use std::lazy::SyncLazy;
use std::str::FromStr;

use mime::Mime;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Unexpected};

use crate::TryCollect;
use crate::video_info::player_response::streaming_data::MimeType;

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<MimeType, <D as Deserializer<'de>>::Error> where
    D: Deserializer<'de> {
    static PATTERN: SyncLazy<Regex> = SyncLazy::new(||
        Regex::new(r#"(\w+/\w+);\scodecs="([a-zA-Z-0-9.,\s]*)""#).unwrap()
    );

    // deserializing into a &str gives back an error
    let s = String::deserialize(deserializer)?;

    let (mime_type, codecs) = PATTERN
        .captures(&s)
        .ok_or_else(|| D::Error::invalid_value(
            Unexpected::Str(&s),
            &"a valid mime type with the format <TYPE>/<SUBTYPE>",
        ))?
        .iter()
        // skip group 0, which is the whole match
        .skip(1)
        .try_collect()
        .and_then(|(m, c)| m.map(|m| c.map(|c| (m.as_str(), c.as_str()))))
        .flatten()
        .ok_or_else(|| D::Error::invalid_value(
            Unexpected::Str(&s),
            &"a valid mime type with the format <TYPE>/<SUBTYPE>",
        ))?;

    let mime = Mime::from_str(mime_type)
        .map_err(|_| D::Error::invalid_value(
            Unexpected::Str(mime_type),
            &r#"a valid mime type with the format `(\w+/\w+);\scodecs="([a-zA-Z-0-9.,\s]*)"`"#,
        ))?;

    let codecs = codecs
        .split(", ")
        .map(str::to_owned)
        .collect();

    Ok(MimeType {
        mime,
        codecs,
    })
}

pub(crate) fn serialize<S>(mime_type: &MimeType, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer {
    let mut s = format!(
        r#"{}/{}; codecs=""#,
        mime_type.mime.type_(),
        mime_type.mime.subtype(),
    );

    for codec in mime_type.codecs.iter() {
        s.push_str(codec);
        s.push(',');
        s.push(' ');
    }

    s.push('"');
    s.serialize(serializer)
}
