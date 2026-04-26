use crate::{
    ConnectivityApplyResult, ConnectivityReport, HardwareAdapterStatus, StateApplyResult,
    StateReport,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenieOsMessage {
    Heartbeat {
        source: String,
        monotonic_ms: u64,
    },
    AdapterStatus {
        source: String,
        status: HardwareAdapterStatus,
    },
    ConnectivityReport {
        report: ConnectivityReport,
    },
    StateReport {
        report: StateReport,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenieOsApplyResult {
    Heartbeat { source: String, monotonic_ms: u64 },
    AdapterStatus { source: String, accepted: bool },
    ConnectivityApplied { result: ConnectivityApplyResult },
    StateApplied { result: StateApplyResult },
}
