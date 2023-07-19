use anyhow::anyhow;
use twilight_model::channel::message::ReactionType;
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

pub use self::model::*;
use super::CommandOptionResolver;
use crate::event::{CachedHttp, CommandContext, ComponentContext, EventHandler, EventResult};
use crate::extend::ReactionTypeExt;
use crate::storage::Storable;
use crate::traits::{button_rows, BuildButtons};
use crate::utility::{BRANDING_COLOR, FAILURE_COLOR, SUCCESS_COLOR};

/// Defines data structures used in the command implementation.
mod model;

crate::command! {
    TYPE = ChatInput,
    NAME = "role",
    DMS = false,
    NSFW = false,
    REQUIRES = [MANAGE_ROLES],
    OPTIONS = [
        crate::option! {
            TYPE = SubCommand,
            NAME = "add",
            OPTIONS = [
                crate::option! {
                    TYPE = Role,
                    NAME = "role",
                    REQUIRED = true,
                },
                crate::option! {
                    TYPE = String,
                    NAME = "icon",
                    REQUIRED = true,
                },
            ],
        },
        crate::option! {
            TYPE = SubCommand,
            NAME = "remove",
            OPTIONS = [
                crate::option! {
                    TYPE = Role,
                    NAME = "role",
                    REQUIRED = true,
                },
            ],
        },
        crate::option! {
            TYPE = SubCommand,
            NAME = "list",
        },
        crate::option! {
            TYPE = SubCommand,
            NAME = "send",
            OPTIONS = [
                crate::option! {
                    TYPE = String,
                    NAME = "text",
                    MAX = 256,
                }
            ],
        },
    ],
}

#[async_trait::async_trait]
impl EventHandler for This {
    async fn command(&self, ctx: &CommandContext) -> EventResult {
        crate::respond!(ctx, {
            KIND = DeferredChannelMessageWithSource,
            FLAGS = [EPHEMERAL],
        })
        .await?;

        let cor = CommandOptionResolver::new(ctx.data);

        if let Ok(cor) = cor.get_subcommand("add") {
            return add(ctx, cor).await;
        }
        if let Ok(cor) = cor.get_subcommand("remove") {
            return remove(ctx, cor).await;
        }
        if let Ok(cor) = cor.get_subcommand("list") {
            return list(ctx, cor).await;
        }
        if let Ok(cor) = cor.get_subcommand("send") {
            return send(ctx, cor).await;
        }

        EventResult::Err(anyhow!("unknown or missing subcommand"))
    }

    async fn component(&self, ctx: &ComponentContext) -> EventResult {
        crate::respond!(ctx, {
            KIND = DeferredChannelMessageWithSource,
            FLAGS = [EPHEMERAL],
        })
        .await?;

        let Some(guild_id) = ctx.event.guild_id else {
            return EventResult::Err(anyhow!("command must be used in a guild"));
        };
        let Some(member) = ctx.event.member.as_ref() else {
            return EventResult::Err(anyhow!("command must be used by a member"));
        };
        let Some(user_id) = member.user.as_ref().map(|u| u.id) else {
            return EventResult::Err(anyhow!("command must be used by a user"));
        };
        let Some(role_id) = ctx.data.1.data().first() else {
            return EventResult::Err(anyhow!("missing role identifier"));
        };
        let Some(role_id) = Id::new_checked(role_id.parse()?) else {
            return EventResult::Err(anyhow!("expected a non-zero role identifier"));
        };

        let member = ctx.http().guild_member(guild_id, user_id).await?;
        let mut roles = member.model().await?.roles;

        let title = if roles.iter().any(|id| id == &role_id) {
            roles.retain(|id| id != &role_id);

            crate::localize!(ctx.locale() => "text.{}.toggle_off", Self::NAME)
        } else {
            roles.push(role_id);

            crate::localize!(ctx.locale() => "text.{}.toggle_on", Self::NAME)
        };

        ctx.http()
            .update_guild_member(guild_id, user_id)
            .roles(&roles)
            .await?;

        let embed = EmbedBuilder::new().color(SUCCESS_COLOR.into()).title(title);

        crate::followup!(ctx, {
            EMBEDS = [embed.build()],
            FLAGS = [EPHEMERAL],
        })
        .await?;

        EventResult::Ok(())
    }
}

async fn add<'cmd>(ctx: &CommandContext<'_>, cor: CommandOptionResolver<'cmd>) -> EventResult {
    let Some(guild_id) = ctx.data.guild_id else {
        return EventResult::Err(anyhow!("command must be used in a guild"));
    };
    let Some(member) = ctx.event.member.as_ref() else {
        return EventResult::Err(anyhow!("command must be used by a member"));
    };
    let Some(user_id) = member.user.as_ref().map(|u| u.id) else {
        return EventResult::Err(anyhow!("command must be used by a user"));
    };

    let role_id = *cor.get_role_id("role")?;
    let icon: Box<str> = cor.get_str("icon")?.trim().into();

    ReactionType::parse(icon.to_string())?;

    let name = if let Some(role) = ctx.cache().role(role_id) {
        role.name.clone()
    } else {
        let roles = ctx.http().roles(guild_id).await?.model().await?;
        let Some(role) = roles.into_iter().find(|r| r.id == role_id) else {
            return EventResult::Err(anyhow!("invalid role identifier '{role_id}'"));
        };

        role.name
    }
    .into_boxed_str();

    let mut selectors = Selectors::saved((guild_id, user_id))
        .read_or_default()
        .await;

    if selectors.get().len() >= 25 {
        let title = crate::localize!(ctx.locale() => "text.{}.max", This::NAME);
        let embed = EmbedBuilder::new().color(FAILURE_COLOR.into()).title(title);

        crate::followup!(ctx, {
            EMBEDS = [embed.build()],
            FLAGS = [EPHEMERAL],
        })
        .await?;
    } else {
        selectors.get_mut().insert(role_id, (name, icon));
        selectors.write().await?;

        let title = crate::localize!(ctx.locale() => "text.{}.added", This::NAME);
        let embed = EmbedBuilder::new().color(SUCCESS_COLOR.into()).title(title);

        crate::followup!(ctx, {
            EMBEDS = [embed.build()],
            FLAGS = [EPHEMERAL],
        })
        .await?;
    }

    EventResult::Ok(())
}

