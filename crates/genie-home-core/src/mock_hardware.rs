use crate::{
    Capability, CommandOrigin, ConnectivityApplyResult, ConnectivityDevice, ConnectivityEntity,
    ConnectivityProtocol, ConnectivityReport, EntityId, EntityState, EntityStateUpdate,
    HomeActionKind, HomeCommand, HomeRuntime, SafetyClass, SafetyDecision, StateApplyResult,
    StateReport, TargetSelector,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub trait HardwareInterface {
    fn source(&self) -> &str;
    fn discovery_report(&self) -> ConnectivityReport;
    fn poll_state(&mut self) -> StateReport;
    fn apply_command(&mut self, command: &HomeCommand) -> MockHardwareCommandResult;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockHardwareBus {
    source: String,
    sequence: u64,
    default_latency_ms: u64,
    entities: BTreeMap<EntityId, MockIoTEntity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockIoTEntity {
    pub stable_id: String,
    pub protocol: ConnectivityProtocol,
    pub manufacturer: String,
    pub model: String,
    pub entity_id: EntityId,
    pub display_name: String,
    pub area: Option<String>,
    pub state: EntityState,
    pub capabilities: BTreeSet<Capability>,
    pub safety_class: SafetyClass,
    pub online: bool,
    pub battery_percent: Option<u8>,
    pub rssi_dbm: i16,
    pub link_quality: u8,
    pub attributes: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockHardwareCommandResult {
    pub source: String,
    pub entity_id: EntityId,
    pub accepted: bool,
    pub message: String,
    pub latency_ms: u64,
    pub state_report: Option<StateReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MockHardwareFault {
    SetOnline {
        entity_id: EntityId,
        online: bool,
    },
    SetState {
        entity_id: EntityId,
        state: EntityState,
    },
    SetRadio {
        entity_id: EntityId,
        rssi_dbm: i16,
        link_quality: u8,
    },
    AddCommandLatency {
        latency_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockHardwareFaultApplied {
    pub source: String,
    pub fault: MockHardwareFault,
    pub applied: bool,
    pub message: String,
    pub state_report: Option<StateReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockHardwareFaultScenarioResult {
    pub discovery_apply_result: ConnectivityApplyResult,
    pub initial_state_apply_result: StateApplyResult,
    pub latency_fault: MockHardwareFaultApplied,
    pub radio_fault: MockHardwareFaultApplied,
    pub offline_fault: MockHardwareFaultApplied,
    pub offline_state_apply_result: Option<StateApplyResult>,
    pub command: HomeCommand,
    pub safety_decision: SafetyDecision,
    pub hardware_command_result: Option<MockHardwareCommandResult>,
    pub recovery_online_fault: MockHardwareFaultApplied,
    pub recovery_state_fault: MockHardwareFaultApplied,
    pub recovery_state_apply_result: Option<StateApplyResult>,
}

impl MockHardwareBus {
    pub fn reference_home() -> Self {
        let entities = [
            MockIoTEntity::new(
                "mock-thread-lamp-1",
                ConnectivityProtocol::Thread,
                "GeniePod",
                "Mock Thread Dimmable Lamp",
                EntityId::new("light.mock_thread_lamp").expect("valid mock entity id"),
                "Mock Thread Lamp",
                Some("living_room"),
                EntityState::Off,
                [Capability::Power, Capability::Brightness],
                SafetyClass::Normal,
            )
            .with_radio(-48, 95)
            .with_battery(88),
            MockIoTEntity::new(
                "mock-matter-front-door-lock",
                ConnectivityProtocol::Matter,
                "GeniePod",
                "Mock Matter Door Lock",
                EntityId::new("lock.mock_front_door").expect("valid mock entity id"),
                "Mock Front Door Lock",
                Some("entry"),
                EntityState::Locked,
                [Capability::Lock],
                SafetyClass::Sensitive,
            )
            .with_radio(-54, 91)
            .with_battery(72),
            MockIoTEntity::new(
                "mock-zigbee-contact-1",
                ConnectivityProtocol::Zigbee,
                "GeniePod",
                "Mock Zigbee Contact Sensor",
                EntityId::new("binary_sensor.mock_entry_contact").expect("valid mock entity id"),
                "Mock Entry Contact",
                Some("entry"),
                EntityState::Closed,
                [Capability::SensorRead],
                SafetyClass::Normal,
            )
            .with_radio(-61, 84)
            .with_battery(67),
            MockIoTEntity::new(
                "mock-ble-temp-1",
                ConnectivityProtocol::Ble,
                "GeniePod",
                "Mock BLE Temperature Sensor",
                EntityId::new("sensor.mock_ble_temperature").expect("valid mock entity id"),
                "Mock BLE Temperature",
                Some("bedroom"),
                EntityState::Numeric(21.4),
                [Capability::SensorRead],
                SafetyClass::Normal,
            )
            .with_radio(-69, 76)
            .with_battery(52),
            MockIoTEntity::new(
                "mock-wifi-media-1",
                ConnectivityProtocol::Wifi,
                "GeniePod",
                "Mock Wi-Fi Media Player",
                EntityId::new("media_player.mock_living_room").expect("valid mock entity id"),
                "Mock Living Room Media",
                Some("living_room"),
                EntityState::Off,
                [
                    Capability::Power,
                    Capability::MediaPlayback,
                    Capability::Brightness,
                ],
                SafetyClass::Normal,
            )
            .with_radio(-43, 98),
            MockIoTEntity::new(
                "mock-matter-vacuum-1",
                ConnectivityProtocol::Matter,
                "GeniePod",
                "Mock Matter Vacuum",
                EntityId::new("vacuum.mock_robot").expect("valid mock entity id"),
                "Mock Robot Vacuum",
                Some("living_room"),
                EntityState::Off,
                [Capability::VacuumControl],
                SafetyClass::Sensitive,
            )
            .with_radio(-58, 87)
            .with_battery(63),
            MockIoTEntity::new(
                "mock-matter-alarm-1",
                ConnectivityProtocol::Matter,
                "GeniePod",
                "Mock Matter Alarm Panel",
                EntityId::new("alarm_control_panel.mock_home").expect("valid mock entity id"),
                "Mock Home Alarm",
                Some("entry"),
                EntityState::Text("disarmed".into()),
                [Capability::AlarmControl],
                SafetyClass::Sensitive,
            )
            .with_radio(-49, 94),
        ]
        .into_iter()
        .map(|entity| (entity.entity_id.clone(), entity))
        .collect();

        Self {
            source: "mock-hardware-reference-home".into(),
            sequence: 0,
            default_latency_ms: 35,
            entities,
        }
    }

    pub fn entities(&self) -> impl Iterator<Item = &MockIoTEntity> {
        self.entities.values()
    }

    pub fn set_online(&mut self, entity_id: &EntityId, online: bool) -> Option<StateReport> {
        let source = self.source.clone();
        let sequence = self.next_sequence();
        let entity = self.entities.get_mut(entity_id)?;
        entity.online = online;
        if !online {
            entity.state = EntityState::Unavailable;
        }
        Some(StateReport {
            source,
            updates: vec![entity.state_update(sequence)],
        })
    }

    pub fn set_radio(
        &mut self,
        entity_id: &EntityId,
        rssi_dbm: i16,
        link_quality: u8,
    ) -> Option<StateReport> {
        let source = self.source.clone();
        let sequence = self.next_sequence();
        let entity = self.entities.get_mut(entity_id)?;
        entity.rssi_dbm = rssi_dbm;
        entity.link_quality = link_quality.min(100);
        Some(StateReport {
            source,
            updates: vec![entity.state_update(sequence)],
        })
    }

    pub fn set_state(&mut self, entity_id: &EntityId, state: EntityState) -> Option<StateReport> {
        let source = self.source.clone();
        let sequence = self.next_sequence();
        let entity = self.entities.get_mut(entity_id)?;
        entity.state = state;
        Some(StateReport {
            source,
            updates: vec![entity.state_update(sequence)],
        })
    }

    pub fn apply_fault(&mut self, fault: MockHardwareFault) -> MockHardwareFaultApplied {
        let source = self.source.clone();
        match &fault {
            MockHardwareFault::SetOnline { entity_id, online } => {
                let state_report = self.set_online(entity_id, *online);
                let applied = state_report.is_some();
                MockHardwareFaultApplied {
                    source,
                    fault,
                    applied,
                    message: if applied {
                        "mock online fault applied".into()
                    } else {
                        "fault target entity does not exist".into()
                    },
                    state_report,
                }
            }
            MockHardwareFault::SetState { entity_id, state } => {
                let state_report = self.set_state(entity_id, state.clone());
                let applied = state_report.is_some();
                MockHardwareFaultApplied {
                    source,
                    fault,
                    applied,
                    message: if applied {
                        "mock state fault applied".into()
                    } else {
                        "fault target entity does not exist".into()
                    },
                    state_report,
                }
            }
            MockHardwareFault::SetRadio {
                entity_id,
                rssi_dbm,
                link_quality,
            } => {
                let state_report = self.set_radio(entity_id, *rssi_dbm, *link_quality);
                let applied = state_report.is_some();
                MockHardwareFaultApplied {
                    source,
                    fault,
                    applied,
                    message: if applied {
                        "mock radio fault applied".into()
                    } else {
                        "fault target entity does not exist".into()
                    },
                    state_report,
                }
            }
            MockHardwareFault::AddCommandLatency { latency_ms } => {
                self.default_latency_ms = self.default_latency_ms.saturating_add(*latency_ms);
                MockHardwareFaultApplied {
                    source,
                    fault,
                    applied: true,
                    message: "mock command latency fault applied".into(),
                    state_report: None,
                }
            }
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }
}

impl HardwareInterface for MockHardwareBus {
    fn source(&self) -> &str {
        &self.source
    }

    fn discovery_report(&self) -> ConnectivityReport {
        ConnectivityReport {
            source: self.source.clone(),
            devices: self
                .entities
                .values()
                .map(MockIoTEntity::connectivity_device)
                .collect(),
        }
    }

    fn poll_state(&mut self) -> StateReport {
        let source = self.source.clone();
        let sequence = self.next_sequence();
        self.advance_sensor_values();
        StateReport {
            source,
            updates: self
                .entities
                .values()
                .map(|entity| entity.state_update(sequence))
                .collect(),
        }
    }

    fn apply_command(&mut self, command: &HomeCommand) -> MockHardwareCommandResult {
        let source = self.source.clone();
        let sequence = self.next_sequence();
        let latency_ms = self.default_latency_ms;
        let entity_id = command.action.target.entity_id.clone();
        let Some(entity) = self.entities.get_mut(&entity_id) else {
            return MockHardwareCommandResult {
                source,
                entity_id,
                accepted: false,
                message: "target entity does not exist on mock hardware bus".into(),
                latency_ms,
                state_report: None,
            };
        };
        if !entity.online {
            return MockHardwareCommandResult {
                source,
                entity_id,
                accepted: false,
                message: "target entity is offline".into(),
                latency_ms,
                state_report: Some(StateReport {
                    source: self.source.clone(),
                    updates: vec![entity.state_update(sequence)],
                }),
            };
        }
        if !entity.supports_action(&command.action.kind) {
            return MockHardwareCommandResult {
                source,
                entity_id,
                accepted: false,
                message: "target entity does not support requested action".into(),
                latency_ms,
                state_report: Some(StateReport {
                    source: self.source.clone(),
                    updates: vec![entity.state_update(sequence)],
                }),
            };
        }

        entity.apply_action(&command.action.kind, command.action.value.as_ref());
        let report = StateReport {
            source: self.source.clone(),
            updates: vec![entity.state_update(sequence)],
        };
        MockHardwareCommandResult {
            source,
            entity_id,
            accepted: true,
            message: "mock hardware accepted command".into(),
            latency_ms,
            state_report: Some(report),
        }
    }
}

impl MockHardwareBus {
    fn advance_sensor_values(&mut self) {
        for entity in self.entities.values_mut() {
            if !entity.online {
                continue;
            }
            if entity.entity_id.domain() == "sensor"
                && let EntityState::Numeric(value) = entity.state
            {
                let drift = if self.sequence.is_multiple_of(2) {
                    0.1
                } else {
                    -0.1
                };
                entity.state = EntityState::Numeric((value + drift).clamp(16.0, 32.0));
            }
        }
    }
}

impl MockIoTEntity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stable_id: impl Into<String>,
        protocol: ConnectivityProtocol,
        manufacturer: impl Into<String>,
        model: impl Into<String>,
        entity_id: EntityId,
        display_name: impl Into<String>,
        area: Option<&str>,
        state: EntityState,
        capabilities: impl IntoIterator<Item = Capability>,
        safety_class: SafetyClass,
    ) -> Self {
        Self {
            stable_id: stable_id.into(),
            protocol,
            manufacturer: manufacturer.into(),
            model: model.into(),
            entity_id,
            display_name: display_name.into(),
            area: area.map(str::to_string),
            state,
            capabilities: capabilities.into_iter().collect(),
            safety_class,
            online: true,
            battery_percent: None,
            rssi_dbm: -55,
            link_quality: 90,
            attributes: BTreeMap::new(),
        }
    }

    pub fn with_battery(mut self, battery_percent: u8) -> Self {
        self.battery_percent = Some(battery_percent.min(100));
        self
    }

    pub fn with_radio(mut self, rssi_dbm: i16, link_quality: u8) -> Self {
        self.rssi_dbm = rssi_dbm;
        self.link_quality = link_quality.min(100);
        self
    }

    fn connectivity_device(&self) -> ConnectivityDevice {
        ConnectivityDevice {
            stable_id: self.stable_id.clone(),
            protocol: self.protocol,
            manufacturer: Some(self.manufacturer.clone()),
            model: Some(self.model.clone()),
            entities: vec![ConnectivityEntity {
                entity_id: self.entity_id.clone(),
                display_name: self.display_name.clone(),
                area: self.area.clone(),
                state: self.state.clone(),
                capabilities: self.capabilities.clone(),
                safety_class: self.safety_class,
            }],
        }
    }

    fn state_update(&self, sequence: u64) -> EntityStateUpdate {
        let mut attributes = self.attributes.clone();
        attributes.insert("mock_sequence".into(), serde_json::json!(sequence));
        attributes.insert("online".into(), serde_json::json!(self.online));
        attributes.insert("protocol".into(), serde_json::json!(self.protocol));
        attributes.insert("rssi_dbm".into(), serde_json::json!(self.rssi_dbm));
        attributes.insert("link_quality".into(), serde_json::json!(self.link_quality));
        if let Some(battery) = self.battery_percent {
            attributes.insert("battery_percent".into(), serde_json::json!(battery));
        }

        EntityStateUpdate {
            entity_id: self.entity_id.clone(),
            state: self.state.clone(),
            attributes,
        }
    }

    fn supports_action(&self, action: &HomeActionKind) -> bool {
        match action {
            HomeActionKind::TurnOn | HomeActionKind::TurnOff | HomeActionKind::Toggle => {
                self.capabilities.contains(&Capability::Power)
            }
            HomeActionKind::SetValue => {
                self.capabilities.contains(&Capability::Brightness)
                    || self.capabilities.contains(&Capability::Temperature)
            }
            HomeActionKind::Lock | HomeActionKind::Unlock => {
                self.capabilities.contains(&Capability::Lock)
            }
            HomeActionKind::Open | HomeActionKind::Close => {
                self.capabilities.contains(&Capability::OpenClose)
            }
            HomeActionKind::Start | HomeActionKind::Stop | HomeActionKind::Pause => {
                self.capabilities.contains(&Capability::MediaPlayback)
                    || self.capabilities.contains(&Capability::VacuumControl)
            }
            HomeActionKind::ReturnToBase => self.capabilities.contains(&Capability::VacuumControl),
            HomeActionKind::Arm | HomeActionKind::Disarm => {
                self.capabilities.contains(&Capability::AlarmControl)
            }
            HomeActionKind::ActivateScene => false,
        }
    }

    fn apply_action(&mut self, action: &HomeActionKind, value: Option<&serde_json::Value>) {
        match action {
            HomeActionKind::TurnOn => self.state = EntityState::On,
            HomeActionKind::TurnOff => self.state = EntityState::Off,
            HomeActionKind::Toggle => {
                self.state = match self.state {
                    EntityState::On => EntityState::Off,
                    EntityState::Off => EntityState::On,
                    _ => self.state.clone(),
                };
            }
            HomeActionKind::Lock => self.state = EntityState::Locked,
            HomeActionKind::Unlock => self.state = EntityState::Unlocked,
            HomeActionKind::Open => self.state = EntityState::Open,
            HomeActionKind::Close => self.state = EntityState::Closed,
            HomeActionKind::Start => self.state = EntityState::On,
            HomeActionKind::Stop | HomeActionKind::Pause => self.state = EntityState::Off,
            HomeActionKind::ReturnToBase => self.state = EntityState::Text("returning".into()),
            HomeActionKind::Arm => self.state = EntityState::Text("armed".into()),
            HomeActionKind::Disarm => self.state = EntityState::Text("disarmed".into()),
            HomeActionKind::SetValue => {
                if let Some(value) = value {
                    self.attributes.insert("target_value".into(), value.clone());
                }
            }
            HomeActionKind::ActivateScene => {}
        }
    }
}

pub fn mock_turn_on_thread_lamp_command() -> HomeCommand {
    HomeCommand::new(
        CommandOrigin::LocalApi,
        crate::HomeAction {
            target: TargetSelector::exact(
                EntityId::new("light.mock_thread_lamp").expect("valid mock entity id"),
            ),
            kind: HomeActionKind::TurnOn,
            value: None,
        },
    )
}

pub fn run_mock_hardware_fault_scenario() -> MockHardwareFaultScenarioResult {
    let mut hardware = MockHardwareBus::reference_home();
    let mut runtime = HomeRuntime::with_default_policy();
    let entity_id = EntityId::new("light.mock_thread_lamp").expect("valid mock entity id");

    let discovery_apply_result = runtime.apply_connectivity_report(hardware.discovery_report());
    let initial_state_apply_result = runtime.apply_state_report(hardware.poll_state());
    let latency_fault =
        hardware.apply_fault(MockHardwareFault::AddCommandLatency { latency_ms: 250 });
    let radio_fault = hardware.apply_fault(MockHardwareFault::SetRadio {
        entity_id: entity_id.clone(),
        rssi_dbm: -91,
        link_quality: 8,
    });
    if let Some(report) = radio_fault.state_report.clone() {
        runtime.apply_state_report(report);
    }

    let offline_fault = hardware.apply_fault(MockHardwareFault::SetOnline {
        entity_id: entity_id.clone(),
        online: false,
    });
    let offline_state_apply_result = offline_fault
        .state_report
        .clone()
        .map(|report| runtime.apply_state_report(report));

    let command = mock_turn_on_thread_lamp_command();
    let safety_decision = runtime.execute(command.clone());
    let hardware_command_result = if safety_decision.allowed {
        Some(hardware.apply_command(&command))
    } else {
        None
    };

    let recovery_online_fault = hardware.apply_fault(MockHardwareFault::SetOnline {
        entity_id: entity_id.clone(),
        online: true,
    });
    let recovery_state_fault = hardware.apply_fault(MockHardwareFault::SetState {
        entity_id,
        state: EntityState::Off,
    });
    let recovery_state_apply_result = recovery_state_fault
        .state_report
        .clone()
        .map(|report| runtime.apply_state_report(report));

    MockHardwareFaultScenarioResult {
        discovery_apply_result,
        initial_state_apply_result,
        latency_fault,
        radio_fault,
        offline_fault,
        offline_state_apply_result,
        command,
        safety_decision,
        hardware_command_result,
        recovery_online_fault,
        recovery_state_fault,
        recovery_state_apply_result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HomeRuntime, RuntimeRequest, RuntimeResponse};

    #[test]
    fn reference_home_discovers_multiple_protocols() {
        let hardware = MockHardwareBus::reference_home();
        let report = hardware.discovery_report();

        assert_eq!(report.devices.len(), 7);
        assert!(
            report
                .devices
                .iter()
                .any(|device| device.protocol == ConnectivityProtocol::Thread)
        );
        assert!(
            report
                .devices
                .iter()
                .any(|device| device.protocol == ConnectivityProtocol::Matter)
        );
    }

    #[test]
    fn mock_command_changes_device_state_report() {
        let mut hardware = MockHardwareBus::reference_home();
        let result = hardware.apply_command(&mock_turn_on_thread_lamp_command());

        assert!(result.accepted);
        let report = result.state_report.unwrap();
        assert_eq!(report.updates.len(), 1);
        assert_eq!(report.updates[0].state, EntityState::On);
        assert_eq!(
            report.updates[0].attributes["protocol"],
            serde_json::json!(ConnectivityProtocol::Thread)
        );
    }

    #[test]
    fn offline_mock_device_rejects_command() {
        let mut hardware = MockHardwareBus::reference_home();
        let id = EntityId::new("light.mock_thread_lamp").unwrap();
        hardware.set_online(&id, false).unwrap();

        let result = hardware.apply_command(&mock_turn_on_thread_lamp_command());

        assert!(!result.accepted);
        assert!(result.message.contains("offline"));
        assert_eq!(
            result.state_report.unwrap().updates[0].state,
            EntityState::Unavailable
        );
    }

    #[test]
    fn faults_can_adjust_latency_and_radio_state() {
        let mut hardware = MockHardwareBus::reference_home();
        let id = EntityId::new("light.mock_thread_lamp").unwrap();
        let latency_fault =
            hardware.apply_fault(MockHardwareFault::AddCommandLatency { latency_ms: 250 });
        let radio_fault = hardware.apply_fault(MockHardwareFault::SetRadio {
            entity_id: id.clone(),
            rssi_dbm: -92,
            link_quality: 7,
        });

        assert!(latency_fault.applied);
        assert!(radio_fault.applied);
        let report = radio_fault.state_report.unwrap();
        assert_eq!(
            report.updates[0].attributes["rssi_dbm"],
            serde_json::json!(-92)
        );
        assert_eq!(
            report.updates[0].attributes["link_quality"],
            serde_json::json!(7)
        );

        let result = hardware.apply_command(&mock_turn_on_thread_lamp_command());
        assert_eq!(result.latency_ms, 285);
    }

    #[test]
    fn fault_scenario_blocks_command_before_hardware_actuation() {
        let scenario = run_mock_hardware_fault_scenario();

        assert!(scenario.offline_fault.applied);
        assert_eq!(
            scenario
                .offline_state_apply_result
                .unwrap()
                .entities_updated,
            1
        );
        assert!(!scenario.safety_decision.allowed);
        assert!(scenario.hardware_command_result.is_none());
        assert!(scenario.recovery_online_fault.applied);
        assert!(scenario.recovery_state_fault.applied);
    }

    #[test]
    fn runtime_can_ingest_mock_discovery_and_state() {
        let mut hardware = MockHardwareBus::reference_home();
        let mut runtime = HomeRuntime::with_default_policy();
        runtime.apply_connectivity_report(hardware.discovery_report());
        runtime.apply_state_report(hardware.poll_state());

        let command = mock_turn_on_thread_lamp_command();
        let decision = runtime.execute(command.clone());
        assert!(decision.allowed);

        let hardware_result = hardware.apply_command(&command);
        assert!(hardware_result.accepted);
        let response = runtime.handle_request(RuntimeRequest::ApplyStateReport {
            report: hardware_result.state_report.unwrap(),
        });

        let RuntimeResponse::StateApplied { result } = response else {
            panic!("expected state apply response");
        };
        assert_eq!(result.entities_updated, 1);
        assert_eq!(
            runtime
                .graph()
                .get(&EntityId::new("light.mock_thread_lamp").unwrap())
                .unwrap()
                .state,
            EntityState::On
        );
    }
}
