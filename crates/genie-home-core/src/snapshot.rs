use crate::{Automation, Device, Entity, Scene, ValidationReport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    pub schema: String,
    pub devices: Vec<Device>,
    pub entities: Vec<Entity>,
    pub scenes: Vec<Scene>,
    pub automations: Vec<Automation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotApplyResult {
    pub changed: bool,
    pub devices: usize,
    pub entities: usize,
    pub scenes: usize,
    pub automations: usize,
    pub validation: ValidationReport,
}

impl RuntimeSnapshot {
    pub fn new(
        devices: Vec<Device>,
        entities: Vec<Entity>,
        scenes: Vec<Scene>,
        automations: Vec<Automation>,
    ) -> Self {
        Self {
            schema: "genie.home.runtime_snapshot.v1".into(),
            devices,
            entities,
            scenes,
            automations,
        }
    }
}
