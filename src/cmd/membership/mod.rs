use anyhow::bail;
use doop_storage::{Stored, Value};
use twilight_model::application::command::{CommandOptionChoice, CommandOptionType};

use crate::bot::interaction::{CommandCtx, ComponentCtx, ModalCtx};
use crate::cmd::membership::submission::{StatusKind, Submission};
use crate::cmd::{CommandOptionResolver, OnCommand, OnComplete, OnComponent, OnModal};
use crate::util::{DataId, Result};

mod configuration;
mod submission;

mod on {
    pub mod command;
    pub mod complete;
    pub mod component;
    pub mod modal;
}

/// The number of defined entry toast titles.
pub const ENTRY_TOASTS: usize = 10;

crate::register_command! {
    ChatInput("membership") {
        let in_dms = false;
        let is_nsfw = false;
        let require = MODERATE_MEMBERS | BAN_MEMBERS | MANAGE_ROLES;
        let options = [
            SubCommand("configure") {
                let options = [
                    String("title") {
                        let required = true;
                        let autocomplete = true;
                        let maximum = 256;
                    },
                    String("description") {
                        let required = true;
                        let autocomplete = true;
                        let maximum = 4096;
                    },
                    Channel("output_channel") {
                        let required = true;
                        let channels = GuildText;
                    },
                    Role("member_role") {
                        let required = true;
                    },
                    String("question_1") {
                        let required = true;
                        let autocomplete = true;
                        let maximum = 45;
                    },
                    String("question_2") {
                        let autocomplete = true;
                        let maximum = 45;
                    },
                    String("question_3") {
                        let autocomplete = true;
                        let maximum = 45;
                    },
                    String("question_4") {
                        let autocomplete = true;
                        let maximum = 45;
                    },
                    String("question_5") {
                        let autocomplete = true;
                        let maximum = 45;
                    },
                ];
            },
            SubCommand("update") {
                let options = [
                    String("user") {
                        let required = true;
                        let autocomplete = true;
                    },
                    Integer("status") {
                        let required = true;
                        let choices = [
                            ("accept", StatusKind::Accepted as i64),
                            ("reject", StatusKind::Rejected as i64),
                            ("revise", StatusKind::Resubmit as i64),
                        ];
                    },
                ];
            },
            SubCommand("discard") {
                let options = [
                    String("user") {
                        let required = true;
                        let autocomplete = true;
                    },
                ];
            },
            SubCommand("view") {
                let options = [
                    String("user") {
                        let required = true;
                        let autocomplete = true;
                    },
                ];
            },
            SubCommand("active") {
                let options = [
                    Boolean("state") {
                        let required = true;
                    },
                ];
            },
        ];
        let handlers = {
            command = self::execute_command;
            complete = self::execute_complete;
            component = self::execute_component;
            modal = self::execute_modal;
        };
    }
}

async fn execute_command<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    mut ctx: CommandCtx<'api, 'evt>,
) -> Result {
    let resolver = CommandOptionResolver::new(ctx.data);

    if let Ok(resolver) = resolver.get_subcommand("update") {
        return self::on::command::update(cmd.entry(), ctx, resolver).await;
    }

    // ^ update responds with a modal, which cannot be deferred.
    ctx.defer(true).await?;

    if let Ok(resolver) = resolver.get_subcommand("configure") {
        return self::on::command::configure(cmd.entry(), ctx, resolver).await;
    }
    if let Ok(resolver) = resolver.get_subcommand("discard") {
        return self::on::command::discard(cmd.entry(), ctx, resolver).await;
    }
    if let Ok(resolver) = resolver.get_subcommand("view") {
        return self::on::command::view(cmd.entry(), ctx, resolver).await;
    }
    if let Ok(resolver) = resolver.get_subcommand("active") {
        return self::on::command::active(cmd.entry(), ctx, resolver).await;
    }

    bail!("unknown or missing subcommand");
}

async fn execute_complete<'api: 'evt, 'evt>(
    acp: &(dyn OnComplete + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
    (name, value, kind): (&'evt str, &'evt str, CommandOptionType),
) -> Result<Vec<CommandOptionChoice>> {
    let resolver = CommandOptionResolver::new(ctx.data);

    if resolver.get_subcommand("configure").is_ok() {
        return self::on::complete::configuration(acp.entry(), ctx, (name, value, kind));
    }

    let ("user", CommandOptionType::String) = (name, kind) else {
        return Ok(vec![]);
    };

    if resolver.get_subcommand("update").is_ok() {
        return self::on::complete::member(acp.entry(), ctx, value, |guild_id, member| {
            let submission = Submission::stored((acp.entry().name, guild_id, member.user.id));
            let Ok(submission) = submission.read().map(Value::get_owned) else {
                return true;
            };

            submission.status.kind == StatusKind::Pending
        })
        .await;
    }
    if resolver.get_subcommand("discard").is_ok() {
        return self::on::complete::member(acp.entry(), ctx, value, |_, _| true).await;
    }
    if resolver.get_subcommand("view").is_ok() {
        return self::on::complete::member(acp.entry(), ctx, value, |_, _| true).await;
    }

    bail!("unknown or missing subcommand");
}

async fn execute_component<'api: 'evt, 'evt>(
    cpn: &(dyn OnComponent + Send + Sync),
    ctx: ComponentCtx<'api, 'evt>,
    id: DataId,
) -> Result {
    match id.kind() {
        "about" => self::on::component::about(cpn.entry(), ctx).await,
        "apply" => self::on::component::apply(cpn.entry(), ctx).await,
        "update" => self::on::component::update(cpn.entry(), ctx, id).await,
        "entries" => self::on::component::entries(cpn.entry(), ctx, id).await,
        "updates" => self::on::component::updates(cpn.entry(), ctx, id).await,
        _ => bail!("unknown or missing component"),
    }
}

async fn execute_modal<'api: 'evt, 'evt>(
    cpn: &(dyn OnModal + Send + Sync),
    ctx: ModalCtx<'api, 'evt>,
    id: DataId,
) -> Result {
    match id.kind() {
        "application" => self::on::modal::application(cpn.entry(), ctx).await,
        "update" => self::on::modal::update(cpn.entry(), ctx, id).await,
        _ => bail!("unknown or missing modal"),
    }
}
