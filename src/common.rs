use std::fmt::{Display, Write};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::all::{
    ChannelId, GuildChannel, GuildId, Message, MessageId, PartialGuild, PrivateChannel,
};
use serenity::prelude::CacheHttp;

use crate::util::{Error, Result};
use crate::{err, err_wrap};

// This seems kind of arbitrary but it helps a lot for data structures in my
// mind. A lot less of having tons of structs all with `guild_id`, `channel_id`,
// etc. Now I get to just put `Anchor` and call it a day.

// And it comes with the nice feature of being able to just type `.to_message`
// and I just get it basically for free.

/// Represents a "message anchor", or link to a specific message in a guild or
/// DM channel
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Anchor {
    /// A message within a DM channel
    Private(ChannelId, MessageId),
    /// A message within a guild channel
    Guild(GuildId, ChannelId, MessageId),
}

impl Anchor {
    /// The base URL for a message link
    pub const URL: &str = "https://discord.com/channels";

    /// Creates a new message anchor
    #[must_use]
    pub const fn new(
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Self {
        if let Some(guild_id) = guild_id {
            Self::Guild(guild_id, channel_id, message_id)
        } else {
            Self::Private(channel_id, message_id)
        }
    }

    /// Creates a new private message anchor
    #[must_use]
    pub const fn new_private(channel_id: ChannelId, message_id: MessageId) -> Self {
        Self::new(None, channel_id, message_id)
    }

    /// Creates a new guild message anchor
    #[must_use]
    pub const fn new_guild(
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Self {
        Self::new(Some(guild_id), channel_id, message_id)
    }

    /// Returns a link to the anchored message
    #[must_use]
    pub fn as_link(&self) -> String {
        let u = Self::URL;

        // I'm sure there's a better way to do this, but I don't really care enough and
        // this is already plenty fast for how scarcely it'll be called.
        match self {
            Self::Private(c, m) => format!("{u}/{c}/{m}"),
            Self::Guild(g, c, m) => format!("{u}/{g}/{c}/{m}"),
        }
    }

    /// Returns the anchor's guild
    pub async fn to_partial_guild(self, cache_http: &impl CacheHttp) -> Result<PartialGuild> {
        let Self::Guild(guild_id, ..) = self else {
            return err_wrap!("invalid anchor variant");
        };

        fetch_partial_guild(cache_http, guild_id).await
    }

    /// Returns the anchor's guild channel
    pub async fn to_guild_channel(self, cache_http: &impl CacheHttp) -> Result<GuildChannel> {
        let Self::Guild(guild_id, channel_id, ..) = self else {
            return err_wrap!("invalid anchor variant");
        };

        fetch_guild_channel(cache_http, guild_id, channel_id).await
    }

    /// Returns the anchor's private channel
    pub async fn to_private_channel(self, cache_http: &impl CacheHttp) -> Result<PrivateChannel> {
        let Self::Private(channel_id, ..) = self else {
            return err_wrap!("invalid anchor variant");
        };

        fetch_private_channel(cache_http, channel_id).await
    }

    /// Returns the anchor's message
    pub async fn to_message(self, cache_http: &impl CacheHttp) -> Result<Message> {
        // Oh man this indentation is terrible.
        // TODO: see if there's something better. *Anything* better.
        match self {
            Self::Private(.., m) => Ok(self
                .to_private_channel(cache_http)
                .await?
                .message(cache_http, m)
                .await?),
            Self::Guild(.., m) => Ok(self
                .to_guild_channel(cache_http)
                .await?
                .message(cache_http, m)
                .await?),
        }
    }
}

impl<T: AsRef<Message>> From<T> for Anchor {
    fn from(value: T) -> Self {
        let message = value.as_ref();

        Self::new(message.guild_id, message.channel_id, message.id)
    }
}

// These Custom IDs are *incredibly* useful for transferring data between
// interactions; it saves me a lot of headache from trying to figure out
// collectors and I get it for basically free. I've been using them like this
// since before I started using Rust, and I honestly can't imagine making a
// complex bot like this without them.
//
// I'm sure a lot of bots also use them like this in their own ways, but this is
// mine so it's OBVIOUSLY 100% better in every way. /s

/// Represents a custom component identifier that contains embedded data
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomId {
    /// The identifier's base string
    pub base: String,
    /// The identifier's name
    pub name: String,
    /// The identifier's embedded data
    pub data: Vec<String>,
}

impl CustomId {
    /// The value that separates the identifier's embedded data
    pub const DATA_SEPARATOR: &str = ";";
    /// The maximum number of characters allowed within the custom identifier
    pub const MAX_LENGTH: usize = 64;
    /// The value that separates the identifier's base, name, and data values
    pub const PART_SEPARATOR: &str = ".";

    /// Creates a new custom identifier with the provided data
    #[must_use]
    pub const fn new_with(base: String, name: String, data: Vec<String>) -> Self {
        Self { base, name, data }
    }

    /// Creates a new custom identifier with no additional data
    #[must_use]
    pub const fn new(base: String, name: String) -> Self {
        Self::new_with(base, name, vec![])
    }

    // I kind of get why types that implement `ToString` don't also implement
    // `Into<String>`, but *MAN* it would be nice. I've definitely thought of
    // writing an impl for it myself at least once or twice.

    /// Appends the given data to the end of the custom identifier's embedded
    /// data
    pub fn append(&mut self, data: impl Into<String>) -> Result<()> {
        let string = data.into();
        let length = self.to_string().len() + string.len();

        if length > Self::MAX_LENGTH {
            return err_wrap!("max id length exceeded ({length} / {})", Self::MAX_LENGTH);
        }

        self.data.push(string);

        Ok(())
    }
}

impl FromStr for CustomId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(Self::PART_SEPARATOR);

        let Some(base) = parts.next().map(ToString::to_string) else {
            return err_wrap!("missing identifier base");
        };
        let Some(name) = parts.next().map(ToString::to_string) else {
            return err_wrap!("missing identifier name");
        };

        if let Some(data) = parts.next().map(|s| {
            s.split(Self::DATA_SEPARATOR)
                .map(ToString::to_string)
                .collect()
        }) {
            Ok(Self::new_with(base, name, data))
        } else {
            Ok(Self::new(base, name))
        }
    }
}

