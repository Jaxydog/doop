use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle};
use twilight_model::channel::message::{Component, ReactionType};
use twilight_model::id::marker::{GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;

use super::This;
use crate::extend::{IteratorExt, ReactionTypeExt};
use crate::storage::format::{MessagePack, Zip};
use crate::storage::{Info, Storable};
use crate::traits::SyncComponentBuilder;
use crate::utility::{DataId, Result};

/// Stores the user's added role selectors
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Selectors(BTreeMap<Id<RoleMarker>, (Box<str>, Box<str>)>);

impl Selectors {
    /// Converts a map entry into a button.
    pub fn entry_to_button(
        base_id: &DataId,
        disabled: bool,
        (id, (name, icon)): (&Id<RoleMarker>, &(Box<str>, Box<str>)),
    ) -> Result<Button> {
        Ok::<_, anyhow::Error>(Button {
            custom_id: Some(base_id.clone().join(id.to_string())?.to_string()),
            disabled,
            emoji: Some(ReactionType::parse(icon.to_string())?),
            label: Some(name.to_string()),
            style: ButtonStyle::Secondary,
            url: None,
        })
    }
}

impl Deref for Selectors {
    type Target = BTreeMap<Id<RoleMarker>, (Box<str>, Box<str>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Selectors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Storable for Selectors {
    type Arguments = (Id<GuildMarker>, Id<UserMarker>);
    type Format = Zip<MessagePack>;

    fn saved((guild_id, user_id): Self::Arguments) -> Info<Self, Self::Format> {
        Info::new(format!("{}/{guild_id}/{user_id}", This::NAME))
    }
}

impl SyncComponentBuilder for Selectors {
    type Arguments = bool;

    fn build_components(&self, disabled: Self::Arguments) -> Result<Vec<Component>> {
        let base_id = DataId::new_empty(This::NAME, Some("toggle"))?;
        let f = |entry| Self::entry_to_button(&base_id, disabled, entry);
        let buttons = self.iter().take(25).try_filter_map(f);

        let capacity = self.len() / 5 + usize::from(self.len() % 5 != 0);
        let mut list = Vec::with_capacity(capacity);
        let mut components = Vec::with_capacity(5);

        for button in buttons {
            if components.len() < 5 {
                components.push(Component::Button(button));
            } else {
                list.push(Component::ActionRow(ActionRow { components }));
                components = Vec::with_capacity(5);
            }
        }

        if !components.is_empty() {
            list.push(Component::ActionRow(ActionRow { components }));
        }

        Ok(list)
    }
}
