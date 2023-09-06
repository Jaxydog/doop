use doop_localizer::{localize, Locale};
use twilight_model::application::command::Command;
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

use crate::bot::interact::{CommandCtx, InteractionHandler};
use crate::util::ext::EmbedAuthorExt;
use crate::util::traits::Localized;
use crate::util::{Result, BRANDING};

crate::command! {
    let name = "help";
    let kind = ChatInput;
    let permissions = USE_SLASH_COMMANDS;
    let allow_dms = true;
    let is_nsfw = false;
}

#[async_trait::async_trait]
impl InteractionHandler for Impl {
    async fn handle_command<'api: 'evt, 'evt>(&self, ctx: CommandCtx<'api, 'evt>) -> Result {
        crate::respond!(as ctx => {
            let kind = DeferredChannelMessageWithSource;
            let flags = EPHEMERAL;
        })
        .await?;

        let locale = ctx.event.author().locale();
        let mut text = localize!(try locale => "text.{}.header", Self::NAME).into_owned();

        if let Some(guild_id) = ctx.event.guild_id {
            let commands = ctx.client().guild_commands(guild_id).await?.model().await?;

            if !commands.is_empty() {
                let header = localize!(try locale => "text.{}.guild_header", Self::NAME);

                text += &format!("\n\n**__{header}__**\n");
                text += &stringify_all(locale, &commands);
            }
        }

        let commands = ctx.client().global_commands().await?.model().await?;
        let header = localize!(try locale => "text.{}.global_header", Self::NAME);
        let footer = localize!(try locale => "text.{}.footer", Self::NAME);

        text += &format!("\n\n**__{header}__**\n");
        text += &if commands.is_empty() {
            format!("> *{}*", localize!(try locale => "text.{}.missing", Self::NAME))
        } else {
            stringify_all(locale, &commands)
        };

        let author = if let Some(user) = ctx.api.cache().current_user() {
            EmbedAuthor::new_from(&user)
        } else {
            let user = ctx.api.http().current_user().await?.model().await?;

            EmbedAuthor::new_from(&user)
        }?;
        let title = localize!(try locale => "text.{}.title", Self::NAME);
        let embed = EmbedBuilder::new()
            .author(author)
            .color(BRANDING)
            .description(text)
            .title(title)
            .footer(EmbedFooterBuilder::new(footer));

        crate::followup!(as ctx => {
            let embeds = &[embed.build()];
            let flags = EPHEMERAL;
        })
        .await?;

        Ok(())
    }
}

/// Returns a stringified list of command entries.
#[inline]
#[must_use]
pub fn stringify_all(locale: Locale, commands: &[Command]) -> String {
    commands.iter().map(|c| stringify(locale, c)).collect::<Vec<_>>().join("\n\n")
}

/// Returns a stringified command entry.
#[must_use]
pub fn stringify(locale: Locale, command: &Command) -> String {
    let Command { name, id, dm_permission, nsfw, options, .. } = command;
    let localized_name = localize!(try locale => "command.{name}.name");
    let localized_description = localize!(try locale => "command.{name}.description");
    let mut flags = vec![];

    if options.iter().any(|o| o.options.is_some()) {
        flags.push(localize!(try locale => "text.{}.has_subcommands", Impl::NAME));
    }
    if dm_permission.unwrap_or_default() {
        flags.push(localize!(try locale => "text.{}.allow_dms", Impl::NAME));
    }
    if nsfw.unwrap_or_default() {
        flags.push(localize!(try locale => "text.{}.is_nsfw", Impl::NAME));
    }

    let entry = id.map_or_else(
        || format!("`/{localized_name}` - {localized_description}"),
        |id| format!("</{name}:{id}> - {localized_description}"),
    );

    if flags.is_empty() {
        entry
    } else {
        format!("{entry}\n> *{}*", flags.join(", "))
    }
}
