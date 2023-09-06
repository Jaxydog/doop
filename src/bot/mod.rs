use std::sync::Arc;

use anyhow::bail;
use doop_localizer::localize;
use doop_logger::{error, info, warn};
use futures_util::StreamExt;
use rand::{thread_rng, Rng};
use tokio::task::JoinSet;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::error::ReceiveMessageError;
use twilight_gateway::stream::{create_recommended, ShardEventStream};
use twilight_gateway::{Config, ConfigBuilder, Event, Intents, Shard};
use twilight_http::Client;
use twilight_model::application::command::Command;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::{InteractionCreate, Ready};
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{ActivityType, MinimalActivity, Status};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::interact::{Api, CId, Ctx, InteractionHandler};
use crate::util::ext::{EmbedAuthorExt, InteractionExt};
use crate::util::traits::Localized;
use crate::util::{Result, FAILURE};

/// Provides traits and types for working with interaction events.
pub mod interact;
// /// Provides traits and types for working with interaction events.
// pub mod interact;

/// The total number of possible error titles.
pub const ERROR_TITLES: usize = 10;
/// The bot's gateway intents
pub const INTENTS: Intents = Intents::DIRECT_MESSAGES
    .union(Intents::GUILDS)
    .union(Intents::GUILD_EMOJIS_AND_STICKERS)
    .union(Intents::GUILD_MEMBERS)
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::GUILD_MESSAGE_REACTIONS)
    .union(Intents::GUILD_MODERATION)
    .union(Intents::MESSAGE_CONTENT);

doop_macros::global! {
    /// The bot client's event handlers.
    static HANDLERS: Box<[&'static dyn InteractionHandler]> = Box::new([
        &crate::cmd::embed::Impl,
        &crate::cmd::help::Impl,
        &crate::cmd::ping::Impl,
        &crate::cmd::role::Impl,
    ]);
}

/// Returns an [`InteractionEventHandler`] with the given name.
pub fn handler(name: impl AsRef<str>) -> Option<&'static dyn InteractionHandler> {
    handlers().iter().find(|h| h.name() == name.as_ref()).copied()
}

/// Returns the bot client's event handlers as commands.
#[inline]
#[must_use]
pub fn commands(guild_id: Option<Id<GuildMarker>>) -> Vec<Command> {
    handlers()
        .iter()
        .filter_map(|h| {
            let command = h.command(guild_id);

            if let Err(ref error) = command {
                warn!("error building command: {error}").ok();
            }

            command.ok().flatten()
        })
        .collect()
}

/// Implements a bot client.
#[derive(Debug)]
pub struct BotClient {
    /// The bot client's HTTP API value.
    http: Arc<Client>,
    /// The bot client's cache value.
    cache: Arc<InMemoryCache>,
    /// The bot client's gateway shards.
    shards: Box<[Shard]>,
}

impl BotClient {
    /// Returns a new [`BotClient`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the client could not create gateway shards.
    pub async fn new() -> Result<Self> {
        let token = crate::util::env::token()?;
        let http = Arc::new(Client::new(token.to_string()));
        let cache = Arc::new(InMemoryCache::new());
        let shards = Self::shards(&http, token.into_string()).await?;

        Ok(Self { http, cache, shards })
    }

    /// Returns the bot's gateway configuration.
    ///
    /// # Errors
    ///
    /// This function will return an error if the bot's presence is invalid.
    fn config(token: String) -> Result<Config> {
        let status = if cfg!(debug_assertions) { Status::Idle } else { Status::Offline };
        let name = if cfg!(debug_assertions) { "for API events" } else { "for /help!" }.to_string();
        let activity = MinimalActivity { kind: ActivityType::Watching, name, url: None }.into();
        let presence = UpdatePresencePayload::new(vec![activity], false, None, status)?;

        Ok(ConfigBuilder::new(token, INTENTS).presence(presence).build())
    }

