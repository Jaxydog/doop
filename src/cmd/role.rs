use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serenity::all::{
    ButtonStyle, CommandInteraction, ComponentInteraction, GuildId, ReactionType, Role, RoleId,
    UserId,
};
use serenity::builder::{
    CreateButton, CreateEmbed, CreateInteractionResponseFollowup, CreateMessage,
};
use serenity::prelude::CacheHttp;

use crate::cmd::CommandDataResolver;
use crate::common::{fetch_guild_channel, CustomId};
use crate::util::data::{DataId, MessagePack, StoredData};
use crate::util::{Result, BOT_BRAND_COLOR, BOT_FAILURE_COLOR, BOT_SUCCESS_COLOR};
use crate::{command, data, err_wrap, option};

#[repr(transparent)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Selectors(BTreeMap<RoleId, (String, ReactionType)>);

impl Selectors {
    pub fn insert(&mut self, role: &Role, icon: &str) -> Result<Option<(String, ReactionType)>> {
        let Ok(icon) = ReactionType::try_from(icon) else {
            return err_wrap!("invalid icon provided; expected a single emoji, found '{icon}'");
        };

        Ok(self.0.insert(role.id, (role.name.clone(), icon)))
    }

    // we just use a reference anyways, so there's no point in having an extra copy
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn remove(&mut self, role_id: &RoleId) -> Option<(String, ReactionType)> {
        self.0.remove(role_id)
    }

    pub fn get_buttons(&self, disabled: bool) -> Result<Vec<CreateButton>> {
        let base_id = CustomId::new(NAME.to_string(), "toggle".to_string());
        let mut buttons = Vec::with_capacity(self.0.len().min(25));

        // Discord only permits up to 25 buttons per message, so we can have a maximum
        // of 25 selectors
        for (role_id, (name, icon)) in self.0.iter().take(25) {
            let mut custom_id = base_id.clone();

            custom_id.append(role_id.to_string())?;

            let button = CreateButton::new(custom_id)
                .disabled(disabled)
                .emoji(icon.clone())
                .label(name)
                .style(ButtonStyle::Secondary);

            buttons.push(button);
        }

        Ok(buttons)
    }
}

impl StoredData for Selectors {
    type Args = (GuildId, UserId);
    type Format = MessagePack;

    fn data_id((guild_id, user_id): Self::Args) -> DataId<Self, Self::Format> {
        data!(MessagePack::Dense, "{NAME}/{guild_id}/{user_id}")
    }
}

command!("role": {
    description: "Create or manage role selectors",
    permissions: MANAGE_ROLES,
    dms_allowed: false,
    options: [
        option!("add" <SubCommand>: {
            description: "Adds the given role selector to the current builder",
            options: [
                option!("role" <Role>: {
                    description: "The target role",
                    required: true,
                }),
                option!("icon" <String>: {
                    description: "The role's icon; should be a single emoji (unicode or otherwise)",
                    required: true,
                }),
            ],
        }),
        option!("remove" <SubCommand>: {
            description: "Removes the role from the list of selectors",
            options: [
                option!("role" <Role>: {
                    description: "The target role",
                    required: true,
                }),
            ],
        }),
        option!("list" <SubCommand>: {
            description: "Displays the list of added role selectors",
            options: [],
        }),
        option!("send" <SubCommand>: {
            description: "Sends the added role selectors",
            options: [
                option!("text" <String>: {
                    description: "The title of the role selector embed",
                    required: true,
                    where <str>: 1 ..= 256,
                }),
            ],
        }),
    ],
});

/// Handles command interactions
pub async fn handle_commands(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
) -> Result<()> {
    command.defer_ephemeral(cache_http).await?;

    let Some(guild_id) = command.guild_id else {
        return err_wrap!("this command must be used within a guild");
    };

    let data = CommandDataResolver::new(command);

    if let Ok(data) = data.get_subcommand("add") {
        return add(cache_http, command, &data, guild_id).await;
    }
    if let Ok(data) = data.get_subcommand("remove") {
        return remove(cache_http, command, &data, guild_id).await;
    }
    if let Ok(data) = data.get_subcommand("list") {
        return list(cache_http, command, &data, guild_id).await;
    }
    if let Ok(data) = data.get_subcommand("send") {
        return send(cache_http, command, &data, guild_id).await;
    }

    err_wrap!("unknown command")
}

