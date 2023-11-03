use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::bail;
use clap::Parser;
use doop_localizer::Locale;
use doop_storage::{Compress, Key, MsgPack};
use serde::{Deserialize, Serialize};
use twilight_http::request::channel::message::UpdateMessage;
use twilight_model::channel::Message;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker};
use twilight_model::id::Id;
use uuid::Uuid;

use crate::bot::client::ApiRef;

/// Provides builder types for various API structures.
pub mod builder;
/// Provides type extension traits.
pub mod extension;
/// Provides getters for bot secrets.
pub mod secrets;
/// Provides commonly used traits.
pub mod traits;

/// A [`Result<T, E>`] with defaulted `T` and `E` generics.
pub type Result<T = (), E = anyhow::Error> = anyhow::Result<T, E>;

/// The bot's command-line arguments.
static ARGUMENTS: OnceLock<Arguments> = OnceLock::new();

/// Returns a reference to the bot's command-line arguments.
pub fn arguments() -> &'static Arguments {
    ARGUMENTS.get_or_init(Arguments::parse)
}

/// Discord content delivery network endpoint base URL.
pub const CDN_URL: &str = "https://cdn.discordapp.com";
/// Discord's emoji repository's base URL.
pub const TWEMOJI_URL: &str = "https://raw.githubusercontent.com/discord/twemoji/master/assets";

/// The bot's branding color.
pub const BRANDING: u32 = 0x24_9F_DE;
/// The bot's success color.
pub const SUCCESS: u32 = 0x59_C1_35;
/// The bot's failure color.
pub const FAILURE: u32 = 0xB4_20_2A;

/// The bot's command-line arguments.
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq, Eq, Parser)]
#[command(author, about, version)]
pub struct Arguments {
    /// Disables the logger's console output.
    #[arg(short = 'q', long = "quiet")]
    pub log_no_print: bool,
    /// Disables the logger's file output.
    #[arg(short = 'e', long = "ephemeral")]
    pub log_no_write: bool,
    /// Disables the logger's color output.
    #[arg(short = 'm', long = "monochrome")]
    pub log_no_color: bool,
    /// The logger's automatic flush timeout in milliseconds.
    #[arg(long = "log-timeout")]
    pub log_queue_timeout: Option<u64>,
    /// The logger's output queue capacity.
    #[arg(long = "log-capacity")]
    pub log_queue_capacity: Option<usize>,
    /// The logger's file output directory.
    #[arg(long = "log-dir")]
    pub log_output_dir: Option<Box<Path>>,

    /// The localizer's preferred directory.
    #[arg(short = 'l', long = "prefer-locale")]
    pub l18n_prefer: Option<Locale>,
    /// The localizer's map input directory.
    #[arg(long = "localization-dir")]
    pub l18n_map_dir: Option<Box<Path>>,

    /// The preferred data storage directory.
    #[arg(short = 'o', long = "data-dir")]
    pub data_dir: Option<Box<Path>>,
}

/// Represents a single message's location.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Anchor {
    /// The message's guild identifier.
    pub guild_id: Option<Id<GuildMarker>>,
    /// The message's channel identifier.
    pub channel_id: Id<ChannelMarker>,
    /// The message's identifier.
    pub message_id: Id<MessageMarker>,
}

impl Anchor {
    /// Creates a new private message anchor.
    #[must_use]
    pub const fn new_private(channel_id: Id<ChannelMarker>, message_id: Id<MessageMarker>) -> Self {
        Self { guild_id: None, channel_id, message_id }
    }

    /// Creates a new guild message anchor.
    #[must_use]
    pub const fn new_guild(
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
    ) -> Self {
        Self { guild_id: Some(guild_id), channel_id, message_id }
    }

    /// Returns the associated [`Message`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the request failed.
    #[inline]
    pub async fn fetch(&self, api: ApiRef<'_>) -> Result<Message> {
        let Self { channel_id, message_id, .. } = *self;

        Ok(api.http.message(channel_id, message_id).await?.model().await?)
    }

    /// Returns a message update builder.
    #[inline]
    pub fn update<'m>(&self, api: ApiRef<'m>) -> UpdateMessage<'m> {
        api.http.update_message(self.channel_id, self.message_id)
    }

    /// Deletes the associated [`Message`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the request failed.
    pub async fn delete(&self, api: ApiRef<'_>) -> Result {
        let Self { channel_id, message_id, .. } = *self;

        api.http.delete_message(channel_id, message_id).await?;

        Ok(())
    }

    /// Deletes the associated [`Message`] if it exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if the request failed.
    #[inline]
    pub async fn delete_if_exists(&self, api: ApiRef<'_>) -> Result {
        if self.fetch(api).await.is_ok() { self.delete(api).await } else { Ok(()) }
    }
}

impl From<Message> for Anchor {
    fn from(value: Message) -> Self {
        let Message { channel_id, guild_id, id, .. } = value;

        Self { guild_id, channel_id, message_id: id }
    }
}

