use crate::entity::{Capability, EntityState, SafetyClass};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantEntityRecord {
    pub entity_id: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationCompatibility {
    Mappable,
    ManualReview,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationCandidate {
    pub entity_id: String,
    pub domain: String,
    pub display_name: String,
    pub compatibility: MigrationCompatibility,
    pub capabilities: BTreeSet<Capability>,
    pub safety_class: SafetyClass,
    pub initial_state: EntityState,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationCounts {
    pub total: usize,
    pub mappable: usize,
    pub manual_review: usize,
    pub unsupported: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationReport {
    pub source: String,
    pub counts: MigrationCounts,
    pub candidates: Vec<MigrationCandidate>,
}

pub fn parse_home_assistant_entities_json(
    input: &str,
) -> Result<Vec<HomeAssistantEntityRecord>, String> {
    let value = serde_json::from_str::<Value>(input).map_err(|err| err.to_string())?;
    let entities = match value {
        Value::Array(_) => value,
        Value::Object(mut object) => object
            .remove("entities")
            .ok_or_else(|| "expected an array or an object with an entities array".to_string())?,
        _ => return Err("expected an array or an object with an entities array".into()),
    };
    serde_json::from_value(entities).map_err(|err| err.to_string())
}

pub fn build_home_assistant_migration_report(
    records: Vec<HomeAssistantEntityRecord>,
) -> MigrationReport {
    let candidates = records.into_iter().map(build_candidate).collect::<Vec<_>>();
    let counts = count_candidates(&candidates);
    MigrationReport {
        source: "home_assistant".into(),
        counts,
        candidates,
    }
}

fn build_candidate(record: HomeAssistantEntityRecord) -> MigrationCandidate {
    let domain = record
        .entity_id
        .split_once('.')
        .map(|(domain, _)| domain.to_string())
        .unwrap_or_default();
    let display_name = record
        .name
        .clone()
        .or_else(|| {
            record
                .attributes
                .get("friendly_name")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| record.entity_id.clone());
    let (compatibility, capabilities, safety_class, notes) = classify_domain(&domain);
    let initial_state = map_state(record.state.as_deref());

    MigrationCandidate {
        entity_id: record.entity_id,
        domain,
        display_name,
        compatibility,
        capabilities,
        safety_class,
        initial_state,
        notes,
    }
}

fn classify_domain(
    domain: &str,
) -> (
    MigrationCompatibility,
    BTreeSet<Capability>,
    SafetyClass,
    Vec<String>,
) {
    let mut capabilities = BTreeSet::new();
    let mut notes = Vec::new();
    let compatibility = match domain {
        "light" => {
            capabilities.insert(Capability::Power);
            capabilities.insert(Capability::Brightness);
            capabilities.insert(Capability::Color);
            MigrationCompatibility::Mappable
        }
        "switch" | "fan" => {
            capabilities.insert(Capability::Power);
            MigrationCompatibility::Mappable
        }
        "lock" => {
            capabilities.insert(Capability::Lock);
            notes.push("lock actions require confirmation under Genie safety policy".into());
            MigrationCompatibility::Mappable
        }
        "cover" | "garage_door" => {
            capabilities.insert(Capability::OpenClose);
            notes.push("open/close actions require confirmation when safety-sensitive".into());
            MigrationCompatibility::Mappable
        }
        "scene" => {
            capabilities.insert(Capability::SceneActivation);
            notes.push("scene actions should be reviewed before import".into());
            MigrationCompatibility::ManualReview
        }
        "sensor" | "binary_sensor" => {
            capabilities.insert(Capability::SensorRead);
            notes.push("sensor entities are read-only in the initial Genie model".into());
            MigrationCompatibility::Mappable
        }
        "climate" => {
            capabilities.insert(Capability::Temperature);
            notes.push("HVAC control requires policy review before actuation".into());
            MigrationCompatibility::ManualReview
        }
        _ => {
            notes.push("domain is not mapped by the initial Genie compatibility table".into());
            MigrationCompatibility::Unsupported
        }
    };
    let safety_class = match domain {
        "lock" | "cover" | "garage_door" | "climate" => SafetyClass::Sensitive,
        _ => SafetyClass::Normal,
    };
    (compatibility, capabilities, safety_class, notes)
}

fn map_state(state: Option<&str>) -> EntityState {
    match state.unwrap_or_default() {
        "on" => EntityState::On,
        "off" => EntityState::Off,
        "locked" => EntityState::Locked,
        "unlocked" => EntityState::Unlocked,
        "open" | "opening" => EntityState::Open,
        "closed" | "closing" => EntityState::Closed,
        "unavailable" => EntityState::Unavailable,
        "unknown" | "" => EntityState::Unknown,
        value => value
            .parse::<f64>()
            .map(EntityState::Numeric)
            .unwrap_or_else(|_| EntityState::Text(value.to_string())),
    }
}

fn count_candidates(candidates: &[MigrationCandidate]) -> MigrationCounts {
    let mut counts = MigrationCounts {
        total: candidates.len(),
        mappable: 0,
        manual_review: 0,
        unsupported: 0,
    };
    for candidate in candidates {
        match candidate.compatibility {
            MigrationCompatibility::Mappable => counts.mappable += 1,
            MigrationCompatibility::ManualReview => counts.manual_review += 1,
            MigrationCompatibility::Unsupported => counts.unsupported += 1,
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_home_assistant_states_array() {
        let input = r#"[
            {"entity_id":"light.kitchen","state":"off","attributes":{"friendly_name":"Kitchen Light"}},
            {"entity_id":"lock.front_door","state":"locked","attributes":{}}
        ]"#;

        let records = parse_home_assistant_entities_json(input).unwrap();
        let report = build_home_assistant_migration_report(records);

        assert_eq!(report.counts.total, 2);
        assert_eq!(report.counts.mappable, 2);
        assert_eq!(report.candidates[0].display_name, "Kitchen Light");
        assert_eq!(report.candidates[1].safety_class, SafetyClass::Sensitive);
    }

    #[test]
    fn unsupported_domains_are_reported_not_imported() {
        let records = vec![HomeAssistantEntityRecord {
            entity_id: "vacuum.robot".into(),
            state: Some("docked".into()),
            name: None,
            attributes: BTreeMap::new(),
        }];

        let report = build_home_assistant_migration_report(records);

        assert_eq!(report.counts.unsupported, 1);
        assert_eq!(
            report.candidates[0].compatibility,
            MigrationCompatibility::Unsupported
        );
    }
}