impl TryFrom<String> for CustomId {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl TryFrom<&str> for CustomId {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl From<CustomId> for String {
    fn from(value: CustomId) -> Self {
        value.to_string()
    }
}

impl From<&CustomId> for String {
    fn from(value: &CustomId) -> Self {
        value.to_string()
    }
}

impl Display for CustomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { base, name, data } = self;

        // Custom IDs should be formatted as {base}{sep}{name} if there's no data, or
        // {base}{sep}{name}{sep}{data} if there is, with data having its own separator
        // character.
        if data.is_empty() {
            write!(f, "{base}{}{name}", Self::PART_SEPARATOR)
        } else {
            let data = data.join(Self::DATA_SEPARATOR);

            write!(f, "{base}{s}{name}{s}{data}", s = Self::PART_SEPARATOR)
        }
    }
}

/// Represents a possible time string format
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeStringFlag {
    /// A time string containing the time in a shorter format
    TimeShort = b't',
    /// A time string containing the time in a longer format
    TimeLong = b'T',
    /// A time string containing the date in a shorter format
    DateShort = b'd',
    /// A time string containing the date in a longer format
    DateLong = b'D',
    /// A time string containing both date and time in a shorter format
    DateTimeShort = b'f',
    /// A time string containing both date and time in a longer format
    DateTimeLong = b'F',
    /// A relative time string
    #[default]
    Relative = b'R',
}

impl Display for TimeStringFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // I'm really glad that I can store enum variant literals as character bytes,
        // that's really cool and really helpful in this specific case.
        //
        // Because you see! We can turn them back into `char`s with casting!
        // This really isn't that remarkable but I find it kinda neat.
        f.write_char(char::from(*self as u8))
    }
}

/// Represents Discord Timestamp markdown; `<t:{unix}:{flag}>`
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeString(i64, Option<TimeStringFlag>);

impl TimeString {
    /// Creates a new time string
    #[must_use]
    pub const fn new_with_flag(unix_ms: i64, flag: TimeStringFlag) -> Self {
        Self(unix_ms, Some(flag))
    }

    /// Creates a new time string with the default flag
    #[must_use]
    pub fn new(unix_ms: i64) -> Self {
        Self::new_with_flag(unix_ms, TimeStringFlag::default())
    }

    /// Creates a new time string
    #[must_use]
    pub fn new_with_flag_in(added_ms: i64, flag: TimeStringFlag) -> Self {
        let now = Utc::now().timestamp_millis();

        Self(now.saturating_add(added_ms), Some(flag))
    }

    /// Creates a new time string with the default flag
    #[must_use]
    pub fn new_in(added_ms: i64) -> Self {
        Self::new_with_flag_in(added_ms, TimeStringFlag::default())
    }
}

impl From<DateTime<Utc>> for TimeString {
    fn from(value: DateTime<Utc>) -> Self {
        Self::new(value.timestamp_millis())
    }
}

impl Display for TimeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // I don't know if it's really necessary to store milliseconds if we're going to
        // convert last second anyways. Maybe good for retaining precision if I add some
        // operations like addition / subtraction in the future? But for now it's just a
        // little silly.
        let unix = self.0 / 1000;
        let flag = self.1.unwrap_or_default();

        write!(f, "<t:{unix}:{flag}>")
    }
}

// The next couple functions are very helpful, mostly for fetching guild or
// channel metadata and for implementing `Anchor`s properly.

/// Fetches a partial guild from the Discord API
pub async fn fetch_partial_guild(
    cache_http: &impl CacheHttp,
    guild_id: GuildId,
) -> Result<PartialGuild> {
    Ok(guild_id.to_partial_guild(cache_http).await?)
}
/// Fetches a guild channel from the Discord API
pub async fn fetch_guild_channel(
    cache_http: &impl CacheHttp,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<GuildChannel> {
    let guild = fetch_partial_guild(cache_http, guild_id).await?;
    let mut list = guild.channels(cache_http.http()).await?;

    // It kills me that this is on a newline.
    list.remove(&channel_id)
        .ok_or_else(|| err!("invalid channel identifier"))
}
/// Fetches a DM channel from the Discord API
pub async fn fetch_private_channel(
    cache_http: &impl CacheHttp,
    channel_id: ChannelId,
) -> Result<PrivateChannel> {
    let channel = channel_id.to_channel(cache_http).await?;
    let Some(channel) = channel.private() else {
        return err_wrap!("invalid channel type");
    };

    Ok(channel)
}
/// Parses escapes in the given string and trims it if needed
#[must_use]
pub fn parse_escapes(string: &str) -> String {
    // This is rarely used but I much prefer having this convenience function over
    // writing it out every time. It's mostly for things like embed descriptions
    // when I want the end-user to have at least some extra formatting options.
    string
        .replace(r"\t", "    ") // Discord doesn't really support `\t`
        .replace(r"\n", "\n")
        .replace(r"\r", "\r")
        .trim()
        .to_string()
}