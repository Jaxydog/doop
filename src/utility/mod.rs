use clap::Parser;
use time::macros::datetime;
use time::OffsetDateTime;

pub use self::anchor::*;
pub use self::color::*;
pub use self::etime::*;
pub use self::id::*;
pub use self::modal::*;

/// Discord's identifier epoch, or the first second of 2015.
pub const DISCORD_EPOCH: OffsetDateTime = datetime!(2015-01-01 00:00:00 UTC);
/// Discord content delivery network endpoint base URL.
pub const CDN_BASE: &str = "https://cdn.discordapp.com";

/// Defines a structure that represents a resolvable message location.
mod anchor;
/// Defines a structure that represents an RGB color.
mod color;
/// Provides functions for fetching various client secrets.
pub(crate) mod env;
/// Defines a structure that represents a Discord embedded timestamp.
mod etime;
/// Defines a custom data-storing identifier for use in components and modals.
mod id;
/// Defines a structure that represents a Discord modal.
mod modal;

/// The bot's custom internal [`Result`] type with a default `T` type.
pub type Result<T = (), E = anyhow::Error> = anyhow::Result<T, E>;

/// Defines and stores the bot's command-line arguments.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Parser, PartialEq, Eq)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Prevents the bot from logging to the console.
    #[arg(short = 'q', long = "no-print-logs")]
    pub no_print_logs: bool,
    /// Prevents the bot from logging to the file system.
    #[arg(short = 'W', long = "no-write-logs")]
    pub no_write_logs: bool,
    /// Disables the bot's application GUI.
    #[arg(short = 'G', long = "no-gui")]
    pub no_gui: bool,
}

crate::global! {{
    /// Returns the bot's command-line arguments.
    [ARGUMENTS] fn arguments() -> Arguments { Arguments::parse }
}}
