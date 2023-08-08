use std::ops::{Deref, DerefMut};

use anyhow::bail;
use doop_localizer::localize;
use doop_macros::Storage;
use doop_storage::{Compress, Key, MsgPack, Storage, Val};
use serde::{Deserialize, Serialize};
use twilight_model::channel::message::component::{Button, ButtonStyle};
use twilight_model::channel::message::ReactionType;
use twilight_model::id::marker::{GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

use crate::bot::interact::{
    CommandCtx, CommandOptionResolver, ComponentCtx, CustomData, InteractionEventHandler,
};
use crate::util::builder::ButtonBuilder;
use crate::util::ext::{LocalizedExt, ReactionTypeParseExt};
use crate::util::{button_rows, Result, BRANDING, FAILURE, SUCCESS};

crate::command! {
    let name = "role";
    let kind = ChatInput;
    let permissions = MANAGE_ROLES | SEND_MESSAGES;
    let allow_dms = false;
    let is_nsfw = false;
    let options = [
        {
            let name = "add";
            let kind = SubCommand;
            let options = [
                {
                    let name = "role";
                    let kind = Role;
                    let required = true;
                },
                {
                    let name = "icon";
                    let kind = String;
                    let required = true;
                },
            ];
        },
        {
            let name = "remove";
            let kind = SubCommand;
            let options = [
                {
                    let name = "role";
                    let kind = Role;
                    let required = true;
                },
            ];
        },
        {
            let name = "list";
            let kind = SubCommand;
        },
        {
            let name = "send";
            let kind = SubCommand;
            let options = [
                {
                    let name = "text";
                    let kind = String;
                    let required = true;
                    let max = 256;
                }
            ];
        },
    ];
}

/// Stores a role selector's data.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector {
    /// The selector's role identifier.
    pub id: Id<RoleMarker>,
    /// The selector's icon.
    pub icon: Box<str>,
    /// The selector's name.
    pub name: Box<str>,
}

impl Selector {
    /// Creates a button from the selector.
    ///
    /// # Errors
    ///
    /// This function will return an error if the button could not be created.
    pub fn button(&self, disabled: bool) -> Result<Button> {
        let mut data = CustomData::new(Impl::NAME, "toggle");

        data.insert(self.id.to_string());
        data.validate()?;

        Ok(ButtonBuilder::new(ButtonStyle::Secondary)
            .custom_id(data)
            .disabled(disabled)
            .emoji(ReactionType::parse(&self.icon)?)
            .label(self.name.clone())
            .build())
    }
}

/// Stores a user's role selector data.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize, Storage)]
#[serde(transparent)]
#[storage(format = Compress<MsgPack>, at = "role/{}/{}", Id<GuildMarker>, Id<UserMarker>)]
pub struct Selectors(Vec<Selector>);

impl Selectors {
    /// Creates a button from the selectors.
    ///
    /// # Errors
    ///
    /// This function will return an error if the buttons could not be created.
    #[inline]
    pub fn buttons(&self, disabled: bool) -> Result<Vec<Button>> {
        self.iter().try_fold(Vec::with_capacity(self.len()), |mut list, selector| {
            list.extend_from_slice(&[selector.button(disabled)?]);

            Ok(list)
        })
    }
}

impl Deref for Selectors {
    type Target = Vec<Selector>;

