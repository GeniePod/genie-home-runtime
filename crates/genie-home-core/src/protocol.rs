use crate::{
    AuditEntry, Automation, AutomationTickResult, ConnectivityApplyResult, ConnectivityReport,
    Device, DomainSupport, Entity, EntityId, HardwareInventory, HomeCommand, RuntimeEvent,
    RuntimeSnapshot, RuntimeStatus, SafetyDecision, Scene, SchedulerCatchUpPolicy,
    SchedulerRunResult, SchedulerWindow, ServiceCall, ServiceCallResult, ServiceSpec,
    SnapshotApplyResult, StateApplyResult, StateReport, ValidationReport,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeRequest {
    Status,
    Validate,
    ListDevices,
    ListEntities,
    ListAutomations,
    ListServices,
    ListDomains,
    HardwareInventory,
    Audit {
        limit: Option<usize>,
    },
    Events {
        limit: Option<usize>,
    },
    ListScenes,
    ExportSnapshot,
    ImportSnapshot {
        snapshot: RuntimeSnapshot,
    },
    Evaluate {
        command: HomeCommand,
    },
    Execute {
        command: HomeCommand,
    },
    CallService {
        call: ServiceCall,
    },
    UpsertScene {
        scene: Scene,
    },
    DeleteScene {
        scene_id: EntityId,
    },
    UpsertAutomation {
        automation: Automation,
    },
    DeleteAutomation {
        automation_id: String,
    },
    ApplyConnectivityReport {
        report: ConnectivityReport,
    },
    ApplyStateReport {
        report: StateReport,
    },
    RunAutomationTick {
        now_hh_mm: String,
    },
    RunSchedulerWindow {
        window: SchedulerWindow,
        policy: SchedulerCatchUpPolicy,
    },
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
    Validation { report: ValidationReport },
    Devices { devices: Vec<DeviceSnapshot> },
    Entities { entities: Vec<EntitySnapshot> },
    Automations { automations: Vec<Automation> },
    Services { services: Vec<ServiceSpec> },
    Domains { domains: Vec<DomainSupport> },
    HardwareInventory { inventory: HardwareInventory },
    Audit { entries: Vec<AuditEntry> },
    Events { events: Vec<RuntimeEvent> },
    Scenes { scenes: Vec<Scene> },
    Snapshot { snapshot: RuntimeSnapshot },
    SnapshotApplied { result: SnapshotApplyResult },
    Command { result: CommandResponse },
    ServiceCall { result: ServiceCallResult },
    ConfigChanged { result: ConfigChangeResult },
    ConnectivityApplied { result: ConnectivityApplyResult },
    StateApplied { result: StateApplyResult },
    AutomationTick { result: AutomationTickResult },
    SchedulerRun { result: SchedulerRunResult },
    Error { error: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigChangeResult {
    pub resource: ConfigResource,
    pub id: String,
    pub changed: bool,
    pub validation: Option<ValidationReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigResource {
    Scene,
    Automation,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    pub device: Device,
}

impl From<&Entity> for EntitySnapshot {
    fn from(entity: &Entity) -> Self {
        Self {
            entity: entity.clone(),
        }
    }
}

impl From<&Device> for DeviceSnapshot {
    fn from(device: &Device) -> Self {
        Self {
            device: device.clone(),
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