async fn add<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    let role = data.get_role("role")?;
    let icon = data.get_str("icon")?;
    let mut selectors = Selectors::data_default((guild_id, command.user.id));

    selectors.get_mut().insert(role, icon)?;
    selectors.write()?;

    let embed = CreateEmbed::new()
        .color(BOT_SUCCESS_COLOR)
        .title(format!("Selector '{}' added!", role.name));
    let response = CreateInteractionResponseFollowup::new().embed(embed);

    command.create_followup(cache_http, response).await?;
    Ok(())
}

async fn remove<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    let role = data.get_role("role")?;
    let mut selectors = Selectors::data_default((guild_id, command.user.id));

    let embed = if let Some((name, _)) = selectors.get_mut().remove(&role.id) {
        CreateEmbed::new()
            .color(BOT_SUCCESS_COLOR)
            .title(format!("Selector '{name}' removed!"))
    } else {
        CreateEmbed::new()
            .color(BOT_FAILURE_COLOR)
            .title(format!("Selector '{}' does not exist!", role.name))
    };

    let response = CreateInteractionResponseFollowup::new().embed(embed);

    command.create_followup(cache_http, response).await?;
    Ok(())
}

async fn list<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    _: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    let selectors = Selectors::data_default((guild_id, command.user.id));
    let buttons = selectors.get().get_buttons(true)?;
    let embed = CreateEmbed::new().color(BOT_BRAND_COLOR).title("Selectors");
    let mut response = CreateInteractionResponseFollowup::new().embed(embed);

    for button in buttons {
        response = response.button(button);
    }

    command.create_followup(cache_http, response).await?;
    Ok(())
}

async fn send<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    let text = data.get_str("text")?;
    let Ok(selectors) = Selectors::data_read((guild_id, command.user.id)) else {
        return err_wrap!("role selectors have not been added");
    };
    let buttons = selectors.get().get_buttons(false)?;

    let channel = fetch_guild_channel(cache_http, guild_id, command.channel_id).await?;
    let embed = CreateEmbed::new().color(BOT_BRAND_COLOR).title(text);
    let mut message = CreateMessage::new().embed(embed);

    for button in buttons {
        message = message.button(button);
    }

    channel.send_message(cache_http, message).await?;
    selectors.remove()?;

    let embed = CreateEmbed::new()
        .color(BOT_SUCCESS_COLOR)
        .title("Sent role selectors!");
    let response = CreateInteractionResponseFollowup::new().embed(embed);

    command.create_followup(cache_http, response).await?;
    Ok(())
}

/// Handles component interactions
pub async fn handle_components(
    cache_http: &impl CacheHttp,
    component: &mut ComponentInteraction,
    custom_id: CustomId,
) -> Result<()> {
    component.defer_ephemeral(cache_http).await?;

    if custom_id.name.as_str() == "toggle" {
        return toggle(cache_http, component, custom_id).await;
    }

    err_wrap!("unknown component")
}

async fn toggle(
    cache_http: &impl CacheHttp,
    component: &mut ComponentInteraction,
    custom_id: CustomId,
) -> Result<()> {
    let Some(member) = component.member.as_mut() else {
        return err_wrap!("this component must be used within a guild");
    };
    let Some(role_id) = custom_id.data.first() else {
        return err_wrap!("missing role identifier");
    };
    let Ok(role_id) = role_id.parse() else {
        return err_wrap!("invalid role identifier: '{role_id}'");
    };
    let role_id = if role_id == 0 {
        return err_wrap!("invalid role identifier (should be non-zero)");
    } else {
        RoleId::new(role_id)
    };

    let mut embed = CreateEmbed::new().color(BOT_SUCCESS_COLOR);

    if member.roles.contains(&role_id) {
        member.remove_role(cache_http, role_id).await?;
        embed = embed.title("Removed role!");
    } else {
        member.add_role(cache_http, role_id).await?;
        embed = embed.title("Added role!");
    }

    let response = CreateInteractionResponseFollowup::new().embed(embed);

    component.create_followup(cache_http, response).await?;
    Ok(())
}
