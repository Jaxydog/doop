use std::num::NonZeroU64;

use anyhow::bail;
use doop_localizer::localize;
use doop_storage::{Stored, Value};
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType,
};
use twilight_model::guild::Member;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

use crate::bot::interaction::CommandCtx;
use crate::cmd::membership::configuration::Config;
use crate::cmd::CommandEntry;
use crate::util::extension::UserExtension;
use crate::util::traits::PreferLocale;
use crate::util::Result;

pub fn configuration<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: CommandCtx<'api, 'evt>,
    (name, value, kind): (&'evt str, &'evt str, CommandOptionType),
) -> Result<Vec<CommandOptionChoice>> {
    let Some(guild_id) = ctx.event.guild_id else {
        bail!("command must be used within a guild");
    };
    let Ok(config) = Config::stored((entry.name, guild_id)).read().map(Value::get_owned) else {
        return Ok(vec![]);
    };

    let locale = ctx.event.preferred_locale();
    let value = value.to_lowercase();

    Ok(match (name, kind) {
        ("title", CommandOptionType::String) => Some(&(*config.entrypoint.title)),
        ("description", CommandOptionType::String) => Some(&(*config.entrypoint.description)),
        ("question_1", CommandOptionType::String) => {
            config.submission.questions.first().map(|s| &**s)
        }
        ("question_2", CommandOptionType::String) => {
            config.submission.questions.get(1).map(|s| &(**s))
        }
        ("question_3", CommandOptionType::String) => {
            config.submission.questions.get(2).map(|s| &(**s))
        }
        ("question_4", CommandOptionType::String) => {
            config.submission.questions.get(3).map(|s| &(**s))
        }
        ("question_5", CommandOptionType::String) => {
            config.submission.questions.get(4).map(|s| &(**s))
        }
        _ => bail!("invalid auto-complete target '{name}' ({kind:?})"),
    }
    .into_iter()
    .filter(|s| s.to_lowercase().contains(&value))
    .map(|s| CommandOptionChoice {
        name: localize!(try in locale, "text.{}.configuration.filled", entry.name).into_string(),
        name_localizations: None,
        value: CommandOptionChoiceValue::String(if matches!(name, "title" | "description") {
            format!("%{name}%")
        } else {
            s.to_string()
        }),
    })
    .collect())
}

pub async fn member<'api: 'evt, 'evt>(
    entry: &CommandEntry,
    ctx: CommandCtx<'api, 'evt>,
    query: &str,
    predicate: impl Send + Sync + Fn(Id<GuildMarker>, &Member) -> bool,
) -> Result<Vec<CommandOptionChoice>> {
    #[inline]
    fn matches_query(member: &Member, query: &str) -> bool {
        member.user.name.to_lowercase().contains(query)
            || member.user.id.to_string().contains(query)
            || member.nick.as_deref().is_some_and(|s| s.to_lowercase().contains(query))
            || member.user.global_name.as_deref().is_some_and(|s| s.to_lowercase().contains(query))
    }

    #[inline]
    fn create_choice(member: &Member) -> CommandOptionChoice {
        CommandOptionChoice {
            name: member.display(),
            name_localizations: None,
            value: CommandOptionChoiceValue::String(member.user.id.to_string()),
        }
    }

    let Some(guild_id) = ctx.event.guild_id else {
        bail!("command must be used in a guild");
    };

    let query = query.to_lowercase();
    let guild = ctx.api.http.guild(guild_id).await?.model().await?;
    let dir = doop_storage::directory().join(entry.name).join(guild_id.to_string());
    let mut options = vec![];

    for entry in std::fs::read_dir(dir)?.filter_map(Result::ok) {
        if !entry.metadata().is_ok_and(|m| m.is_file()) {
            continue;
        }
        let Some(stem) = entry.path().file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        let Ok(user_id) = stem.parse::<NonZeroU64>().map(Id::from) else {
            continue;
        };

        if let Some(member) = guild.members.iter().find(|m| m.user.id == user_id) {
            if predicate(guild_id, member) && matches_query(member, &query) {
                options.push(create_choice(member));
            }
        } else {
            let Ok(member) = ctx.api.http.guild_member(guild_id, user_id).await else {
                continue;
            };
            let Ok(member) = member.model().await else {
                continue;
            };

            if predicate(guild_id, &member) && matches_query(&member, &query) {
                options.push(create_choice(&member));
            }
        }
    }

    options.dedup();
    options.truncate(25);

    Ok(options)
}
