use crate::prelude::*;

define_command!("ping" {
    description: "Calculates the bot's API response time",
    permissions: USE_APPLICATION_COMMANDS,
    allow_dms: true,
});

pub async fn command(context: &Context, command: &CommandInteraction) -> Result<()> {
    command.defer_ephemeral(context).await?;

    let bot = context.http().get_current_user().await?;
    let author = CreateEmbedAuthor::new(bot.tag()).icon_url(bot.face());
    let color = bot.accent_colour.unwrap_or(BOT_COLOR);
    let mut embed = CreateEmbed::new()
        .author(author)
        .color(color)
        .title("Calculating...");

    let builder = CreateInteractionResponseFollowup::new()
        .embed(embed.clone())
        .ephemeral(true);
    let message = command.create_followup(context, builder).await?;

    let sent = message.id.created_at().timestamp_millis();
    let received = command.id.created_at().timestamp_millis();
    let delay = sent - received;

    embed = embed.title(format!("doop! ({delay}ms)"));

    let builder = CreateInteractionResponseFollowup::new().embed(embed);

    command.edit_followup(context, message.id, builder).await?;

    Ok(())
}
