use std::sync::Arc;

use futures_util::StreamExt;
use tokio::task::JoinSet;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::stream::{create_recommended, ShardEventStream};
use twilight_gateway::{Config, ConfigBuilder, Intents, Shard};
use twilight_http::Client;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{ActivityType, MinimalActivity, Status};

use crate::utility::Result;
use crate::{error, warn};

/// The bot's gateway intents
pub const INTENTS: Intents = Intents::DIRECT_MESSAGES
    .union(Intents::GUILDS)
    .union(Intents::GUILD_EMOJIS_AND_STICKERS)
    .union(Intents::GUILD_MEMBERS)
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::GUILD_MESSAGE_REACTIONS)
    .union(Intents::GUILD_MODERATION)
    .union(Intents::MESSAGE_CONTENT);

/// Defines the bot process' state.
#[derive(Debug)]
pub struct State {
    /// The bot's HTTP client.
    pub http: Arc<Client>,
    /// The bot's in-memory cache.
    pub cache: Arc<InMemoryCache>,
    /// The bot's API shards.
    pub shards: Box<[Shard]>,
}

impl State {
    /// Creates a new bot state instance.
    pub async fn new() -> Result<Self> {
        let token = crate::utility::env::token()?;
        let http = Arc::new(Client::new(token.to_string()));
        let cache = Arc::new(InMemoryCache::new());
        let shards = Self::new_shards(&http, token.into_string()).await?;

        Ok(Self { http, cache, shards })
    }

    /// Returns the bot's API sharding configuration.
    pub(crate) fn new_config(token: String) -> Result<Config> {
        let status = if cfg!(debug_assertions) { Status::Idle } else { Status::Online };
        let name = if cfg!(debug_assertions) { "for API events" } else { "for /help!" }.to_string();
        let activity = MinimalActivity { kind: ActivityType::Watching, name, url: None }.into();
        let presence = UpdatePresencePayload::new(vec![activity], false, None, status)?;

        Ok(Config::builder(token, INTENTS).presence(presence).build())
    }

    /// Returns a list of automatically generated bot API shards.
    pub(crate) async fn new_shards(http: &Client, token: String) -> Result<Box<[Shard]>> {
        let config = Self::new_config(token)?;
        let builder = |_, b: ConfigBuilder| b.build();

        Ok(create_recommended(http, config, builder).await?.collect())
    }

    /// Runs the bot's process.
    pub async fn run(mut self) -> Result {
        let mut stream = ShardEventStream::new(self.shards.iter_mut());
        let mut tasks = JoinSet::new();

        while let Some((_, event)) = stream.next().await {
            let event = match event {
                Ok(event) => event,
                Err(fatal) if fatal.is_fatal() => {
                    error!("fatal error receiving event: {fatal}")?;
                    break;
                }
                Err(error) => {
                    warn!("error receiving event: {error}")?;
                    continue;
                }
            };

            self.cache.update(&event);

            tasks.spawn(crate::event::handle_event(
                Arc::clone(&self.http),
                Arc::clone(&self.cache),
                event,
            ));
        }

        drop(stream);

        while tasks.join_next().await.is_some() {}

        Ok(())
    }
}
