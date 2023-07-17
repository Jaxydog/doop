use std::fmt::Display;

use serde::{Deserialize, Serialize};
use twilight_http::request::channel::message::UpdateMessage;
use twilight_http::Client;
use twilight_model::channel::Message;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker};
use twilight_model::id::Id;

use super::Result;

/// Defines a structure that represents a resolvable message location.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    /// The anchor's guild identifier
    pub guild_id: Option<Id<GuildMarker>>,
    /// The anchor's channel identifier
    pub channel_id: Id<ChannelMarker>,
    /// The anchor's message identifier
    pub message_id: Id<MessageMarker>,
}

impl Anchor {
    /// Resolves and returns the represented message.
    pub async fn fetch(&self, http: &Client) -> Result<Message> {
        let Self { channel_id, message_id, .. } = *self;

        Ok(http.message(channel_id, message_id).await?.model().await?)
    }

    /// Returns a message update interface.
    pub const fn update<'um>(&self, http: &'um Client) -> UpdateMessage<'um> {
        http.update_message(self.channel_id, self.message_id)
    }

    /// Deletes the represented message.
    pub async fn delete(&self, http: &Client) -> Result {
        let Self { channel_id, message_id, .. } = *self;

        http.delete_message(channel_id, message_id).await?;

        Ok(())
    }

    /// Deletes the represented message if it exists.
    ///
    /// This is less error-prone than just calling `delete`.
    #[inline]
    pub async fn delete_if_exists(&self, http: &Client) -> Result {
        if self.fetch(http).await.is_ok() { self.delete(http).await } else { Ok(()) }
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { guild_id, channel_id: c, message_id: m } = self;
        let g = guild_id.map_or_else(|| "@me".to_string(), |g| g.to_string());

        write!(f, "https://discord.com/channels/{g}/{c}/{m}")
    }
}