    /// Creates the bot's gateway shards.
    ///
    /// # Errors
    ///
    /// This function will return an error if the shards could not be created.
    async fn shards(http: &Client, token: String) -> Result<Box<[Shard]>> {
        let config = Self::config(token)?;

        Ok(create_recommended(http, config, |_, b| b.build()).await?.collect())
    }

    /// Starts the bot process.
    ///
    /// # Errors
    ///
    /// This function will return an error if execution fails.
    pub async fn start(mut self) -> Result {
        let mut stream = ShardEventStream::new(self.shards.iter_mut());
        let mut tasks = JoinSet::new();

        while let Some((_, event)) = stream.next().await {
            let result = Self::on_event(&self.http, &self.cache, &mut tasks, event);

            if result.is_err() {
                break;
            };
        }

        drop(stream);

        while tasks.join_next().await.is_some() {}

        Ok(())
    }

    /// Spawns a task that handles an incoming event.
    ///
    /// # Errors
    ///
    /// This function will return an error if the event was an error.
    fn on_event(
        http: &Arc<Client>,
        cache: &Arc<InMemoryCache>,
        tasks: &mut JoinSet<Result>,
        event: Result<Event, ReceiveMessageError>,
    ) -> Result {
        let event = match event {
            Ok(event) => event,
            Err(fatal) if fatal.is_fatal() => {
                error!("fatal error receiving event: {fatal}")?;
                return Err(fatal.into());
            }
            Err(error) => return Ok(warn!("error receiving event: {error}")?),
        };

        cache.update(&event);
        tasks.spawn(handle_event(Arc::clone(http), Arc::clone(cache), event));

        Ok(())
    }
}

/// Handles an incoming event in a new task.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
async fn handle_event(http: Arc<Client>, cache: Arc<InMemoryCache>, event: Event) -> Result {
    let api = Api::new(&http, &cache);
    let result = match event {
        Event::Ready(event) => handle_ready(api, *event).await,
        Event::InteractionCreate(event) => handle_interaction(api, *event).await,
        _ => Ok(()),
    };

    match result {
        Ok(()) => Ok(()),
        Err(error) => Ok(warn!("error handling event: {error}")?),
    }
}

/// Handles a ready event.
///
/// # Errors
///
/// This function will return an error if the client's command list could not be updated.
async fn handle_ready(api: Api<'_>, event: Ready) -> Result {
    info!("connected to the discord api")?;

    let client = api.http().interaction(event.application.id);

    if let Ok(id) = crate::util::env::testing_guild_id() {
        let list = client.set_guild_commands(id, &commands(Some(id))).await?;
        let count = list.model().await?.len();

        info!("patched {count} guild commands")?;
    }

    if cfg!(not(debug_assertions)) {
        let list = client.set_global_commands(&commands(None)).await?;
        let count = list.model().await?.len();

        info!("patched {count} global commands")?;
    }

    Ok(())
}

/// Handles an interaction event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
async fn handle_interaction(api: Api<'_>, event: InteractionCreate) -> Result {
    info!("received interaction: {}", event.label())?;

    let result = match event.kind {
        InteractionType::ApplicationCommandAutocomplete => handle_autocomplete(api, &event).await,
        InteractionType::ApplicationCommand => handle_command(api, &event).await,
        InteractionType::MessageComponent => handle_component(api, &event).await,
        InteractionType::ModalSubmit => handle_modal(api, &event).await,
        _ => Ok(()),
    };

    if let Err(ref error) = result {
        warn!("interaction failed: {} - {error}", event.label())?;
        handle_error(api, &event.0, error).await?;
    } else {
        info!("interaction succeeded: {}", event.label())?;
    }

    result
}

