use std::num::NonZeroU64;

use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::id::Id;

use crate::util::Result;

/// Returns the bot's client token.
///
/// This is configurable through the `CLIENT_TOKEN` environment variable.
///
/// # Errors
///
/// This function will return an error if the secret has not been set.
pub fn token() -> Result<Box<str>> {
    Ok(std::env::var("CLIENT_TOKEN")?.into_boxed_str())
}

/// Returns an identifier from the environment.
///
/// # Errors
///
/// This function will return an error if the secret has not been set or is not a valid identifier.
fn generic_id<T>(key: &str) -> Result<Id<T>> {
    Ok(std::env::var(key)?.parse::<NonZeroU64>().map(Id::from)?)
}

/// Returns the bot's test guild identifier.
///
/// This is configurable through the `TEST_GUILD_ID` environment variable.
///
/// # Errors
///
/// This function will return an error if the secret has not been set or is not a valid identifier.
#[inline]
pub fn test_guild_id() -> Result<Id<GuildMarker>> {
    generic_id("TEST_GUILD_ID")
}

/// Returns the bot's error channel identifier.
///
/// This is configurable through the `ERROR_CHANNEL_ID` environment variable.
///
/// # Errors
///
/// This function will return an error if the secret has not been set or is not a valid identifier.
#[inline]
pub fn error_channel_id() -> Result<Id<ChannelMarker>> {
    generic_id("ERROR_CHANNEL_ID")
}
