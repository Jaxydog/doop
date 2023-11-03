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
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::{InteractionCreate, Ready};
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{ActivityType, MinimalActivity, Status};
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::client::{Api, ApiRef};
use crate::bot::interaction::Ctx;
use crate::util::extension::{EmbedAuthorExtension, InteractionExtension};
use crate::util::traits::PreferLocale;
use crate::util::{DataId, Result, FAILURE};

/// Provides types and implementations for the bot client.
pub mod client;
/// Provides types and traits for working with interaction events.
pub mod interaction;

/// The number of defined error titles.
pub const ERROR_TITLES: usize = 10;
/// The bot's gateway intentions.
pub const INTENTS: Intents = Intents::DIRECT_MESSAGES
    .union(Intents::GUILDS)
    .union(Intents::GUILD_EMOJIS_AND_STICKERS)
    .union(Intents::GUILD_MEMBERS)
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::GUILD_MESSAGE_REACTIONS)
    .union(Intents::GUILD_MODERATION)
    .union(Intents::MESSAGE_CONTENT);

/// Implements the bot's client.
#[derive(Debug)]
pub struct BotClient {
    /// The bot's API.
    api: Api,
    /// The bot's shards.
    shards: Box<[Shard]>,
}

impl BotClient {
    /// Returns a new [`BotClient`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the client could not generate shards.
    pub async fn new() -> Result<Self> {
        let token = crate::util::secrets::token()?.to_string();
        let http = Arc::new(Client::new(token.clone()));
        let cache = Arc::new(InMemoryCache::new());
        let config = Self::config(token)?;
        let shards = create_recommended(&http, config, |_, b| b.build()).await?.collect();
        let api = Api { http, cache };

        Ok(Self { api, shards })
    }

    /// Returns the bot's gateway configuration.
    ///
    /// # Errors
    ///
    /// This function will return an error if the presence payload could not be generated.
    fn config(token: String) -> Result<Config> {
        let status = if cfg!(debug_assertions) { Status::Idle } else { Status::Online };
        let name = if cfg!(debug_assertions) { "for API events" } else { "for /help!" }.to_string();
        let activity = MinimalActivity { kind: ActivityType::Watching, name, url: None };
        let presence = UpdatePresencePayload::new(vec![activity.into()], false, None, status)?;

        Ok(ConfigBuilder::new(token, INTENTS).presence(presence).build())
    }

    /// Starts the bot application.
    ///
    /// # Errors
    ///
    /// This function will return an error if the bot encounters an unhandled exception.
    pub async fn start(mut self) -> Result {
        let mut stream = ShardEventStream::new(self.shards.iter_mut());
        let mut tasks = JoinSet::new();

        while let Some((_, event)) = stream.next().await {
            if let Err(error) = handle_event(self.api.into_ref(), &mut tasks, event) {
                error!("{error}")?;

                break;
            }
        }

        drop(stream);

        while tasks.join_next().await.is_some() {}

        Ok(())
    }
}

/// Handles an incoming API event by spawning a new task.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
fn handle_event(
    api: ApiRef<'_>,
    tasks: &mut JoinSet<Result>,
    event: Result<Event, ReceiveMessageError>,
) -> Result {
    let event = match event {
        Ok(event) => event,
        Err(fatal) if fatal.is_fatal() => {
            error!("{fatal}")?;

            return Err(fatal.into());
        }
        Err(error) => return Ok(warn!("{error}")?),
    };

    api.cache.update(&event);
    tasks.spawn(handle_event_task(api.into_owned(), event));

    Ok(())
}

/// Handles an incoming API event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled successfully.
async fn handle_event_task(api: Api, event: Event) -> Result {
    if let Err(error) = match event {
        Event::Ready(event) => on_ready(api, *event).await,
        Event::InteractionCreate(event) => on_interaction(api.into_ref(), *event).await,
        _ => Ok(()),
    } {
        warn!("event handling failed: {error}")?;
    }

    Ok(())
}

/// Handles an incoming [`Ready`] event.
///
/// # Errors
///
/// This function will return an error if the client's commands could not be updated.
async fn on_ready(api: Api, event: Ready) -> Result {
    info!("api connection established")?;

    let client = api.http.interaction(event.application.id);
    let registry = crate::cmd::registry();

    if let Ok(id) = crate::util::secrets::test_guild_id() {
        let list = registry.build_all(Some(id));
        let count = client.set_guild_commands(id, &list).await?.model().await?.len();

        info!("patched {count} server commands")?;
    }

    if cfg!(not(debug_assertions)) {
        let list = registry.build_all(None);
        let count = client.set_global_commands(&list).await?.model().await?.len();

        info!("patched {count} global commands")?;
    }

    Ok(())
}

/// Handles an incoming [`InteractionCreate`] event.
///
/// # Errors
///
/// This function will return an error if the event could not be handled.
async fn on_interaction(api: ApiRef<'_>, event: InteractionCreate) -> Result {
    info!("interaction received: {}", event.marker())?;

    let result: Result = match event.kind {
        InteractionType::ApplicationCommand => self::on_command(api, &event).await,
        InteractionType::ApplicationCommandAutocomplete => self::on_complete(api, &event).await,
        InteractionType::MessageComponent => self::on_component(api, &event).await,
        InteractionType::ModalSubmit => self::on_modal(api, &event).await,
        _ => Ok(()),
    };

    if let Err(ref error) = result {
        warn!("interaction failed: {} - {error}", event.marker())?;
        self::on_error(api, &event, error).await?;
    } else {
        info!("interaction succeeded: {}", event.marker())?;
    }

    result
}