/// Handles an autocomplete interaction event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
async fn handle_autocomplete(api: Api<'_>, event: &Interaction) -> Result {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };
    let Some(handler) = handler(&data.name) else {
        bail!("missing interaction event handler");
    };
    let Some(focus) = data.options.iter().find_map(|o| match o.value {
        CommandOptionValue::Focused(ref n, k) => Some((&(**n), k)),
        _ => None,
    }) else {
        bail!("an option is not currently focused");
    };

    let ctx = Ctx::new(api, event, &(**data));
    let choices = handler.handle_autocomplete(ctx, focus).await?;

    crate::respond!(as ctx => {
        let kind = ApplicationCommandAutocompleteResult;
        let choices = choices;
    })
    .await?;

    Ok(())
}

/// Handles a command interaction event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
async fn handle_command(api: Api<'_>, event: &Interaction) -> Result {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };
    let Some(handler) = handler(&data.name) else {
        bail!("missing interaction event handler");
    };

    handler.handle_command(Ctx::new(api, event, data)).await
}

/// Handles a component interaction event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
async fn handle_component(api: Api<'_>, event: &Interaction) -> Result {
    let Some(InteractionData::MessageComponent(ref data)) = event.data else {
        bail!("missing component data");
    };

    let custom = data.custom_id.parse::<CId>()?;
    let Some(handler) = handler(custom.name()) else {
        bail!("missing interaction event handler");
    };

    handler.handle_component(Ctx::new(api, event, data), custom).await
}

/// Handles a modal interaction event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
async fn handle_modal(api: Api<'_>, event: &Interaction) -> Result {
    let Some(InteractionData::ModalSubmit(ref data)) = event.data else {
        bail!("missing modal data");
    };

    let custom = data.custom_id.parse::<CId>()?;
    let Some(handler) = handler(custom.name()) else {
        bail!("missing interaction event handler");
    };

    handler.handle_modal(Ctx::new(api, event, data), custom).await
}

/// Called to notify an executing user and the bot developer(s) when an error occurs.
///
/// # Errors
///
/// This function will return an error if the logger could not print properly.
async fn handle_error(api: Api<'_>, event: &Interaction, error: &anyhow::Error) -> Result {
    if let Err(error) = error_notify_user(api, event, error).await {
        error!("unable to notify executing user: {error}")?;
    }
    if let Err(error) = error_notify_devs(api, event, error).await {
        error!("unable to notify bot developers: {error}")?;
    }

    Ok(())
}

/// Notifies an executing user that an error has occurred.
///
/// # Errors
///
/// This function will return an error if the user could not be notified.
async fn error_notify_user(api: Api<'_>, event: &Interaction, error: &anyhow::Error) -> Result {
    let locale = event.author().locale();
    let index = thread_rng().gen_range(0..ERROR_TITLES);
    let title = localize!(try locale => "text.error.title_{index}");
    let embed = EmbedBuilder::new()
        .color(FAILURE)
        .description(format!("> {error}"))
        .title(title);

    crate::respond!(as api.http(), event => {
        let kind = DeferredChannelMessageWithSource;
        let flags = EPHEMERAL;
    })
    .await
    .ok();

    crate::followup!(as api.http(), event => {
        let embeds = &[embed.build()];
        let flags = EPHEMERAL;
    })
    .await?;

    Ok(())
}

/// Notifies the bot developer(s) that an error has occurred.
///
/// # Errors
///
/// This function will return an error if the developers could not be notified.
async fn error_notify_devs(api: Api<'_>, event: &Interaction, error: &anyhow::Error) -> Result {
    let index = thread_rng().gen_range(0..ERROR_TITLES);
    let title = localize!("text.error.title_{index}");
    let mut embed = EmbedBuilder::new()
        .color(FAILURE)
        .description(format!("**ID:** `{}`\n\n```json\n{error}\n```", event.label()))
        .title(title);

    if let Some(user) = event.author() {
        embed = embed.author(EmbedAuthor::new_from(user)?);
    }

    api.http()
        .create_message(crate::util::env::error_channel_id()?)
        .embeds(&[embed.build()])?
        .flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
        .await?;

    Ok(())
}

