use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, json::JsonString, serde_as, SerializeAs};

#[serde_as]
#[derive(Deserialize, Serialize)]
pub(crate) struct Range {
    #[serde_as(as = "JsonString")]
    start: u64,
    #[serde_as(as = "JsonString")]
    end: u64,
}

impl<'de> DeserializeAs<'de, std::ops::Range<u64>> for Range {
    fn deserialize_as<D>(deserializer: D) -> Result<std::ops::Range<u64>, D::Error>
        where
            D: Deserializer<'de> {
        let range = Range::deserialize(deserializer)?;
        Ok(std::ops::Range { start: range.start, end: range.end })
    }
}

impl SerializeAs<std::ops::Range<u64>> for Range {
    fn serialize_as<S>(&std::ops::Range { start, end }: &std::ops::Range<u64>, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where
        S: Serializer {
        Range { start, end }.serialize(serializer)
    }
}
