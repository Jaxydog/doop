use doop_localizer::{localize, Locale};
use twilight_model::application::command::Command;
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

use crate::bot::interaction::CommandCtx;
use crate::cmd::{CommandEntry, OnCommand};
use crate::util::extension::EmbedAuthorExtension;
use crate::util::traits::PreferLocale;
use crate::util::{Result, BRANDING};

crate::register_command! {
    ChatInput("help") {
        let in_dms = true;
        let is_nsfw = false;
        let require = USE_SLASH_COMMANDS | SEND_MESSAGES;
        let handlers = {
            command = self::execute_command;
        };
    }
}

async fn execute_command<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    mut ctx: CommandCtx<'api, 'evt>,
) -> Result {
    ctx.defer(true).await?;

    let locale = ctx.event.author().preferred_locale();
    let mut text = localize!(try in locale, "text.{}.header", cmd.entry().name).into_owned();

    if let Some(guild_id) = ctx.event.guild_id {
        let commands = ctx.client().guild_commands(guild_id).await?.model().await?;

        if !commands.is_empty() {
            let header = localize!(try in locale, "text.{}.server_header", cmd.entry().name);

            text += &format!("\n\n**__{header}__**\n");
            text += &self::stringify_all(*cmd.entry(), locale, &commands);
        }
    }

    let commands = ctx.client().global_commands().await?.model().await?;
    let header = localize!(try in locale, "text.{}.global_header", cmd.entry().name);
    let footer = localize!(try in locale, "text.{}.footer", cmd.entry().name);

    text += &format!("\n\n**__{header}__**\n");
    text += &if commands.is_empty() {
        format!("> *{}*", localize!(try in locale, "text.{}.missing_commands", cmd.entry().name))
    } else {
        self::stringify_all(*cmd.entry(), locale, &commands)
    };

    let author = if let Some(user) = ctx.api.cache.current_user() {
        EmbedAuthor::parse(&user)
    } else {
        let user = ctx.api.http.current_user().await?.model().await?;

        EmbedAuthor::parse(&user)
    }?;
    let title = localize!(try in locale, "text.{}.title", cmd.entry().name);
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

#[inline]
fn stringify_all(entry: CommandEntry, locale: Locale, commands: &[Command]) -> String {
    commands.iter().map(|c| self::stringify(entry, locale, c)).collect::<Vec<_>>().join("\n")
}

fn stringify(entry: CommandEntry, locale: Locale, command: &Command) -> String {
    let Command { name, id, dm_permission, nsfw, options, .. } = command;

    let localized_name = localize!(try in locale, "command.{name}.name");
    let localized_description = localize!(try in locale, "command.{name}.description");

    let mut flags = vec![];
    let content = id.map_or_else(
        || format!("- `/{localized_name}` - {localized_description}"),
        |id| format!("- </{name}:{id}> - {localized_description}"),
    );

    if options.iter().any(|o| o.options.is_some()) {
        flags.push(localize!(try in locale, "text.{}.has_subcommands", entry.name));
    }
    if dm_permission.unwrap_or(true) {
        flags.push(localize!(try in locale, "text.{}.allows_dms", entry.name));
    }
    if nsfw.unwrap_or(false) {
        flags.push(localize!(try in locale, "text.{}.is_nsfw", entry.name));
    }

    if flags.is_empty() { content } else { format!("{content}\n> *{}*", flags.join(", ")) }
}
