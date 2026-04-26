use crate::command::{CommandOrigin, HomeAction, HomeActionKind, HomeCommand, TargetSelector};
use crate::entity::{Capability, Entity, EntityGraph, EntityId, EntityState};
use crate::protocol::{CommandResponse, EntitySnapshot, RuntimeRequest, RuntimeResponse};
use crate::safety::{SafetyDecision, SafetyPolicy, evaluate_command};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub entity_count: usize,
    pub audit_count: usize,
    pub safety_policy: SafetyPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub ts: OffsetDateTime,
    pub command: HomeCommand,
    pub decision: SafetyDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeRuntime {
    graph: EntityGraph,
    policy: SafetyPolicy,
    audit: Vec<AuditEntry>,
}

impl HomeRuntime {
    pub fn new(policy: SafetyPolicy) -> Self {
        Self {
            graph: EntityGraph::default(),
            policy,
            audit: Vec::new(),
        }
    }

    pub fn with_default_policy() -> Self {
        Self::new(SafetyPolicy::default())
    }

    pub fn upsert_entity(&mut self, entity: Entity) {
        self.graph.upsert(entity);
    }

    pub fn graph(&self) -> &EntityGraph {
        &self.graph
    }

    pub fn status(&self) -> RuntimeStatus {
        RuntimeStatus {
            entity_count: self.graph.len(),
            audit_count: self.audit.len(),
            safety_policy: self.policy.clone(),
        }
    }

    pub fn audit(&self) -> &[AuditEntry] {
        &self.audit
    }

    pub fn evaluate(&self, command: &HomeCommand) -> SafetyDecision {
        evaluate_command(&self.graph, command, &self.policy)
    }

    pub fn execute(&mut self, command: HomeCommand) -> SafetyDecision {
        let decision = self.evaluate(&command);
        if decision.allowed {
            self.apply_state_change(&command);
        }
        self.audit.push(AuditEntry {
            ts: OffsetDateTime::now_utc(),
            command,
            decision: decision.clone(),
        });
        decision
    }

    pub fn handle_request(&mut self, request: RuntimeRequest) -> RuntimeResponse {
        match request {
            RuntimeRequest::Status => RuntimeResponse::Status {
                status: self.status(),
            },
            RuntimeRequest::ListEntities => RuntimeResponse::Entities {
                entities: self.graph.entities().map(EntitySnapshot::from).collect(),
            },
            RuntimeRequest::Evaluate { command } => {
                let decision = self.evaluate(&command);
                RuntimeResponse::Command {
                    result: CommandResponse {
                        decision,
                        executed: false,
                    },
                }
            }
            RuntimeRequest::Execute { command } => {
                let decision = self.execute(command);
                RuntimeResponse::Command {
                    result: CommandResponse {
                        executed: decision.allowed,
                        decision,
                    },
                }
            }
        }
    }

    pub fn handle_request_json(&mut self, input: &str) -> String {
        let response = match serde_json::from_str::<RuntimeRequest>(input) {
            Ok(request) => self.handle_request(request),
            Err(err) => RuntimeResponse::Error {
                error: format!("invalid runtime request: {err}"),
            },
        };
        serde_json::to_string(&response).unwrap_or_else(|err| {
            serde_json::json!({
                "type": "error",
                "error": format!("failed to serialize runtime response: {err}")
            })
            .to_string()
        })
    }

    fn apply_state_change(&mut self, command: &HomeCommand) {
        let Some(current) = self.graph.get(&command.action.target.entity_id).cloned() else {
            return;
        };
        let next_state = match command.action.kind {
            HomeActionKind::TurnOn => EntityState::On,
            HomeActionKind::TurnOff => EntityState::Off,
            HomeActionKind::Lock => EntityState::Locked,
            HomeActionKind::Unlock => EntityState::Unlocked,
            HomeActionKind::Open => EntityState::Open,
            HomeActionKind::Close => EntityState::Closed,
            HomeActionKind::Toggle => match &current.state {
                EntityState::On => EntityState::Off,
                EntityState::Off => EntityState::On,
                other => other.clone(),
            },
            HomeActionKind::SetValue | HomeActionKind::ActivateScene => current.state.clone(),
        };
        self.graph.upsert(current.with_state(next_state));
    }
}

pub fn demo_runtime() -> HomeRuntime {
    let mut runtime = HomeRuntime::with_default_policy();
    let kitchen_light = EntityId::new("light.kitchen").expect("valid demo entity id");
    let front_door = EntityId::new("lock.front_door").expect("valid demo entity id");
    runtime.upsert_entity(
        Entity::new(kitchen_light, "Kitchen Light")
            .with_area("kitchen")
            .with_state(EntityState::Off)
            .with_capability(Capability::Power),
    );
    runtime.upsert_entity(
        Entity::new(front_door, "Front Door")
            .with_area("entry")
            .with_state(EntityState::Locked)
            .with_capability(Capability::Lock),
    );
    runtime
}

pub fn demo_turn_on_kitchen_command() -> HomeCommand {
    HomeCommand::new(
        CommandOrigin::Voice,
        HomeAction {
            target: TargetSelector::exact(
                EntityId::new("light.kitchen").expect("valid demo entity id"),
            ),
            kind: HomeActionKind::TurnOn,
            value: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityId;

    #[test]
    fn executes_allowed_action_and_records_audit() {
        let id = EntityId::new("light.kitchen").unwrap();
        let mut runtime = HomeRuntime::with_default_policy();
        runtime.upsert_entity(
            Entity::new(id.clone(), "Kitchen Light")
                .with_state(EntityState::Off)
                .with_capability(Capability::Power),
        );
        let decision = runtime.execute(HomeCommand::new(
            CommandOrigin::Dashboard,
            HomeAction {
                target: TargetSelector::exact(id.clone()),
                kind: HomeActionKind::TurnOn,
                value: None,
            },
        ));

        assert!(decision.allowed);
        assert_eq!(runtime.audit().len(), 1);
        assert_eq!(runtime.graph().get(&id).unwrap().state, EntityState::On);
    }

    #[test]
    fn handle_evaluate_request_does_not_mutate_state() {
        let id = EntityId::new("light.kitchen").unwrap();
        let mut runtime = demo_runtime();
        let response = runtime.handle_request(RuntimeRequest::Evaluate {
            command: demo_turn_on_kitchen_command(),
        });

        let RuntimeResponse::Command { result } = response else {
            panic!("expected command response");
        };
        assert!(result.decision.allowed);
        assert!(!result.executed);
        assert_eq!(runtime.audit().len(), 0);
        assert_eq!(runtime.graph().get(&id).unwrap().state, EntityState::Off);
    }

    #[test]
    fn handle_execute_request_mutates_state_and_audits() {
        let id = EntityId::new("light.kitchen").unwrap();
        let mut runtime = demo_runtime();
        let response = runtime.handle_request(RuntimeRequest::Execute {
            command: demo_turn_on_kitchen_command(),
        });

        let RuntimeResponse::Command { result } = response else {
            panic!("expected command response");
        };
        assert!(result.executed);
        assert_eq!(runtime.audit().len(), 1);
        assert_eq!(runtime.graph().get(&id).unwrap().state, EntityState::On);
    }

    #[test]
    fn handle_request_json_reports_invalid_input() {
        let mut runtime = demo_runtime();
        let output = runtime.handle_request_json("{not json");
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["type"], "error");
        assert!(
            parsed["error"]
                .as_str()
                .unwrap()
                .contains("invalid runtime request")
        );
    }
}
