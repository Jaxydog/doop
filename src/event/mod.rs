use std::sync::Arc;

use anyhow::anyhow;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::Event;
use twilight_http::Client;
use twilight_model::application::command::{Command, CommandOptionChoice};
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::{InteractionCreate, Ready};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder};

pub use self::context::*;
pub use self::result::*;
use crate::command::CommandState;
use crate::extend::{InteractionCreateExt, IteratorExt};
use crate::traits::TryFromUser;
use crate::utility::{DataId, Result, FAILURE_COLOR};
use crate::{error, info, warn};

/// Contains interaction context definitions.
mod context;
/// Defines a custom event-specific result type.
mod result;

/// Implements an API event handler for bot commands.
#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait EventHandler: CommandState + Send + Sync {
    /// Supplies an application command interaction's autofill.
    async fn autofill(&self, ctx: &CommandCtx<'_>) -> EventResult<Vec<CommandOptionChoice>> {
        EventResult::Err(anyhow!("unsupported interaction type"))
    }
    /// Handles an application command interaction.
    async fn command(&self, ctx: &CommandCtx<'_>) -> EventResult {
        EventResult::Err(anyhow!("unsupported interaction type"))
    }
    /// Handles a message component interaction.
    async fn component(&self, ctx: &ComponentCtx<'_>) -> EventResult {
        EventResult::Err(anyhow!("unsupported interaction type"))
    }
    /// Handles a modal submission interaction.
    async fn modal(&self, ctx: &ModalCtx<'_>) -> EventResult {
        EventResult::Err(anyhow!("unsupported interaction type"))
    }
}

crate::global! {{
    /// Returns the bot's list of event handlers.
    [HANDLERS] fn handlers() -> Box<[&'static dyn EventHandler]> { || crate::heap!(box [
        &crate::command::embed::This,
        &crate::command::help::This,
        &crate::command::ping::This,
        &crate::command::role::This,
    ])}
}}

/// Returns an event handler with the provided name.
#[inline]
#[must_use]
pub fn handler(name: &str) -> Option<&'static dyn EventHandler> {
    handlers().iter().find(|h| h.name() == name).copied()
}

/// Builds and returns the bot's list of commands.
#[inline]
#[must_use]
pub fn commands(guild_id: Option<Id<GuildMarker>>) -> Box<[Command]> {
    let commands = handlers().iter().try_filter_map(|handler| {
        let result = handler.build(guild_id);

        if let Err(ref error) = result {
            warn!("unable to build command: {error}")?;
        }

        result
    });

    commands.collect()
}

/// Handles bot shard API events.
pub async fn handle(http: Arc<Client>, cache: Arc<InMemoryCache>, event: Event) -> Result {
    let result = match event {
        Event::Ready(event) => ready(&http, *event).await,
        Event::InteractionCreate(event) => interaction(&http, &cache, *event).await,
        _ => EventResult::Ok(()),
    };

    match result {
        EventResult::Ok(()) => Ok(()),
        EventResult::Err(error) => warn!("{error}"),
        EventResult::Fatal(fatal) => error!("{fatal}").and(Err(fatal)),
    }
}

/// Handles the bot's ready events.
async fn ready(http: &Client, event: Ready) -> EventResult {
    info!("connected to the discord api")?;

    let client = http.interaction(event.application.id);

    if let Ok(id) = crate::utility::env::guild_id() {
        client.set_guild_commands(id, &commands(Some(id))).await?;
    }
    if cfg!(not(debug_assertions)) {
        client.set_global_commands(&commands(None)).await?;
    }

    EventResult::Ok(())
}

/// Handles the bot's interaction create events.
async fn interaction(
    http: &Client,
    cache: &InMemoryCache,
    event: InteractionCreate,
) -> EventResult {
    info!("received interaction: {}", event.label())?;

    let result = match event.kind {
        InteractionType::ApplicationCommandAutocomplete => autofill(http, cache, &event.0).await,
        InteractionType::ApplicationCommand => command(http, cache, &event.0).await,
        InteractionType::MessageComponent => component(http, cache, &event.0).await,
        InteractionType::ModalSubmit => modal(http, cache, &event.0).await,
        _ => EventResult::Ok(()),
    };

    match result.as_ref() {
        EventResult::Ok(()) => info!("interaction succeeded: {}", event.label()),
        EventResult::Err(error) | EventResult::Fatal(error) => {
            warn!("interaction failed: {}", event.label())?;
            handle_error(error, http, &event).await
        }
    }?;

    result
}

