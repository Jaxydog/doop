use std::fmt::Write;

use anyhow::bail;
use doop_localizer::{localize, Locale};
use doop_macros::Storage;
use doop_storage::{Compress, MsgPack};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::OffsetDateTime;
use twilight_model::channel::message::component::{ActionRow, ButtonStyle, TextInputStyle};
use twilight_model::channel::message::embed::EmbedAuthor;
use twilight_model::channel::message::{Component, Embed, ReactionType};
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};

use crate::bot::client::ApiRef;
use crate::cmd::membership::ENTRY_TOASTS;
use crate::cmd::CommandEntry;
use crate::util::builder::{ButtonBuilder, Modal, ModalBuilder, TextInputBuilder};
use crate::util::extension::{EmbedAuthorExtension, ReactionTypeExtension, UserExtension};
use crate::util::traits::{IntoImageSource, Localized, PreferLocale};
use crate::util::{Anchor, DataId, Result, BRANDING};

/// A user's membership application submission.
#[allow(clippy::unsafe_derive_deserialize)] // does not affect ser/de.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Storage)]
#[format(Compress<MsgPack, 6>)]
// #[format(Compress<doop_storage::Toml, 6>)]
#[location("{}/{}/{}", &'static str, Id<GuildMarker>, Id<UserMarker>)]
pub struct Submission {
    /// The applicant's identifier.
    pub id: Id<UserMarker>,
    /// The target guild's identifier.
    pub guild_id: Id<GuildMarker>,
    /// The creation date and time of the submission.
    pub timestamp: OffsetDateTime,
    /// The application's current status.
    pub status: Status,
    /// The applicant's questions and answers.
    #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
    pub answers: Box<[(Box<str>, Box<str>)]>,
    /// The message containing the embedded submission form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    /// The user's past submissions.
    #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
    pub archives: Box<[SubmissionArchive]>,
}

impl Submission {
    /// Updates the status of the submission.
    #[allow(unsafe_code)]
    pub fn update_status(&mut self, kind: StatusKind, mut update: StatusUpdate) {
        update.previous = Some(Box::new(self.status.clone()));
        self.status = Status { kind, update: Some(update) };
    }

    /// Informs the user of a change to their application, returning whether a message was sent.
    ///
    /// # Errors
    ///
    /// This function will return an error if the user could not be notified.
    pub async fn inform(
        &self,
        api: ApiRef<'_>,
        title: impl Into<String> + Send + Sync,
        description: impl Into<String> + Send + Sync,
    ) -> Result<bool> {
        let guild = api.http.guild(self.guild_id).await?.model().await?;

        let Ok(channel) = api.http.create_private_channel(self.id).await else {
            return Ok(false);
        };
        let channel = channel.model().await?;

        let embed = EmbedBuilder::new()
            .author(EmbedAuthor::parse(&guild)?)
            .color(BRANDING)
            .description(description)
            .title(title);

        api.http.create_message(channel.id).embeds(&[embed.build()])?.await?;

        Ok(true)
    }

    /// Builds the submission form.
    ///
    /// # Errors
    ///
    /// This function will return an error if the form could not be constructed.
    pub async fn build_form(
        &self,
        entry: &CommandEntry,
        api: ApiRef<'_>,
    ) -> Result<(Embed, Vec<Component>)> {
        let guild = api.http.guild(self.guild_id).await?.model().await?;
        let user = api.http.user(self.id).await?.model().await?;

        let locale = guild.preferred_locale();
        let mut text = String::new();

        /// Appends a label to the `text` buffer.
        macro_rules! label {
            ($key:literal, $value:expr) => {
                text.write_fmt(format_args!("**{}:** {}\n", localize!(try in locale, "text.{}.form.{}", entry.name, $key), $value))
            };
        }

        label!("profile", format!("<@{}>", self.id))?;
        label!("created", format!("<t:{}:R>", self.timestamp.unix_timestamp()))?;
        label!("status", self.status.kind.localize_in(locale, *entry))?;
        label!("entries", self.archives.len() + 1)?;

        if let Some(ref data) = self.status.update {
            text.push('\n');

            label!("author", format!("<@{}>", data.author_id))?;
            label!("updated", format!("<t:{}:R>", data.timestamp.unix_timestamp()))?;

            if let Some(ref comment) = data.comment {
                label!("comment", comment)?;
            }
            if let Some(ref reason) = data.reason {
                label!("reason", reason)?;
            }
        }

        let index = thread_rng().gen_range(0 .. ENTRY_TOASTS);
        let mut embed = EmbedBuilder::new()
            .author(EmbedAuthor::parse(&user)?)
            .color(user.color())
            .description(text)
            .thumbnail((&user).into_image_source()?)
            .title(localize!(try in locale, "text.{}.title_{index}", entry.name));

        for (question, answer) in self.answers.iter() {
            let field = EmbedFieldBuilder::new(&(**question), format!("> {answer}"));

            embed = embed.field(field.build());
        }

        let id = DataId::new(entry.name, "update").with(self.id.to_string());

        /// Creates a new update button.
        macro_rules! update_button {
            ($style:ident, $status:expr, $icon:literal) => {
                ButtonBuilder::new(ButtonStyle::$style)
                    .custom_id(id.clone().with(($status as u8).to_string()))
                    .disabled(self.status.kind != StatusKind::Pending)
                    .emoji(ReactionType::parse($icon)?)
                    .label(localize!(try in locale, "button.{}.update.{}_label", entry.name, $status.localization_key()))
                    .into()
            };
        }

        let accept = update_button!(Success, StatusKind::Accepted, '👍');
        let reject = update_button!(Danger, StatusKind::Rejected, '👎');
        let revise = update_button!(Primary, StatusKind::Resubmit, '🤷');
        let update_row = ActionRow { components: vec![accept, reject, revise] };

        let updates = ButtonBuilder::new(ButtonStyle::Secondary)
            .custom_id(DataId::new(entry.name, "updates").with(self.id.to_string()))
            .emoji(ReactionType::parse('📚')?)
            .label(localize!(try in locale, "button.{}.updates.label", entry.name))
            .into();
        let entries = ButtonBuilder::new(ButtonStyle::Secondary)
            .custom_id(DataId::new(entry.name, "entries").with(self.id.to_string()))
            .emoji(ReactionType::parse('📖')?)
            .label(localize!(try in locale, "button.{}.entries.label", entry.name))
            .into();
        let history_row = ActionRow { components: vec![updates, entries] };

        Ok((embed.build(), vec![update_row.into(), history_row.into()]))
    }

