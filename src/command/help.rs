use crate::prelude::*;

const HEADER: &str = include_str!("include/help/header.txt");
const FOOTER: &str = include_str!("include/help/footer.txt");

define_command!("help" {
    description: "Displays a list of bot commands",
    permissions: USE_APPLICATION_COMMANDS,
    allow_dms: true,
});

pub async fn command(context: &Context, command: &CommandInteraction) -> Result<()> {
    command.defer_ephemeral(context).await?;

    let mut description = HEADER.to_string();

    if let Some(guild_id) = command.guild_id {
        let commands = guild_id.get_application_commands(context).await?;

        if !commands.is_empty() {
            description.push_str("\n\n**__Guild commands__**\n");
            description.push_str(&stringify(commands));
        }
    }

    let commands = context.http().get_global_application_commands().await?;

    description.push_str("\n\n**__Global commands__**\n");

    if commands.is_empty() {
        description.push_str("> *...looks like there's nothing here!*");
    } else {
        description.push_str(&stringify(commands));
    }

    description.push_str(&format!("\n\n{FOOTER}"));

    let bot = context.http().get_current_user().await?;
    let author = CreateEmbedAuthor::new(bot.tag()).icon_url(bot.face());
    let color = bot.accent_colour.unwrap_or(BOT_COLOR);
    let embed = CreateEmbed::new()
        .author(author)
        .color(color)
        .description(description)
        .title("Thank you for using Doop!");
    let builder = CreateInteractionResponseFollowup::new()
        .embed(embed)
        .ephemeral(true);

    command.create_followup(context, builder).await?;
    Ok(())
}

fn stringify(commands: Vec<Command>) -> String {
    commands
        .into_iter()
        .map(|command| {
            let (name, id, desc) = (&command.name, command.id, &command.description);
            let entry;
            let mut flags = vec![];

            if command.options.iter().any(|o| !o.options.is_empty()) {
                entry = format!("`/{name}` - {desc}");
                flags.push("*Has sub-commands*");
            } else {
                entry = format!("</{name}:{id}> - {desc}");
            }
            if command.dm_permission.unwrap_or(true) {
                flags.push("*Usable in DMs*");
            }

            if flags.is_empty() {
                entry
            } else {
                format!("{entry}\n> {}", flags.join(", "))
            }
        })
        .fold(String::new(), |s, e| format!("{s}\n\n{e}"))
        .trim_start()
        .to_string()
}
