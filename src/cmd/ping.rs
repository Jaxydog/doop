use doop_localizer::localize;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::interact::{CommandCtx, InteractionEventHandler};
use crate::util::ext::{CreatedAtExt, LocalizedExt};
use crate::util::{Result, BRANDING};

crate::command! {
    let name = "ping";
    let kind = ChatInput;
    let permissions = USE_SLASH_COMMANDS;
    let allow_dms = true;
    let is_nsfw = false;
}

#[async_trait::async_trait]
impl InteractionEventHandler for Impl {
    async fn handle_command(&self, ctx: CommandCtx<'_>) -> Result {
        let locale = ctx.event.author().locale();
        let title = localize!(try locale => "text.{}.loading", Self::NAME);
        let embed = EmbedBuilder::new().color(BRANDING).title(title);

        crate::respond!(as ctx => {
            let kind = ChannelMessageWithSource;
            let embeds = [embed.clone().build()];
            let flags = EPHEMERAL;
        })
        .await?;

        let response = ctx.client().response(&ctx.event.token).await?.model().await?;
        let delay = response.id.created_at() - ctx.event.id.created_at();
        let title = localize!(try locale => "text.{}.ready", Self::NAME);
        let embed = embed.title(format!("{title} ({delay})")).build();

        ctx.client().update_response(&ctx.event.token).embeds(Some(&[embed]))?.await?;

        Ok(())
    }
}