/// Handles the bot's autofill interaction shard events.
async fn autofill(http: &Client, cache: &InMemoryCache, interaction: &Interaction) -> EventResult {
    let Some(InteractionData::ApplicationCommand(data)) = &interaction.data else {
        return EventResult::Err(anyhow!("missing component data"));
    };
    let Some(handler) = handler(&data.name) else {
        return EventResult::Err(anyhow!("missing handler for '{}'", data.name));
    };
    let ctx = Ctx { http, cache, interaction, data: &(**data) };

    crate::respond!(ctx, {
        KIND = ApplicationCommandAutocompleteResult,
        CHOICES = handler.autofill(&ctx).await?,
    })
    .await?;

    EventResult::Ok(())
}

/// Handles the bot's command interaction shard events.
async fn command(http: &Client, cache: &InMemoryCache, interaction: &Interaction) -> EventResult {
    let Some(InteractionData::ApplicationCommand(data)) = &interaction.data else {
        return EventResult::Err(anyhow!("missing component data"));
    };
    let Some(handler) = handler(&data.name) else {
        return EventResult::Err(anyhow!("missing handler for '{}'", data.name));
    };
    let ctx = Ctx { http, cache, interaction, data: &(**data) };

    handler.command(&ctx).await
}

/// Handles the bot's component interaction shard events.
async fn component(http: &Client, cache: &InMemoryCache, interaction: &Interaction) -> EventResult {
    let Some(InteractionData::MessageComponent(data)) = &interaction.data else {
        return EventResult::Err(anyhow!("missing component data"));
    };

    let id = DataId::try_from(data.custom_id.as_str())?;

    let Some(handler) = handler(id.base()) else {
        return EventResult::Err(anyhow!("missing handler for '{}'", id.base()));
    };
    let ctx = Ctx { http, cache, interaction, data: (data, id) };

    handler.component(&ctx).await
}

/// Handles the bot's modal interaction shard events.
async fn modal(http: &Client, cache: &InMemoryCache, interaction: &Interaction) -> EventResult {
    let Some(InteractionData::ModalSubmit(data)) = &interaction.data else {
        return EventResult::Err(anyhow!("missing modal data"));
    };

    let id = DataId::try_from(data.custom_id.as_str())?;

    let Some(handler) = handler(id.base()) else {
        return EventResult::Err(anyhow!("missing handler for '{}'", id.base()));
    };
    let ctx = Ctx { http, cache, interaction, data: (data, id) };

    handler.modal(&ctx).await
}

/// Called when an event handler returns an error.
async fn handle_error(error: &anyhow::Error, http: &Client, event: &InteractionCreate) -> Result {
    if let Err(error) = handle_error_notify(error, http, event).await {
        error!("unable to notify executing user: {error}")?;
    }
    if let Err(error) = handle_error_store(error, http, event).await {
        error!("unable to store in error channel: {error}")?;
    }

    Ok(())
}

/// Notifies the executing user when an error occurs.
async fn handle_error_notify(
    error: &anyhow::Error,
    http: &Client,
    event: &InteractionCreate,
) -> Result {
    let locale = event.user.as_ref().and_then(|u| u.locale.as_deref());
    let embed = EmbedBuilder::new()
        .color(FAILURE_COLOR.into())
        .description(format!("> {error}"))
        .title(crate::localize!(locale => "text.error.title"));

    crate::respond!(http, event, {
        KIND = DeferredChannelMessageWithSource,
        FLAGS = [EPHEMERAL],
    })
    .await
    .ok();

    crate::followup!(http, event, {
        EMBEDS = [embed.build()],
        FLAGS = [EPHEMERAL],
    })
    .await?;

    Ok(())
}

/// Stores an error in the developer error channel.
async fn handle_error_store(
    error: &anyhow::Error,
    http: &Client,
    event: &InteractionCreate,
) -> Result {
    let mut embed = EmbedBuilder::new()
        .color(FAILURE_COLOR.into())
        .description(format!("ID: `{}`\n\n> {error}", event.label()))
        .title(crate::localize!("text.error.title"));

    if let Some(user) = event.user.as_ref() {
        embed = embed.author(EmbedAuthorBuilder::try_from_user(user)?);
    }

    http.create_message(crate::utility::env::channel_id()?)
        .embeds(&[embed.build()])?
        .flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
        .await?;

    Ok(())
}
