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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainSupport {
    pub domain: String,
    pub support_level: DomainSupportLevel,
    pub services: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainSupportLevel {
    SafetyGatedActuation,
    ReadOnlyViaEntityState,
    Planned,
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
        spec(
            "switch",
            "toggle",
            "Toggle a switch",
            HomeActionKind::Toggle,
        ),
        spec("fan", "turn_on", "Turn on a fan", HomeActionKind::TurnOn),
        spec("fan", "turn_off", "Turn off a fan", HomeActionKind::TurnOff),
        spec("fan", "toggle", "Toggle a fan", HomeActionKind::Toggle),
        spec(
            "fan",
            "set_percentage",
            "Set fan percentage",
            HomeActionKind::SetValue,
        ),
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
            "cover",
            "stop_cover",
            "Stop a cover entity",
            HomeActionKind::Stop,
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
        spec(
            "media_player",
            "turn_on",
            "Turn on a media player",
            HomeActionKind::TurnOn,
        ),
        spec(
            "media_player",
            "turn_off",
            "Turn off a media player",
            HomeActionKind::TurnOff,
        ),
        spec(
            "media_player",
            "media_play",
            "Start media playback",
            HomeActionKind::Start,
        ),
        spec(
            "media_player",
            "media_pause",
            "Pause media playback",
            HomeActionKind::Pause,
        ),
        spec(
            "media_player",
            "media_stop",
            "Stop media playback",
            HomeActionKind::Stop,
        ),
        spec(
            "media_player",
            "volume_set",
            "Set media player volume",
            HomeActionKind::SetValue,
        ),
        spec("vacuum", "start", "Start a vacuum", HomeActionKind::Start),
        spec("vacuum", "stop", "Stop a vacuum", HomeActionKind::Stop),
        spec(
            "vacuum",
            "return_to_base",
            "Return a vacuum to base",
            HomeActionKind::ReturnToBase,
        ),
        spec(
            "alarm_control_panel",
            "alarm_arm_home",
            "Arm alarm in home mode",
            HomeActionKind::Arm,
        ),
        spec(
            "alarm_control_panel",
            "alarm_arm_away",
            "Arm alarm in away mode",
            HomeActionKind::Arm,
        ),
        spec(
            "alarm_control_panel",
            "alarm_disarm",
            "Disarm alarm",
            HomeActionKind::Disarm,
        ),
    ]
}

pub fn domain_support_matrix() -> Vec<DomainSupport> {
    let mut domains = std::collections::BTreeMap::<String, Vec<String>>::new();
    for spec in service_specs() {
        domains.entry(spec.domain).or_default().push(spec.service);
    }

    let mut supported = domains
        .into_iter()
        .map(|(domain, services)| DomainSupport {
            notes: domain_notes(&domain),
            domain,
            support_level: DomainSupportLevel::SafetyGatedActuation,
            services,
        })
        .collect::<Vec<_>>();

    supported.extend([
        planned_domain(
            "sensor",
            DomainSupportLevel::ReadOnlyViaEntityState,
            "Sensor entities are represented as state snapshots. Direct actuation is intentionally unavailable.",
        ),
        planned_domain(
            "binary_sensor",
            DomainSupportLevel::ReadOnlyViaEntityState,
            "Binary sensors are represented as state snapshots. Direct actuation is intentionally unavailable.",
        ),
    ]);
    supported.sort_by(|left, right| left.domain.cmp(&right.domain));
    supported
}

fn planned_domain(domain: &str, support_level: DomainSupportLevel, note: &str) -> DomainSupport {
    DomainSupport {
        domain: domain.into(),
        support_level,
        services: Vec::new(),
        notes: vec![note.into()],
    }
}

