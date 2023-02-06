use serenity::prelude::GatewayIntents;

use crate::prelude::*;

pub mod anchor;
pub mod custom;
pub mod format;
pub mod logger;
pub mod stored;
pub mod traits;

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub const IS_DEV: bool = cfg!(debug_assertions);
pub const INTENTS: GatewayIntents = GatewayIntents::non_privileged();
pub const BOT_COLOR: Color = Color::BLITZ_BLUE;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error decoding value: {0}")]
    Decoding(#[from] rmp_serde::decode::Error),
    #[error("error encoding value: {0}")]
    Encoding(#[from] rmp_serde::encode::Error),
    #[error("error loading environment: {0}")]
    Environment(#[from] dotenvy::Error),
    #[error("{0}")]
    Infallible(#[from] std::convert::Infallible),
    #[error("{0}")]
    Serenity(#[from] serenity::Error),
    #[error("io error: {0}")]
    StandardIO(#[from] std::io::Error),
    #[error("error fetching variable: {0}")]
    Variable(#[from] std::env::VarError),
    #[error("{0}")]
    Custom(String),
}

impl Error {
    pub(crate) fn custom(error: impl Into<String>) -> Self {
        Self::Custom(error.into())
    }
    pub(crate) fn custom_wrap<T>(error: impl Into<String>) -> Result<T> {
        Err(Self::Custom(error.into()))
    }
}

impl Default for Error {
    fn default() -> Self {
        err!("unknown error")
    }
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)+) => {
        Error::custom(format_args!($($arg)+).to_string())
    };
}
#[macro_export]
macro_rules! err_wrap {
    ($($arg:tt)+) => {
        Error::custom_wrap(format_args!($($arg)+).to_string())
    };
}

pub fn get_token() -> Result<String> {
    let key = if IS_DEV { "DEV_TOKEN" } else { "PRD_TOKEN" };

    std::env::var(key).map_err(Into::into)
}

pub fn get_dev_guild() -> Result<GuildId> {
    let raw = std::env::var("DEV_GUILD")?;
    let Ok(id) = raw.parse() else {
        return err_wrap!("invalid guild identifier");
    };

    Ok(GuildId::new(id))
}

pub fn get_error_guild() -> Result<GuildId> {
    let raw = std::env::var("ERROR_GUILD")?;
    let Ok(id) = raw.parse() else {
        return err_wrap!("invalid guild identifier");
    };

    Ok(GuildId::new(id))
}
pub fn get_error_channel() -> Result<ChannelId> {
    let raw = std::env::var("ERROR_CHANNEL")?;
    let Ok(id) = raw.parse() else {
        return err_wrap!("invalid channel identifier");
    };

    Ok(ChannelId::new(id))
}
