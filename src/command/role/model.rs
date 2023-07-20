use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use twilight_model::channel::message::component::{Button, ButtonStyle};
use twilight_model::channel::message::ReactionType;
use twilight_model::id::marker::{GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;

use super::This;
use crate::extend::{IteratorExt, ReactionTypeExt};
use crate::storage::format::{MessagePack, Zip};
use crate::storage::{Info, Storable};
use crate::traits::BuildButtons;
use crate::utility::{ButtonBuilder, DataId, Result};

/// Stores the user's added role selectors.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Selectors(BTreeMap<Id<RoleMarker>, (Box<str>, Box<str>)>);

impl Deref for Selectors {
    type Target = BTreeMap<Id<RoleMarker>, (Box<str>, Box<str>)>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Selectors {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Storable for Selectors {
    type Arguments = (Id<GuildMarker>, Id<UserMarker>);
    type Format = Zip<MessagePack>;

    fn info((guild_id, user_id): Self::Arguments) -> Info<Self, Self::Format> {
        Info::new(format!("{}/{guild_id}/{user_id}", This::NAME))
    }
}

impl BuildButtons for Selectors {
    fn build_buttons(&self, disabled: bool, _: ()) -> Result<Vec<Button>> {
        let base_id = DataId::new_empty(This::NAME, Some("toggle"))?;
        let buttons = self.iter().take(25).try_filter_map(|(id, (name, icon))| {
            let button = ButtonBuilder::new(ButtonStyle::Secondary)
                .custom_id(base_id.clone().join(id.to_string())?)
                .disabled(disabled)
                .emoji(ReactionType::parse(&(**icon))?)
                .label(&(**name));

            Ok::<_, anyhow::Error>(button.build())
        });

        Ok(buttons.collect())
    }
}
