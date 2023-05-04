use serenity::all::{Interaction, OnlineStatus, Ready};
use serenity::async_trait;
use serenity::builder::{
    CreateCommand, CreateEmbed, CreateEmbedAuthor, CreateInteractionResponseFollowup, CreateMessage,
};
use serenity::gateway::ActivityData;
use serenity::http::Http;
use serenity::prelude::Context;

use crate::common::{fetch_guild_channel, CustomId};
use crate::util::{
    get_dev_guild_id, get_err_channel_id, Error, Result, BOT_FAILURE_COLOR, DEV_BUILD,
};
use crate::{err_wrap, error, info, warn};

/// The bot's event handler structure
#[derive(Debug, Default)]
pub struct EventHandler;

impl EventHandler {
    /// Returns the bot's application commands
    #[must_use]
    pub fn get_command_builders() -> Vec<CreateCommand> {
        // I think it's better to have these in their own function rather than inside of
        // the `create_commands` function like I used to do on older bots.
        vec![
            crate::cmd::embed::create(),
            crate::cmd::help::create(),
            crate::cmd::mail::create(),
            crate::cmd::ping::create(),
            crate::cmd::role::create(),
        ]
    }

    /// Returns the internal label for the provided interaction
    #[must_use]
    pub fn get_interaction_label(interaction: &Interaction) -> String {
        format!("<{:?}::{}>", interaction.kind(), interaction.id()).to_lowercase()
    }

    /// Registers all of the client's application commands
    pub async fn create_commands(&self, http: &Http) -> Result<()> {
        let guild_id = get_dev_guild_id()?;
        let commands = Self::get_command_builders();

        let local = http.create_guild_commands(guild_id, &commands).await?;

        info!("found {} local commands", local.len());

        let global = if DEV_BUILD {
            http.get_global_commands().await?
        } else {
            http.create_global_commands(&commands).await?
        };

        info!("found {} global commands", global.len());

        Ok(())
    }

    /// Displays an error message to the user upon encountering an error
    pub async fn error_notify_user(
        &self,
        context: &Context,
        interaction: &Interaction,
        error: &Error,
    ) -> Result<()> {
        let embed = CreateEmbed::new()
            .color(BOT_FAILURE_COLOR)
            .description(format!("> {error}"))
            .title("Something went wrong!");
        let builder = CreateInteractionResponseFollowup::new()
            .embed(embed)
            .ephemeral(true);

        match interaction {
            Interaction::Command(i) => i.create_followup(context, builder).await?,
            Interaction::Autocomplete(i) => i.create_followup(context, builder).await?,
            Interaction::Component(i) => i.create_followup(context, builder).await?,
            Interaction::Modal(i) => i.create_followup(context, builder).await?,
            _ => err_wrap!("interaction does not support follow-ups")?,
        };

        Ok(())
    }

    /// Displays an error message to the configured error channel upon
    /// encountering an error
    pub async fn error_output_log(
        &self,
        context: &Context,
        interaction: &Interaction,
        error: &Error,
    ) -> Result<()> {
        let label = Self::get_interaction_label(interaction);
        let mut embed = CreateEmbed::new()
            .color(BOT_FAILURE_COLOR)
            .description(format!("ID: `{label}`\n\n> {error}"))
            .title("Encountered an error!");

        if let Some(user) = match interaction {
            Interaction::Command(i) | Interaction::Autocomplete(i) => Some(&i.user),
            Interaction::Component(i) => Some(&i.user),
            Interaction::Modal(i) => Some(&i.user),
            _ => None,
        } {
            embed = embed.author(CreateEmbedAuthor::new(user.tag()).icon_url(user.face()));
        }

        let builder = CreateMessage::new().embed(embed);
        let guild_id = get_dev_guild_id()?;
        let channel_id = get_err_channel_id()?;
        let channel = fetch_guild_channel(context, guild_id, channel_id).await?;

        channel.send_message(context, builder).await?;

        Ok(())
    }

    /// Called when an error is encountered to handle it automatically
    pub async fn error(&self, context: &Context, interaction: &Interaction, error: &Error) {
        if let Err(error) = self.error_notify_user(context, interaction, error).await {
            error!("unable to notify user: {error}");
        }
        if let Err(error) = self.error_output_log(context, interaction, error).await {
            error!("unable to log error: {error}");
        }
    }
}

#[async_trait]
impl serenity::all::EventHandler for EventHandler {
    async fn ready(&self, context: Context, ready: Ready) {
        info!("connected as '{}'", ready.user.tag());

        if let Some(count) = ready.shard.map(|s| s.total) {
            info!("using {count} shards");
        }

        context.set_presence(Some(ActivityData::listening("/help")), OnlineStatus::Idle);

        if let Err(error) = self.create_commands(&context.http).await {
            error!("error updating commands: {error}");
        }
    }

    #[allow(clippy::match_single_binding)] // TODO: remove this once all single cases are gone
    async fn interaction_create(&self, context: Context, mut interaction: Interaction) {
        let label = Self::get_interaction_label(&interaction);

        let result = match &mut interaction {
            Interaction::Autocomplete(a) => match a.data.name.as_str() {
                n => err_wrap!("unknown autocomplete: {n}"),
            },
            Interaction::Command(c) => match c.data.name.as_str() {
                crate::cmd::embed::NAME => crate::cmd::embed::handle_commands(&context, c).await,
                crate::cmd::help::NAME => crate::cmd::help::handle_commands(&context, c).await,
                crate::cmd::mail::NAME => crate::cmd::mail::handle_commands(&context, c).await,
                crate::cmd::ping::NAME => crate::cmd::ping::handle_commands(&context, c).await,
                crate::cmd::role::NAME => crate::cmd::role::handle_commands(&context, c).await,
                n => err_wrap!("unknown command: {n}"),
            },
            Interaction::Component(c) => match CustomId::try_from(c.data.custom_id.as_str()) {
                Err(e) => Err(e),
                Ok(id) => match id.base.as_str() {
                    crate::cmd::role::NAME => {
                        crate::cmd::role::handle_components(&context, c, id).await
                    }
                    n => err_wrap!("unknown component: {n}"),
                },
            },
            Interaction::Modal(m) => match CustomId::try_from(m.data.custom_id.as_str()) {
                Err(e) => Err(e),
                Ok(id) => match id.base.as_str() {
                    n => err_wrap!("unknown modal: {n}"),
                },
            },
            Interaction::Ping(_) => err_wrap!("unexpected ping"),
            _ => err_wrap!("unsupported interaction type"),
        };

        if let Err(error) = result {
            warn!("interaction failed: {label} - {error}");

            self.error(&context, &interaction, &error).await;
        } else {
            info!("interaction succeeded: {label}");
        }
    }
}