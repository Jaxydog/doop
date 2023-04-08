use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::all::{
    ButtonStyle, ChannelId, ChannelType, CommandInteraction, ComponentInteraction, GuildId,
    Message, MessageId, ModalInteraction, UserId,
};
use serenity::builder::{
    CreateButton, CreateEmbed, CreateEmbedAuthor, CreateInteractionResponseFollowup, CreateMessage,
    GetMessages,
};
use serenity::prelude::CacheHttp;

use crate::cmd::CommandDataResolver;
use crate::common::{fetch_guild_channel, Anchor, CustomId, TimeString, TimeStringFlag};
use crate::util::data::{Data, DataId, MessagePack, Toml};
use crate::util::{Result, BOT_COLOR};
use crate::{command, data, err_wrap, option, warn};

const DESCRIPTION: &str = include_str!("./include/mail/description.txt");

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Global {
    pub guilds: Vec<GuildId>,
}

impl Global {
    pub fn id() -> DataId<Self, MessagePack> {
        // While we want this to be smaller than `Toml`, I'd rather be safe and not
        // compress it so it's relatively quick to open.
        data!(<_> MessagePack::Plain, "{NAME}/.global")
    }

    pub fn create() -> Data<Self, MessagePack> {
        Self::id().create(Self::default())
    }

