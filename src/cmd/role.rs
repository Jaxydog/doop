use std::num::NonZeroU64;

use anyhow::bail;
use doop_localizer::localize;
use doop_macros::Storage;
use doop_storage::{Compress, MsgPack, Stored};
use serde::{Deserialize, Serialize};
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType,
};
use twilight_model::channel::message::component::{Button, ButtonStyle};
use twilight_model::channel::message::{Component, ReactionType};
use twilight_model::id::marker::{GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::interaction::{CommandCtx, ComponentCtx};
use crate::cmd::{CommandEntry, CommandOptionResolver, OnCommand, OnComplete, OnComponent};
use crate::util::builder::{ActionRowBuilder, ButtonBuilder};
use crate::util::extension::ReactionTypeExtension;
use crate::util::traits::PreferLocale;
use crate::util::{DataId, Result, BRANDING};

crate::register_command! {
    ChatInput("role") {
        let in_dms = false;
        let is_nsfw = false;
        let require = USE_SLASH_COMMANDS | SEND_MESSAGES | MANAGE_ROLES;
        let options = [
            SubCommand("create") {
                let options = [
                    String("role") {
                        let required = true;
                        let autocomplete = true;
                    },
                    String("icon") {
                        let required = true;
                    },
                ];
            },
            SubCommand("remove") {
                let options = [
                    String("role") {
                        let required = true;
                        let autocomplete = true;
                    },
                ];
            },
            SubCommand("view") {},
            SubCommand("send") {
                let options = [
                    String("text") {
                        let required = true;
                        let maximum = 256;
                    }
                ];
            },
        ];
        let handlers = {
            command = self::execute_command;
            complete = self::execute_complete;
            component = self::execute_component;
        };
    }
}

/// A role selector.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct Selector {
    /// The role identifier.
    pub id: Id<RoleMarker>,
    /// The selector's icon.
    pub icon: Box<str>,
    /// The selector's name.
    pub name: Box<str>,
}

impl Selector {
    /// The selector component name.
    pub const NAME: &'static str = "select";

    /// Builds a new button from the role selector.
    ///
    /// # Errors
    ///
    /// This function will return an error if the button could not be constructed.
    pub fn build(&self, entry: &CommandEntry, disabled: bool) -> Result<Button> {
        let id = DataId::new(entry.name, Self::NAME).with(self.id.to_string());

        Ok(ButtonBuilder::new(ButtonStyle::Secondary)
            .custom_id(id.validate()?)
            .disabled(disabled)
            .emoji(ReactionType::parse(&(*self.icon))?)
            .label(&(*self.name))
            .build())
    }
}

/// A list of role selectors assigned to a user.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Storage)]
#[format(Compress<MsgPack, 5>)]
#[location("role/{}/{}", Id<GuildMarker>, Id<UserMarker>)]
struct Selectors {
    inner: Vec<Selector>,
}

impl Selectors {
    /// Returns the length of this [`Selectors`] list.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether this [`Selectors`] is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns whether this [`Selectors`] list contains the given role.
    #[inline]
    #[must_use]
    pub fn contains(&self, role_id: Id<RoleMarker>) -> bool {
        self.inner.iter().any(|s| s.id == role_id)
    }

    /// Inserts the given selector into the list.
    pub fn insert(&mut self, selector: Selector) -> bool {
        if self.contains(selector.id) {
            false
        } else {
            self.inner.push(selector);

            true
        }
    }

    /// Removes the given role identifier from the list.
    pub fn remove(&mut self, role_id: Id<RoleMarker>) -> Option<Selector> {
        if let Some((index, _)) = self.inner.iter().enumerate().find(|(_, s)| s.id == role_id) {
            Some(self.inner.remove(index))
        } else {
            None
        }
    }

    /// Builds a list of components from the list of selectors.
    ///
    /// # Errors
    ///
    /// This function will return an error if a selector could not be constructed.
    pub fn build(&self, entry: &CommandEntry, disabled: bool) -> Result<Vec<Component>> {
        let count = self.inner.len().div_ceil(5).min(5);
        let mut components = vec![ActionRowBuilder::new(); count];

        for (index, selector) in self.inner.iter().enumerate() {
            let button = selector.build(entry, disabled)?;
            let row_index = index / 5;

            components[row_index].push(button)?;
        }

        Ok(components.into_iter().map(|r| Component::ActionRow(r.build())).collect())
    }
}

