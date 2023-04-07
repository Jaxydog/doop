use serenity::all::CommandInteraction;
use serenity::builder::{
    CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::model::Color;
use serenity::prelude::CacheHttp;

use super::CommandDataResolver;
use crate::common::parse_escapes;
use crate::util::{Result, BOT_COLOR};
use crate::{command, err_wrap, option};

command!("embed": {
    description: "Creates an embedded message",
    permissions: MANAGE_MESSAGES,
    dms_allowed: false,
    options: [
        option!("author_icon" <String>: {
            description: "The embed author's icon URL",
            required: false,
        }),
        option!("author_link" <String>: {
            description: "The embed author's URL",
            required: false,
        }),
        option!("author_name" <String>: {
            description: "The embed author's name",
            required: false,
            where <str>: 1..=256,
        }),
        option!("color" <Integer>: {
            description: "The embed's color",
            required: false,
            match <str> {
                "User" => String::new(),
                "Bot" => BOT_COLOR.hex(),
                "Red" => Color::RED.hex(),
                "Orange" => Color::ORANGE.hex(),
                "Yellow" => Color::GOLD.hex(),
                "Green" => Color::KERBAL.hex(),
                "Blue" => Color::BLUE.hex(),
                "Purple" => Color::PURPLE.hex(),
                "Pink" => Color::FABLED_PINK.hex(),
                "Dark Red" => Color::DARK_RED.hex(),
                "Dark Orange" => Color::DARK_ORANGE.hex(),
                "Dark Yellow" => Color::DARK_GOLD.hex(),
                "Dark Green" => Color::DARK_GREEN.hex(),
                "Dark Blue" => Color::DARK_BLUE.hex(),
                "Dark Purple" => Color::DARK_PURPLE.hex(),
                "Dark Pink" => Color::MEIBE_PINK.hex(),
                "White" => Color::LIGHTER_GREY.hex(),
                "Gray" => Color::LIGHT_GREY.hex(),
                "Dark Gray" => Color::DARK_GREY.hex(),
                "Black" => Color::DARKER_GREY.hex(),
            },
        }),
        option!("description" <String>: {
            description: "The embed's description",
            required: false,
            where <str>: 1..=4096,
        }),
        option!("footer_icon" <String>: {
            description: "The embed footers's icon URL",
            required: false,
        }),
        option!("footer_text" <String>: {
            description: "The embed footers's text",
            required: false,
            where <str>: 1..=2048,
        }),
        option!("image_link" <String>: {
            description: "The embed's image URL",
            required: false,
        }),
        option!("thumbnail_link" <String>: {
            description: "The embed's thumbnail URL",
            required: false,
        }),
        option!("title_link" <String>: {
            description: "The embed title's URL",
            required: false,
        }),
        option!("title_text" <String>: {
            description: "The embed title's text",
            required: false,
            where <str>: 1..=256,
        }),
        option!("ephemeral" <Boolean>: {
            description: "Whether the embed is ephemeral (only visible to you)",
            required: false,
        }),
    ],
});

/// Handles command interactions
pub async fn handle_commands(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
) -> Result<()> {
    let data = CommandDataResolver::new(command);
    let mut embed = CreateEmbed::new();
    let mut length = 0;
    let mut valid = false;

    if let Ok(name) = data.get_str("author_name") {
        let mut author = CreateEmbedAuthor::new(name);

        if let Ok(icon_url) = data.get_str("author_icon") {
            author = author.icon_url(icon_url);
        }
        if let Ok(url) = data.get_str("author_link") {
            author = author.url(url);
        }

        embed = embed.author(author);
        length += name.chars().count();
        valid = true;
    }

    if let Ok(hex) = data.get_str("color") {
        let color = if hex.is_empty() {
            let user = cache_http.http().get_user(command.user.id).await?;

            user.accent_colour
        } else {
            u32::from_str_radix(hex, 16).ok().map(Color::new)
        }
        .unwrap_or(BOT_COLOR);

        embed = embed.color(color);
    }

    if let Ok(description) = data.get_str("description") {
        let description = &parse_escapes(description);

        embed = embed.description(description);
        length += description.chars().count();
        valid = true;
    }

    if let Ok(text) = data.get_str("footer_text") {
        let mut footer = CreateEmbedFooter::new(text);

        if let Ok(icon_url) = data.get_str("footer_icon") {
            footer = footer.icon_url(icon_url);
        }

        embed = embed.footer(footer);
        length += text.chars().count();
        valid = true;
    }

    if let Ok(url) = data.get_str("image_link") {
        embed = embed.image(url);
        valid = true;
    }

    if let Ok(url) = data.get_str("thumbnail_link") {
        embed = embed.thumbnail(url);
        valid = true;
    }

    if let Ok(title) = data.get_str("title") {
        if let Ok(url) = data.get_str("title_link") {
            embed = embed.url(url);
        }

        embed = embed.title(title);
        length += title.chars().count();
        valid = true;
    }

    if !valid {
        command.defer_ephemeral(cache_http).await?;

        return err_wrap!("embeds must contain at least one visible element");
    }
    if length > 6000 {
        command.defer_ephemeral(cache_http).await?;

        return err_wrap!("embed content must have at most 6000 total characters");
    }

    let ephemeral = data.get_bool("ephemeral").unwrap_or_default();
    let builder = CreateInteractionResponseMessage::new().embed(embed);
    let builder = CreateInteractionResponse::Message(builder.ephemeral(ephemeral));

    command.create_response(cache_http, builder).await?;

    Ok(())
}