    #[inline]
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Selectors {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

#[async_trait::async_trait]
impl InteractionEventHandler for Impl {
    async fn handle_command(&self, ctx: CommandCtx<'_>) -> Result {
        crate::respond!(as ctx => {
            let kind = DeferredChannelMessageWithSource;
            let flags = EPHEMERAL;
        })
        .await?;

        let resolver = CommandOptionResolver::new(ctx.data);

        if let Ok(resolver) = resolver.get_subcommand("add") {
            return add(ctx, resolver).await;
        }
        if let Ok(resolver) = resolver.get_subcommand("remove") {
            return remove(ctx, resolver).await;
        }
        if let Ok(resolver) = resolver.get_subcommand("list") {
            return list(ctx, resolver).await;
        }
        if let Ok(resolver) = resolver.get_subcommand("send") {
            return send(ctx, resolver).await;
        }

        bail!("unknown or missing subcommand");
    }

    async fn handle_component(&self, ctx: ComponentCtx<'_>, data: CustomData) -> Result {
        crate::respond!(as ctx => {
            let kind = DeferredChannelMessageWithSource;
            let flags = EPHEMERAL | SUPPRESS_NOTIFICATIONS;
        })
        .await?;

        let Some(guild_id) = ctx.event.guild_id else {
            bail!("component must be used in a guild");
        };
        let Some(user_id) = ctx.event.author_id() else {
            bail!("component must be used by a user");
        };
        let Some(role_id) = data.data().first() else {
            bail!("missing role identifier");
        };
        let Some(role_id) = Id::new_checked(role_id.parse()?) else {
            bail!("expected a non-zero role identifier");
        };

        let mut member = ctx.api.http().guild_member(guild_id, user_id).await?.model().await?;
        let locale = member.locale();
        let title = if member.roles.iter().any(|id| id == &role_id) {
            member.roles.retain(|id| id != &role_id);

            localize!(try locale => "text.{}.toggle_off", Self::NAME)
        } else {
            member.roles.push(role_id);

            localize!(try locale => "text.{}.toggle_on", Self::NAME)
        };

        ctx.api
            .http()
            .update_guild_member(guild_id, user_id)
            .roles(&member.roles)
            .await?;

        let embed = EmbedBuilder::new().color(SUCCESS).title(title);

        crate::followup!(as ctx => {
            let embeds = &[embed.build()];
            let flags = EPHEMERAL | SUPPRESS_NOTIFICATIONS;
        })
        .await?;

        Ok(())
    }
}

/// The add subcommand.
///
/// # Errors
///
/// This function will return an error if execution fails.
async fn add<'c>(ctx: CommandCtx<'c>, resolver: CommandOptionResolver<'c>) -> Result {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used in a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = user.locale();
    let mut selectors = Selectors::read_or_default((guild_id, user.id));

    if selectors.data().len() >= 25 {
        let title = localize!(try locale => "text.{}.full", Impl::NAME);
        let embed = EmbedBuilder::new().color(FAILURE).title(title);

        crate::followup!(as ctx => {
            let embeds = &[embed.build()];
            let flags = EPHEMERAL;
        })
        .await?;

        return Ok(());
    }

    let role_id = *resolver.get_role_id("role")?;
    let icon = resolver.get_str("icon")?.trim().into();
    let name = if let Some(role) = ctx.api.cache().role(role_id) {
        role.name.clone()
    } else {
        let roles = ctx.api.http().roles(guild_id).await?.model().await?;
        let Some(role) = roles.into_iter().find(|r| r.id == role_id) else {
            bail!("invalid role identifier '{role_id}'");
        };

        role.name
    };

    if ReactionType::parse(&icon).is_err() {
        let title = localize!(try locale => "text.{}.invalid_icon", Impl::NAME);
        let embed = EmbedBuilder::new().color(FAILURE).title(title);

        crate::followup!(as ctx => {
            let embeds = &[embed.build()];
            let flags = EPHEMERAL;
        })
        .await?;

        return Ok(());
    }

    selectors.data_mut().retain(|s| s.id != role_id);
    selectors.data_mut().push(Selector { id: role_id, icon, name: name.into() });
    selectors.write()?;

    let title = localize!(try locale => "text.{}.added", Impl::NAME);
    let embed = EmbedBuilder::new().color(SUCCESS).title(title);

    crate::followup!(as ctx => {
        let embeds = &[embed.build()];
        let flags = EPHEMERAL;
    })
    .await?;

    Ok(())
}

/// The remove subcommand.
///
/// # Errors
///
/// This function will return an error if execution fails.
async fn remove<'c>(ctx: CommandCtx<'c>, resolver: CommandOptionResolver<'c>) -> Result {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used in a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = user.locale();
    let role_id = *resolver.get_role_id("role")?;
    let Ok(mut selectors) = Selectors::read((guild_id, user.id)) else {
        let title = localize!(try locale => "text.{}.empty", Impl::NAME);
        let embed = EmbedBuilder::new().color(FAILURE).title(title);

        crate::followup!(as ctx => {
            let embeds = &[embed.build()];
            let flags = EPHEMERAL;
        })
        .await?;

        return Ok(());
    };

