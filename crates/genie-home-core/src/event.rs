use crate::{CommandOrigin, EntityId, EntityState};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEvent {
    #[serde(with = "time::serde::rfc3339")]
    pub ts: OffsetDateTime,
    pub kind: RuntimeEventKind,
}

impl RuntimeEvent {
    pub fn new(kind: RuntimeEventKind) -> Self {
        Self {
            ts: OffsetDateTime::now_utc(),
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeEventKind {
    StateChanged {
        entity_id: EntityId,
        old_state: EntityState,
        new_state: EntityState,
        origin: CommandOrigin,
    },
    ServiceCalled {
        domain: String,
        service: String,
        targets: usize,
        executed: usize,
    },
    ConnectivityApplied {
        source: String,
        devices_seen: usize,
        entities_upserted: usize,
    },
    StateReportApplied {
        source: String,
        updates_seen: usize,
        entities_updated: usize,
        unknown_entities: usize,
    },
    GenieOsHeartbeat {
        source: String,
        monotonic_ms: u64,
    },
    GenieOsAdapterStatus {
        source: String,
        protocol: crate::ConnectivityProtocol,
        state: crate::HardwareAdapterState,
    },
    AutomationTick {
        now_hh_mm: String,
        automations_triggered: usize,
        actions_executed: usize,
        blocked: usize,
    },
}
