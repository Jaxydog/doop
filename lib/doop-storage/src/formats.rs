use serde::{Deserialize, Serialize};

use crate::Format;

/// The [MessagePack](https://msgpack.io/index.html) data format.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MsgPack;

impl Format for MsgPack {
    type EncodingError = rmp_serde::encode::Error;
    type DecodingError = rmp_serde::decode::Error;

    fn extension(&self) -> String {
        "pack".to_string()
    }

    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, Self::EncodingError> {
        rmp_serde::to_vec_named(value)
    }

    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::DecodingError> {
        rmp_serde::from_slice(bytes)
    }
}

/// The [JSON](https://www.json.org/json-en.html) data format.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Json;

impl Format for Json {
    type EncodingError = serde_json::Error;
    type DecodingError = serde_json::Error;

    fn extension(&self) -> String {
        "json".to_string()
    }

    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, Self::EncodingError> {
        serde_json::to_vec_pretty(value)
    }

    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::DecodingError> {
        serde_json::from_slice(bytes)
    }
}

/// The [TOML](https://toml.io/en/) data format.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Toml;

impl Format for Toml {
    type EncodingError = toml::ser::Error;
    type DecodingError = toml::de::Error;

    fn extension(&self) -> String {
        "toml".to_string()
    }

    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, Self::EncodingError> {
        toml::to_string_pretty(value).map(String::into_bytes)
    }

    fn decode<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8]) -> Result<T, Self::DecodingError> {
        toml::from_str(&String::from_utf8_lossy(bytes))
    }
}
