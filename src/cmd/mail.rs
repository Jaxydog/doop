use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::all::{
    ButtonStyle, ChannelId, ChannelType, CommandInteraction, GuildId, Message, MessageId, UserId,
};
use serenity::builder::{
    CreateButton, CreateEmbed, CreateInteractionResponseFollowup, CreateMessage, GetMessages,
};
use serenity::prelude::CacheHttp;

use crate::cmd::CommandDataResolver;
use crate::common::{fetch_guild_channel, Anchor, CustomId, TimeString, TimeStringFlag};
use crate::util::data::{DataId, MessagePack, StoredData, Toml};
use crate::util::{Result, BOT_BRAND_COLOR, BOT_SUCCESS_COLOR};
use crate::{command, data, err_wrap, option, warn};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Global {
    pub guilds: Vec<GuildId>,
}

impl StoredData for Global {
    type Args = ();
    type Format = Toml;

    fn data_id(_: Self::Args) -> DataId<Self, Self::Format> {
        data!(Toml, "{NAME}/.global")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Config {
    pub anchor: Anchor,
    pub category_id: ChannelId,
    pub timeout: i64,
}

impl StoredData for Config {
    type Args = GuildId;
    type Format = Toml;

    fn data_id(guild_id: Self::Args) -> DataId<Self, Self::Format> {
        data!(Toml, "{NAME}/{guild_id}/.config")
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct State {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub archives: Vec<(UserId, ChannelId)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    channels: Vec<MailChannel>,
}

impl State {
    pub fn has(&self, user_id: UserId) -> bool {
        self.channels.iter().any(|c| c.user_id == user_id)
    }

    pub fn get(&self, user_id: UserId) -> Option<&MailChannel> {
        self.channels.iter().find(|c| c.user_id == user_id)
    }

    pub fn get_mut(&mut self, user_id: UserId) -> Option<&mut MailChannel> {
        self.channels.iter_mut().find(|c| c.user_id == user_id)
    }

    pub fn take(&mut self, user_id: UserId) -> Option<MailChannel> {
        self.channels
            .iter()
            .enumerate()
            .find_map(|(i, c)| (c.user_id == user_id).then_some(i))
            .map(|i| self.channels.remove(i))
    }

    pub fn insert(&mut self, channel: MailChannel) -> Result<()> {
        if self.has(channel.user_id) {
            return err_wrap!("user already has an existing channel");
        }

        self.channels.push(channel);
        Ok(())
    }

    pub async fn create(
        &mut self,
        config: &Config,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<ChannelId> {
        todo!()
    }

    pub async fn archive(
        &mut self,
        cache_http: &impl CacheHttp,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<()> {
        let mut indices = self.channels.iter().enumerate();
        let index = indices.find_map(|(i, a)| (a.user_id == user_id).then_some(i));

        let Some(index) = index else {
            return err_wrap!("user does not have an existing channel");
        };

        // This is fine because we just ensured the index is valid, therefore the
        // compiler *should* optimize the bounds check away.
        //
        // We don't just take the value itself because we need to remove it from the
        // vector after we ensure the archive is saved properly which will avoid cases
        // of possible data loss.
        let channel = &self.channels[index];
        let channel_id = channel.channel_id;
        let archive = MailArchive::new(cache_http, guild_id, channel, Utc::now()).await?;
        let archive = MailArchive::data_create((guild_id, user_id, channel_id), archive);

        archive.write()?;
        self.archives.push((user_id, channel_id));
        self.channels.remove(index);

        Ok(())
    }
}

impl StoredData for State {
    type Args = GuildId;
    type Format = MessagePack;

    fn data_id(guild_id: Self::Args) -> DataId<Self, Self::Format> {
        data!(MessagePack::Standard, "{NAME}/{guild_id}/.state")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MailChannel {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub timeout: Option<i64>,
    pub messages: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MailArchive {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub closed_at: DateTime<Utc>,
    pub messages: Vec<MailArchiveMessage>,
}

impl MailArchive {
    pub const MAX_FETCH_ITER: usize = 200;

    pub async fn new(
        cache_http: &impl CacheHttp,
        guild_id: GuildId,
        mail_channel: &MailChannel,
        closed_at: DateTime<Utc>,
    ) -> Result<Self> {
        let MailChannel { user_id, channel_id, messages, .. } = *mail_channel;
        let channel = fetch_guild_channel(cache_http, guild_id, channel_id).await?;
        let mut messages = Vec::with_capacity(messages);
        let mut last_id = channel.last_message_id;
        let mut iteration = 0;

        // We may only fetch 100 messages at a time form the API, so we need to make a
        // loop like this to fetch over and over. I'm hoping that Serenity has a good
        // rate-limit implementation so that we aren't just spamming their API but I'm
        // not in the mood to check so let's just pray and hope it goes well.
        //
        // Also added a limit to how many times the loop can run to prevent any possible
        // cases where our internal message counter is incorrect from stalling the
        // program forever. This has the added benefit of only allowing `100 *
        // MAX_LOOP_ITER` total messages per archive, which in this case is 2000 total.
        // I find it hard to believe that a modmail channel would have even close to
        // that many, but I suppose it could happen.
        while mail_channel.messages - messages.len() > 0 && iteration < Self::MAX_FETCH_ITER {
            let mut request = GetMessages::new().limit(100);

            if let Some(last_id) = last_id {
                request = request.before(last_id);
            }

            let mut fetched = channel.messages(cache_http, request).await?;

            fetched.sort_unstable_by_key(|m| m.id);
            last_id = fetched.last().map(|m| m.id);
            iteration += 1;

            for message in fetched.iter().map(MailArchiveMessage::from) {
                messages.push(message);
            }
        }

        Ok(Self { user_id, channel_id, closed_at, messages })
    }
}

impl StoredData for MailArchive {
    type Args = (GuildId, UserId, ChannelId);
    type Format = MessagePack;

    fn data_id((guild, user, channel): Self::Args) -> DataId<Self, Self::Format> {
        // We'll be heavily compressing archives, since I imagine that they'll rarely be
        // opened up and there will probably be a lot of them. Considering how much data
        // they could have, I'd rather not have a lot of my storage used up by mail
        // archives. Every little bit helps.
        data!(MessagePack::Dense, "{NAME}/{guild}/{user}_{channel}")
    }
}

impl Display for MailArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { user_id, channel_id, closed_at, messages } = &self;
        let closed = closed_at.timestamp_millis();
        let closed = TimeString::new_with_flag(closed, TimeStringFlag::DateTimeLong);
        let messages: Vec<_> = messages.iter().map(ToString::to_string).collect();

        writeln!(f, "<#{channel_id}> (opened by <@{user_id}>)")?;
        writeln!(f, "Closed: {closed}")?;
        writeln!(f, "Messages: {}", messages.len())?;
        writeln!(f, "\n---\n\n{}", messages.join("\n\n"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MailArchiveMessage {
    pub author_id: UserId,
    pub message_id: MessageId,
    pub content: String,
    pub mentions: Vec<UserId>,
    pub has_attachments: bool,
    pub has_embeds: bool,
}

impl From<&Message> for MailArchiveMessage {
    fn from(value: &Message) -> Self {
        Self {
            author_id: value.author.id,
            message_id: value.id,
            content: value.content.clone(),
            mentions: value.mentions.iter().map(|u| u.id).collect(),
            has_attachments: !value.attachments.is_empty(),
            has_embeds: !value.embeds.is_empty(),
        }
    }
}

impl Display for MailArchiveMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { author_id, message_id, content, mentions, .. } = &self;
        let created = message_id.created_at().timestamp_millis();
        let created = TimeString::new_with_flag(created, TimeStringFlag::DateTimeLong);

        writeln!(f, "<@{author_id}> - {created}\n>>> {content}")?;

        if !mentions.is_empty() {
            let ids: Vec<_> = mentions.iter().map(|i| format!("<@{i}>")).collect();

            writeln!(f, "**Mentions:** {}", ids.join(", "))?;
        }
        if self.has_attachments {
            writeln!(f, "*+ attachments*")?;
        }
        if self.has_embeds {
            writeln!(f, "*+ embeds*")?;
        }

        Ok(())
    }
}

command!("mail": {
    description: "Manage the server's ModMail system",
    permissions: MODERATE_MEMBERS,
    dms_allowed: false,
    options: [
        option!("configure" <SubCommand>: {
            description: "Configures ModMail for your server",
            options: [
                option!("category" <Channel>: {
                    description: "The category that mail channels are created in",
                    required: true,
                    channels: [Category],
                }),
                option!("close_after" <Integer>: {
                    description: "How long a mail channel should be inactive before automatically closing; defaults to 1 day.",
                    required: false,
                    match <i32> {
                        "Never" => -1,
                        "10 minutes" => 10,
                        "30 minutes" => 30,
                        "1 hour" => 60,
                        "3 hours" => 60 * 3,
                        "6 hours" => 60 * 6,
                        "12 hours" => 60 * 12,
                        "1 day" => 60 * 24,
                        "3 days" => 60 * 24 * 3,
                        "7 days" => 60 * 24 * 7,
                    },
                }),
            ],
        }),
        option!("channel" <SubCommandGroup>: {
            description: "Manages ModMail channels",
            options: [
                option!("open" <SubCommand>: {
                    description: "Opens a new ModMail channel",
                    options: [],
                }),
                option!("close" <SubCommand>: {
                    description: "Closes the current ModMail channel",
                    options: [
                        option!("discard" <Boolean>: {
                            description: "If this is True, the channel will not be archived",
                            required: false,
                        }),
                    ],
                }),
                option!("add" <SubCommand>: {
                    description: "Adds the specified user to the current ModMail channel",
                    options: [
                        option!("user" <User>: {
                            description: "The user to add",
                            required: true,
                        }),
                    ],
                }),
                option!("remove" <SubCommand>: {
                    description: "Removes the specified user from the current ModMail channel",
                    options: [
                        option!("user" <User>: {
                            description: "The user to remove",
                            required: true,
                        }),
                    ],
                }),
                option!("close-after" <SubCommand>: {
                    description: "Overrides the automatic close timer for the current ModMail channel",
                    options: [
                        option!("minutes" <Integer>: {
                            description: "The duration in minutes",
                            required: true,
                            where <i32>: 1 ..= 43_200, // 60 * 24 * 30, 1 month
                        }),
                    ],
                }),
            ],
        }),
        option!("archives" <SubCommand>: {
            description: "Opens the ModMail archive browser",
            options: [
                option!("user" <User>: {
                    description: "An option user used to to filter archives",
                    required: false,
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

    if let Ok(data) = data.get_subcommand("configure") {
        return configure(cache_http, command, &data, guild_id).await;
    }

    err_wrap!("unknown subcommand or subcommand group")
}

async fn configure<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    let timeout = data.get_i64("close_after").unwrap_or(60 * 24);
    let category = data.get_partial_channel("category")?;

    if category.kind != ChannelType::Category {
        return err_wrap!("invalid channel type; expected category");
    }

    let channel = fetch_guild_channel(cache_http, guild_id, command.channel_id).await?;

    let embed = CreateEmbed::new()
        .color(BOT_BRAND_COLOR)
        .description(include_str!("./include/mail/description.txt"))
        .title("Create a ModMail Channel");
    let entry_button = CreateButton::new(CustomId::new(NAME.to_string(), "entry".to_string()))
        .emoji('📨')
        .label("Create Channel")
        .style(ButtonStyle::Primary);
    let about_button = CreateButton::new(CustomId::new(NAME.to_string(), "about".to_string()))
        .emoji('🤔')
        .label("About ModMail")
        .style(ButtonStyle::Secondary);
    let message = CreateMessage::new()
        .embed(embed)
        .button(entry_button)
        .button(about_button);

    if let Ok(config) = Config::data_read(guild_id) {
        let message = config.get().anchor.to_message(cache_http).await?;

        // We don't actually care if this fails; worst-case scenario, the admins need to
        // delete it manually. We just try to be convenient by doing so automatically
        message.delete(cache_http).await.ok();
    }

    let message = channel.send_message(cache_http, message).await?;
    let anchor = Anchor::new_guild(guild_id, channel.id, message.id);
    let config = Config { anchor, category_id: category.id, timeout };

    Config::data_create(guild_id, config).write()?;

    let embed = CreateEmbed::new()
        .color(BOT_SUCCESS_COLOR)
        .title("Configured ModMail!");
    let response = CreateInteractionResponseFollowup::new().embed(embed);

    command.create_followup(cache_http, response).await?;
    Ok(())
}

async fn channel_open<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    todo!()
}

async fn channel_close<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    todo!()
}

async fn channel_add<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    todo!()
}

async fn channel_remove<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    todo!()
}

async fn channel_close_after<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    todo!()
}

async fn archives<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
    guild_id: GuildId,
) -> Result<()> {
    todo!()
}

/// Runs once per function loop tick
pub async fn on_loop(cache_http: &impl CacheHttp) -> Result<()> {
    let global = Global::data_default(());

    for guild_id in &global.get().guilds {
        // While I'm tempted to remove invalid guild ids from the global config to
        // prevent dead guild instances, I don't want to risk the potentially
        // huge data loss that this can cause. I'd rather just deal with the extra bytes
        // and processing time that they'll take up.
        let Ok(config) = Config::data_read(*guild_id) else { continue };
        let Ok(mut state) = State::data_read(*guild_id) else { continue };

        let timeout_ms = config.get().timeout * 60 * 1000;
        let mut to_archive = vec![];

        for MailChannel { user_id, channel_id, .. } in &state.get().channels {
            // Once again, I really want to remove invalid channels but I don't have an easy
            // way of telling why the request failed at this level so I just need to deal
            // with the extra unnecessary processing time.
            let Ok(channel) = fetch_guild_channel(cache_http, *guild_id, *channel_id).await else { continue };

            // If the `last_message_id` is `None`, fall back to the channel creation date.
            let last_ms = if let Some(last_id) = channel.last_message_id {
                last_id.created_at().timestamp_millis()
            } else {
                channel.id.created_at().timestamp_millis()
            };

            if Utc::now().timestamp_millis() - last_ms >= timeout_ms {
                to_archive.push(*user_id);
            }
        }

        // Archiving is handled separately from the check to avoid a reference conflict
        let state = state.get_mut();

        for user_id in to_archive {
            let result = state.archive(cache_http, *guild_id, user_id).await;

            if let Err(error) = result {
                warn!("error occurred during mail archive: {error}");
            }
        }
    }

    Ok(())
}
