use crate::prelude::*;

define_command!("embed" {
    description: "Creates an embedded message",
    permissions: MANAGE_MESSAGES,
    allow_dms: false,
    options: [
        define_option!("author_icon" (String) {
            description: "The embed author's icon URL",
            required: false,
        }),
        define_option!("author_link" (String) {
            description: "The embed author's URL",
            required: false,
        }),
        define_option!("author_name" (String) {
            description: "The embed author's name",
            required: false,
            range(str): 1..=256,
        }),
        define_option!("color" (String) {
            description: "The embed author's color",
            required: false,
            choices(str): {
                "User": String::new(),
                "Bot": BOT_COLOR.hex(),
                "Red": Color::RED.hex(),
                "Orange": Color::ORANGE.hex(),
                "Yellow": Color::GOLD.hex(),
                "Green": Color::KERBAL.hex(),
                "Blue": Color::BLUE.hex(),
                "Purple": Color::PURPLE.hex(),
                "Pink": Color::FABLED_PINK.hex(),
                "Dark Red": Color::DARK_RED.hex(),
                "Dark Orange": Color::DARK_ORANGE.hex(),
                "Dark Yellow": Color::DARK_GOLD.hex(),
                "Dark Green": Color::DARK_GREEN.hex(),
                "Dark Blue": Color::DARK_BLUE.hex(),
                "Dark Purple": Color::DARK_PURPLE.hex(),
                "Dark Pink": Color::MEIBE_PINK.hex(),
                "White": Color::LIGHTER_GREY.hex(),
                "Gray": Color::LIGHT_GREY.hex(),
                "Dark Gray": Color::DARK_GREY.hex(),
                "Black": Color::DARKER_GREY.hex(),
            },
        }),
        define_option!("description" (String) {
            description: "The embed's description (supports \\n and Discord's markdown variant)",
            required: false,
            range(str): 1..=4096,
        }),
        define_option!("footer_icon" (String) {
            description: "The embed footers's icon URL",
            required: false,
        }),
        define_option!("footer_text" (String) {
            description: "The embed footers's text",
            required: false,
            range(str): 1..=2048,
        }),
        define_option!("image_link" (String) {
            description: "The embed's image URL",
            required: false,
        }),
        define_option!("thumbnail_link" (String) {
            description: "The embed's thumbnail URL",
            required: false,
        }),
        define_option!("title_link" (String) {
            description: "The embed title's URL",
            required: false,
        }),
        define_option!("title_text" (String) {
            description: "The embed title's text",
            required: false,
            range(str): 1..=256,
        }),
        define_option!("ephemeral" (Boolean) {
            description: "Whether the embed is ephemeral (only visible to you)",
            required: false,
        }),
    ],
});

pub async fn command(context: &Context, command: &CommandInteraction) -> Result<()> {
    let o = &command.data.options();
    let mut embed = CreateEmbed::new();
    let mut length = 0;
    let mut valid = false;

    if let Ok(name) = get_str(o, "author_name") {
        let mut author = CreateEmbedAuthor::new(name);

        if let Ok(icon) = get_str(o, "author_icon") {
            author = author.icon_url(icon);
        }
        if let Ok(link) = get_str(o, "author_link") {
            author = author.url(link);
        }

        embed = embed.author(author);
        length += name.chars().count();
        valid = true;
    }

    if let Ok(hex) = get_str(o, "color") {
        let color = if hex.is_empty() {
            let user = context.http().get_user(command.user.id).await?;

            user.accent_colour
        } else {
            u32::from_str_radix(hex, 16).ok().map(Color::new)
        };

        embed = embed.color(color.unwrap_or(Color::ROSEWATER));
    }

    if let Ok(description) = get_str(o, "description") {
        let description = description.replace(r"\n", "\n");

        embed = embed.description(description.trim());
        length += description.trim().chars().count();
        valid = true;
    }

    if let Ok(text) = get_str(o, "footer_text") {
        let mut footer = CreateEmbedFooter::new(text);

        if let Ok(icon) = get_str(o, "footer_icon") {
            footer = footer.icon_url(icon);
        }

        embed = embed.footer(footer);
        length += text.chars().count();
        valid = true;
    }

    if let Ok(image) = get_str(o, "image_link") {
        embed = embed.image(image);
        valid = true;
    }

    if let Ok(thumbnail) = get_str(o, "thumbnail_link") {
        embed = embed.thumbnail(thumbnail);
        valid = true;
    }

    if let Ok(title) = get_str(o, "title_text") {
        if let Ok(url) = get_str(o, "title_link") {
            embed = embed.url(url);
        }

        embed = embed.title(title);
        length += title.chars().count();
        valid = true;
    }

    if !valid {
        command.defer_ephemeral(context).await?;

        return err_wrap!("embeds must contain a visible element");
    }
    if length > 6000 {
        command.defer_ephemeral(context).await?;

        return err_wrap!("embed content must have at most 6000 total characters");
    }

    let ephemeral = get_bool(o, "ephemeral").unwrap_or_default();
    let builder = CreateInteractionResponseMessage::new()
        .embed(embed)
        .ephemeral(ephemeral);
    let builder = CreateInteractionResponse::Message(builder);

    command.create_response(context, builder).await?;
    Ok(())
}
