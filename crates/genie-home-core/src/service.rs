use crate::command::{CommandOrigin, HomeAction, HomeActionKind, HomeCommand, TargetSelector};
use crate::entity::{EntityGraph, EntityId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceSpec {
    pub domain: String,
    pub service: String,
    pub description: String,
    pub action_kind: Option<HomeActionKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceCall {
    pub domain: String,
    pub service: String,
    pub target: ServiceTarget,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default = "default_service_origin")]
    pub origin: CommandOrigin,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceTarget {
    pub entity_ids: Vec<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceCallResult {
    pub domain: String,
    pub service: String,
    pub targets: usize,
    pub executed: usize,
    pub results: Vec<ServiceActionResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceActionResult {
    pub entity_id: EntityId,
    pub executed: bool,
    pub decision: crate::SafetyDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ServiceCallError {
    #[error("unsupported service: {domain}.{service}")]
    UnsupportedService { domain: String, service: String },
    #[error("service call requires at least one target")]
    EmptyTarget,
    #[error("target domain mismatch: service domain {domain}, entity {entity_id}")]
    TargetDomainMismatch { domain: String, entity_id: EntityId },
    #[error("target entity is unknown: {entity_id}")]
    UnknownTarget { entity_id: EntityId },
}

pub fn default_service_origin() -> CommandOrigin {
    CommandOrigin::LocalApi
}

pub fn service_specs() -> Vec<ServiceSpec> {
    vec![
        spec(
            "light",
            "turn_on",
            "Turn on a light",
            HomeActionKind::TurnOn,
        ),
        spec(
            "light",
            "turn_off",
            "Turn off a light",
            HomeActionKind::TurnOff,
        ),
        spec("light", "toggle", "Toggle a light", HomeActionKind::Toggle),
        spec(
            "switch",
            "turn_on",
            "Turn on a switch",
            HomeActionKind::TurnOn,
        ),
        spec(
            "switch",
            "turn_off",
            "Turn off a switch",
            HomeActionKind::TurnOff,
        ),
        spec("fan", "turn_on", "Turn on a fan", HomeActionKind::TurnOn),
        spec("fan", "turn_off", "Turn off a fan", HomeActionKind::TurnOff),
        spec("lock", "lock", "Lock a lock entity", HomeActionKind::Lock),
        spec(
            "lock",
            "unlock",
            "Unlock a lock entity",
            HomeActionKind::Unlock,
        ),
        spec(
            "cover",
            "open_cover",
            "Open a cover entity",
            HomeActionKind::Open,
        ),
        spec(
            "cover",
            "close_cover",
            "Close a cover entity",
            HomeActionKind::Close,
        ),
        spec(
            "scene",
            "turn_on",
            "Activate a scene",
            HomeActionKind::ActivateScene,
        ),
        spec(
            "climate",
            "set_temperature",
            "Set a climate target temperature",
            HomeActionKind::SetValue,
        ),
    ]
}

pub fn service_call_to_commands(
    graph: &EntityGraph,
    call: &ServiceCall,
) -> Result<Vec<HomeCommand>, ServiceCallError> {
    let action_kind = service_to_action(&call.domain, &call.service).ok_or_else(|| {
        ServiceCallError::UnsupportedService {
            domain: call.domain.clone(),
            service: call.service.clone(),
        }
    })?;
    if call.target.entity_ids.is_empty() {
        return Err(ServiceCallError::EmptyTarget);
    }

    let mut commands = Vec::new();
    for entity_id in &call.target.entity_ids {
        if entity_id.domain() != call.domain {
            return Err(ServiceCallError::TargetDomainMismatch {
                domain: call.domain.clone(),
                entity_id: entity_id.clone(),
            });
        }
        if !graph.contains(entity_id) {
            return Err(ServiceCallError::UnknownTarget {
                entity_id: entity_id.clone(),
            });
        }
        let mut command = HomeCommand::new(
            call.origin,
            HomeAction {
                target: TargetSelector::exact(entity_id.clone()),
                kind: action_kind.clone(),
                value: service_value(&call.domain, &call.service, &call.data),
            },
        );
        if call.confirmed {
            command = command.confirmed();
        }
        commands.push(command);
    }
    Ok(commands)
}

fn spec(
    domain: &str,
    service: &str,
    description: &str,
    action_kind: HomeActionKind,
) -> ServiceSpec {
    ServiceSpec {
        domain: domain.into(),
        service: service.into(),
        description: description.into(),
        action_kind: Some(action_kind),
    }
}

fn service_to_action(domain: &str, service: &str) -> Option<HomeActionKind> {
    match (domain, service) {
        ("light" | "switch" | "fan", "turn_on") => Some(HomeActionKind::TurnOn),
        ("light" | "switch" | "fan", "turn_off") => Some(HomeActionKind::TurnOff),
        ("light" | "switch" | "fan", "toggle") => Some(HomeActionKind::Toggle),
        ("lock", "lock") => Some(HomeActionKind::Lock),
        ("lock", "unlock") => Some(HomeActionKind::Unlock),
        ("cover", "open_cover") => Some(HomeActionKind::Open),
        ("cover", "close_cover") => Some(HomeActionKind::Close),
        ("scene", "turn_on") => Some(HomeActionKind::ActivateScene),
        ("climate", "set_temperature") => Some(HomeActionKind::SetValue),
        _ => None,
    }
}

fn service_value(
    domain: &str,
    service: &str,
    data: &serde_json::Value,
) -> Option<serde_json::Value> {
    match (domain, service) {
        ("climate", "set_temperature") => data.get("temperature").cloned(),
        ("light", "turn_on") => {
            if data.is_object() && !data.as_object().unwrap().is_empty() {
                Some(data.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Capability, Entity, EntityState};

    #[test]
    fn translates_light_service_to_command() {
        let id = EntityId::new("light.kitchen").unwrap();
        let mut graph = EntityGraph::default();
        graph.upsert(
            Entity::new(id.clone(), "Kitchen Light")
                .with_state(EntityState::Off)
                .with_capability(Capability::Power),
        );
        let call = ServiceCall {
            domain: "light".into(),
            service: "turn_on".into(),
            target: ServiceTarget {
                entity_ids: vec![id],
            },
            data: serde_json::Value::Null,
            origin: CommandOrigin::LocalApi,
            confirmed: false,
        };

        let commands = service_call_to_commands(&graph, &call).unwrap();

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action.kind, HomeActionKind::TurnOn);
    }

    #[test]
    fn rejects_domain_mismatch() {
        let id = EntityId::new("lock.front_door").unwrap();
        let mut graph = EntityGraph::default();
        graph.upsert(Entity::new(id.clone(), "Front Door").with_state(EntityState::Locked));
        let call = ServiceCall {
            domain: "light".into(),
            service: "turn_on".into(),
            target: ServiceTarget {
                entity_ids: vec![id],
            },
            data: serde_json::Value::Null,
            origin: CommandOrigin::LocalApi,
            confirmed: false,
        };

        assert!(matches!(
            service_call_to_commands(&graph, &call),
            Err(ServiceCallError::TargetDomainMismatch { .. })
        ));
    }
}