/// Handles a command interaction event.
///
/// # Errors
///
/// This function will return an error if command execution fails.
async fn on_command(api: ApiRef<'_>, event: &Interaction) -> Result {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };
    let Some(command) = crate::cmd::registry().get(&data.name) else {
        bail!("missing command '{}'", data.name);
    };
    let Some(executor) = command.command() else {
        bail!("missing command handler for '{}'", data.name);
    };

    executor.execute(Ctx::new(api, event, &(**data))).await
}

/// Handles an auto-completion interaction event.
///
/// # Errors
///
/// This function will return an error if completion execution fails.
async fn on_complete(api: ApiRef<'_>, event: &Interaction) -> Result {
    let Some(InteractionData::ApplicationCommand(ref data)) = event.data else {
        bail!("missing command data");
    };
    let Some(command) = crate::cmd::registry().get(&data.name) else {
        bail!("missing bot command '{}'", data.name);
    };
    let Some(executor) = command.complete() else {
        bail!("missing bot command handler for '{}'", data.name);
    };

    let Some(focus) = data.options.iter().find_map(|o| match &o.value {
        CommandOptionValue::Focused(n, k) => Some((&(*o.name), &(**n), *k)),
        CommandOptionValue::SubCommand(c) => c.iter().find_map(|o| match &o.value {
            CommandOptionValue::Focused(n, k) => Some((&(*o.name), &(**n), *k)),
            _ => None,
        }),
        CommandOptionValue::SubCommandGroup(g) => g.iter().find_map(|c| match &c.value {
            CommandOptionValue::SubCommand(c) => c.iter().find_map(|o| match &o.value {
                CommandOptionValue::Focused(n, k) => Some((&(*o.name), &(**n), *k)),
                _ => None,
            }),
            _ => None,
        }),
        _ => None,
    }) else {
        bail!("an option is not currently focused");
    };

    let ctx = Ctx::new(api, event, &(**data));
    let choices = executor.execute(ctx, focus).await?;

    crate::respond!(as ctx => {
        let kind = ApplicationCommandAutocompleteResult;
        let choices = choices;
    })
    .await?;

    Ok(())
}

/// Handles a component interaction event.
///
/// # Errors
///
/// This function will return an error if component execution fails.
async fn on_component(api: ApiRef<'_>, event: &Interaction) -> Result {
    let Some(InteractionData::MessageComponent(ref data)) = event.data else {
        bail!("missing component data");
    };
    let id = data.custom_id.parse::<DataId>()?;

    let Some(command) = crate::cmd::registry().get(id.name()) else {
        bail!("missing component '{}'", id.name());
    };
    let Some(executor) = command.component() else {
        bail!("missing component handler for '{}'", id.name());
    };

    executor.execute(Ctx::new(api, event, data), id).await
}

/// Handles a modal interaction event.
///
/// # Errors
///
/// This function will return an error if modal execution fails.
async fn on_modal(api: ApiRef<'_>, event: &Interaction) -> Result {
    let Some(InteractionData::ModalSubmit(ref data)) = event.data else {
        bail!("missing modal data");
    };
    let id = data.custom_id.parse::<DataId>()?;

    let Some(command) = crate::cmd::registry().get(id.name()) else {
        bail!("missing modal '{}'", id.name());
    };
    let Some(executor) = command.modal() else {
        bail!("missing modal handler for '{}'", id.name());
    };

    executor.execute(Ctx::new(api, event, data), id).await
}

/// Called to notify an executing user and the bot developer(s) when an error occurs.
///
/// # Errors
///
/// This function will return an error if the logger could not queue a log.
async fn on_error(api: ApiRef<'_>, event: &Interaction, error: &anyhow::Error) -> Result {
    if let Err(error) = self::on_error_notify_user(api, event, error).await {
        error!("unable to notify executing user: {error}")?;
    }
    if let Err(error) = self::on_error_notify_devs(api, event, error).await {
        error!("unable to notify bot developers: {error}")?;
    }

    Ok(())
}

/// Called to notify an executing user when an error occurs.
///
/// # Errors
///
/// This function will return an error if the logger could not queue a log.
async fn on_error_notify_user(
    api: ApiRef<'_>,
    event: &Interaction,
    error: &anyhow::Error,
) -> Result {
    use InteractionType::{ApplicationCommand, MessageComponent, ModalSubmit};

    if !matches!(event.kind, ApplicationCommand | MessageComponent | ModalSubmit) {
        return Ok(());
    }

    let locale = event.author().preferred_locale();
    let index = thread_rng().gen_range(0 .. ERROR_TITLES);
    let title = localize!(try in locale, "text.error.title_{index}");
    let embed = EmbedBuilder::new().color(FAILURE).description(format!("> {error}")).title(title);

    crate::respond!(as api.http, event => {
        let kind = DeferredChannelMessageWithSource;
        let flags = EPHEMERAL;
    })
    .await
    .ok();

    crate::followup!(as api.http, event => {
        let embeds = &[embed.build()];
        let flags = EPHEMERAL;
    })
    .await?;

    Ok(())
}

/// Called to notify the bot developer(s) when an error occurs.
///
/// # Errors
///
/// This function will return an error if the logger could not queue a log.
async fn on_error_notify_devs(
    api: ApiRef<'_>,
    event: &Interaction,
    error: &anyhow::Error,
) -> Result {
    let index = thread_rng().gen_range(0 .. ERROR_TITLES);
    let title = localize!("text.error.title_{index}");
    let mut embed = EmbedBuilder::new()
        .color(FAILURE)
        .description(format!("**ID:** `{}`\n\n```json\n{error}\n```", event.marker()))
        .title(title);

    if let Some(user) = event.author() {
        embed = embed.author(EmbedAuthor::parse(user)?);
    }

    api.http
        .create_message(crate::util::secrets::error_channel_id()?)
        .embeds(&[embed.build()])?
        .flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
        .await?;

    Ok(())
}
