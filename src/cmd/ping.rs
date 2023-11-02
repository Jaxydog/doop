use doop_localizer::localize;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::interaction::CommandCtx;
use crate::cmd::OnCommand;
use crate::util::traits::{Created, PreferLocale};
use crate::util::{Result, BRANDING};

crate::register_command! {
    ChatInput("ping") {
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
    ctx: CommandCtx<'api, 'evt>,
) -> Result {
    let locale = ctx.event.author().preferred_locale();
    let title = localize!(try in locale, "text.{}.calculate", cmd.entry().name);
    let embed = EmbedBuilder::new().color(BRANDING).title(title);

    crate::respond!(as ctx => {
        let kind = ChannelMessageWithSource;
        let embeds = [embed.clone().build()];
        let flags = EPHEMERAL;
    })
    .await?;

    let response = ctx.client().response(&ctx.event.token).await?.model().await?;
    let delay = response.id.created_at() - ctx.event.id.created_at();
    let title = localize!("text.{}.finished", cmd.entry().name);
    let embed = embed.title(format!("{title} ({delay})")).build();

    ctx.client().update_response(&ctx.event.token).embeds(Some(&[embed]))?.await?;

    Ok(())
}
