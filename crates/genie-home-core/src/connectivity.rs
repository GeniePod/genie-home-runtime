use crate::entity::{Capability, Entity, EntityId, EntityIdError, EntityState, SafetyClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectivityProtocol {
    Matter,
    Thread,
    Zigbee,
    ZWave,
    Ble,
    Wifi,
    Uart,
    Esp32C6,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectivityEntity {
    pub entity_id: EntityId,
    pub display_name: String,
    pub area: Option<String>,
    pub state: EntityState,
    pub capabilities: BTreeSet<Capability>,
    pub safety_class: SafetyClass,
}

impl ConnectivityEntity {
    pub fn into_entity(self) -> Entity {
        Entity {
            id: self.entity_id,
            display_name: self.display_name,
            area: self.area,
            state: self.state,
            capabilities: self.capabilities,
            safety_class: self.safety_class,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectivityDevice {
    pub stable_id: String,
    pub protocol: ConnectivityProtocol,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub entities: Vec<ConnectivityEntity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectivityReport {
    pub source: String,
    pub devices: Vec<ConnectivityDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectivityApplyResult {
    pub source: String,
    pub devices_seen: usize,
    pub entities_upserted: usize,
}

impl ConnectivityReport {
    pub fn esp32c6_thread_demo() -> Result<Self, EntityIdError> {
        let mut capabilities = BTreeSet::new();
        capabilities.insert(Capability::Power);

        Ok(Self {
            source: "genie-os-esp32c6-uart".into(),
            devices: vec![ConnectivityDevice {
                stable_id: "esp32c6-thread-demo-light".into(),
                protocol: ConnectivityProtocol::Thread,
                manufacturer: Some("GeniePod".into()),
                model: Some("ESP32-C6 Thread Demo".into()),
                entities: vec![ConnectivityEntity {
                    entity_id: EntityId::new("light.thread_demo")?,
                    display_name: "Thread Demo Light".into(),
                    area: Some("lab".into()),
                    state: EntityState::Off,
                    capabilities,
                    safety_class: SafetyClass::Normal,
                }],
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_report_is_valid() {
        let report = ConnectivityReport::esp32c6_thread_demo().unwrap();

        assert_eq!(report.source, "genie-os-esp32c6-uart");
        assert_eq!(report.devices[0].protocol, ConnectivityProtocol::Thread);
        assert_eq!(
            report.devices[0].entities[0].entity_id.as_str(),
            "light.thread_demo"
        );
    }
}
