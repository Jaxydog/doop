use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use super::CommandOptionResolver;
use crate::event::{CommandContext, EventHandler, EventResult};
use crate::extend::{StrExt, UserExt};
use crate::utility::BRANDING_COLOR;

crate::command! {
    TYPE = ChatInput,
    NAME = "embed",
    DMS = false,
    NSFW = false,
    REQUIRES = [MANAGE_MESSAGES],
    OPTIONS = [
        crate::option! {
            TYPE = String,
            NAME = "author_icon",
            REQUIRED = false,
        },
        crate::option! {
            TYPE = String,
            NAME = "author_link",
            REQUIRED = false,
        },
        crate::option! {
            TYPE = String,
            NAME = "author_name",
            REQUIRED = false,
            MAX = 256,
        },
        crate::option! {
            TYPE = Integer,
            NAME = "color",
            REQUIRED = false,
            CHOICES = [
                ("User", -1),
                ("Red", 0xB4_20_2A),
                ("Orange", 0xB4_20_2A),
                ("Yellow", 0xFF_D5_41),
                ("Green", 0x59_C1_35),
                ("Blue", 0x24_9F_DE),
                ("Purple", 0xBC_4A_9B),
                ("Pink", 0xF5_A0_97),
                ("Dark Red", 0x73_17_2D),
                ("Dark Orange", 0xDF_3E_23),
                ("Dark Yellow", 0xF9_A3_1B),
                ("Dark Green", 0x1A_7A_3E),
                ("Dark Blue", 0x28_5C_C4),
                ("Dark Purple", 0x79_3A_80),
                ("Dark Pink", 0xE8_6A_73),
                ("White", 0xDA_E0_EA),
                ("Gray", 0x8B_93_AF),
                ("Dark Gray", 0x6D_75_8D),
                ("Black", 0x33_39_41),
            ],
            MIN = -1,
            MAX = i64::from(u32::MAX),
        },
        crate::option! {
            TYPE = String,
            NAME = "description",
            REQUIRED = false,
            MAX = 4096,
        },
        crate::option! {
            TYPE = String,
            NAME = "footer_icon",
            REQUIRED = false,
        },
        crate::option! {
            TYPE = String,
            NAME = "footer_text",
            REQUIRED = false,
            MAX = 2048,
        },
        crate::option! {
            TYPE = String,
            NAME = "image_link",
            REQUIRED = false,
        },
        crate::option! {
            TYPE = String,
            NAME = "thumbnail_link",
            REQUIRED = false,
        },
        crate::option! {
            TYPE = String,
            NAME = "title_link",
            REQUIRED = false,
        },
        crate::option! {
            TYPE = String,
            NAME = "title_text",
            REQUIRED = false,
            MAX = 256,
        },
        crate::option! {
            TYPE = Boolean,
            NAME = "ephemeral",
            REQUIRED = false,
        },
    ],
}

#[async_trait::async_trait]
impl EventHandler for This {
    async fn command(&self, ctx: &CommandContext) -> EventResult {
        let resolver = CommandOptionResolver::new(ctx.data);
        let mut embed = EmbedBuilder::new();

        if let Ok(name) = resolver.get_str("author_name") {
            let mut author = EmbedAuthorBuilder::new(name);

            if let Ok(icon) = resolver.get_str("author_icon") {
                author = author.icon_url(ImageSource::url(icon)?);
            }
            if let Ok(link) = resolver.get_str("author_link") {
                author = author.url(link);
            }

            embed = embed.author(author);
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        if let Ok(color) = resolver.get_i64("color").copied() {
            let color = if color > 0 {
                color as u32
            } else if let Some(ref user) = ctx.event.user {
                user.color()
            } else {
                BRANDING_COLOR.into()
            };

            embed = embed.color(color);
        } else {
            embed = embed.color(BRANDING_COLOR.into());
        }

        if let Ok(description) = resolver.get_str("description") {
            embed = embed.description(description.flatten_escapes().trim());
        }

        if let Ok(text) = resolver.get_str("footer_text") {
            let mut footer = EmbedFooterBuilder::new(text);

            if let Ok(icon) = resolver.get_str("footer_icon") {
                footer = footer.icon_url(ImageSource::url(icon)?);
            }

            embed = embed.footer(footer);
        }

        if let Ok(link) = resolver.get_str("image_link") {
            embed = embed.image(ImageSource::url(link)?);
        }

        if let Ok(link) = resolver.get_str("thumbnail_link") {
            embed = embed.thumbnail(ImageSource::url(link)?);
        }

        if let Ok(text) = resolver.get_str("title_text") {
            if let Ok(link) = resolver.get_str("title_link") {
                embed = embed.url(link);
            }

            embed = embed.title(text);
        }

        if resolver.get_bool("ephemeral").copied().unwrap_or(false) {
            crate::respond!(ctx, {
                KIND = ChannelMessageWithSource,
                EMBEDS = [embed.build()],
                FLAGS = [EPHEMERAL],
            })
            .await
        } else {
            crate::respond!(ctx, {
                KIND = ChannelMessageWithSource,
                EMBEDS = [embed.validate()?.build()],
            })
            .await
        }?;

        EventResult::Ok(())
    }
}