async fn remove<'cmd>(ctx: &CommandContext<'_>, cor: CommandOptionResolver<'cmd>) -> EventResult {
    let Some(guild_id) = ctx.data.guild_id else {
        return EventResult::Err(anyhow!("command must be used in a guild"));
    };
    let Some(member) = ctx.event.member.as_ref() else {
        return EventResult::Err(anyhow!("command must be used by a member"));
    };
    let Some(user_id) = member.user.as_ref().map(|u| u.id) else {
        return EventResult::Err(anyhow!("command must be used by a user"));
    };

    let role_id = *cor.get_role_id("role")?;
    let mut selectors = Selectors::saved((guild_id, user_id))
        .read_or_default()
        .await;

    let (title, color) = if selectors.get_mut().remove_entry(&role_id).is_some() {
        let title = crate::localize!(ctx.locale() => "text.{}.removed", This::NAME);

        (title, SUCCESS_COLOR.into())
    } else {
        let title = crate::localize!(ctx.locale() => "text.{}.missing", This::NAME);

        (title, FAILURE_COLOR.into())
    };

    if selectors.get().is_empty() {
        selectors.remove().await?;
    } else {
        selectors.write().await?;
    }

    let embed = EmbedBuilder::new().color(color).title(title);

    crate::followup!(ctx, {
        EMBEDS = [embed.build()],
        FLAGS = [EPHEMERAL],
    })
    .await?;

    EventResult::Ok(())
}

async fn list<'cmd>(ctx: &CommandContext<'_>, _: CommandOptionResolver<'cmd>) -> EventResult {
    let Some(guild_id) = ctx.data.guild_id else {
        return EventResult::Err(anyhow!("command must be used in a guild"));
    };
    let Some(member) = ctx.event.member.as_ref() else {
        return EventResult::Err(anyhow!("command must be used by a member"));
    };
    let Some(user_id) = member.user.as_ref().map(|u| u.id) else {
        return EventResult::Err(anyhow!("command must be used by a user"));
    };
    let Ok(selectors) = Selectors::saved((guild_id, user_id)).read().await else {
        let title = crate::localize!(ctx.locale() => "text.{}.missing", This::NAME);
        let embed = EmbedBuilder::new().color(FAILURE_COLOR.into()).title(title);

        crate::followup!(ctx, {
            EMBEDS = [embed.build()],
            FLAGS = [EPHEMERAL],
        })
        .await?;

        return EventResult::Ok(());
    };

    let components = button_rows(selectors.get().build_buttons(true, ())?);
    let title = crate::localize!(ctx.locale() => "text.{}.list", This::NAME);
    let embed = EmbedBuilder::new()
        .color(BRANDING_COLOR.into())
        .title(title);

    crate::followup!(ctx, {
        COMPONENTS = &components,
        EMBEDS = [embed.build()],
        FLAGS = [EPHEMERAL],
    })
    .await?;

    EventResult::Ok(())
}

async fn send<'cmd>(ctx: &CommandContext<'_>, cor: CommandOptionResolver<'cmd>) -> EventResult {
    let Some(guild_id) = ctx.data.guild_id else {
        return EventResult::Err(anyhow!("command must be used in a guild"));
    };
    let Some(channel_id) = ctx.event.channel.as_ref().map(|c| c.id) else {
        return EventResult::Err(anyhow!("command must be used in a channel"));
    };
    let Some(member) = ctx.event.member.as_ref() else {
        return EventResult::Err(anyhow!("command must be used by a member"));
    };
    let Some(user_id) = member.user.as_ref().map(|u| u.id) else {
        return EventResult::Err(anyhow!("command must be used by a user"));
    };

    let selectors = Selectors::saved((guild_id, user_id))
        .read_or_default()
        .await;
    let components = button_rows(selectors.get().build_buttons(false, ())?);

    if components.is_empty() {
        let title = crate::localize!(ctx.locale() => "text.{}.empty", This::NAME);
        let embed = EmbedBuilder::new().color(FAILURE_COLOR.into()).title(title);

        crate::followup!(ctx, {
            EMBEDS = [embed.build()],
            FLAGS = [EPHEMERAL],
        })
        .await?;

        return EventResult::Ok(());
    }

    let text = cor.get_str("text")?;
    let embed = EmbedBuilder::new().color(BRANDING_COLOR.into()).title(text);

    ctx.http()
        .create_message(channel_id)
        .embeds(&[embed.build()])?
        .components(&components)?
        .await?;

    selectors.remove().await?;

    let title = crate::localize!(ctx.locale() => "text.{}.sent", This::NAME);
    let embed = EmbedBuilder::new().color(SUCCESS_COLOR.into()).title(title);

    crate::followup!(ctx, {
        EMBEDS = [embed.build()],
        FLAGS = [EPHEMERAL],
    })
    .await?;

    EventResult::Ok(())
}
