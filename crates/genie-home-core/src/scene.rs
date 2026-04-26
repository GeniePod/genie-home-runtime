use crate::command::HomeAction;
use crate::entity::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub id: EntityId,
    pub display_name: String,
    pub actions: Vec<HomeAction>,
}

impl Scene {
    pub fn new(id: EntityId, display_name: impl Into<String>) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            actions: Vec::new(),
        }
    }

    pub fn with_action(mut self, action: HomeAction) -> Self {
        self.actions.push(action);
        self
    }
}
