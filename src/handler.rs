use serenity::{
    all::{OnlineStatus, Ready},
    gateway::ActivityData,
    prelude::EventHandler,
};

use crate::prelude::*;

pub struct Handler {
    pub logger: Logger,
}

impl Handler {
    pub const fn new(logger: Logger) -> Self {
        Self { logger }
    }

    pub async fn create_commands(&self, http: &Http) -> Result<()> {
        let guild_id = get_dev_guild()?;
        let cmds = vec![
            crate::command::data::new(),
            crate::command::embed::new(),
            crate::command::help::new(),
            crate::command::ping::new(),
        ];

        let global = if IS_DEV {
            http.get_global_application_commands().await?.len()
        } else {
            http.create_global_application_commands(&cmds).await?.len()
        };

        info!(self.logger, "Created {global} global application commands");

        let local = guild_id.set_application_commands(http, cmds).await?.len();

        info!(self.logger, "Created {local} local application commands");

        Ok(())
    }

    async fn error_show(
        &self,
        context: &Context,
        interaction: &Interaction,
        error: &Error,
    ) -> Result<()> {
        let embed = CreateEmbed::new()
            .color(Color::RED)
            .description(format!("> {error}"))
            .title("An error occurred!");
        let builder = CreateInteractionResponseFollowup::new()
            .embed(embed)
            .ephemeral(true);

        match &interaction {
            Interaction::Autocomplete(i) => i.create_followup(&context, builder).await?,
            Interaction::Command(i) => i.create_followup(&context, builder).await?,
            Interaction::Component(i) => i.create_followup(&context, builder).await?,
            Interaction::Modal(i) => i.create_followup(&context, builder).await?,
            Interaction::Ping(_) => err_wrap!("interaction does not support followups")?,
        };

        Ok(())
    }
    async fn error_log(
        &self,
        context: &Context,
        interaction: &Interaction,
        id: &str,
        error: &Error,
    ) -> Result<()> {
        let mut embed = CreateEmbed::new()
            .color(Color::RED)
            .description(format!("ID: `{id}`\n\n> {error}"))
            .title("Encountered an error");

        if let Some(user) = match interaction {
            Interaction::Autocomplete(i) | Interaction::Command(i) => Some(&i.user),
            Interaction::Component(i) => Some(&i.user),
            Interaction::Modal(i) => Some(&i.user),
            Interaction::Ping(_) => None,
        } {
            embed = embed.author(CreateEmbedAuthor::new(user.tag()).icon_url(user.face()));
        }

        let builder = CreateMessage::new().embed(embed);
        let guild_id = get_error_guild()?;
        let channel_id = get_error_channel()?;
        let channel = Anchor::get_guild_channel(context, guild_id, channel_id).await?;

        channel.send_message(context, builder).await?;
        Ok(())
    }
    async fn error(&self, context: &Context, interaction: &Interaction, id: &str, error: &Error) {
        error!(self.logger, "Interaction failed: {id} - {error}");

        if let Err(error) = self.error_show(context, interaction, error).await {
            error!(self.logger, "Unable to show error to member: {error}");
        }
        if let Err(error) = self.error_log(context, interaction, id, error).await {
            error!(self.logger, "Unable to log error: {error}");
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, context: Context, ready: Ready) {
        info!(self.logger, "Connected as {}!", ready.user.tag());

        if let Some(count) = ready.shard.map(|s| s.total) {
            info!(self.logger, "Using {count} shard(s)");
        }

        context.set_presence(Some(ActivityData::listening("/help")), OnlineStatus::Idle);

        if let Err(error) = self.create_commands(&context.http).await {
            error!(self.logger, "Error creating commands: {error}");
        }
    }

    #[allow(clippy::match_single_binding)]
    async fn interaction_create(&self, context: Context, mut interaction: Interaction) {
        let id = match &interaction {
            Interaction::Autocomplete(i) => format!("{}<acp:{}>", i.data.name, i.id),
            Interaction::Command(i) => format!("{}<cmd:{}>", i.data.name, i.id),
            Interaction::Component(i) => format!("{}<cpn:{}>", i.data.custom_id, i.id),
            Interaction::Modal(i) => format!("{}<mdl:{}>", i.data.custom_id, i.id),
            Interaction::Ping(i) => format!("{}<png:{}>", i.token, i.id),
        };

        let result: Result<()> = match &mut interaction {
            Interaction::Autocomplete(acp) => match acp.data.name.as_str() {
                _ => err_wrap!("unknown autocomplete: {id}"),
            },
            Interaction::Command(cmd) => match cmd.data.name.as_str() {
                crate::command::data::NAME => crate::command::data::command(&context, cmd).await,
                crate::command::embed::NAME => crate::command::embed::command(&context, cmd).await,
                crate::command::help::NAME => crate::command::help::command(&context, cmd).await,
                crate::command::ping::NAME => crate::command::ping::command(&context, cmd).await,
                _ => err_wrap!("unknown command: {id}"),
            },
            Interaction::Component(cpn) => match CustomId::from_str(cpn.data.custom_id.as_str()) {
                Ok(custom_id) => match custom_id.base.as_str() {
                    _ => err_wrap!("unknown component: {id}"),
                },
                Err(error) => Err(error),
            },
            Interaction::Modal(mdl) => match CustomId::from_str(mdl.data.custom_id.as_str()) {
                Ok(custom_id) => match custom_id.base.as_str() {
                    _ => err_wrap!("unknown modal: {id}"),
                },
                Err(error) => Err(error),
            },
            Interaction::Ping(_) => Ok(()),
        };

        if let Err(error) = result {
            self.error(&context, &interaction, &id, &error).await;
        } else {
            info!(self.logger, "Interaction succeeded: {id}");
        }
    }
}
