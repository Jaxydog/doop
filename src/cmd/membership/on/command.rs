use std::num::NonZeroU64;

use anyhow::bail;
use doop_localizer::localize;
use doop_storage::{Stored, Value};
use time::OffsetDateTime;
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::interaction::CommandCtx;
use crate::cmd::membership::configuration::Config;
use crate::cmd::membership::submission::{StatusKind, StatusUpdate, Submission};
use crate::cmd::{CommandEntry, CommandOptionResolver};
use crate::util::extension::EmbedAuthorExtension;
use crate::util::traits::PreferLocale;
use crate::util::{Anchor, Result, FAILURE, SUCCESS};

pub async fn configure<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: CommandCtx<'api, 'evt>,
    resolver: CommandOptionResolver<'evt>,
) -> Result {
    let Some(guild_id) = ctx.event.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(ref channel) = ctx.event.channel else {
        bail!("command must be used within a channel");
    };

    let locale = ctx.event.preferred_locale();
    let key = Config::stored((entry.name, guild_id));
    let previous = key.read().ok().map(Value::get_owned);
    let mut config = Config::new(guild_id, &resolver, previous.as_ref())?;
    let (embed, components) = config.build_entrypoint(entry, ctx.api).await?;

    if let Some(mut anchor) = key.read().ok().and_then(|v| v.get_owned().anchor) {
        if anchor.fetch(ctx.api).await.is_ok() {
            anchor.update(ctx.api).embeds(Some(&[embed]))?.components(Some(&components))?.await?;
        } else {
            let message = ctx.api.http.create_message(channel.id);
            let message = message.embeds(&[embed])?.components(&components)?.await?.model().await?;

            anchor = Anchor::from(message);
        }

        config.anchor = Some(anchor);
    } else {
        let message = ctx.api.http.create_message(channel.id);
        let message = message.embeds(&[embed])?.components(&components)?.await?.model().await?;

        config.anchor = Some(Anchor::from(message));
    }

    key.write(&config)?;

    ctx.success(locale, format!("{}.configured", entry.name), false).await
}

pub async fn update<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: CommandCtx<'api, 'evt>,
    resolver: CommandOptionResolver<'evt>,
) -> Result {
    let Some(guild_id) = ctx.event.guild_id else {
        bail!("command must be used within a guild");
    };

    let locale = ctx.event.preferred_locale();
    let status = StatusKind::try_from(*resolver.get_i64("status")?)?;
    let Ok(user_id) = resolver.get_str("user")?.parse::<NonZeroU64>().map(Id::from) else {
        return ctx.failure(locale, format!("{}.invalid_user", entry.name), false).await;
    };

    let submission = Submission::stored((entry.name, guild_id, user_id));

    if let Ok(submission) = submission.read() {
        if submission.get().status.kind != StatusKind::Pending {
            return ctx.failure(locale, format!("{}.not_pending", entry.name), false).await;
        }

        ctx.modal(submission.get().build_update_modal(entry, ctx.api, status).await?).await
    } else {
        ctx.failure(locale, format!("{}.no_entry", entry.name), false).await
    }
}

pub async fn discard<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: CommandCtx<'api, 'evt>,
    resolver: CommandOptionResolver<'evt>,
) -> Result {
    let Some(guild_id) = ctx.event.guild_id else {
        bail!("command must be used within a guild");
    };

    let locale = ctx.event.preferred_locale();
    let Ok(user_id) = resolver.get_str("user")?.parse::<NonZeroU64>().map(Id::from) else {
        return ctx.failure(locale, format!("{}.invalid_user", entry.name), false).await;
    };

    let config = Config::stored((entry.name, guild_id));
    let Ok(config) = config.read() else {
        return ctx.failure(locale, format!("{}.no_config", entry.name), false).await;
    };

    let submission = Submission::stored((entry.name, guild_id, user_id));
    let Ok(mut submission) = submission.read() else {
        return ctx.failure(locale, format!("{}.no_entry", entry.name), false).await;
    };
    if submission.get().status.kind == StatusKind::Discarded {
        return ctx.failure(locale, format!("{}.discarded", entry.name), false).await;
    }

    submission.get_mut().update_status(
        StatusKind::Discarded,
        StatusUpdate {
            author_id: user_id,
            timestamp: OffsetDateTime::now_utc(),
            reason: None,
            comment: None,
            previous: None,
        },
    );

    let (embed, components) = submission.get().build_form(entry, ctx.api).await?;

    if let Some(anchor) = submission.get().anchor {
        let message = anchor.update(ctx.api).components(Some(&components))?;

        message.embeds(Some(&[embed]))?.await?.model().await?;
    } else {
        let message = ctx.api.http.create_message(config.get().submission.output_channel_id);
        let message = message.components(&components)?.embeds(&[embed])?.await?.model().await?;

        submission.get_mut().anchor = Some(Anchor::from(message));
    }

    submission.write()?;

    let key = submission.get().status.kind.localization_key();
    ctx.success(locale, format!("{}.updated_{key}", entry.name), false).await
}

pub async fn view<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: CommandCtx<'api, 'evt>,
    resolver: CommandOptionResolver<'evt>,
) -> Result {
    let Some(guild_id) = ctx.event.guild_id else {
        bail!("command must be used within a guild");
    };

    let locale = ctx.event.preferred_locale();
    let Ok(user_id) = resolver.get_str("user")?.parse::<NonZeroU64>().map(Id::from) else {
        return ctx.failure(locale, format!("{}.invalid_user", entry.name), false).await;
    };

    let submission = Submission::stored((entry.name, guild_id, user_id));
    let Ok(submission) = submission.read() else {
        return ctx.failure(locale, format!("{}.no_entry", entry.name), false).await;
    };

    let (embed, components) = submission.get().build_form(entry, ctx.api).await?;

    // this will always be deferred, so using follow-up is fine.
    crate::followup!(as ctx => {
        let components = &components;
        let embeds = &[embed];
        let flags = EPHEMERAL;
    })
    .await?;

    Ok(())
}

pub async fn active<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: CommandCtx<'api, 'evt>,
    resolver: CommandOptionResolver<'evt>,
) -> Result {
    let Some(guild_id) = ctx.event.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = ctx.event.preferred_locale();
    let state = *resolver.get_bool("state")?;
    let config = Config::stored((entry.name, guild_id));
    let Ok(mut config) = config.read() else {
        return ctx.failure(locale, format!("{}.no_config", entry.name), false).await;
    };

    config.get_mut().entrypoint.open = state;
    config.write()?;

    let (color, key) = if state { (SUCCESS, "on") } else { (FAILURE, "off") };
    let title = localize!(try in locale, "success.{}.active_{key}.title", entry.name);
    let embed = EmbedBuilder::new().author(EmbedAuthor::parse(user)?).color(color).title(title);
    let channel_id = config.get().submission.output_channel_id;

    ctx.api.http.create_message(channel_id).embeds(&[embed.build()])?.await?;
    ctx.success(locale, format!("{}.active_{key}", entry.name), false).await
}