fn domain_notes(domain: &str) -> Vec<String> {
    match domain {
        "lock" => vec!["Unlock requires confirmation for unsafe origins.".into()],
        "cover" => vec!["Open/close actions are treated as sensitive physical actuation.".into()],
        "climate" => {
            vec!["Temperature setpoint is supported; full HVAC mode policy is planned.".into()]
        }
        "scene" => {
            vec!["Scene activation evaluates every nested action before any mutation.".into()]
        }
        "media_player" => {
            vec!["Media control is supported for basic power/playback/volume only.".into()]
        }
        "vacuum" => {
            vec!["Vacuum movement commands are sensitive and require confirmation for unsafe origins.".into()]
        }
        "alarm_control_panel" => {
            vec!["Alarm arm/disarm commands are sensitive and require confirmation for unsafe origins.".into()]
        }
        _ => {
            vec!["Direct service calls are translated into Genie commands and safety-gated.".into()]
        }
    }
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
        ("fan", "set_percentage") => Some(HomeActionKind::SetValue),
        ("lock", "lock") => Some(HomeActionKind::Lock),
        ("lock", "unlock") => Some(HomeActionKind::Unlock),
        ("cover", "open_cover") => Some(HomeActionKind::Open),
        ("cover", "close_cover") => Some(HomeActionKind::Close),
        ("cover", "stop_cover") => Some(HomeActionKind::Stop),
        ("scene", "turn_on") => Some(HomeActionKind::ActivateScene),
        ("climate", "set_temperature") => Some(HomeActionKind::SetValue),
        ("media_player", "turn_on") => Some(HomeActionKind::TurnOn),
        ("media_player", "turn_off") => Some(HomeActionKind::TurnOff),
        ("media_player", "media_play") => Some(HomeActionKind::Start),
        ("media_player", "media_pause") => Some(HomeActionKind::Pause),
        ("media_player", "media_stop") => Some(HomeActionKind::Stop),
        ("media_player", "volume_set") => Some(HomeActionKind::SetValue),
        ("vacuum", "start") => Some(HomeActionKind::Start),
        ("vacuum", "stop") => Some(HomeActionKind::Stop),
        ("vacuum", "return_to_base") => Some(HomeActionKind::ReturnToBase),
        ("alarm_control_panel", "alarm_arm_home" | "alarm_arm_away") => Some(HomeActionKind::Arm),
        ("alarm_control_panel", "alarm_disarm") => Some(HomeActionKind::Disarm),
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
        ("fan", "set_percentage") => data.get("percentage").cloned(),
        ("media_player", "volume_set") => data.get("volume_level").cloned(),
        ("alarm_control_panel", "alarm_arm_home" | "alarm_arm_away" | "alarm_disarm") => {
            data.get("code").cloned()
        }
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

    #[test]
    fn domain_matrix_reports_supported_and_planned_domains() {
        let matrix = domain_support_matrix();
        let light = matrix
            .iter()
            .find(|domain| domain.domain == "light")
            .unwrap();
        let alarm = matrix
            .iter()
            .find(|domain| domain.domain == "alarm_control_panel")
            .unwrap();

        assert_eq!(
            light.support_level,
            DomainSupportLevel::SafetyGatedActuation
        );
        assert!(light.services.contains(&"turn_on".into()));
        assert_eq!(
            alarm.support_level,
            DomainSupportLevel::SafetyGatedActuation
        );
        assert!(alarm.services.contains(&"alarm_disarm".into()));
    }

    #[test]
    fn translates_media_and_vacuum_services() {
        let media_id = EntityId::new("media_player.living_room").unwrap();
        let vacuum_id = EntityId::new("vacuum.robot").unwrap();
        let mut graph = EntityGraph::default();
        graph.upsert(
            Entity::new(media_id.clone(), "Living Room Media")
                .with_state(EntityState::Off)
                .with_capability(Capability::Power)
                .with_capability(Capability::MediaPlayback),
        );
        graph.upsert(
            Entity::new(vacuum_id.clone(), "Robot Vacuum")
                .with_state(EntityState::Off)
                .with_capability(Capability::VacuumControl),
        );

        let media = service_call_to_commands(
            &graph,
            &ServiceCall {
                domain: "media_player".into(),
                service: "media_play".into(),
                target: ServiceTarget {
                    entity_ids: vec![media_id],
                },
                data: serde_json::Value::Null,
                origin: CommandOrigin::LocalApi,
                confirmed: false,
            },
        )
        .unwrap();
        let vacuum = service_call_to_commands(
            &graph,
            &ServiceCall {
                domain: "vacuum".into(),
                service: "return_to_base".into(),
                target: ServiceTarget {
                    entity_ids: vec![vacuum_id],
                },
                data: serde_json::Value::Null,
                origin: CommandOrigin::LocalApi,
                confirmed: true,
            },
        )
        .unwrap();

        assert_eq!(media[0].action.kind, HomeActionKind::Start);
        assert_eq!(vacuum[0].action.kind, HomeActionKind::ReturnToBase);
    }
}
