use anyhow::bail;
use doop_localizer::localize;
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType,
};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::bot::interaction::CommandCtx;
use crate::cmd::{CommandOptionResolver, OnCommand, OnComplete};
use crate::util::extension::{StrExtension, UserExtension};
use crate::util::traits::PreferLocale;
use crate::util::{Result, BRANDING, FAILURE};

const COLORS: &[(&str, &str)] = &[
    ("user", "user"),
    ("red", "#B4202A"),
    ("orange", "#FA6A0A"),
    ("yellow", "#FFD541"),
    ("green", "#59C135"),
    ("blue", "#249FDE"),
    ("purple", "#BC4A9B"),
    ("pink", "#F5A097"),
    ("dark_red", "#73172D"),
    ("dark_orange", "#71413B"),
    ("dark_yellow", "#F9A31B"),
    ("dark_green", "#1A7A3E"),
    ("dark_blue", "#285CC4"),
    ("dark_purple", "#793A80"),
    ("dark_pink", "#E86A73"),
    ("white", "#DAE0EA"),
    ("gray", "#8B93AF"),
    ("dark_gray", "#6D758D"),
    ("black", "#333941"),
];

crate::register_command! {
    ChatInput("embed") {
        let in_dms = true;
        let is_nsfw = false;
        let require = USE_SLASH_COMMANDS | MANAGE_MESSAGES;
        let options = [
            String("author_icon") {},
            String("author_link") {},
            String("author_name") {
                let maximum = 256;
            },
            String("color") {
                let autocomplete = true;
            },
            String("description") {
                let maximum = 4096;
            },
            String("footer_icon") {},
            String("footer_text") {
                let maximum = 2048;
            },
            String("image_link") {},
            String("thumbnail_link") {},
            String("title_link") {},
            String("title_text") {
                let maximum = 256;
            },
            Boolean("ephemeral") {},
        ];
        let handlers = {
            command = self::execute_command;
            complete = self::execute_complete;
        };
    }
}

#[allow(clippy::too_many_lines)]
async fn execute_command<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
) -> Result {
    let locale = ctx.event.author().preferred_locale();
    let resolver = CommandOptionResolver::new(ctx.data);

    let mut empty = true;
    let mut embed = EmbedBuilder::new().color(match resolver.get_str("color") {
        Ok(color) if color.chars().all(|c| c.is_ascii_digit()) => color.parse()?,
        Ok(color)
            if color.starts_with('#') && color.chars().skip(1).all(|c| c.is_ascii_hexdigit()) =>
        {
            u32::from_str_radix(color.trim_start_matches('#'), 16)?
        }
        Ok("user") if ctx.event.author().is_some() => {
            // Safety: this is always `Some`.
            #[allow(unsafe_code)]
            unsafe { ctx.event.author().unwrap_unchecked() }.color()
        }
        _ => BRANDING,
    });

    if let Ok(name) = resolver.get_str("author_name") {
        let mut author = EmbedAuthorBuilder::new(name);

        if let Ok(icon) = resolver.get_str("author_icon") {
            let Ok(icon) = ImageSource::url(icon) else {
                return ctx.failure(locale, "invalid_url", true).await;
            };

            author = author.icon_url(icon);
        }
        if let Ok(link) = resolver.get_str("author_link") {
            author = author.url(link);
        }

        embed = embed.author(author);
        empty = false;
    }

    if let Ok(description) = resolver.get_str("description") {
        embed = embed.description(description.collapse().trim());
        empty = false;
    }

    if let Ok(text) = resolver.get_str("footer_text") {
        let mut footer = EmbedFooterBuilder::new(text);

        if let Ok(icon) = resolver.get_str("footer_icon") {
            let Ok(icon) = ImageSource::url(icon) else {
                return ctx.failure(locale, "invalid_url", true).await;
            };

            footer = footer.icon_url(icon);
        }

        embed = embed.footer(footer);
        empty = false;
    }

    if let Ok(link) = resolver.get_str("image_link") {
        let Ok(link) = ImageSource::url(link) else {
            return ctx.failure(locale, "invalid_url", true).await;
        };

        embed = embed.image(link);
        empty = false;
    }

    if let Ok(link) = resolver.get_str("thumbnail_link") {
        let Ok(link) = ImageSource::url(link) else {
            return ctx.failure(locale, "invalid_url", true).await;
        };

        embed = embed.thumbnail(link);
        empty = false;
    }

    if let Ok(text) = resolver.get_str("title_text") {
        if let Ok(link) = resolver.get_str("title_link") {
            embed = embed.url(link);
        }

        embed = embed.title(text);
        empty = false;
    }

    if empty {
        let locale = ctx.event.author().preferred_locale();

        return ctx.failure(locale, format!("text.{}.empty", cmd.entry().name), false).await;
    }

    match embed.validate() {
        Ok(embed) => {
            let ephemeral = resolver.get_bool("ephemeral").copied().unwrap_or_default();

            if ephemeral {
                crate::respond!(as ctx => {
                    let kind = ChannelMessageWithSource;
                    let embeds = [embed.build()];
                    let flags = EPHEMERAL;
                })
                .await?;
            } else {
                crate::respond!(as ctx => {
                    let kind = ChannelMessageWithSource;
                    let embeds = [embed.build()];
                })
                .await?;
            }

            Ok(())
        }
        Err(error) => {
            let title = localize!(try in locale, "failure.{}.invalid.title", cmd.entry().name);
            let description = format!("> {error}");
            let embed = EmbedBuilder::new().color(FAILURE).description(description).title(title);

            crate::respond!(as ctx => {
                let kind = ChannelMessageWithSource;
                let embeds = [embed.build()];
                let flags = EPHEMERAL;
            })
            .await?;

            Ok(())
        }
    }
}

#[allow(clippy::unused_async)] // this must be async
async fn execute_complete<'api: 'evt, 'evt>(
    acp: &(dyn OnComplete + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
    (name, value, kind): (&'evt str, &'evt str, CommandOptionType),
) -> Result<Vec<CommandOptionChoice>> {
    let ("color", CommandOptionType::String) = (name, kind) else {
        bail!("invalid auto-complete target '{name}' ({kind:?})");
    };

    let locale = ctx.event.author().preferred_locale();
    let strip = value.trim_start_matches('#').to_lowercase();

    let options = COLORS
        .iter()
        .filter(|(name, color)| {
            strip.is_empty() || name.contains(&strip) || color.to_lowercase().contains(&strip)
        })
        .map(|(name, color)| CommandOptionChoice {
            name: localize!(try in locale, "text.{}.color.{name}", acp.entry().name).into_owned(),
            name_localizations: Some(localize!(in *, "text.{}.color.{name}", acp.entry().name)),
            value: CommandOptionChoiceValue::String((*color).to_string()),
        })
        .collect::<Vec<_>>();

    if !options.is_empty() {
        return Ok(options);
    }

    let options = value.strip_prefix('#').and_then(|s| {
        i64::from_str_radix(s, 16).ok().map(|color| CommandOptionChoice {
            name: format!("#{color:0<6X}"),
            name_localizations: None,
            value: CommandOptionChoiceValue::String(format!("#{color:0<6X}")),
        })
    });

    Ok(options.into_iter().collect())
}
