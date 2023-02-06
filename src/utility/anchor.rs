use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    pub guild_id: Option<GuildId>,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

impl Anchor {
    const URL: &str = "https://discord.com/channels";

    pub async fn get_partial_guild(
        cache_http: &impl CacheHttp,
        guild_id: GuildId,
    ) -> Result<PartialGuild> {
        guild_id
            .to_partial_guild(cache_http)
            .await
            .map_err(Into::into)
    }
    pub async fn get_guild_channel(
        cache_http: &impl CacheHttp,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Result<GuildChannel> {
        let guild = Self::get_partial_guild(cache_http, guild_id).await?;
        let mut list = guild.channels(cache_http.http()).await?;

        list.remove(&channel_id)
            .ok_or_else(|| err!("invalid channel identifier"))
    }
    pub async fn get_private_channel(
        cache_http: &impl CacheHttp,
        channel_id: ChannelId,
    ) -> Result<PrivateChannel> {
        channel_id
            .to_channel(cache_http)
            .await
            .map_err(Into::into)
            .and_then(|c| c.private().ok_or_else(|| err!("invalid channel type")))
    }

    pub const fn new(
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Self {
        Self {
            guild_id,
            channel_id,
            message_id,
        }
    }
    pub const fn new_private(channel_id: ChannelId, message_id: MessageId) -> Self {
        Self::new(None, channel_id, message_id)
    }
    pub const fn new_guild(
        guild_id: GuildId,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Self {
        Self::new(Some(guild_id), channel_id, message_id)
    }

    pub fn as_link(self) -> String {
        let (u, c, m) = (Self::URL, self.channel_id, self.message_id);

        self.guild_id
            .map_or_else(|| format!("{u}/{c}/{m}"), |g| format!("{u}/{g}/{c}/{m}"))
    }
    pub async fn to_partial_guild(self, cache_http: &impl CacheHttp) -> Result<PartialGuild> {
        Self::get_partial_guild(
            cache_http,
            self.guild_id
                .ok_or_else(|| err!("missing guild identifier"))?,
        )
        .await
    }
    pub async fn to_guild_channel(self, cache_http: &impl CacheHttp) -> Result<GuildChannel> {
        Self::get_guild_channel(
            cache_http,
            self.guild_id
                .ok_or_else(|| err!("missing guild identifier"))?,
            self.channel_id,
        )
        .await
    }
    pub async fn to_private_channel(self, cache_http: &impl CacheHttp) -> Result<PrivateChannel> {
        Self::get_private_channel(cache_http, self.channel_id).await
    }
    pub async fn to_message(self, cache_http: &impl CacheHttp) -> Result<Message> {
        if self.guild_id.is_some() {
            self.to_guild_channel(cache_http)
                .await?
                .message(cache_http, self.message_id)
                .await
        } else {
            self.to_private_channel(cache_http)
                .await?
                .message(cache_http, self.message_id)
                .await
        }
        .map_err(Into::into)
    }
}

impl<T: AsRef<Message>> From<T> for Anchor {
    fn from(value: T) -> Self {
        let message = value.as_ref();

        Self::new(message.guild_id, message.channel_id, message.id)
    }
}

pub trait TryAsAnchor {
    fn try_as_anchor(&self) -> Result<Anchor>;

    fn is_anchored(&self) -> bool {
        self.try_as_anchor().is_ok()
    }
    fn is_floating(&self) -> bool {
        self.try_as_anchor().is_err()
    }
}

pub trait AsAnchor {
    fn as_anchor(&self) -> Anchor;
}

impl<T: AsAnchor> TryAsAnchor for T {
    fn try_as_anchor(&self) -> Result<Anchor> {
        Ok(self.as_anchor())
    }
}
