use crate::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Config {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub entry: Anchor,
    pub timeout: i64,
    pub routes: Vec<Route>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Route {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub timeout: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Archive {
    pub route: Route,
    pub created: DateTime<Utc>,
    pub archived: DateTime<Utc>,
    pub events: BTreeMap<DateTime<Utc>, ArchiveEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ArchiveUser {
    pub id: UserId,
    pub tag: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ArchiveAttachment {
    pub id: AttachmentId,
    pub context: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ArchiveMessage {
    pub user: ArchiveUser,
    pub content: String,
    pub mentions: Vec<ArchiveUser>,
    pub attachments: Vec<ArchiveAttachment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum ArchiveEvent {
    UserJoin(UserId),
    UserLeave(UserId),
    Message(ArchiveMessage),
}

define_command!("mail" {
    description: "Manage the guild's mod-mail system",
    permissions: MANAGE_CHANNELS,
    allow_dms: false,
});