async fn execute_command<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    mut ctx: CommandCtx<'api, 'evt>,
) -> Result {
    ctx.defer(true).await?;

    let resolver = CommandOptionResolver::new(ctx.data);

    if let Ok(resolver) = resolver.get_subcommand("create") {
        let role_id = Id::from(resolver.get_str("role")?.parse::<NonZeroU64>()?);
        let icon = resolver.get_str("icon")?;

        if ReactionType::parse(icon).is_err() {
            let locale = ctx.event.author().preferred_locale();

            return ctx.failure(locale, "invalid_emoji", false).await;
        };

        return self::create(cmd, ctx, role_id, icon).await;
    }
    if let Ok(resolver) = resolver.get_subcommand("remove") {
        let role_id = Id::from(resolver.get_str("role")?.parse::<NonZeroU64>()?);

        return self::remove(cmd, ctx, role_id).await;
    }
    if resolver.get_subcommand("view").is_ok() {
        return self::view(cmd, ctx).await;
    }
    if let Ok(resolver) = resolver.get_subcommand("send") {
        let text = resolver.get_str("text")?;

        return self::send(cmd, ctx, text).await;
    }

    bail!("unknown or missing subcommand");
}

async fn create<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
    role_id: Id<RoleMarker>,
    icon: &'evt str,
) -> Result {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = ctx.event.author().preferred_locale();
    let selectors = Selectors::stored((guild_id, user.id));
    let mut selectors = selectors.read_or_default();

    if selectors.get().len() >= 25 {
        return ctx.failure(locale, format!("{}.max_len", cmd.entry().name), true).await;
    }

    let name = if let Some(role) = ctx.api.cache.role(role_id) {
        role.name.clone()
    } else {
        let roles = ctx.api.http.roles(guild_id).await?.model().await?;
        let Some(role) = roles.into_iter().find(|r| r.id == role_id) else {
            bail!("invalid role identifier '{role_id}'");
        };

        role.name
    };

    let selector = Selector { id: role_id, icon: icon.into(), name: name.into_boxed_str() };

    if selectors.get_mut().insert(selector) {
        selectors.write()?;

        ctx.success(locale, format!("{}.created", cmd.entry().name), false).await
    } else {
        ctx.failure(locale, format!("{}.exists", cmd.entry().name), false).await
    }
}

async fn remove<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
    role_id: Id<RoleMarker>,
) -> Result {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = ctx.event.author().preferred_locale();
    let selectors = Selectors::stored((guild_id, user.id));
    let Ok(mut selectors) = selectors.read() else {
        return ctx.failure(locale, format!("{}.empty", cmd.entry().name), false).await;
    };

    if selectors.get_mut().remove(role_id).is_some() {
        if selectors.get().is_empty() { selectors.remove() } else { selectors.write() }?;

        ctx.success(locale, format!("{}.removed", cmd.entry().name), false).await
    } else {
        ctx.failure(locale, format!("{}.missing", cmd.entry().name), false).await
    }
}

async fn view<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
) -> Result {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = ctx.event.author().preferred_locale();
    let selectors = Selectors::stored((guild_id, user.id));
    let Ok(selectors) = selectors.read() else {
        return ctx.failure(locale, format!("{}.empty", cmd.entry().name), false).await;
    };

    let components = selectors.get().build(cmd.entry(), true)?;
    let title = localize!(try in locale, "text.{}.listing", cmd.entry().name);
    let embed = EmbedBuilder::new().color(BRANDING).title(title).build();

    crate::followup!(as ctx => {
        let components = &components;
        let embeds = &[embed];
    })
    .await?;

    Ok(())
}

