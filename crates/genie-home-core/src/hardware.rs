use crate::ConnectivityProtocol;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareInventory {
    pub adapters: Vec<HardwareAdapterStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareAdapterStatus {
    pub protocol: ConnectivityProtocol,
    pub state: HardwareAdapterState,
    pub support_level: HardwareSupportLevel,
    pub capabilities: BTreeSet<HardwareCapability>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareAdapterState {
    BoundaryReady,
    RequiresGenieOs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareSupportLevel {
    RuntimeBoundary,
    RequiresGenieOsDriver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareCapability {
    HostTransport,
    DiscoveryReport,
    StateEvents,
    ActuationReport,
    Commissioning,
    BorderRouter,
    NetworkBackhaul,
}

pub fn default_hardware_inventory() -> HardwareInventory {
    HardwareInventory {
        adapters: vec![
            boundary_adapter(
                ConnectivityProtocol::Uart,
                [
                    HardwareCapability::HostTransport,
                    HardwareCapability::DiscoveryReport,
                    HardwareCapability::StateEvents,
                    HardwareCapability::ActuationReport,
                ],
                "Runtime can ingest structured UART-originated reports. GenieOS owns the serial driver, framing, and reconnect policy.",
            ),
            boundary_adapter(
                ConnectivityProtocol::Esp32C6,
                [
                    HardwareCapability::HostTransport,
                    HardwareCapability::DiscoveryReport,
                    HardwareCapability::StateEvents,
                    HardwareCapability::ActuationReport,
                ],
                "Runtime supports ESP32-C6 as a reported connectivity source. ESP-Hosted-NG, Thread radio firmware, and host drivers belong in GenieOS.",
            ),
            genie_os_adapter(
                ConnectivityProtocol::Matter,
                [
                    HardwareCapability::DiscoveryReport,
                    HardwareCapability::StateEvents,
                    HardwareCapability::ActuationReport,
                    HardwareCapability::Commissioning,
                ],
                "Matter commissioning, fabric storage, and controller transport are GenieOS responsibilities. Runtime accepts normalized entities and safety-gates actions.",
            ),
            genie_os_adapter(
                ConnectivityProtocol::Thread,
                [
                    HardwareCapability::DiscoveryReport,
                    HardwareCapability::StateEvents,
                    HardwareCapability::ActuationReport,
                    HardwareCapability::Commissioning,
                    HardwareCapability::BorderRouter,
                ],
                "Thread border-router and radio integration require GenieOS. Runtime can model Thread-backed devices after discovery.",
            ),
            genie_os_adapter(
                ConnectivityProtocol::Zigbee,
                [
                    HardwareCapability::DiscoveryReport,
                    HardwareCapability::StateEvents,
                    HardwareCapability::ActuationReport,
                    HardwareCapability::Commissioning,
                ],
                "Zigbee coordinator firmware, pairing, and radio transport require GenieOS or a bridge adapter.",
            ),
            genie_os_adapter(
                ConnectivityProtocol::ZWave,
                [
                    HardwareCapability::DiscoveryReport,
                    HardwareCapability::StateEvents,
                    HardwareCapability::ActuationReport,
                    HardwareCapability::Commissioning,
                ],
                "Z-Wave controller support requires GenieOS or a certified bridge adapter.",
            ),
            genie_os_adapter(
                ConnectivityProtocol::Ble,
                [
                    HardwareCapability::DiscoveryReport,
                    HardwareCapability::StateEvents,
                    HardwareCapability::ActuationReport,
                    HardwareCapability::Commissioning,
                ],
                "BLE scanning, pairing, and GATT transport are GenieOS responsibilities.",
            ),
            genie_os_adapter(
                ConnectivityProtocol::Wifi,
                [
                    HardwareCapability::HostTransport,
                    HardwareCapability::NetworkBackhaul,
                    HardwareCapability::DiscoveryReport,
                ],
                "Wi-Fi is treated as GenieOS networking/backhaul unless a higher adapter reports concrete controllable entities.",
            ),
        ],
    }
}

fn boundary_adapter(
    protocol: ConnectivityProtocol,
    capabilities: impl IntoIterator<Item = HardwareCapability>,
    note: &str,
) -> HardwareAdapterStatus {
    HardwareAdapterStatus {
        protocol,
        state: HardwareAdapterState::BoundaryReady,
        support_level: HardwareSupportLevel::RuntimeBoundary,
        capabilities: capabilities.into_iter().collect(),
        notes: vec![note.into()],
    }
}

fn genie_os_adapter(
    protocol: ConnectivityProtocol,
    capabilities: impl IntoIterator<Item = HardwareCapability>,
    note: &str,
) -> HardwareAdapterStatus {
    HardwareAdapterStatus {
        protocol,
        state: HardwareAdapterState::RequiresGenieOs,
        support_level: HardwareSupportLevel::RequiresGenieOsDriver,
        capabilities: capabilities.into_iter().collect(),
        notes: vec![note.into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_exposes_esp32c6_boundary_and_thread_requirement() {
        let inventory = default_hardware_inventory();
        let esp32c6 = inventory
            .adapters
            .iter()
            .find(|adapter| adapter.protocol == ConnectivityProtocol::Esp32C6)
            .unwrap();
        let thread = inventory
            .adapters
            .iter()
            .find(|adapter| adapter.protocol == ConnectivityProtocol::Thread)
            .unwrap();

        assert_eq!(esp32c6.state, HardwareAdapterState::BoundaryReady);
        assert_eq!(esp32c6.support_level, HardwareSupportLevel::RuntimeBoundary);
        assert_eq!(thread.state, HardwareAdapterState::RequiresGenieOs);
        assert_eq!(
            thread.support_level,
            HardwareSupportLevel::RequiresGenieOsDriver
        );
        assert!(
            thread
                .capabilities
                .contains(&HardwareCapability::BorderRouter)
        );
    }

    #[test]
    fn radio_protocols_do_not_claim_runtime_driver_support() {
        let inventory = default_hardware_inventory();
        for protocol in [
            ConnectivityProtocol::Matter,
            ConnectivityProtocol::Thread,
            ConnectivityProtocol::Zigbee,
            ConnectivityProtocol::ZWave,
            ConnectivityProtocol::Ble,
        ] {
            let adapter = inventory
                .adapters
                .iter()
                .find(|adapter| adapter.protocol == protocol)
                .unwrap();
            assert_eq!(
                adapter.support_level,
                HardwareSupportLevel::RequiresGenieOsDriver
            );
        }
    }
}
