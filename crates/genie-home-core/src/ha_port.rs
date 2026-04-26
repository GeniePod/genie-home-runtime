use crate::{ConnectivityProtocol, EntityState, HardwareInterface, MockIoTEntity};
use crate::{
    HomeAssistantEntityRecord, HomeRuntime, MigrationImportPlan, MigrationReport, MockHardwareBus,
    RuntimeStatus, ValidationReport, build_home_assistant_import_plan,
    build_home_assistant_migration_report, validate_runtime,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockHomeAssistantPortResult {
    pub source: String,
    pub ha_states: Vec<HomeAssistantEntityRecord>,
    pub migration_report: MigrationReport,
    pub import_plan: MigrationImportPlan,
    pub runtime_status: RuntimeStatus,
    pub validation: ValidationReport,
}

pub fn mock_hardware_to_home_assistant_states(
    hardware: &MockHardwareBus,
) -> Vec<HomeAssistantEntityRecord> {
    hardware
        .entities()
        .map(mock_entity_to_home_assistant_record)
        .collect()
}

pub fn run_mock_home_assistant_port() -> MockHomeAssistantPortResult {
    let hardware = MockHardwareBus::reference_home();
    let ha_states = mock_hardware_to_home_assistant_states(&hardware);
    let migration_report = build_home_assistant_migration_report(ha_states.clone());
    let import_plan = build_home_assistant_import_plan(ha_states.clone());
    let mut runtime = HomeRuntime::with_default_policy();
    runtime.apply_connectivity_report(import_plan.report.clone());
    let validation = validate_runtime(&runtime);
    let runtime_status = runtime.status();

    MockHomeAssistantPortResult {
        source: hardware.source().into(),
        ha_states,
        migration_report,
        import_plan,
        runtime_status,
        validation,
    }
}

fn mock_entity_to_home_assistant_record(entity: &MockIoTEntity) -> HomeAssistantEntityRecord {
    let mut attributes = BTreeMap::new();
    attributes.insert(
        "friendly_name".into(),
        Value::String(entity.display_name.clone()),
    );
    attributes.insert("device_id".into(), Value::String(entity.stable_id.clone()));
    attributes.insert(
        "manufacturer".into(),
        Value::String(entity.manufacturer.clone()),
    );
    attributes.insert("model".into(), Value::String(entity.model.clone()));
    attributes.insert(
        "genie_protocol".into(),
        Value::String(protocol_name(entity.protocol).into()),
    );
    attributes.insert("online".into(), Value::Bool(entity.online));
    attributes.insert("rssi_dbm".into(), serde_json::json!(entity.rssi_dbm));
    attributes.insert(
        "link_quality".into(),
        serde_json::json!(entity.link_quality),
    );
    if let Some(area) = &entity.area {
        attributes.insert("area_id".into(), Value::String(area.clone()));
    }
    if let Some(battery_percent) = entity.battery_percent {
        attributes.insert("battery_percent".into(), serde_json::json!(battery_percent));
    }

    HomeAssistantEntityRecord {
        entity_id: entity.entity_id.as_str().into(),
        state: Some(state_name(&entity.state)),
        name: Some(entity.display_name.clone()),
        attributes,
    }
}

fn state_name(state: &EntityState) -> String {
    match state {
        EntityState::Unknown => "unknown".into(),
        EntityState::Unavailable => "unavailable".into(),
        EntityState::Off => "off".into(),
        EntityState::On => "on".into(),
        EntityState::Locked => "locked".into(),
        EntityState::Unlocked => "unlocked".into(),
        EntityState::Open => "open".into(),
        EntityState::Closed => "closed".into(),
        EntityState::Numeric(value) => value.to_string(),
        EntityState::Text(value) => value.clone(),
    }
}

fn protocol_name(protocol: ConnectivityProtocol) -> &'static str {
    match protocol {
        ConnectivityProtocol::Matter => "matter",
        ConnectivityProtocol::Thread => "thread",
        ConnectivityProtocol::Zigbee => "zigbee",
        ConnectivityProtocol::ZWave => "z_wave",
        ConnectivityProtocol::Ble => "ble",
        ConnectivityProtocol::Wifi => "wifi",
        ConnectivityProtocol::Uart => "uart",
        ConnectivityProtocol::Esp32C6 => "esp32_c6",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_hardware_generates_home_assistant_states() {
        let hardware = MockHardwareBus::reference_home();
        let states = mock_hardware_to_home_assistant_states(&hardware);

        assert_eq!(states.len(), 7);
        assert!(states.iter().any(|state| {
            state.entity_id == "light.mock_thread_lamp"
                && state.attributes["genie_protocol"] == "thread"
        }));
        assert!(states.iter().any(|state| {
            state.entity_id == "lock.mock_front_door"
                && state.attributes["genie_protocol"] == "matter"
        }));
    }

    #[test]
    fn mock_home_assistant_port_imports_all_mappable_devices() {
        let result = run_mock_home_assistant_port();

        assert_eq!(result.migration_report.counts.total, 7);
        assert_eq!(result.migration_report.counts.mappable, 7);
        assert_eq!(result.import_plan.skipped.len(), 0);
        assert_eq!(result.import_plan.report.devices.len(), 7);
        assert_eq!(result.runtime_status.entity_count, 7);
        assert_eq!(result.runtime_status.device_count, 7);
        assert!(result.validation.ok);
        assert!(result.import_plan.report.devices.iter().any(|device| {
            device.stable_id == "mock-thread-lamp-1"
                && device.protocol == ConnectivityProtocol::Thread
        }));
    }
}