    /// Builds the submission's update modal.
    ///
    /// # Errors
    ///
    /// This function will return an error if the modal could not be constructed.
    pub async fn build_update_modal(
        &self,
        entry: &CommandEntry,
        api: ApiRef<'_>,
        status: StatusKind,
    ) -> Result<Modal> {
        let user = api.http.user(self.id).await?.model().await?;
        let locale = user.preferred_locale();

        let title = localize!(try in locale, "modal.{}.update.title_{}", entry.name, status.localization_key());
        let comment = localize!(try in locale, "modal.{}.update.comment_label", entry.name);
        let reason = localize!(try in locale, "modal.{}.update.reason_label", entry.name);
        let custom_id = DataId::new(entry.name, "update")
            .with(self.id.to_string())
            .with((status as u8).to_string());

        let comment = TextInputBuilder::new("comment", comment, TextInputStyle::Short)
            .max_length(256)
            .required(false);
        let reason = TextInputBuilder::new("reason", reason, TextInputStyle::Short)
            .max_length(256)
            .required(false);

        let mut modal = ModalBuilder::new(custom_id, title);

        modal.push(comment)?;
        modal.push(reason)?;

        Ok(modal.build())
    }
}

/// A user's past membership application submission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmissionArchive {
    /// The creation date and time of the submission.
    pub timestamp: OffsetDateTime,
    /// The application's current status.
    pub status: Status,
    /// The applicant's questions and answers.
    pub answers: Box<[(Box<str>, Box<str>)]>,
}

/// A member's application status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// The current status type.
    pub kind: StatusKind,
    /// The current status' update data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<StatusUpdate>,
}

/// A member application's status type.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
pub enum StatusKind {
    /// The application is pending.
    #[default]
    Pending = 0,
    /// The application was accepted.
    Accepted = 1,
    /// The application was rejected.
    Rejected = 2,
    /// The application should be revised and resubmitted.
    Resubmit = 3,
    /// The application was discarded.
    Discarded = 4,
}

impl StatusKind {
    /// Returns this status type's localization key.
    pub const fn localization_key(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accept",
            Self::Rejected => "reject",
            Self::Resubmit => "revise",
            Self::Discarded => "discard",
        }
    }
}

impl Localized for StatusKind {
    type Arguments = CommandEntry;

    fn localize_in(&self, locale: Locale, entry: Self::Arguments) -> std::borrow::Cow<str> {
        localize!(try in locale, "text.{}.status.{}", entry.name, self.localization_key())
    }
}

impl TryFrom<i64> for StatusKind {
    type Error = anyhow::Error;

    #[allow(unsafe_code, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            // Safety: here, `n` is always 0..=4, and all variants are 0..=4.
            n @ 0 ..= 4 => Ok(unsafe { std::mem::transmute(n as u8) }),
            n => bail!("invalid status identifier: '{n}'"),
        }
    }
}

/// An update to a member's application status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusUpdate {
    /// The user identifier of the moderator that updated the status.
    pub author_id: Id<UserMarker>,
    /// The date and time that the update occurred.
    pub timestamp: OffsetDateTime,
    /// The reason that the application was updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Box<str>>,
    /// The comment displayed to the applicant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<Box<str>>,
    /// The previous application status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<Box<Status>>,
}
