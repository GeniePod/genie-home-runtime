use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn new(value: impl Into<String>) -> Result<Self, DeviceIdError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(DeviceIdError::Empty);
        }
        if !trimmed.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '_' | '-' | '.')
        }) {
            return Err(DeviceIdError::Invalid(trimmed.to_string()));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DeviceIdError {
    #[error("device id cannot be empty")]
    Empty,
    #[error("invalid device id: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub display_name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub area: Option<String>,
    pub identifiers: BTreeSet<String>,
    pub connections: BTreeSet<String>,
}

impl Device {
    pub fn new(id: DeviceId, display_name: impl Into<String>) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            manufacturer: None,
            model: None,
            area: None,
            identifiers: BTreeSet::new(),
            connections: BTreeSet::new(),
        }
    }

    pub fn with_manufacturer(mut self, manufacturer: impl Into<String>) -> Self {
        self.manufacturer = Some(manufacturer.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_area(mut self, area: impl Into<String>) -> Self {
        self.area = Some(area.into());
        self
    }

    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifiers.insert(identifier.into());
        self
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DeviceRegistry {
    devices: BTreeMap<DeviceId, Device>,
}

impl DeviceRegistry {
    pub fn upsert(&mut self, device: Device) {
        self.devices.insert(device.id.clone(), device);
    }

    pub fn get(&self, id: &DeviceId) -> Option<&Device> {
        self.devices.get(id)
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_device_id() {
        assert!(DeviceId::new("esp32c6-thread-demo").is_ok());
        assert!(matches!(
            DeviceId::new("bad id"),
            Err(DeviceIdError::Invalid(_))
        ));
    }

    #[test]
    fn registry_upserts_devices() {
        let id = DeviceId::new("device.kitchen_light").unwrap();
        let mut registry = DeviceRegistry::default();
        registry.upsert(Device::new(id.clone(), "Kitchen Light"));

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(&id).unwrap().display_name, "Kitchen Light");
    }
}
