use std::num::NonZeroU64;

use anyhow::bail;
use doop_storage::Stored;
use twilight_model::id::Id;

use crate::bot::interaction::ComponentCtx;
use crate::cmd::membership::configuration::Config;
use crate::cmd::membership::submission::{StatusKind, Submission};
use crate::cmd::CommandEntry;
use crate::util::traits::PreferLocale;
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
    ctx: ComponentCtx<'api, 'evt>,
    id: DataId,
) -> Result {
    bail!("not yet implemented");
}
