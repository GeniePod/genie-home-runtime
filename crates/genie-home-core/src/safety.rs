use crate::command::{CommandOrigin, HomeActionKind, HomeCommand};
use crate::entity::{Capability, Entity, EntityGraph, EntityState, SafetyClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyPolicy {
    pub min_target_confidence: f32,
    pub require_available_state: bool,
    pub require_confirmation_for_sensitive_indirect: bool,
    pub allow_critical_without_confirmation: bool,
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        Self {
            min_target_confidence: 0.78,
            require_available_state: true,
            require_confirmation_for_sensitive_indirect: true,
            allow_critical_without_confirmation: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyReason {
    Allowed,
    UnknownTarget,
    LowTargetConfidence,
    TargetUnavailable,
    UnsupportedCapability,
    ConfirmationRequired,
    CriticalActionBlocked,
    SceneDefinitionMissing,
    SceneActionBlocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyDecision {
    pub allowed: bool,
    pub requires_confirmation: bool,
    pub reason: SafetyReason,
    pub message: String,
}

impl SafetyDecision {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            requires_confirmation: false,
            reason: SafetyReason::Allowed,
            message: "allowed".into(),
        }
    }

    pub fn require_confirmation(message: impl Into<String>) -> Self {
        Self {
            allowed: false,
            requires_confirmation: true,
            reason: SafetyReason::ConfirmationRequired,
            message: message.into(),
        }
    }

    pub fn block(reason: SafetyReason, message: impl Into<String>) -> Self {
        Self {
            allowed: false,
            requires_confirmation: false,
            reason,
            message: message.into(),
        }
    }
}

pub fn evaluate_command(
    graph: &EntityGraph,
    command: &HomeCommand,
    policy: &SafetyPolicy,
) -> SafetyDecision {
    if command.action.target.confidence < policy.min_target_confidence {
        return SafetyDecision::block(
            SafetyReason::LowTargetConfidence,
            "target confidence is below runtime threshold",
        );
    }

    let Some(entity) = graph.get(&command.action.target.entity_id) else {
        return SafetyDecision::block(SafetyReason::UnknownTarget, "target entity is unknown");
    };

    if policy.require_available_state && !entity.state.is_available() {
        return SafetyDecision::block(
            SafetyReason::TargetUnavailable,
            "target state is unavailable or unknown",
        );
    }

    if !entity_supports_action(entity, &command.action.kind) {
        return SafetyDecision::block(
            SafetyReason::UnsupportedCapability,
            "target does not expose the required capability",
        );
    }

    if entity.safety_class == SafetyClass::Critical
        && !command.confirmed
        && !policy.allow_critical_without_confirmation
    {
        return SafetyDecision::block(
            SafetyReason::CriticalActionBlocked,
            "critical target requires a stronger runtime approval path",
        );
    }

    if should_require_confirmation(entity, command, policy) {
        return SafetyDecision::require_confirmation(
            "sensitive physical action requires confirmation",
        );
    }

    SafetyDecision::allow()
}

fn should_require_confirmation(
    entity: &Entity,
    command: &HomeCommand,
    policy: &SafetyPolicy,
) -> bool {
    if command.confirmed {
        return false;
    }
    if entity.safety_class == SafetyClass::Sensitive {
        return true;
    }
    if !policy.require_confirmation_for_sensitive_indirect {
        return false;
    }
    command.action.kind.is_sensitive()
        && matches!(
            command.origin,
            CommandOrigin::Voice
                | CommandOrigin::Agent
                | CommandOrigin::Automation
                | CommandOrigin::Schedule
                | CommandOrigin::Bridge
                | CommandOrigin::LocalApi
        )
}

fn entity_supports_action(entity: &Entity, action: &HomeActionKind) -> bool {
    match action {
        HomeActionKind::TurnOn | HomeActionKind::TurnOff | HomeActionKind::Toggle => {
            entity.capabilities.contains(&Capability::Power)
        }
        HomeActionKind::SetValue => {
            entity.capabilities.contains(&Capability::Brightness)
                || entity.capabilities.contains(&Capability::Temperature)
        }
        HomeActionKind::Lock | HomeActionKind::Unlock => {
            entity.capabilities.contains(&Capability::Lock)
        }
        HomeActionKind::Open | HomeActionKind::Close => {
            entity.capabilities.contains(&Capability::OpenClose)
        }
        HomeActionKind::ActivateScene => entity.capabilities.contains(&Capability::SceneActivation),
    }
}

#[allow(dead_code)]
fn _state_is_mutable(_state: &EntityState) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{HomeAction, TargetSelector};
    use crate::entity::{Capability, Entity, EntityId, EntityState};

    fn graph_with(entity: Entity) -> EntityGraph {
        let mut graph = EntityGraph::default();
        graph.upsert(entity);
        graph
    }

    #[test]
    fn allows_normal_light_action() {
        let id = EntityId::new("light.kitchen").unwrap();
        let graph = graph_with(
            Entity::new(id.clone(), "Kitchen Light")
                .with_state(EntityState::Off)
                .with_capability(Capability::Power),
        );
        let command = HomeCommand::new(
            CommandOrigin::Voice,
            HomeAction {
                target: TargetSelector::exact(id),
                kind: HomeActionKind::TurnOn,
                value: None,
            },
        );

        assert!(evaluate_command(&graph, &command, &SafetyPolicy::default()).allowed);
    }

    #[test]
    fn requires_confirmation_for_voice_unlock() {
        let id = EntityId::new("lock.front_door").unwrap();
        let graph = graph_with(
            Entity::new(id.clone(), "Front Door")
                .with_state(EntityState::Locked)
                .with_capability(Capability::Lock),
        );
        let command = HomeCommand::new(
            CommandOrigin::Voice,
            HomeAction {
                target: TargetSelector::exact(id),
                kind: HomeActionKind::Unlock,
                value: None,
            },
        );

        let decision = evaluate_command(&graph, &command, &SafetyPolicy::default());
        assert!(!decision.allowed);
        assert!(decision.requires_confirmation);
    }

    #[test]
    fn requires_confirmation_for_automation_unlock() {
        let id = EntityId::new("lock.front_door").unwrap();
        let graph = graph_with(
            Entity::new(id.clone(), "Front Door")
                .with_state(EntityState::Locked)
                .with_capability(Capability::Lock),
        );
        let command = HomeCommand::new(
            CommandOrigin::Automation,
            HomeAction {
                target: TargetSelector::exact(id),
                kind: HomeActionKind::Unlock,
                value: None,
            },
        );

        let decision = evaluate_command(&graph, &command, &SafetyPolicy::default());
        assert!(!decision.allowed);
        assert!(decision.requires_confirmation);
    }

    #[test]
    fn blocks_low_confidence_target() {
        let id = EntityId::new("light.kitchen").unwrap();
        let graph = graph_with(
            Entity::new(id.clone(), "Kitchen Light")
                .with_state(EntityState::Off)
                .with_capability(Capability::Power),
        );
        let command = HomeCommand::new(
            CommandOrigin::Agent,
            HomeAction {
                target: TargetSelector {
                    entity_id: id,
                    confidence: 0.4,
                },
                kind: HomeActionKind::TurnOn,
                value: None,
            },
        );

        assert_eq!(
            evaluate_command(&graph, &command, &SafetyPolicy::default()).reason,
            SafetyReason::LowTargetConfidence
        );
    }
}
