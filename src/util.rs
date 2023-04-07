use std::fmt::Display;
use std::sync::OnceLock;

use clap::Parser;
use serenity::all::{ChannelId, GuildId};
use serenity::model::Color;
use serenity::prelude::GatewayIntents;

use self::log::Logger;

/// Provides an interface for reading and writing data to / from the bot's
/// storage
pub mod data;
/// Provides an interface for log output to / from the console and file system
pub mod log;

/// Wraps a [`std::result::Result`], providing a defaulted error type `E`
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The bot's default branding color
pub const BOT_COLOR: Color = Color::new(0x27_89_CD);
/// The bot's gateway intents
pub const BOT_INTENTS: GatewayIntents =
    GatewayIntents::non_privileged().union(GatewayIntents::GUILD_MEMBERS);
/// Whether the bot is running in development mode
pub const DEV_BUILD: bool = cfg!(debug_assertions);

/// The bot process' command-line arguments
static ARGUMENTS: OnceLock<Arguments> = OnceLock::new();
/// The bot process' logger instance
static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Defines and contains the bot process' command-line arguments
#[derive(Debug, Parser)]
#[clap(about, author, version)]
pub struct Arguments {
    /// Whether log console output is disabled
    #[arg(long = "quiet", short = 'q')]
    pub disable_log_output: bool,
    /// Whether log file output is disabled
    #[arg(long = "ephemeral", short = 'e')]
    pub disable_log_storage: bool,
    /// The number of seconds between function loop ticks
    #[arg(long = "seconds", short = 's', default_value = "10")]
    pub function_loop_delay: u64,
}

/// Wraps common error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Wraps an [`clap::Error`] error
    #[error("{0}")]
    Argument(#[from] clap::Error),
    /// Can wrap any string; typically constructed from the [`err!`] macro
    #[error("{0}")]
    Custom(String),
    /// Wraps an [`dotenvy::Error`] error
    #[error("error loading environment: {0}")]
    Environment(#[from] dotenvy::Error),
    /// Wraps an [`std::convert::Infallible`] error
    #[error("w- what did you do??? {0}")]
    Infallible(#[from] std::convert::Infallible),
    /// Wraps an [`std::io::Error`] error
    #[error("io error: {0}")]
    InOut(#[from] std::io::Error),
    /// Wraps an [`rmp_serde::decode::Error`] error
    #[error("decoding error: {0}")]
    MsgpackDecode(#[from] rmp_serde::decode::Error),
    /// Wraps an [`rmp_serde::encode::Error`] error
    #[error("encoding error: {0}")]
    MsgpackEncode(#[from] rmp_serde::encode::Error),
    /// Wraps an [`serenity::Error`] error
    #[error("serenity error: {0}")]
    Serenity(#[from] serenity::Error),
    /// Wraps an [`toml::de::Error`] error
    #[error("decoding error: {0}")]
    TomlDecode(#[from] toml::de::Error),
    /// Wraps an [`toml::ser::Error`] error
    #[error("encoding error: {0}")]
    TomlEncode(#[from] toml::ser::Error),
    /// Wraps a [`std::env::VarError`] error
    #[error("error fetching variable: {0}")]
    Variable(#[from] std::env::VarError),
}

impl Error {
    /// Creates a custom error type
    pub fn custom(error: impl Display) -> Self {
        Self::Custom(error.to_string())
    }

    /// Creates a custom error type, wrapped in [`Err`].
    ///
    /// This will never return [`Ok(T)`]
    pub fn custom_wrap<T>(error: impl Display) -> Result<T> {
        Err(Self::custom(error))
    }
}

impl Default for Error {
    fn default() -> Self {
        crate::err!("unknown error")
    }
}

/// Constructs a new [`Error`] from the provided format string
#[macro_export]
macro_rules! err {
    ($( $arg: tt )+) => {
        $crate::util::Error::custom(format_args!($( $arg )+))
    };
}
/// Constructs a new [`Error`] from the provided format string, wrapped in
/// [`Err`].
///
/// This will never return [`Ok(T)`]
#[macro_export]
macro_rules! err_wrap {
    ($( $arg: tt )+) => {
        $crate::util::Error::custom_wrap(format_args!($( $arg )+))
    };
}

/// Outputs an info log through the logger instance
#[macro_export]
macro_rules! info {
    ($( $arg: tt )+) => {
        $crate::util::get_logger().output($crate::util::log::LogKind::Info, &format!($( $arg )+)).ok()
    };
}
/// Outputs an warning log through the logger instance
#[macro_export]
macro_rules! warn {
    ($( $arg: tt )+) => {
        $crate::util::get_logger().output($crate::util::log::LogKind::Warn, &format!($( $arg )+)).ok()
    };
}
/// Outputs an error log through the logger instance
#[macro_export]
macro_rules! error {
    ($( $arg: tt )+) => {
        $crate::util::get_logger().output($crate::util::log::LogKind::Error, &format!($( $arg )+)).ok()
    };
}

/// Returns the bot's token from the environment
pub fn get_bot_token() -> Result<String> {
    let key = if DEV_BUILD { "DEV_TOKEN" } else { "PRD_TOKEN" };

    std::env::var(key).map_err(Into::into)
}
/// Returns the bot's development guild identifier from the environment
pub fn get_dev_guild_id() -> Result<GuildId> {
    let id = std::env::var("DEV_GUILD")?;
    let Ok(id) = id.parse() else {
        return err_wrap!("invalid guild identifier");
    };

    if id == 0 {
        err_wrap!("expected non-zero identifier")
    } else {
        Ok(GuildId::new(id))
    }
}
/// Returns the bot's error log channel identifier from the environment
pub fn get_err_channel_id() -> Result<ChannelId> {
    let id = std::env::var("DEV_GUILD")?;
    let Ok(id) = id.parse() else {
        return err_wrap!("invalid guild identifier");
    };

    if id == 0 {
        err_wrap!("expected non-zero identifier")
    } else {
        Ok(ChannelId::new(id))
    }
}
/// Returns the bot process' command-line arguments
pub fn get_arguments() -> &'static Arguments {
    ARGUMENTS.get_or_init(Arguments::parse)
}
/// Returns the bot process' logger instance
pub fn get_logger() -> &'static Logger {
    LOGGER.get_or_init(Logger::new)
}
