use std::fmt::Write;
use std::num::NonZeroU64;

use anyhow::bail;
use doop_localizer::{localize, localizer, Locale};
use doop_storage::{Stored, Value};
use time::OffsetDateTime;
use twilight_model::id::Id;

use crate::bot::interaction::ModalCtx;
use crate::cmd::membership::configuration::Config;
use crate::cmd::membership::submission::{
    Status, StatusKind, StatusUpdate, Submission, SubmissionArchive,
};
use crate::cmd::{CommandEntry, ModalFieldResolver};
use crate::util::traits::PreferLocale;
use crate::util::{Anchor, DataId, Result};

pub async fn application<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    mut ctx: ModalCtx<'api, 'evt>,
) -> Result {
    ctx.defer(true).await?;

    let Some(guild_id) = ctx.event.guild_id else {
        bail!("modal must be used within a guild");
    };
    let Some(user_id) = ctx.event.author_id() else {
        bail!("modal must be used by a user");
    };
    let Some(ref locale) = ctx.event.guild_locale else {
        bail!("modal must be used within a guild");
    };

    let locale = Locale::get(locale).unwrap_or_else(|| *localizer().preferred_locale());
    let config = Config::stored((entry.name, guild_id));
    let Ok(config) = config.read() else {
        return ctx.failure(locale, format!("{}.no_config", entry.name), false).await;
    };

    let resolver = ModalFieldResolver::new(ctx.data);
    let key = Submission::stored((entry.name, guild_id, user_id));
    let mut submission = Submission {
        id: user_id,
        guild_id,
        timestamp: OffsetDateTime::now_utc(),
        status: Status::default(),
        answers: Box::default(),
        anchor: None,
        archives: Box::default(),
    };

    let mut answers = Vec::with_capacity(config.get().submission.questions.len());

    for (index, question) in config.get().submission.questions.iter().enumerate() {
        let answer = resolver
            .get(&index.to_string())
            .map_or_else(|_| localize!(try in locale, "text.{}.no_answer", entry.name), Into::into);

        answers.push((question.clone(), answer));
    }

    submission.answers = answers.into_boxed_slice();

    if let Ok(previous) = key.read().map(Value::get_owned) {
        let mut archives = previous.archives.to_vec();

        archives.push(SubmissionArchive {
            timestamp: previous.timestamp,
            status: previous.status.clone(),
            answers: previous.answers,
        });

        submission.archives = archives.into_boxed_slice();
        submission.status.update = Some(StatusUpdate {
            author_id: user_id,
            timestamp: OffsetDateTime::now_utc(),
            reason: None,
            comment: None,
            previous: Some(Box::new(previous.status)),
        });
    };

    let (embed, components) = submission.build_form(entry, ctx.api).await?;
    let message = ctx.api.http.create_message(config.get().submission.output_channel_id);
    let message = message.components(&components)?.embeds(&[embed])?.await?.model().await?;

    submission.anchor = Some(Anchor::from(message));
    key.write(&submission)?;

    let status = submission.status.kind.localization_key();
    let title = localize!(try in locale, "text.{}.update_{status}.title", entry.name);
    let mut description =
        localize!(try in locale, "text.{}.update_{status}.description", entry.name).to_string();

    if let Ok(comment) = resolver.get("comment") {
        description.write_fmt(format_args!("\n\n> {comment}"))?;
    }

    submission.inform(ctx.api, title, description).await?;

    ctx.success(locale, format!("{}.received", entry.name), false).await
}

pub async fn update<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    mut ctx: ModalCtx<'api, 'evt>,
    id: DataId,
) -> Result {
    ctx.defer(true).await?;

    let Some(guild_id) = ctx.event.guild_id else {
        bail!("modal must be used within a guild");
    };
    let Some(author_id) = ctx.event.author_id() else {
        bail!("modal must be used by a user");
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

    let config = Config::stored((entry.name, guild_id));
    let Ok(config) = config.read() else {
        return ctx.failure(locale, format!("{}.no_config", entry.name), false).await;
    };

    let submission = Submission::stored((entry.name, guild_id, user_id));
    let Ok(mut submission) = submission.read() else {
        return ctx.failure(locale, format!("{}.no_entry", entry.name), false).await;
    };

    let mut member = ctx.api.http.guild_member(guild_id, user_id).await?.model().await?;
    let resolver = ModalFieldResolver::new(ctx.data);

    submission.get_mut().update_status(
        status,
        StatusUpdate {
            author_id,
            timestamp: OffsetDateTime::now_utc(),
            reason: resolver.get("reason").ok().map(Into::into),
            comment: resolver.get("comment").ok().map(Into::into),
            previous: None,
        },
    );
    submission.write()?;

    let (embed, components) = submission.get().build_form(entry, ctx.api).await?;

    if let Some(anchor) = submission.get().anchor {
        let message = anchor.update(ctx.api).components(Some(&components))?;

        message.embeds(Some(&[embed]))?.await?.model().await?;
    } else {
        let message = ctx.api.http.create_message(config.get().submission.output_channel_id);
        let message = message.components(&components)?.embeds(&[embed])?.await?.model().await?;

        submission.get_mut().anchor = Some(Anchor::from(message));
    }

    let member_role_id = config.get().submission.member_role_id;

    if status == StatusKind::Accepted && !member.roles.contains(&member_role_id) {
        member.roles.push(config.get().submission.member_role_id);
    } else if member.roles.contains(&member_role_id) {
        member.roles.retain(|id| id != &member_role_id);
    }

    ctx.api.http.update_guild_member(guild_id, user_id).roles(&member.roles).await?;

    let key = status.localization_key();
    let title = localize!(try in locale, "text.{}.update_{key}.title", entry.name);
    let mut description =
        localize!(try in locale, "text.{}.update_{key}.description", entry.name).to_string();

    if let Ok(comment) = resolver.get("comment") {
        description.write_fmt(format_args!("\n\n> {comment}"))?;
    }

    if submission.get().inform(ctx.api, title, description).await? {
        ctx.success(locale, format!("{}.updated_{key}", entry.name), false).await
    } else {
        ctx.notify(locale, format!("{}.updated_{key}", entry.name), true).await
    }
}
