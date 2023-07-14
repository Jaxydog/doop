use serde::{Deserialize, Serialize};
use twilight_model::channel::message::Component;

/// Represents a Discord modal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modal {
    /// The modal's component list.
    pub components: Vec<Component>,
    /// The modal's custom identifier
    pub custom_id: Box<str>,
    /// The modal's title.
    pub title: Box<str>,
}
