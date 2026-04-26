use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId(String);

impl EntityId {
    pub fn new(value: impl Into<String>) -> Result<Self, EntityIdError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(EntityIdError::Empty);
        }
        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '_' | '.'))
        {
            return Err(EntityIdError::Invalid(trimmed.to_string()));
        }
        if !trimmed.contains('.') {
            return Err(EntityIdError::MissingDomain(trimmed.to_string()));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn domain(&self) -> &str {
        self.0
            .split_once('.')
            .map(|(domain, _)| domain)
            .unwrap_or("")
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EntityIdError {
    #[error("entity id cannot be empty")]
    Empty,
    #[error("entity id must include a domain prefix: {0}")]
    MissingDomain(String),
    #[error("invalid entity id: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Power,
    Brightness,
    Color,
    Temperature,
    Lock,
    OpenClose,
    SceneActivation,
    SensorRead,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityState {
    Unknown,
    Unavailable,
    Off,
    On,
    Locked,
    Unlocked,
    Open,
    Closed,
    Numeric(f64),
    Text(String),
}

impl EntityState {
    pub fn is_available(&self) -> bool {
        !matches!(self, Self::Unknown | Self::Unavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub display_name: String,
    pub area: Option<String>,
    pub state: EntityState,
    pub capabilities: BTreeSet<Capability>,
    pub safety_class: SafetyClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClass {
    Normal,
    Sensitive,
    Critical,
}

impl Entity {
    pub fn new(id: EntityId, display_name: impl Into<String>) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            area: None,
            state: EntityState::Unknown,
            capabilities: BTreeSet::new(),
            safety_class: SafetyClass::Normal,
        }
    }

    pub fn with_area(mut self, area: impl Into<String>) -> Self {
        self.area = Some(area.into());
        self
    }

    pub fn with_state(mut self, state: EntityState) -> Self {
        self.state = state;
        self
    }

    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.insert(capability);
        self
    }

    pub fn with_safety_class(mut self, safety_class: SafetyClass) -> Self {
        self.safety_class = safety_class;
        self
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EntityGraph {
    entities: BTreeMap<EntityId, Entity>,
}

impl EntityGraph {
    pub fn upsert(&mut self, entity: Entity) {
        self.entities.insert(entity.id.clone(), entity);
    }

    pub fn get(&self, id: &EntityId) -> Option<&Entity> {
        self.entities.get(id)
    }

    pub fn contains(&self, id: &EntityId) -> bool {
        self.entities.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_requires_domain() {
        assert!(EntityId::new("light.kitchen").is_ok());
        assert!(matches!(
            EntityId::new("kitchen"),
            Err(EntityIdError::MissingDomain(_))
        ));
    }

    #[test]
    fn entity_graph_upserts_entities() {
        let mut graph = EntityGraph::default();
        let id = EntityId::new("light.kitchen").unwrap();
        graph.upsert(Entity::new(id.clone(), "Kitchen Light").with_state(EntityState::Off));

        assert_eq!(graph.len(), 1);
        assert_eq!(graph.get(&id).unwrap().display_name, "Kitchen Light");
    }
}