async fn send<'api: 'evt, 'evt>(
    cmd: &(dyn OnCommand + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
    text: &'evt str,
) -> Result {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(channel_id) = ctx.event.channel.as_ref().map(|c| c.id) else {
        bail!("command must be used in a channel");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = ctx.event.author().preferred_locale();
    let selectors = Selectors::stored((guild_id, user.id));
    let Ok(selectors) = selectors.read() else {
        return ctx.failure(locale, format!("{}.empty", cmd.entry().name), false).await;
    };

    let components = selectors.get().build(cmd.entry(), false)?;
    let embed = EmbedBuilder::new().color(BRANDING).title(text).build();

    ctx.api.http.create_message(channel_id).embeds(&[embed])?.components(&components)?.await?;
    selectors.remove()?;

    ctx.success(locale, format!("{}.finished", cmd.entry().name), false).await
}

async fn execute_complete<'api: 'evt, 'evt>(
    _: &(dyn OnComplete + Send + Sync),
    ctx: CommandCtx<'api, 'evt>,
    (name, value, kind): (&'evt str, &'evt str, CommandOptionType),
) -> Result<Vec<CommandOptionChoice>> {
    let ("role", CommandOptionType::String) = (name, kind) else {
        bail!("invalid auto-complete target '{name}' ({kind:?})");
    };

    let resolver = CommandOptionResolver::new(ctx.data);

    if resolver.get_subcommand("create").is_ok() {
        return self::fill_missing_role(ctx, value).await;
    }
    if resolver.get_subcommand("remove").is_ok() {
        return self::fill_contained_role(ctx, value);
    }

    bail!("unknown or missing subcommand");
}

fn fill_contained_role<'api: 'evt, 'evt>(
    ctx: CommandCtx<'api, 'evt>,
    value: &'evt str,
) -> Result<Vec<CommandOptionChoice>> {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let selectors = Selectors::stored((guild_id, user.id));
    let selectors = selectors.read_or_default();
    let options = selectors.get().inner.iter().filter(|&s| s.name.contains(value)).map(|s| {
        CommandOptionChoice {
            name: s.name.to_string(),
            name_localizations: None,
            value: CommandOptionChoiceValue::String(s.id.to_string()),
        }
    });

    Ok(options.collect())
}

async fn fill_missing_role<'api: 'evt, 'evt>(
    ctx: CommandCtx<'api, 'evt>,
    value: &'evt str,
) -> Result<Vec<CommandOptionChoice>> {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let selectors = Selectors::stored((guild_id, user.id));
    let selectors = selectors.read_or_default();
    let roles = ctx.api.http.roles(guild_id).await?.model().await?;
    let value = value.to_lowercase();

    let options = roles.into_iter().filter_map(|role| {
        if selectors.get().contains(role.id)
            || !role.name.to_lowercase().contains(&value)
            // ignore @everyone
            || role.id.cast() == guild_id
        {
            return None;
        }

        Some(CommandOptionChoice {
            name: role.name,
            name_localizations: None,
            value: CommandOptionChoiceValue::String(role.id.to_string()),
        })
    });

    Ok(options.collect())
}

async fn execute_component<'api: 'evt, 'evt>(
    cpn: &(dyn OnComponent + Send + Sync),
    mut ctx: ComponentCtx<'api, 'evt>,
    id: DataId,
) -> Result {
    ctx.defer_update(true).await?;

    let Some(guild_id) = ctx.event.guild_id else {
        bail!("command must be used within a guild");
    };
    let Some(user_id) = ctx.event.author_id() else {
        bail!("command must be used by a user");
    };
    let Some(role_id) = id.data(0) else {
        bail!("missing role identifier");
    };

    let role_id = Id::<RoleMarker>::from(role_id.parse::<NonZeroU64>()?);
    let locale = ctx.event.author().preferred_locale();

    let Ok(member) = ctx.api.http.guild_member(guild_id, user_id).await else {
        return ctx.failure(locale, "not_member", false).await;
    };
    let mut roles = member.model().await?.roles;

    if let Some((index, _)) = roles.iter().enumerate().find(|(_, id)| *id == &role_id) {
        roles.remove(index);

        ctx.api.http.update_guild_member(guild_id, user_id).roles(&roles).await?;
        ctx.success(locale, format!("{}.toggle_off", cpn.entry().name), false).await
    } else {
        roles.push(role_id);

        ctx.api.http.update_guild_member(guild_id, user_id).roles(&roles).await?;
        ctx.success(locale, format!("{}.toggle_on", cpn.entry().name), false).await
    }
}
