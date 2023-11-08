use std::path::PathBuf;

use anyhow::bail;
use doop_localizer::{localize, Locale};
use doop_logger::info;
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType,
};
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::interaction::CommandCtx;
use crate::cmd::{CommandEntry, CommandOptionResolver, OnCommand, OnComplete};
use crate::util::traits::PreferLocale;
use crate::util::{Result, BRANDING};

crate::register_command! {
    #[developer(true)]
    ChatInput("lang") {
        let in_dms = false;
        let is_nsfw = false;
        let require = ADMINISTRATOR;
        let options = [
            SubCommand("reload") {},
            SubCommand("localize") {
                let options = [
                    String("key") {
                        let required = true;
                    },
                    String("locale") {
                        let autocomplete = true;
                    },
                ];
            },
        ];
        let handlers = {
            command = self::execute_command;
            complete = self::execute_complete;
        };
    }
}

async fn execute_command<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    mut ctx: CommandCtx<'api, 'evt>,
) -> Result {
    ctx.defer(true).await?;

    let resolver = CommandOptionResolver::new(ctx.data);

    if resolver.get_subcommand("reload").is_ok() {
        return self::reload(cmd.entry(), ctx).await;
    }
    if let Ok(resolver) = resolver.get_subcommand("localize") {
        return self::localize(ctx, resolver).await;
    }

    bail!("unknown or missing subcommand");
}

async fn reload<'api: 'evt, 'evt>(entry: &CommandEntry, ctx: CommandCtx<'api, 'evt>) -> Result {
    info!("reloading localizer instance")?;

    let arguments = crate::util::arguments();
    let dir = arguments.data_dir.clone().unwrap_or_else(|| PathBuf::from("res").into());
    let dir = arguments.l18n_map_dir.clone().unwrap_or_else(|| dir.join("lang").into());
    let prefer = *doop_localizer::localizer().preferred_locale();

    doop_localizer::reload(prefer, dir);

    ctx.success(ctx.event.preferred_locale(), format!("{}.reloaded", entry.name), false).await
}

async fn localize<'api: 'evt, 'evt>(
    ctx: CommandCtx<'api, 'evt>,
    resolver: CommandOptionResolver<'evt>,
) -> Result {
    let key = resolver.get_str("key")?;

    #[allow(clippy::option_if_let_else)]
    let text = if let Some(locale) = resolver.get_str("locale").ok().and_then(Locale::get) {
        localize!(in locale, "{key}")
    } else {
        localize!("{key}")
    };

    crate::followup!(as ctx => {
        let embeds = &[EmbedBuilder::new().color(BRANDING).description(text).build()];
    })
    .await?;

    Ok(())
}

#[allow(clippy::unused_async)]
async fn execute_complete<'api: 'evt, 'evt>(
    _: &(dyn OnComplete + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
    (name, value, kind): (&'evt str, &'evt str, CommandOptionType),
) -> Result<Vec<CommandOptionChoice>> {
    if CommandOptionResolver::new(ctx.data).get_subcommand("localize").is_err() {
        return Ok(vec![]);
    }
    let ("locale", CommandOptionType::String) = (name, kind) else {
        bail!("invalid auto-complete target '{name}' ({kind:?})");
    };

    let value = value.to_lowercase();

    Ok(Locale::LIST
        .iter()
        .filter(|l| {
            l.key().to_lowercase().contains(&value) || l.to_string().to_lowercase().contains(&value)
        })
        .map(|locale| CommandOptionChoice {
            name: locale.to_string(),
            name_localizations: None,
            value: CommandOptionChoiceValue::String(locale.key().to_string()),
        })
        .take(25)
        .collect())
}
