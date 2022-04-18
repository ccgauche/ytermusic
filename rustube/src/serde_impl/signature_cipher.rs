use serde::{Deserialize, Deserializer};
use serde::de::{Error, Unexpected};
use url::Url;

use crate::video_info::player_response::streaming_data::SignatureCipher;

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<SignatureCipher, D::Error>
    where
        D: serde_with::serde::Deserializer<'de> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EitherUrlOrCipher {
        url: Option<Url>,
        #[serde(default)]
        #[serde(alias = "Cipher")]
        #[serde(deserialize_with = "deserialize_signature_cipher")]
        signature_cipher: Option<SignatureCipher>,
    }

    let both: EitherUrlOrCipher = serde_with::serde::Deserialize::deserialize(deserializer)?;
    match (both.url, both.signature_cipher) {
        (Some(url), None) => Ok(SignatureCipher { url, s: None }),
        (None, Some(s)) => Ok(s),
        (None, None) => Err(serde_with::serde::de::Error::missing_field("signatureCipher")),
        (Some(_), Some(_)) => Err(serde_with::serde::de::Error::duplicate_field("url")),
    }
}

fn deserialize_signature_cipher<'de, D>(deserializer: D) -> Result<Option<SignatureCipher>, <D as Deserializer<'de>>::Error> where
    D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?;
    serde_qs::from_str::<SignatureCipher>(s.as_str())
        .map(Some)
        .map_err(|_| D::Error::invalid_value(
            Unexpected::Str(s.as_str()),
            &"a valid SignatureCipher",
        ))
}