    pub fn read() -> Result<Data<Self, MessagePack>> {
        Self::id().read()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Config {
    pub entrypoint: Anchor,
    pub category_id: ChannelId,
    pub timeout: i64,
}

impl Config {
    pub fn id(guild_id: GuildId) -> DataId<Self, Toml> {
        data!(<_> "{NAME}/{guild_id}/.config")
    }

    pub fn create(
        guild_id: GuildId,
        entrypoint: Anchor,
        category_id: ChannelId,
        timeout: i64,
    ) -> Data<Self, Toml> {
        Self::id(guild_id).create(Self { entrypoint, category_id, timeout })
    }

    pub fn read(guild_id: GuildId) -> Result<Data<Self, Toml>> {
        Self::id(guild_id).read()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct State {
    active: Vec<ActiveChannel>,
    archived: Vec<(UserId, ChannelId)>,
}

impl State {
    pub fn id(guild_id: GuildId) -> DataId<Self, MessagePack> {
        // While we want this to be smaller than `Toml`, I'd rather be safe and not
        // compress it so it's relatively quick to open.
        data!(<_> MessagePack::Plain, "{NAME}/{guild_id}/.state")
    }

    pub fn create(guild_id: GuildId) -> Data<Self, MessagePack> {
        Self::id(guild_id).create(Self::default())
    }

    pub fn read(guild_id: GuildId) -> Result<Data<Self, MessagePack>> {
        Self::id(guild_id).read()
    }

    pub fn get_archived(&self) -> &[(UserId, ChannelId)] {
        &self.archived
    }

    pub fn has(&self, user_id: UserId) -> bool {
        self.active.iter().any(|a| a.id.0 == user_id)
    }

    pub fn get(&self, user_id: UserId) -> Option<&ActiveChannel> {
        self.active.iter().find(|a| a.id.0 == user_id)
    }

    pub fn get_mut(&mut self, user_id: UserId) -> Option<&mut ActiveChannel> {
        self.active.iter_mut().find(|a| a.id.0 == user_id)
    }

    pub fn insert(&mut self, channel: ActiveChannel) -> Result<()> {
        if self.has(channel.id.0) {
            err_wrap!("user already has an existing channel")
        } else {
            self.active.push(channel);
            Ok(())
        }
    }

    pub async fn archive(
        &mut self,
        cache_http: &impl CacheHttp,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<()> {
        let mut indices = self.active.iter().enumerate();
        let index = indices.find_map(|(i, a)| (a.id.0 == user_id).then_some(i));

        let Some(index) = index else {
            return err_wrap!("user does not have an existing channel");
        };

        // This is fine because we just ensured the index is valid, therefore the
        // compiler *should* optimize the bounds check away.
        //
        // We don't just take the value itself because we need to remove it from the
        // vector after we ensure the archive is saved properly which will avoid cases
        // of possible data loss.
        let active = self.active[index];
        let archived = ArchivedChannel::create(cache_http, guild_id, active, Utc::now()).await?;

        archived.write()?;

        self.active.remove(index);
        self.archived.push(active.id);

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct ActiveChannel {
    pub id: (UserId, ChannelId),
    pub timeout: Option<i64>,
    pub counter: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ArchivedChannel {
    pub id: (UserId, ChannelId),
    pub closed_at: DateTime<Utc>,
    pub messages: Vec<ArchivedMessage>,
}

impl ArchivedChannel {
    pub const MAX_LOOP_ITER: usize = 200;

    pub fn id(
        guild_id: GuildId,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> DataId<Self, MessagePack> {
        // We'll be heavily compressing archives, since I imagine that they'll rarely be
        // opened up and there will probably be a lot of them. Considering how much data
        // they could have, I'd rather not have a lot of my storage used up by mail
        // archives. Every little bit helps.
        data!(
            MessagePack::Dense,
            "{NAME}/{guild_id}/archived/{user_id}_{channel_id}"
        )
    }

    pub async fn create(
        cache_http: &impl CacheHttp,
        guild_id: GuildId,
        active: ActiveChannel,
        closed_at: DateTime<Utc>,
    ) -> Result<Data<Self, MessagePack>> {
        let channel = fetch_guild_channel(cache_http, guild_id, active.id.1).await?;
        let mut messages = Vec::with_capacity(active.counter);
        let mut last = channel.last_message_id;
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
        while active.counter - messages.len() > 0 && iteration < Self::MAX_LOOP_ITER {
            let mut request = GetMessages::new().limit(100);

            if let Some(last) = last {
                request = request.before(last);
            }

            let mut fetched = channel.messages(cache_http, request).await?;

            fetched.sort_unstable_by_key(|m| m.id);
            last = fetched.last().map(|m| m.id);
            iteration += 1;

            for message in fetched.into_iter().map(ArchivedMessage::from) {
                messages.push(message);
            }
        }

        let archive = Self { id: active.id, closed_at, messages };

        Ok(Self::id(guild_id, active.id.0, active.id.1).create(archive))
    }

    pub fn read(
        guild_id: GuildId,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<Data<Self, MessagePack>> {
        Self::id(guild_id, user_id, channel_id).read()
    }
}

impl Display for ArchivedChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let closed = self.closed_at.timestamp_millis();
        let closed = TimeString::new_with_flag(closed, TimeStringFlag::DateTimeLong);
        let messages: Vec<_> = self.messages.iter().map(ToString::to_string).collect();

        writeln!(f, "<#{}> (opened by <@{}>)", self.id.1, self.id.0)?;
        writeln!(f, "closed at {closed}; {} messages\n", self.messages.len())?;
        writeln!(f, "---\n\n{}", messages.join("\n\n"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ArchivedMessage {
    pub author_id: UserId,
    pub message_id: MessageId,
    pub content: String,
    pub mentions: Vec<UserId>,
    pub has_attachments: bool,
    pub has_embeds: bool,
}

impl From<Message> for ArchivedMessage {
    fn from(value: Message) -> Self {
        Self {
            author_id: value.author.id,
            message_id: value.id,
            content: value.content,
            mentions: value.mentions.into_iter().map(|u| u.id).collect(),
            has_attachments: !value.attachments.is_empty(),
            has_embeds: !value.embeds.is_empty(),
        }
    }
}

impl Display for ArchivedMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let created = self.message_id.created_at().timestamp_millis();
        let created = TimeString::new_with_flag(created, TimeStringFlag::DateTimeLong);

        writeln!(f, "<@{}> - {created}\n>>> {}", self.author_id, self.content)?;

        if !self.mentions.is_empty() {
            let ids: Vec<_> = self.mentions.iter().map(|id| format!("<@{id}>")).collect();

            writeln!(f, "*Mentions: {}", ids.join(", "))?;
        }
        if self.has_attachments {
            write!(f, "+ attachments ")?;
        }
        if self.has_embeds {
            write!(f, "+ embeds ")?;
        }

        Ok(())
    }
}

fn get_entry_embed() -> CreateEmbed {
    CreateEmbed::new()
        .color(BOT_COLOR)
        .description(DESCRIPTION)
        .title("Create a ModMail Ticket")
}

fn get_entry_button() -> CreateButton {
    let id = CustomId::new(NAME.to_string(), "entry".to_string());

    CreateButton::new(id)
        .emoji('📨')
        .label("Create Channel")
        .style(ButtonStyle::Primary)
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

    let data = CommandDataResolver::new(command);

    if let Ok(data) = data.get_subcommand("configure") {
        return configure(cache_http, command, &data).await;
    }

    err_wrap!("unknown subcommand or subcommand group")
}

async fn configure<'cdr>(
    cache_http: &impl CacheHttp,
    command: &CommandInteraction,
    data: &'cdr CommandDataResolver<'cdr>,
) -> Result<()> {
    let timeout = data.get_i64("close_after").unwrap_or(60 * 24);
    let category = data.get_partial_channel("category")?;

    if category.kind != ChannelType::Category {
        return err_wrap!("invalid category channel");
    }
    let Some(guild_id) = command.guild_id else {
        return err_wrap!("command must be used in a guild");
    };

    let channel = fetch_guild_channel(cache_http, guild_id, command.channel_id).await?;
    let message = CreateMessage::new()
        .embed(get_entry_embed())
        .button(get_entry_button());
    let message = channel.send_message(cache_http, message).await?;
    let entrypoint = Anchor::new_guild(guild_id, channel.id, message.id);

    Config::create(guild_id, entrypoint, category.id, timeout).write()?;

    let bot = cache_http.http().get_current_user().await?;
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new(bot.tag()).icon_url(bot.face()))
        .color(bot.accent_colour.unwrap_or(BOT_COLOR))
        .title("ModMail has been configured!");
    let builder = CreateInteractionResponseFollowup::new()
        .embed(embed)
        .ephemeral(true);

    command.create_followup(cache_http, builder).await?;

    Ok(())
}

/// Handles component interactions
pub async fn handle_components(
    cache_http: &impl CacheHttp,
    component: &ComponentInteraction,
    custom_id: CustomId,
) -> Result<()> {
    component.defer_ephemeral(cache_http).await?;

    todo!()
}

/// Handles modal interactions
pub async fn handle_modals(
    cache_http: &impl CacheHttp,
    modal: &ModalInteraction,
    custom_id: CustomId,
) -> Result<()> {
    modal.defer_ephemeral(cache_http).await?;

    todo!()
}

/// Runs once per function loop tick
pub async fn on_loop(cache_http: &impl CacheHttp) -> Result<()> {
    // If the global config is not present, no guilds have been configured and it's
    // safe to return early.
    let Ok(global) = Global::read() else { return Ok(()) };

    for guild_id in &global.get().guilds {
        // While I'm tempted to remove invalid guild ids from the global config to
        // prevent dead guild instances, I don't want to risk the potentially
        // huge data loss that this can cause. I'd rather just deal with the extra bytes
        // and processing time that they'll take up.
        let Ok(config) = Config::read(*guild_id) else { continue };
        let Ok(mut state) = State::read(*guild_id) else { continue };

        let timeout = config.get().timeout * 1000;
        let mut to_archive = vec![];

        for &ActiveChannel { id: (user_id, channel_id), .. } in &state.get().active {
            // Once again, I really want to remove invalid channels but I don't have an easy
            // way of telling why the request failed at this level so I just need to deal
            // with the extra unnecessary processing time.
            let Ok(channel) = fetch_guild_channel(cache_http, *guild_id, channel_id).await else { continue };

            let last = if let Some(last) = channel.last_message_id {
                last.created_at().timestamp_millis()
            } else {
                // If the `last_message_id` value is `None`, try fetching the last message and
                // if that fails finally fall back to the channel creation date.
                let request = GetMessages::new().limit(1);
                let messages = channel.messages(cache_http, request).await.ok();
                let id = messages.and_then(|m| m.first().map(|m| m.id.created_at()));
                let last = id.unwrap_or_else(|| channel.id.created_at());

                last.timestamp_millis()
            };

            // We could store this at the top of the function to avoid repeating calls,
            // however calling during the check could lead to higher accuracy especially
            // with all of the `.await`s.
            if Utc::now().timestamp_millis() - last >= timeout {
                to_archive.push(user_id);
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
