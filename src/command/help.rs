use twilight_model::application::command::Command;
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder};

use crate::event::{CachedHttp, CommandContext, EventHandler, EventResult};
use crate::traits::TryFromUser;
use crate::utility::BRANDING_COLOR;

crate::command! {
    TYPE = ChatInput,
    NAME = "help",
    DMS = true,
    NSFW = false,
    REQUIRES = [USE_SLASH_COMMANDS],
}

#[async_trait::async_trait]
impl EventHandler for This {
    async fn command(&self, ctx: &CommandContext) -> EventResult {
        crate::respond!(ctx, {
            KIND = DeferredChannelMessageWithSource,
            FLAGS = [EPHEMERAL],
        })
        .await?;

        let mut text = crate::localize!(ctx.locale() => "text.{}.header", Self::NAME).into_owned();

        if let Some(guild_id) = ctx.event.guild_id {
            let commands = ctx.client().guild_commands(guild_id).await?.model().await?;

            if !commands.is_empty() {
                let header = crate::localize!(ctx.locale() => "text.{}.guild", Self::NAME);

                text += format!("\n\n**__{header}__**\n").as_str();
                text += &stringify_all(ctx.locale(), &commands);
            }
        }

        let commands = ctx.client().global_commands().await?.model().await?;
        let header = crate::localize!(ctx.locale() => "text.{}.global", Self::NAME);
        let footer = crate::localize!(ctx.locale() => "text.{}.footer", Self::NAME);

        text += format!("\n\n**__{header}__**\n").as_str();
        text += &stringify_all(ctx.locale(), &commands);
        text += &if commands.is_empty() {
            let missing = crate::localize!(ctx.locale() => "text.{}.missing", Self::NAME);

            format!("> *{missing}*").into_boxed_str()
        } else {
            stringify_all(ctx.locale(), &commands)
        };
        text += format!("\n\n{footer}").as_str();

        let user = if let Some(user) = ctx.cache().current_user() {
            user
        } else {
            ctx.http().current_user().await?.model().await?
        };

        let embed = EmbedBuilder::new()
            .author(EmbedAuthorBuilder::try_from_user(&user)?)
            .color(BRANDING_COLOR.into())
            .description(text)
            .title(crate::localize!(ctx.locale() => "text.{}.title", Self::NAME));

        crate::followup!(ctx, {
            EMBEDS = [embed.build()],
            FLAGS = [EPHEMERAL],
        })
        .await?;

        EventResult::Ok(())
    }
}

/// Returns a stringified list of command entries.
#[inline]
#[must_use]
pub fn stringify_all(locale: Option<&str>, commands: &[Command]) -> Box<str> {
    let string = commands.iter().fold(String::new(), |s, c| {
        s + format!("\n\n{}", stringify(locale, c)).as_str()
    });

    string.trim().into()
}

/// Returns a stringified command entry.
#[must_use]
pub fn stringify(locale: Option<&str>, command: &Command) -> Box<str> {
    let Command { name, id, .. } = command;
    let localized_name = crate::localize!(locale => "command.{name}.name");
    let description = crate::localize!(locale => "command.{name}.description");
    let mut flags = vec![];

    let entry = id.map_or_else(
        || format!("`/{localized_name}` - {description}"),
        |id| format!("</{name}:{id}> - {description}"),
    );

    if command.dm_permission.unwrap_or(false) {
        flags.push(crate::localize!(locale => "text.{}.dms", This::NAME));
    }
    if command.options.iter().any(|o| o.options.is_some()) {
        flags.push(crate::localize!(locale => "text.{}.subcommands", This::NAME));
    }
    if command.nsfw.unwrap_or(false) {
        flags.push(crate::localize!(locale => "text.{}.nsfw", This::NAME));
    }

    if flags.is_empty() {
        entry
    } else {
        format!("{entry}\n> *{}*", flags.join(", "))
    }
    .into_boxed_str()
}
