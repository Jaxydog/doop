use std::fmt::Write;
use std::num::NonZeroU64;

use anyhow::bail;
use doop_localizer::localize;
use doop_storage::{Stored, Value};
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::interaction::ComponentCtx;
use crate::cmd::membership::configuration::Config;
use crate::cmd::membership::submission::{StatusKind, Submission};
use crate::cmd::CommandEntry;
use crate::util::extension::{EmbedAuthorExtension, UserExtension};
use crate::util::traits::{IntoImageSource, Localized, PreferLocale};
use crate::util::{DataId, Result};

#[inline]
pub async fn about<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: ComponentCtx<'api, 'evt>,
) -> Result {
    ctx.notify(ctx.event.preferred_locale(), format!("{}.about", entry.name), true).await
}

pub async fn apply<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: ComponentCtx<'api, 'evt>,
) -> Result {
    let Some(guild_id) = ctx.event.guild_id else {
        bail!("component must be used within a guild");
    };
    let Some(user_id) = ctx.event.author_id() else {
        bail!("component must be used by a user");
    };

    let locale = ctx.event.preferred_locale();
    let config = &Config::stored((entry.name, guild_id));
    let Ok(config) = config.read() else {
        return ctx.failure(locale, format!("{}.no_config", entry.name), false).await;
    };

    if let Ok(submission) = Submission::stored((entry.name, guild_id, user_id)).read() {
        let status = submission.get().status.kind;

        if matches!(status, StatusKind::Pending | StatusKind::Accepted | StatusKind::Rejected) {
            let key = status.localization_key();

            return ctx.failure(locale, format!("{}.exists_{key}", entry.name), false).await;
        }
    }

    if config.get().entrypoint.open {
        ctx.modal(config.get().build_application(entry, user_id, locale)?).await
    } else {
        ctx.failure(locale, format!("{}.closed", entry.name), false).await
    }
}

pub async fn update<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: ComponentCtx<'api, 'evt>,
    id: DataId,
) -> Result {
    let Some(guild_id) = ctx.event.guild_id else {
        bail!("component must be used within a guild");
    };

    let Some(user_id) = id.data(0) else {
        bail!("missing user identifier");
    };
    let user_id = Id::from(user_id.parse::<NonZeroU64>()?);

    let Some(status) = id.data(1) else {
        bail!("missing status identifier");
    };
    let status = StatusKind::try_from(status.parse::<i64>()?)?;

    let locale = ctx.event.preferred_locale();
    let submission = Submission::stored((entry.name, guild_id, user_id));
    let Ok(submission) = submission.read() else {
        return ctx.failure(locale, format!("{}.no_entry", entry.name), false).await;
    };

    ctx.modal(submission.get().build_update_modal(entry, ctx.api, status).await?).await
}

pub async fn entries<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: ComponentCtx<'api, 'evt>,
    id: DataId,
) -> Result {
    bail!("not yet implemented");
}

pub async fn updates<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    mut ctx: ComponentCtx<'api, 'evt>,
    id: DataId,
) -> Result {
    ctx.defer(true).await?;

    let Some(guild_id) = ctx.event.guild_id else {
        bail!("component must be used within a guild");
    };

    let Some(user_id) = id.data(0) else {
        bail!("missing user identifier");
    };
    let user_id = Id::from(user_id.parse::<NonZeroU64>()?);
    let user = ctx.api.http.user(user_id).await?.model().await?;

    let locale = ctx.event.preferred_locale();
    let submission = Submission::stored((entry.name, guild_id, user_id));
    let Ok(submission) = submission.read().map(Value::get_owned) else {
        return ctx.failure(locale, format!("{}.no_entry", entry.name), false).await;
    };

    let mut text = String::new();
    let mut update = submission.status.update.as_ref().map(|u| (submission.status.kind, u));

    /// Appends a label to the `text` buffer.
    macro_rules! label {
        ($key:literal, $value:expr) => {
            text.write_fmt(format_args!("> **{}:** {}\n", localize!(try in locale, "text.{}.form.{}", entry.name, $key), $value))
        };
    }

    while let Some((kind, data)) = update {
        text.write_str("\n")?;

        label!("status", kind.localize_in(locale, *entry))?;
        label!("author", format!("<@{}>", data.author_id))?;
        label!("updated", format!("<t:{}:R>", data.timestamp.unix_timestamp()))?;

        if let Some(ref comment) = data.comment {
            label!("comment", comment)?;
        }
        if let Some(ref reason) = data.reason {
            label!("reason", reason)?;
        }

        update = data.previous.as_ref().and_then(|s| s.update.as_ref().map(|u| (s.kind, u)));
    }

    if text.is_empty() {
        text = format!("> *{}*", localize!(try in locale, "text.{}.updates.empty", entry.name));
    }

    let embed = EmbedBuilder::new()
        .author(EmbedAuthor::parse(&user)?)
        .color(user.color())
        .description(text.trim())
        .thumbnail((&user).into_image_source()?)
        .title(localize!(try in locale, "text.{}.updates.title", entry.name));

    crate::followup!(as ctx => {
        let embeds = &[embed.build()];
        let flags = EPHEMERAL;
    })
    .await?;

    Ok(())
}
