use crate::Format;

/// The [MsgPack](https://msgpack.io/index.html) data format.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MsgPack;

#[cfg(feature = "default-systems")]
impl crate::systems::FileFormat for MsgPack {
    #[inline]
    fn extension(&self) -> String { "pack".to_string() }
}

impl Format for MsgPack {
    type EncodeError = rmp_serde::encode::Error;
    type DecodeError = rmp_serde::decode::Error;

    #[inline]
    fn encode<T>(&self, value: &T) -> Result<Vec<u8>, Self::EncodeError>
    where
        T: serde::Serialize,
    {
        rmp_serde::to_vec(value)
    }

    #[inline]
    fn decode<T>(&self, bytes: &[u8]) -> Result<T, Self::DecodeError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        rmp_serde::from_slice(bytes)
    }
}

/// The [TOML](https://toml.io/en/) data format.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Toml;

#[cfg(feature = "default-systems")]
impl crate::systems::FileFormat for Toml {
    #[inline]
    fn extension(&self) -> String { "toml".to_string() }
}

impl Format for Toml {
    type EncodeError = toml::ser::Error;
    type DecodeError = toml::de::Error;

    #[inline]
    fn encode<T>(&self, value: &T) -> Result<Vec<u8>, Self::EncodeError>
    where
        T: serde::Serialize,
    {
        toml::to_string_pretty(value).map(String::into_bytes)
    }

    #[inline]
    fn decode<T>(&self, bytes: &[u8]) -> Result<T, Self::DecodeError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        toml::from_str(&String::from_utf8_lossy(bytes))
    }
}
