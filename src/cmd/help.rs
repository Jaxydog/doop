use serenity::all::{Command, CommandInteraction};
use serenity::builder::{CreateEmbed, CreateEmbedAuthor, CreateInteractionResponseFollowup};
use serenity::prelude::CacheHttp;

use crate::command;
use crate::util::{Result, BOT_COLOR};

const HEADER: &str = include_str!("include/help/header.txt");
const FOOTER: &str = include_str!("include/help/footer.txt");

command!("help": {
    description: "Displays a list of the bot's commands",
    permissions: USE_APPLICATION_COMMANDS,
    dms_allowed: true,
});

/// Executes the command
pub async fn execute(cache_http: &impl CacheHttp, command: &CommandInteraction) -> Result<()> {
    command.defer_ephemeral(cache_http).await?;

    let mut description = HEADER.to_string();

    if let Some(guild_id) = command.guild_id {
        let commands = guild_id.get_application_commands(cache_http.http()).await?;

        if !commands.is_empty() {
            description += "\n\n**__Guild commands__**\n";
            description += &stringify_commands(&commands);
        }
    }

    let commands = cache_http.http().get_global_application_commands().await?;

    description.push_str("\n\n**__Global commands__**\n");

    if commands.is_empty() {
        description += "> *...looks like there's nothing here?*";
    } else {
        description += &stringify_commands(&commands);
    }

    description += &format!("\n\n{FOOTER}");

    let bot = cache_http.http().get_current_user().await?;
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new(bot.tag()).icon_url(bot.face()))
        .color(bot.accent_colour.unwrap_or(BOT_COLOR))
        .description(description)
        .title("Thank you for using Doop!");
    let builder = CreateInteractionResponseFollowup::new()
        .embed(embed)
        .ephemeral(true);

    command.create_followup(cache_http, builder).await?;

    Ok(())
}

fn stringify_commands(commands: &[Command]) -> String {
    commands
        .iter()
        .map(|c| {
            let (name, id, desc) = (&c.name, c.id, &c.description);
            let mut flags = vec![];

            let entry = if c.options.iter().any(|o| !o.options.is_empty()) {
                flags.push("Has sub-commands");

                format!("/{name} - {desc}")
            } else {
                format!("</{name}:{id}> - {desc}")
            };

            if c.dm_permission.unwrap_or(true) {
                flags.push("Usable in DMs");
            }

            if flags.is_empty() {
                entry
            } else {
                format!("{entry}\n> *{}*", flags.join(", "))
            }
        })
        .fold(String::new(), |s, e| format!("{s}\n\n{e}"))
        .trim_start()
        .to_string()
}