    let len = selectors.data().len();

    selectors.data_mut().retain(|s| s.id != role_id);

    if selectors.data().len() == len {
        let title = localize!(try locale => "text.{}.missing", Impl::NAME);
        let embed = EmbedBuilder::new().color(FAILURE).title(title);

        crate::followup!(as ctx => {
            let embeds = &[embed.build()];
            let flags = EPHEMERAL;
        })
        .await?;

        return Ok(());
    }

    if selectors.data().is_empty() {
        selectors.key_owned().remove()?;
    } else {
        selectors.write()?;
    }

    let title = localize!(try locale => "text.{}.removed", Impl::NAME);
    let embed = EmbedBuilder::new().color(SUCCESS).title(title);

    crate::followup!(as ctx => {
        let embeds = &[embed.build()];
        let flags = EPHEMERAL;
    })
    .await?;

    Ok(())
}

/// The list subcommand.
///
/// # Errors
///
/// This function will return an error if execution fails.
async fn list<'c>(ctx: CommandCtx<'c>, _resolver: CommandOptionResolver<'c>) -> Result {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used in a guild");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = user.locale();

    let Ok(selectors) = Selectors::read((guild_id, user.id)) else {
        let title = localize!(try locale => "text.{}.empty", Impl::NAME);
        let embed = EmbedBuilder::new().color(FAILURE).title(title);

        crate::followup!(as ctx => {
            let embeds = &[embed.build()];
            let flags = EPHEMERAL;
        })
        .await?;

        return Ok(());
    };

    let buttons = button_rows(selectors.data().buttons(true)?);
    let title = localize!(try locale => "text.{}.list", Impl::NAME);
    let embed = EmbedBuilder::new().color(BRANDING).title(title);

    crate::followup!(as ctx => {
        let components = &buttons;
        let embeds = &[embed.build()];
        let flags = EPHEMERAL;
    })
    .await?;

    Ok(())
}

/// The send subcommand.
///
/// # Errors
///
/// This function will return an error if execution fails.
async fn send<'c>(ctx: CommandCtx<'c>, resolver: CommandOptionResolver<'c>) -> Result {
    let Some(guild_id) = ctx.data.guild_id else {
        bail!("command must be used in a guild");
    };
    let Some(channel_id) = ctx.event.channel.as_ref().map(|c| c.id) else {
        bail!("command must be used in a channel");
    };
    let Some(user) = ctx.event.author() else {
        bail!("command must be used by a user");
    };

    let locale = user.locale();
    let text = resolver.get_str("text")?;

    let Ok(selectors) = Selectors::read((guild_id, user.id)) else {
        let title = localize!(try locale => "text.{}.empty", Impl::NAME);
        let embed = EmbedBuilder::new().color(FAILURE).title(title);

        crate::followup!(as ctx => {
            let embeds = &[embed.build()];
            let flags = EPHEMERAL;
        })
        .await?;

        return Ok(());
    };

    let buttons = button_rows(selectors.data().buttons(false)?);
    let embed = EmbedBuilder::new().color(BRANDING).title(text);

    ctx.api
        .http()
        .create_message(channel_id)
        .embeds(&[embed.build()])?
        .components(&buttons)?
        .await?;

    selectors.key_owned().remove()?;

    let title = localize!(try locale => "text.{}.sent", Impl::NAME);
    let embed = EmbedBuilder::new().color(SUCCESS).title(title);

    crate::followup!(as ctx => {
        let embeds = &[embed.build()];
        let flags = EPHEMERAL;
    })
    .await?;

    Ok(())
}
