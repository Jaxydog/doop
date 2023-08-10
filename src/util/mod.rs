use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use clap::Parser;
use doop_localizer::Locale;
use serde::{Deserialize, Serialize};
use twilight_http::request::channel::message::UpdateMessage;
use twilight_model::channel::message::component::{Button, TextInput};
use twilight_model::channel::message::Component;
use twilight_model::channel::Message;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker};
use twilight_model::id::Id;

use self::builder::ActionRowBuilder;
use crate::bot::interact::Api;

/// Provides model builders.
pub mod builder;
/// Provides getters for various client secrets.
pub mod env;
/// Provides type extension traits.
pub mod ext;
/// Provides common trait definitions.
pub mod traits;

doop_macros::global! {
    /// The bot's command-line arguments.
    static ARGUMENTS: Arguments = Arguments::parse();
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

/// Wraps an [`anyhow::Result<T, E>`], providing a defaulted `T` generic type.
pub type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

/// The bot's command-line arguments.
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq, Eq, Parser)]
#[command(author, about, version)]
pub struct Arguments {
    /// Disables logger printing.
    #[arg(short = 'q', long = "quiet")]
    pub log_no_print: bool,
    /// Disables log file writing.
    #[arg(short = 'e', long = "ephemeral")]
    pub log_no_write: bool,
    /// Disables error log file writing.
    #[arg(short = 'E', long = "ephemeral-errors")]
    pub log_no_error: bool,

    /// The bot's preferred locale.
    #[arg(short = 'l', long = "preferred-locale")]
    pub lang_prefer_locale: Option<Locale>,

    /// The directory to store log files within.
    #[arg(long = "log-directory")]
    pub log_write_dir: Option<PathBuf>,
    /// The directory to store error log files within.
    #[arg(long = "error-log-directory")]
    pub log_error_dir: Option<PathBuf>,
    /// The directory that contains the bot's localization files.
    #[arg(short = 'L', long = "lang-directory")]
    pub lang_file_dir: Option<PathBuf>,
    /// The directory that contains the bot's data files.
    #[arg(short = 'd', long = "data-directory")]
    pub data_file_dir: Option<PathBuf>,
}

/// Represents a single message's location.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Anchor {
    /// The guild identifier.
    pub guild_id: Option<Id<GuildMarker>>,
    /// The channel identifier.
    pub channel_id: Id<ChannelMarker>,
    /// The message identifier.
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
    pub async fn fetch(&self, api: Api<'_>) -> Result<Message> {
        let Self { channel_id, message_id, .. } = *self;

        Ok(api.http().message(channel_id, message_id).await?.model().await?)
    }

    /// Returns a message update builder.
    #[inline]
    pub fn update<'m>(&self, api: Api<'m>) -> UpdateMessage<'m> {
        api.http().update_message(self.channel_id, self.message_id)
    }

    /// Deletes the associated [`Message`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the request failed.
    pub async fn delete(&self, api: Api<'_>) -> Result {
        let Self { channel_id, message_id, .. } = *self;

        api.http().delete_message(channel_id, message_id).await?;

        Ok(())
    }

    /// Deletes the associated [`Message`] if it exists.
    ///
    /// # Errors
    ///
    /// This function will return an error if the request failed.
    #[inline]
    pub async fn delete_if_exists(&self, api: Api<'_>) -> Result {
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

/// A modal.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modal {
    /// The modal's title.
    pub title: String,
    /// The modal's custom identifier.
    pub custom_id: String,
    /// The modal's components.
    pub components: Vec<Component>,
}

/// Automatically sorts buttons into action rows.
pub fn button_rows(buttons: impl IntoIterator<Item = impl Into<Button>>) -> Vec<Component> {
    let mut components = Vec::with_capacity(5);
    let mut action_row = Vec::with_capacity(5);

    for button in buttons {
        if action_row.len() < 5 {
            action_row.push(Component::Button(button.into()));
        } else {
            components.push(ActionRowBuilder::new(action_row).into());
            action_row = Vec::with_capacity(5);
        }
    }

    if !action_row.is_empty() {
        components.push(ActionRowBuilder::new(action_row).into());
    }

    components
}

/// Automatically sorts text inputs into action rows.
#[inline]
pub fn text_input_rows(inputs: impl IntoIterator<Item = impl Into<TextInput>>) -> Vec<Component> {
    inputs
        .into_iter()
        .map(Into::into)
        .map(|i| Component::from(ActionRowBuilder::new([i])))
        .collect()
}