/// Responds to an interaction event.
///
/// # Examples
///
/// ```
/// respond!(as api.http(), event => {
///     let kind = DeferredChannelMessageWithSource;
///     let embeds = &[embed.build()];
/// })
/// .await?;
///
/// respond!(as ctx => {
///     let kind = DeferredChannelMessageWithSource;
///     let embeds = &[embed.build()];
/// })
/// .await?;
/// ```
#[macro_export]
macro_rules! respond {
    (as $http:expr, $event:expr => { $($args:tt)+ }) => {
        $crate::respond!(@($http.interaction($event.application_id), $event.id, &$event.token, { $($args)+ }))
    };
    (as $ctx:expr => { $($args:tt)+ }) => {
        $crate::respond!(@($ctx.client(), $ctx.event.id, &$ctx.event.token, { $($args)+ }))
    };
    (@($client:expr, $id:expr, $token:expr, {
        let kind = $kind:ident;
        $(let attachments = $attachments:expr;)?
        $(let choices = $choices:expr;)?
        $(let components = $components:expr;)?
        $(let content = $content:expr;)?
        $(let custom_id = $custom_id:expr;)?
        $(let embeds = $embeds:expr;)?
        $(let flags = $($flag:ident)|*;)?
        $(let mentions = { $($mentions:tt)+ })?
        $(let title = $title:expr;)?
        $(let tts = $tts:literal;)?
    })) => {
        $client.create_response($id, $token, &::twilight_model::http::interaction::InteractionResponse {
            kind: ::twilight_model::http::interaction::InteractionResponseType::$kind,
            data: Some(
                ::twilight_util::builder::InteractionResponseDataBuilder::new()
                    $(.attachments($attachments))?
                    $(.choices($choices))?
                    $(.components($components))?
                    $(.content($content))?
                    $(.custom_id($custom_id))?
                    $(.embeds($embeds))?
                    $(.flags(::twilight_model::channel::message::MessageFlags::empty()$(.union(::twilight_model::channel::message::MessageFlags::$flag))*))?
                    $(.allowed_mentions(::twilight_model::channel::message::AllowedMentions { $($mentions)+ }))?
                    $(.title($title))?
                    $(.tts($tts))?
                    .build()
            ),
        })
    };
}

/// Follows-up an interaction event response.
///
/// # Examples
///
/// ```
/// followup!(as api.http(), event => {
///     let embeds = &[embed.build()];
///     let flags = EPHEMERAL;
/// })
/// .await?;
///
/// followup!(as ctx => {
///     let embeds = &[embed.build()];
///     let flags = EPHEMERAL;
/// })
/// .await?;
/// ```
#[macro_export]
macro_rules! followup {
    (as $http:expr, $event:expr => { $($args:tt)* }) => {
        $crate::followup!(@($http.interaction($event.application_id), &$event.token, { $($args)* }))
    };
    (as $ctx:expr => { $($args:tt)* }) => {
        $crate::followup!(@($ctx.client(), &$ctx.event.token, { $($args)* }))
    };
    (@($client:expr, $token:expr, {
        $(let attachments = $attachments:expr;)?
        $(let components = $components:expr;)?
        $(let content = $content:expr;)?
        $(let embeds = $embeds:expr;)?
        $(let flags = $($flag:ident)|*;)?
        $(let mentions = { $($mentions:tt)+ })?
        $(let tts = $tts:literal;)?
    })) => {
        $client.create_followup($token)
            $(.attachments($attachments)?)?
            $(.components($components)?)?
            $(.content($content)?)?
            $(.embeds($embeds)?)?
            $(.flags(::twilight_model::channel::message::MessageFlags::empty()$(.union(::twilight_model::channel::message::MessageFlags::$flag))*))?
            $(.allowed_mentions(::twilight_model::channel::message::AllowedMentions { $($mentions)+ }))?
            $(.tts($tts))?
    };
}
