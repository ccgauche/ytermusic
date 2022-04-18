use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserializer, Serializer};
use serde::de::{Error, Unexpected};
use serde_with::{DeserializeAs, SerializeAs};
use serde_with::json::JsonString;

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, <D as Deserializer<'de>>::Error> where
    D: Deserializer<'de> {
    let seconds: i64 = JsonString::deserialize_as(deserializer)?;
    Utc
        .timestamp_opt(seconds, 0)
        .single()
        .ok_or_else(|| D::Error::invalid_value(
            Unexpected::Signed(seconds),
            &"a valid UNIX time stamp in microseconds",
        ))
}

pub(crate) fn serialize<S>(time: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer {
    let seconds: i64 = time.timestamp() * 1000;
    JsonString::serialize_as(&seconds, serializer)
}
