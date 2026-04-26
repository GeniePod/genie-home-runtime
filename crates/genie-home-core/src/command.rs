use crate::entity::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOrigin {
    Agent,
    Voice,
    Dashboard,
    Automation,
    Schedule,
    Bridge,
    LocalApi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeActionKind {
    TurnOn,
    TurnOff,
    Toggle,
    SetValue,
    Lock,
    Unlock,
    Open,
    Close,
    Start,
    Stop,
    Pause,
    ReturnToBase,
    Arm,
    Disarm,
    ActivateScene,
}

impl HomeActionKind {
    pub fn is_sensitive(&self) -> bool {
        matches!(
            self,
            Self::Unlock | Self::Open | Self::Start | Self::ReturnToBase | Self::Arm | Self::Disarm
        )
    }

    pub fn is_physical_mutation(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetSelector {
    pub entity_id: EntityId,
    pub confidence: f32,
}

impl TargetSelector {
    pub fn exact(entity_id: EntityId) -> Self {
        Self {
            entity_id,
            confidence: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAction {
    pub target: TargetSelector,
    pub kind: HomeActionKind,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeCommand {
    pub origin: CommandOrigin,
    pub action: HomeAction,
    pub confirmed: bool,
    pub reason: Option<String>,
}

impl HomeCommand {
    pub fn new(origin: CommandOrigin, action: HomeAction) -> Self {
        Self {
            origin,
            action,
            confirmed: false,
            reason: None,
        }
    }

    pub fn confirmed(mut self) -> Self {
        self.confirmed = true;
        self
    }
}
