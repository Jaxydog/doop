use anyhow::anyhow;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::id::Id;

use super::Result;

/// Returns the bot's client token.
///
/// This can be configured using the `CLIENT_TOKEN` environment variable.
///
/// # Errors
///
/// This function will return an error if the variable was not set.
#[inline]
pub fn token() -> Result<Box<str>> { Ok(std::env::var("CLIENT_TOKEN")?.into_boxed_str()) }

/// Returns the bot's data encryption key.
///
/// This can be configured using the `ENCRYPT_KEY` environment variable.
///
/// # Errors
///
/// This function will return an error if the variable was not set.
#[inline]
pub fn encrypt_key() -> Result<Box<str>> { Ok(std::env::var("ENCRYPT_KEY")?.into_boxed_str()) }

/// Returns the bot's testing guild identifier.
///
/// This can be configured using the `TESTING_GUILD_ID` environment variable.
///
/// # Errors
///
/// This function will return an error if the variable was not set or is equal to zero.
#[inline]
pub fn testing_guild_id() -> Result<Id<GuildMarker>> { self::generic_id("TESTING_GUILD_ID") }

/// Returns the bot's error channel identifier.
///
/// This can be configured using the `ERROR_CHANNEL_ID` environment variable.
///
/// # Errors
///
/// This function will return an error if the variable was not set or is equal to zero.
#[inline]
pub fn error_channel_id() -> Result<Id<ChannelMarker>> { self::generic_id("ERROR_CHANNEL_ID") }

/// Returns a generic identifier from the environment.
///
/// # Errors
///
/// This function will return an error if the variable was not set or is equal to zero.
fn generic_id<T>(key: &str) -> Result<Id<T>> {
    let var = std::env::var(key)?.parse()?;

    Id::new_checked(var).ok_or_else(|| anyhow!("expected a non-zero identifier for '{key}'"))
}
