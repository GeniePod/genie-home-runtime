use crate::command::HomeAction;
use crate::entity::{EntityId, EntityState};
use crate::safety::SafetyDecision;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Automation {
    pub id: String,
    pub display_name: String,
    pub enabled: bool,
    pub trigger: AutomationTrigger,
    pub conditions: Vec<AutomationCondition>,
    pub actions: Vec<HomeAction>,
}

impl Automation {
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        trigger: AutomationTrigger,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            enabled: true,
            trigger,
            conditions: Vec::new(),
            actions: Vec::new(),
        }
    }

    pub fn with_condition(mut self, condition: AutomationCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn with_action(mut self, action: HomeAction) -> Self {
        self.actions.push(action);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationTrigger {
    TimeOfDay { hh_mm: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationCondition {
    EntityStateIs {
        entity_id: EntityId,
        state: EntityState,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationTickResult {
    pub now_hh_mm: String,
    pub automations_checked: usize,
    pub automations_triggered: usize,
    pub actions_executed: usize,
    pub blocked: Vec<AutomationBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationBlock {
    pub automation_id: String,
    pub decision: SafetyDecision,
}
