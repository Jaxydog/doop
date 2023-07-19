use std::ffi::OsStr;

use anyhow::anyhow;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::id::Id;

use super::Result;

/// Returns the bot's token from the environment.
pub fn token() -> Result<Box<str>> { Ok(std::env::var("BOT_TOKEN")?.into_boxed_str()) }

/// Returns a generic identifier from the environment.
fn __id<T>(key: impl AsRef<OsStr>) -> Result<Id<T>> {
    let id = std::env::var(key)?.parse()?;

    Id::new_checked(id).ok_or_else(|| anyhow!("expected non-zero identifier"))
}

/// Returns the bot's development guild identifier from the environment.
#[inline]
pub fn guild_id() -> Result<Id<GuildMarker>> { __id("DEV_GUILD_ID") }

/// Returns the bot's development channel identifier from the environment.
#[inline]
pub fn channel_id() -> Result<Id<ChannelMarker>> { __id("DEV_CHANNEL_ID") }
