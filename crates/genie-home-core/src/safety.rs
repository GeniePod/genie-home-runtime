use crate::command::{CommandOrigin, HomeActionKind, HomeCommand};
use crate::entity::{Capability, Entity, EntityGraph, EntityState, SafetyClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyPolicy {
    pub min_target_confidence: f32,
    pub require_available_state: bool,
    pub require_confirmation_for_sensitive_indirect: bool,
    pub require_structured_approval_for_indirect: bool,
    pub allow_critical_without_confirmation: bool,
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        Self {
            min_target_confidence: 0.78,
            require_available_state: true,
            require_confirmation_for_sensitive_indirect: true,
            require_structured_approval_for_indirect: true,
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
    ApprovalRequired,
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

    if entity.safety_class == SafetyClass::Critical && !policy.allow_critical_without_confirmation {
        if !command.confirmed {
            return SafetyDecision::block(
                SafetyReason::CriticalActionBlocked,
                "critical target requires a stronger runtime approval path",
            );
        }
        if !has_structured_approval(command) {
            return SafetyDecision::block(
                SafetyReason::ApprovalRequired,
                "critical target requires a scoped approval token",
            );
        }
    }

    if should_require_confirmation(entity, command, policy) {
        return SafetyDecision::require_confirmation(
            "sensitive physical action requires confirmation",
        );
    }

    if command.confirmed
        && policy.require_structured_approval_for_indirect
        && is_indirect_origin(command.origin)
        && action_needs_confirmation(entity, command, policy)
        && !has_structured_approval(command)
    {
        return SafetyDecision::block(
            SafetyReason::ApprovalRequired,
            "indirect sensitive action requires a scoped approval token",
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
    action_needs_confirmation(entity, command, policy)
}

fn action_needs_confirmation(
    entity: &Entity,
    command: &HomeCommand,
    policy: &SafetyPolicy,
) -> bool {
    if entity.safety_class == SafetyClass::Sensitive {
        return true;
    }
    policy.require_confirmation_for_sensitive_indirect
        && command.action.kind.is_sensitive()
        && (matches!(command.origin, CommandOrigin::Voice) || is_indirect_origin(command.origin))
}

fn has_structured_approval(command: &HomeCommand) -> bool {
    command
        .approval
        .as_ref()
        .is_some_and(|approval| approval.is_valid_for(&command.action))
}

fn is_indirect_origin(origin: CommandOrigin) -> bool {
    matches!(
        origin,
        CommandOrigin::Agent
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
        HomeActionKind::Start | HomeActionKind::Stop | HomeActionKind::Pause => {
            entity.capabilities.contains(&Capability::MediaPlayback)
                || entity.capabilities.contains(&Capability::VacuumControl)
                || entity.capabilities.contains(&Capability::OpenClose)
        }
        HomeActionKind::ReturnToBase => entity.capabilities.contains(&Capability::VacuumControl),
        HomeActionKind::Arm | HomeActionKind::Disarm => {
            entity.capabilities.contains(&Capability::AlarmControl)
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
    fn requires_confirmation_for_alarm_disarm() {
        let id = EntityId::new("alarm_control_panel.home").unwrap();
        let graph = graph_with(
            Entity::new(id.clone(), "Home Alarm")
                .with_state(EntityState::Text("armed".into()))
                .with_capability(Capability::AlarmControl)
                .with_safety_class(SafetyClass::Sensitive),
        );
        let command = HomeCommand::new(
            CommandOrigin::Agent,
            HomeAction {
                target: TargetSelector::exact(id),
                kind: HomeActionKind::Disarm,
                value: None,
            },
        );

        let decision = evaluate_command(&graph, &command, &SafetyPolicy::default());
        assert!(!decision.allowed);
        assert!(decision.requires_confirmation);
    }

    #[test]
    fn confirmed_indirect_sensitive_action_requires_scoped_approval() {
        let id = EntityId::new("lock.front_door").unwrap();
        let graph = graph_with(
            Entity::new(id.clone(), "Front Door")
                .with_state(EntityState::Locked)
                .with_capability(Capability::Lock),
        );
        let command = HomeCommand::new(
            CommandOrigin::LocalApi,
            HomeAction {
                target: TargetSelector::exact(id),
                kind: HomeActionKind::Unlock,
                value: None,
            },
        )
        .confirmed();

        let decision = evaluate_command(&graph, &command, &SafetyPolicy::default());
        assert!(!decision.allowed);
        assert_eq!(decision.reason, SafetyReason::ApprovalRequired);
    }

    #[test]
    fn scoped_approval_allows_indirect_sensitive_action() {
        let id = EntityId::new("lock.front_door").unwrap();
        let graph = graph_with(
            Entity::new(id.clone(), "Front Door")
                .with_state(EntityState::Locked)
                .with_capability(Capability::Lock),
        );
        let command = HomeCommand::new(
            CommandOrigin::LocalApi,
            HomeAction {
                target: TargetSelector::exact(id),
                kind: HomeActionKind::Unlock,
                value: None,
            },
        )
        .approved("approval-1", "dashboard");

        let decision = evaluate_command(&graph, &command, &SafetyPolicy::default());
        assert!(decision.allowed);
    }

    #[test]
    fn mismatched_scoped_approval_is_rejected() {
        let id = EntityId::new("lock.front_door").unwrap();
        let other = EntityId::new("lock.back_door").unwrap();
        let graph = graph_with(
            Entity::new(id.clone(), "Front Door")
                .with_state(EntityState::Locked)
                .with_capability(Capability::Lock),
        );
        let mut command = HomeCommand::new(
            CommandOrigin::LocalApi,
            HomeAction {
                target: TargetSelector::exact(id),
                kind: HomeActionKind::Unlock,
                value: None,
            },
        )
        .approved("approval-1", "dashboard");
        command.approval.as_mut().unwrap().entity_id = other;

        let decision = evaluate_command(&graph, &command, &SafetyPolicy::default());
        assert!(!decision.allowed);
        assert_eq!(decision.reason, SafetyReason::ApprovalRequired);
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