impl From<&Message> for Anchor {
    fn from(value: &Message) -> Self {
        let &Message { channel_id, guild_id, id, .. } = value;

        Self { guild_id, channel_id, message_id: id }
    }
}

impl Display for Anchor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { guild_id, channel_id: c, message_id: m } = self;
        let g = guild_id.map_or_else(|| "@me".to_string(), |g| g.to_string());

        write!(f, "https://discord.com/channels/{g}/{c}/{m}")
    }
}

/// A custom identifier with data storage.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DataId {
    /// The name of the handler and its component.
    name: (Box<str>, Box<str>),
    /// The internal stringified data.
    data: Vec<Box<str>>,
    /// The internal storage key identifier.
    uuid: Option<Uuid>,
}

impl DataId {
    /// The maximum length of an identifier in bytes.
    pub const MAX_LEN: usize = 100;
    /// The character used to separate each part of the identifier.
    pub const PART_SEP: char = '$';
    /// The character used to serparate data values within the identifier.
    pub const DATA_SEP: char = ';';

    /// Creates a new [`CId`].
    pub fn new(handler: impl AsRef<str>, component: impl AsRef<str>) -> Self {
        let name = (handler.as_ref().into(), component.as_ref().into());

        Self { name, data: vec![], uuid: None }
    }

    /// Returns a reference to the event handler name of this [`CId`].
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &str {
        &self.name.0
    }

    /// Returns a reference to the component kind of this [`CId`].
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> &str {
        &self.name.1
    }

    /// Returns the data at the given index.
    #[inline]
    #[must_use]
    pub fn data(&self, index: usize) -> Option<&str> {
        self.data.get(index).map(|b| &(**b))
    }

    /// Returns the storage key of this [`CId`].
    #[inline]
    #[must_use]
    pub fn key<T>(&self) -> Option<Key<T, Compress<MsgPack, 4>>>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        Some(format!(".cid/{}/{}/{}", self.name.0, self.name.1, self.uuid?).into())
    }

    /// Generates a new random storage key for this [`CId`].
    #[must_use]
    pub fn with_key(mut self) -> Self {
        self.uuid = Some(Uuid::new_v4());

        self
    }

    /// Inserts the given data into the identifier.
    #[must_use]
    pub fn with(mut self, data: impl Into<String>) -> Self {
        self.data.push(data.into().into_boxed_str());

        self
    }

    /// Validates the length of this [`CId`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the identifier is too long.
    pub fn validate(self) -> Result<Self> {
        let string = self.to_string();

        // currently, afaik, this is the only check we need; from what i can find custom identifiers
        // are not limited by a specific charset, only by length.
        if string.len() >= Self::MAX_LEN {
            bail!("maximum identifier length exceeded ({}/{} bytes)", string.len(), Self::MAX_LEN);
        }

        Ok(self)
    }
}

impl TryFrom<&String> for DataId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(&(**value))
    }
}

impl TryFrom<String> for DataId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(&(*value))
    }
}

impl TryFrom<&Box<str>> for DataId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &Box<str>) -> Result<Self, Self::Error> {
        Self::try_from(&(**value))
    }
}

impl TryFrom<Box<str>> for DataId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: Box<str>) -> Result<Self, Self::Error> {
        Self::try_from(&(*value))
    }
}

impl TryFrom<Cow<'_, str>> for DataId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: Cow<str>) -> Result<Self, Self::Error> {
        Self::try_from(&(*value))
    }
}

impl TryFrom<&str> for DataId {
    type Error = <Self as FromStr>::Err;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl FromStr for DataId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(Self::PART_SEP).take(4);

        // every valid CID must have a handler name and component kind identifier.
        let Some(name) = parts.next() else {
            bail!("missing event handler name");
        };
        let Some(kind) = parts.next() else {
            bail!("missing component kind");
        };

        let mut cid = Self::new(name, kind);

        // this will only run zero, one, or two times.
        for part in parts {
            // we prefix the storage key with "K_" to *try* not to read data that contains a UUID as
            // the storage key identifier.
            if part.starts_with("K_") {
                cid.uuid = Some(part.trim_start_matches("K_").parse()?);
            } else {
                cid.data = part.split(Self::DATA_SEP).map(Into::into).collect();
            }
        }

        Ok(cid)
    }
}

impl From<DataId> for String {
    #[inline]
    fn from(value: DataId) -> Self {
        value.to_string()
    }
}

impl From<DataId> for Box<str> {
    #[inline]
    fn from(value: DataId) -> Self {
        value.to_string().into_boxed_str()
    }
}

impl Display for DataId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { name: (name, kind), data, uuid } = self;

        write!(f, "{name}{}{kind}", Self::PART_SEP)?;
        // only write the UUID if it exists; shorthand for an if-let-some statement.
        uuid.map_or(Ok(()), |uuid| write!(f, "{}K_{uuid}", Self::PART_SEP))?;

        if data.is_empty() {
            Ok(())
        } else {
            // write all stringified internal data joined by the data separator character.
            write!(f, "{}{}", Self::PART_SEP, data.join(&Self::DATA_SEP.to_string()))
        }
    }
}
