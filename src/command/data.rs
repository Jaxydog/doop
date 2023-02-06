use crate::prelude::*;

const CONTENT: &str = include_str!("include/data.txt");

define_command!("data" {
    description: "View the bot's data privacy statement",
    permissions: USE_APPLICATION_COMMANDS,
    allow_dms: true,
});

pub async fn command(context: &Context, command: &CommandInteraction) -> Result<()> {
    command.defer_ephemeral(context).await?;

    let bot = context.http().get_current_user().await?;
    let author = CreateEmbedAuthor::new(bot.tag()).icon_url(bot.face());
    let color = bot.accent_colour.unwrap_or(BOT_COLOR);
    let embed = CreateEmbed::new()
        .author(author)
        .color(color)
        .description(CONTENT)
        .title("Data Usage and Privacy");
    let builder = CreateInteractionResponseFollowup::new()
        .embed(embed)
        .ephemeral(true);

    command.create_followup(context, builder).await?;
    Ok(())
}
