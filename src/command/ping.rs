use twilight_util::builder::embed::EmbedBuilder;

use crate::event::{CommandCtx, EventHandler, EventResult};
use crate::extend::IdExt;
use crate::utility::BRANDING_COLOR;

crate::command! {
    TYPE = ChatInput,
    NAME = "ping",
    DMS = true,
    NSFW = false,
    REQUIRES = [USE_SLASH_COMMANDS],
}

#[async_trait::async_trait]
impl EventHandler for This {
    async fn command(&self, ctx: &CommandCtx<'_>) -> EventResult {
        let title = crate::localize!(ctx.locale() => "text.{}.wait", Self::NAME);
        let embed = EmbedBuilder::new()
            .color(BRANDING_COLOR.into())
            .title(title);

        crate::respond!(ctx, {
            KIND = ChannelMessageWithSource,
            EMBEDS = [embed.clone().build()],
            FLAGS = [EPHEMERAL],
        })
        .await?;

        let response = ctx.client().response(ctx.token()).await?.model().await?;
        let delay = response.id.created_at() - ctx.created_at();
        let title = crate::localize!(ctx.locale() => "text.{}.pong", Self::NAME);
        let embed = embed.title(format!("{title} ({delay})"));

        ctx.client()
            .update_response(ctx.token())
            .embeds(Some(&[embed.build()]))?
            .await?;

        EventResult::Ok(())
    }
}
