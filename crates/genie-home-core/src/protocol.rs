use crate::{
    AuditEntry, Automation, AutomationTickResult, ConnectivityApplyResult, ConnectivityReport,
    Entity, HomeCommand, RuntimeStatus, SafetyDecision, Scene,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeRequest {
    Status,
    ListEntities,
    ListAutomations,
    Audit { limit: Option<usize> },
    ListScenes,
    Evaluate { command: HomeCommand },
    Execute { command: HomeCommand },
    ApplyConnectivityReport { report: ConnectivityReport },
    RunAutomationTick { now_hh_mm: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteCommandRequest {
    pub command: HomeCommand,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeResponse {
    Status { status: RuntimeStatus },
    Entities { entities: Vec<EntitySnapshot> },
    Automations { automations: Vec<Automation> },
    Audit { entries: Vec<AuditEntry> },
    Scenes { scenes: Vec<Scene> },
    Command { result: CommandResponse },
    ConnectivityApplied { result: ConnectivityApplyResult },
    AutomationTick { result: AutomationTickResult },
    Error { error: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResponse {
    pub decision: SafetyDecision,
    pub executed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub entity: Entity,
}

impl From<&Entity> for EntitySnapshot {
    fn from(entity: &Entity) -> Self {
        Self {
            entity: entity.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandOrigin, EntityId, HomeAction, HomeActionKind, TargetSelector};

    #[test]
    fn runtime_request_uses_tagged_json_contract() {
        let request = RuntimeRequest::Evaluate {
            command: crate::HomeCommand::new(
                CommandOrigin::Agent,
                HomeAction {
                    target: TargetSelector::exact(EntityId::new("light.kitchen").unwrap()),
                    kind: HomeActionKind::TurnOn,
                    value: None,
                },
            ),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["type"], "evaluate");
        assert_eq!(json["command"]["origin"], "agent");
        assert_eq!(json["command"]["action"]["kind"], "turn_on");
    }
}
