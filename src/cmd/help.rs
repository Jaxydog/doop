use serenity::all::{Command, CommandInteraction};
use serenity::builder::{CreateEmbed, CreateEmbedAuthor, CreateInteractionResponseFollowup};
use serenity::prelude::CacheHttp;

use crate::command;
use crate::util::{Result, BOT_BRAND_COLOR};

command!("help": {
    description: "Displays a list of the bot's commands",
    permissions: USE_APPLICATION_COMMANDS,
    dms_allowed: true,
});

/// Handles command interactions
pub async fn handle_commands(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
) -> Result<()> {
    command.defer_ephemeral(cache_http).await?;

    // Simply including these as a string in the binary is a lot easier and faster
    // than reading it every time the command is run.
    let mut description = include_str!("include/help/header.txt").to_string();

    // No reason to print these if we aren't in a guild or if there aren't any
    // commands.
    if let Some(guild_id) = command.guild_id {
        let commands = guild_id.get_commands(cache_http.http()).await?;

        if !commands.is_empty() {
            description += "\n\n**__Guild commands__**\n";
            description += &stringify_commands(&commands);
        }
    }

    let commands = cache_http.http().get_global_commands().await?;

    description += "\n\n**__Global commands__**\n";

    // This returning true would be a really weird case considering this is a
    // command by itself so there should always be one but i'll check anyways
    // because I don't want any weird cases to make the bot look bad.
    if commands.is_empty() {
        description += "> *...looks like there's nothing here?*";
    } else {
        description += &stringify_commands(&commands);
    }

    // I can't believe it took me until maybe a month ago to learn that you can just
    // add assign onto a String like this... I don't wanna talk about it.
    //
    // Anyways what I was going to say is that I don't know how I feel about
    // allocating just to append but I doubt it'll be an issue. If I'm having
    // performance issues because i'm allocating this specific string that's just
    // impressive.
    description += &format!("\n\n{}", include_str!("include/help/footer.txt"));

    let bot = cache_http.http().get_current_user().await?;
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new(bot.tag()).icon_url(bot.face()))
        .color(bot.accent_colour.unwrap_or(BOT_BRAND_COLOR))
        .description(description)
        .title("Thank you for using Doop!");
    let builder = CreateInteractionResponseFollowup::new()
        .embed(embed)
        .ephemeral(true);

    command.create_followup(cache_http, builder).await?;

    Ok(())
}

fn stringify_commands(commands: &[Command]) -> String {
    // I love iterators but this body indentation is just atrocious.
    commands
        .iter()
        .map(|c| {
            let (name, id, desc) = (&c.name, c.id, &c.description);
            let mut flags = vec![];

            // In the cases where the command contains sub-commands, the Discord 
            // markdown for embedded commands doesn't work the same. So, to save me
            // a headache and some clutter, we just print it without the surrounding 
            // brackets as plain text.
            let entry = if c.options.iter().any(|o| !o.options.is_empty()) {
                flags.push("Has sub-commands");

                format!("`/{name}` - {desc}")
            } else {
                format!("</{name}:{id}> - {desc}")
            };

            if c.dm_permission.unwrap_or(false) {
                flags.push("Usable in DMs");
            }

            if flags.is_empty() {
                entry
            } else {
                format!("{entry}\n> *{}*", flags.join(", "))
            }
        })
        // This format allocation within the fold slightly upsets me  but I highly 
        // doubt it will cause any issues. It probably  just gets optimized away.
        .fold(String::new(), |s, e| s + &format!("\n\n{e}"))
        .trim_start()
        .to_string()
}
