use doop_localizer::localize;
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::bot::interact::{CommandCtx, CommandOptionResolver, InteractionEventHandler};
use crate::util::ext::{StrExt, UserExt};
use crate::util::traits::Localized;
use crate::util::{Result, BRANDING, FAILURE};

crate::command! {
    let name = "embed";
    let kind = ChatInput;
    let permissions = MANAGE_MESSAGES;
    let allow_dms = false;
    let is_nsfw = false;
    let options = [
        {
            let name = "author_icon";
            let kind = String;
            let required = false;
        },
        {
            let name = "author_link";
            let kind = String;
            let required = false;
        },
        {
            let name = "author_name";
            let kind = String;
            let required = false;
            let max = 256;
        },
        {
            let name = "color";
            let kind = Integer;
            let required = false;
            let min = -1;
            let max = 0xFF_FF_FF;
            let choices = [
                ("user", -1),
                ("red", 0xB4_20_2A),
                ("orange", 0xB4_20_2A),
                ("yellow", 0xFF_D5_41),
                ("green", 0x59_C1_35),
                ("blue", 0x24_9F_DE),
                ("purple", 0xBC_4A_9B),
                ("pink", 0xF5_A0_97),
                ("dark_red", 0x73_17_2D),
                ("dark_orange", 0xDF_3E_23),
                ("dark_yellow", 0xF9_A3_1B),
                ("dark_green", 0x1A_7A_3E),
                ("dark_blue", 0x28_5C_C4),
                ("dark_purple", 0x79_3A_80),
                ("dark_pink", 0xE8_6A_73),
                ("white", 0xDA_E0_EA),
                ("gray", 0x8B_93_AF),
                ("dark_gray", 0x6D_75_8D),
                ("black", 0x33_39_41),
            ];
        },
        {
            let name = "description";
            let kind = String;
            let required = false;
            let max = 4096;
        },
        {
            let name = "footer_icon";
            let kind = String;
            let required = false;
        },
        {
            let name = "footer_text";
            let kind = String;
            let required = false;
            let max = 2048;
        },
        {
            let name = "image_link";
            let kind = String;
            let required = false;
        },
        {
            let name = "thumbnail_link";
            let kind = String;
            let required = false;
        },
        {
            let name = "title_link";
            let kind = String;
            let required = false;
        },
        {
            let name = "title_text";
            let kind = String;
            let required = false;
            let max = 256;
        },
        {
            let name = "ephemeral";
            let kind = Boolean;
            let required = false;
        },
    ];
}

#[async_trait::async_trait]
impl InteractionEventHandler for Impl {
    async fn handle_command(&self, ctx: CommandCtx<'_>) -> Result {
        let resolver = CommandOptionResolver::new(ctx.data);
        let mut embed = EmbedBuilder::new();
        let mut empty = true;

        if resolver.get_bool("ephemeral").copied().unwrap_or(false) {
            crate::respond!(as ctx => {
                let kind = DeferredChannelMessageWithSource;
                let flags = EPHEMERAL;
            })
            .await
        } else {
            crate::respond!(as ctx => {
                let kind = DeferredChannelMessageWithSource;
            })
            .await
        }?;

        if let Ok(name) = resolver.get_str("author_name") {
            let mut author = EmbedAuthorBuilder::new(name);

            if let Ok(icon) = resolver.get_str("author_icon") {
                author = author.icon_url(ImageSource::url(icon)?);
            }
            if let Ok(link) = resolver.get_str("author_link") {
                author = author.url(link);
            }

            embed = embed.author(author);
            empty = false;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        if let Ok(color) = resolver.get_i64("color").copied() {
            let color = if color > 0 {
                color as u32
            } else if let Some(ref user) = ctx.event.user {
                user.color()
            } else {
                BRANDING
            };

            embed = embed.color(color);
        } else {
            embed = embed.color(BRANDING);
        }

        if let Ok(description) = resolver.get_str("description") {
            embed = embed.description(description.collapse().trim());
            empty = false;
        }

        if let Ok(text) = resolver.get_str("footer_text") {
            let mut footer = EmbedFooterBuilder::new(text);

            if let Ok(icon) = resolver.get_str("footer_icon") {
                footer = footer.icon_url(ImageSource::url(icon)?);
            }

            embed = embed.footer(footer);
            empty = false;
        }

        if let Ok(link) = resolver.get_str("image_link") {
            embed = embed.image(ImageSource::url(link)?);
            empty = false;
        }

        if let Ok(link) = resolver.get_str("thumbnail_link") {
            embed = embed.thumbnail(ImageSource::url(link)?);
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
            let locale = ctx.event.author().locale();
            let title = localize!(try locale => "text.{}.empty", Self::NAME);
            let embed = EmbedBuilder::new().color(FAILURE).title(title);

            crate::followup!(as ctx => {
                let embeds = &[embed.build()];
                let flags = EPHEMERAL;
            })
            .await?;

            return Ok(());
        }

        let embed = match embed.validate() {
            Ok(embed) => embed,
            Err(error) => {
                let locale = ctx.event.author().locale();
                let title = localize!(try locale => "text.{}.invalid", Self::NAME);
                let reason = format!("> {error}");
                let embed = EmbedBuilder::new().color(FAILURE).title(title).description(reason);

                crate::followup!(as ctx => {
                    let embeds = &[embed.build()];
                    let flags = EPHEMERAL;
                })
                .await?;

                return Ok(());
            }
        };

        crate::followup!(as ctx => {
            let embeds = &[embed.validate()?.build()];
        })
        .await?;

        Ok(())
    }
}
