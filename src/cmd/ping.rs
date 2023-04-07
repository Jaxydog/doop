use serenity::all::CommandInteraction;
use serenity::builder::{
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
};
use serenity::prelude::CacheHttp;

use crate::command;
use crate::util::{Result, BOT_COLOR};

command!("ping": {
    description: "Calculates the bot's API response time",
    permissions: USE_APPLICATION_COMMANDS,
    dms_allowed: true,
});

/// Handles command interactions
pub async fn handle_commands(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
) -> Result<()> {
    let embed = CreateEmbed::new().color(BOT_COLOR).title("Calculating...");

    let response = CreateInteractionResponseMessage::new()
        .embed(embed.clone())
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(response);

    command.create_response(cache_http, response).await?;

    let message = command.get_response(cache_http.http()).await?;
    let ms = message.id.created_at().timestamp_millis();
    let ms = ms - command.id.created_at().timestamp_millis();
    let embed = embed.title(format!("Pong! (~{ms}ms)"));
    let response = EditInteractionResponse::new().embed(embed);

    command.edit_response(cache_http, response).await?;

    Ok(())
}
