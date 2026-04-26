use crate::automation::{
    Automation, AutomationBlock, AutomationCondition, AutomationTickResult, AutomationTrigger,
};
use crate::command::{CommandOrigin, HomeAction, HomeActionKind, HomeCommand, TargetSelector};
use crate::connectivity::{ConnectivityApplyResult, ConnectivityReport};
use crate::device::{Device, DeviceId, DeviceRegistry};
use crate::entity::{Capability, Entity, EntityGraph, EntityId, EntityState};
use crate::event::{RuntimeEvent, RuntimeEventKind};
use crate::protocol::{
    CommandResponse, DeviceSnapshot, EntitySnapshot, RuntimeRequest, RuntimeResponse,
};
use crate::safety::{SafetyDecision, SafetyPolicy, SafetyReason, evaluate_command};
use crate::scene::Scene;
use crate::service::{
    ServiceActionResult, ServiceCall, ServiceCallResult, service_call_to_commands, service_specs,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub device_count: usize,
    pub entity_count: usize,
    pub scene_count: usize,
    pub automation_count: usize,
    pub audit_count: usize,
    pub event_count: usize,
    pub safety_policy: SafetyPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEntry {
    #[serde(with = "time::serde::rfc3339")]
    pub ts: OffsetDateTime,
    pub command: HomeCommand,
    pub decision: SafetyDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeRuntime {
    devices: DeviceRegistry,
    graph: EntityGraph,
    scenes: BTreeMap<EntityId, Scene>,
    automations: BTreeMap<String, Automation>,
    policy: SafetyPolicy,
    audit: Vec<AuditEntry>,
    events: Vec<RuntimeEvent>,
}

impl HomeRuntime {
    pub fn new(policy: SafetyPolicy) -> Self {
        Self {
            graph: EntityGraph::default(),
            devices: DeviceRegistry::default(),
            scenes: BTreeMap::new(),
            automations: BTreeMap::new(),
            policy,
            audit: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn with_default_policy() -> Self {
        Self::new(SafetyPolicy::default())
    }

    pub fn upsert_entity(&mut self, entity: Entity) {
        self.graph.upsert(entity);
    }

    pub fn upsert_device(&mut self, device: Device) {
        self.devices.upsert(device);
    }

    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.devices()
    }

    pub fn upsert_scene(&mut self, scene: Scene) {
        self.scenes.insert(scene.id.clone(), scene);
    }

    pub fn upsert_automation(&mut self, automation: Automation) {
        self.automations.insert(automation.id.clone(), automation);
    }

    pub fn graph(&self) -> &EntityGraph {
        &self.graph
    }

    pub fn scenes(&self) -> impl Iterator<Item = &Scene> {
        self.scenes.values()
    }

    pub fn automations(&self) -> impl Iterator<Item = &Automation> {
        self.automations.values()
    }

    pub fn status(&self) -> RuntimeStatus {
        RuntimeStatus {
            device_count: self.devices.len(),
            entity_count: self.graph.len(),
            scene_count: self.scenes.len(),
            automation_count: self.automations.len(),
            audit_count: self.audit.len(),
            event_count: self.events.len(),
            safety_policy: self.policy.clone(),
        }
    }

    pub fn audit(&self) -> &[AuditEntry] {
        &self.audit
    }

    pub fn events(&self) -> &[RuntimeEvent] {
        &self.events
    }

    pub fn restore_events(&mut self, entries: impl IntoIterator<Item = RuntimeEvent>) {
        self.events.extend(entries);
    }

    pub fn restore_audit_entries(&mut self, entries: impl IntoIterator<Item = AuditEntry>) {
        self.audit.extend(entries);
    }

    pub fn evaluate(&self, command: &HomeCommand) -> SafetyDecision {
        evaluate_command(&self.graph, command, &self.policy)
    }

    pub fn execute(&mut self, command: HomeCommand) -> SafetyDecision {
        let decision = if command.action.kind == HomeActionKind::ActivateScene {
            self.evaluate_scene_command(&command)
        } else {
            self.evaluate(&command)
        };
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
            RuntimeRequest::ListDevices => RuntimeResponse::Devices {
                devices: self.devices().map(DeviceSnapshot::from).collect(),
            },
            RuntimeRequest::ListEntities => RuntimeResponse::Entities {
                entities: self.graph.entities().map(EntitySnapshot::from).collect(),
            },
            RuntimeRequest::ListAutomations => RuntimeResponse::Automations {
                automations: self.automations().cloned().collect(),
            },
            RuntimeRequest::ListServices => RuntimeResponse::Services {
                services: service_specs(),
            },
            RuntimeRequest::Audit { limit } => RuntimeResponse::Audit {
                entries: self.recent_audit(limit.unwrap_or(20)),
            },
            RuntimeRequest::Events { limit } => RuntimeResponse::Events {
                events: self.recent_events(limit.unwrap_or(50)),
            },
            RuntimeRequest::ListScenes => RuntimeResponse::Scenes {
                scenes: self.scenes().cloned().collect(),
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
            RuntimeRequest::CallService { call } => match self.call_service(call) {
                Ok(result) => RuntimeResponse::ServiceCall { result },
                Err(err) => RuntimeResponse::Error {
                    error: err.to_string(),
                },
            },
            RuntimeRequest::ApplyConnectivityReport { report } => {
                let result = self.apply_connectivity_report(report);
                RuntimeResponse::ConnectivityApplied { result }
            }
            RuntimeRequest::RunAutomationTick { now_hh_mm } => {
                let result = self.run_automation_tick(now_hh_mm);
                RuntimeResponse::AutomationTick { result }
            }
        }
    }

    pub fn audit_len(&self) -> usize {
        self.audit.len()
    }

    pub fn audit_since(&self, index: usize) -> &[AuditEntry] {
        if index >= self.audit.len() {
            &[]
        } else {
            &self.audit[index..]
        }
    }

    pub fn recent_audit(&self, limit: usize) -> Vec<AuditEntry> {
        let len = self.audit.len();
        let start = len.saturating_sub(limit);
        self.audit[start..].to_vec()
    }

    pub fn event_len(&self) -> usize {
        self.events.len()
    }

    pub fn events_since(&self, index: usize) -> &[RuntimeEvent] {
        if index >= self.events.len() {
            &[]
        } else {
            &self.events[index..]
        }
    }

    pub fn recent_events(&self, limit: usize) -> Vec<RuntimeEvent> {
        let len = self.events.len();
        let start = len.saturating_sub(limit);
        self.events[start..].to_vec()
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

    pub fn apply_connectivity_report(
        &mut self,
        report: ConnectivityReport,
    ) -> ConnectivityApplyResult {
        let mut entities_upserted = 0;
        for device in &report.devices {
            let device_id = DeviceId::new(device.stable_id.clone())
                .unwrap_or_else(|_| DeviceId::new("unknown-device").expect("valid fallback id"));
            let mut registry_device = Device::new(
                device_id.clone(),
                device
                    .model
                    .clone()
                    .unwrap_or_else(|| device.stable_id.clone()),
            );
            if let Some(manufacturer) = &device.manufacturer {
                registry_device = registry_device.with_manufacturer(manufacturer.clone());
            }
            if let Some(model) = &device.model {
                registry_device = registry_device.with_model(model.clone());
            }
            registry_device = registry_device.with_identifier(device.stable_id.clone());
            self.upsert_device(registry_device);
            for entity in &device.entities {
                self.upsert_entity(
                    entity
                        .clone()
                        .into_entity()
                        .with_device_id(device_id.clone()),
                );
                entities_upserted += 1;
            }
        }
        let result = ConnectivityApplyResult {
            source: report.source,
            devices_seen: report.devices.len(),
            entities_upserted,
        };
        self.events
            .push(RuntimeEvent::new(RuntimeEventKind::ConnectivityApplied {
                source: result.source.clone(),
                devices_seen: result.devices_seen,
                entities_upserted: result.entities_upserted,
            }));
        result
    }

    pub fn call_service(
        &mut self,
        call: ServiceCall,
    ) -> Result<ServiceCallResult, crate::ServiceCallError> {
        let commands = service_call_to_commands(&self.graph, &call)?;
        let mut results = Vec::new();
        let mut blocked = false;
        for command in &commands {
            let decision = self.evaluate_for_execution(command);
            if !decision.allowed {
                blocked = true;
            }
            results.push(ServiceActionResult {
                entity_id: command.action.target.entity_id.clone(),
                executed: false,
                decision,
            });
        }

        if !blocked {
            for (index, command) in commands.into_iter().enumerate() {
                let decision = self.execute(command);
                results[index].executed = decision.allowed;
                results[index].decision = decision;
            }
        }

        let executed = results.iter().filter(|result| result.executed).count();
        let result = ServiceCallResult {
            domain: call.domain,
            service: call.service,
            targets: results.len(),
            executed,
            results,
        };
        self.events
            .push(RuntimeEvent::new(RuntimeEventKind::ServiceCalled {
                domain: result.domain.clone(),
                service: result.service.clone(),
                targets: result.targets,
                executed: result.executed,
            }));
        Ok(result)
    }

    pub fn run_automation_tick(&mut self, now_hh_mm: String) -> AutomationTickResult {
        let automations = self.automations().cloned().collect::<Vec<_>>();
        let mut result = AutomationTickResult {
            now_hh_mm: now_hh_mm.clone(),
            automations_checked: automations.len(),
            automations_triggered: 0,
            actions_executed: 0,
            blocked: Vec::new(),
        };

        for automation in automations {
            if !automation.enabled || !automation_triggered(&automation.trigger, &now_hh_mm) {
                continue;
            }
            if !self.automation_conditions_pass(&automation.conditions) {
                continue;
            }
            result.automations_triggered += 1;

            let mut blocked = None;
            for action in &automation.actions {
                let command = HomeCommand::new(CommandOrigin::Automation, action.clone());
                let decision = self.evaluate_for_execution(&command);
                if !decision.allowed {
                    blocked = Some(decision);
                    break;
                }
            }

            if let Some(decision) = blocked {
                result.blocked.push(AutomationBlock {
                    automation_id: automation.id,
                    decision,
                });
                continue;
            }

            for action in automation.actions {
                let decision = self.execute(HomeCommand::new(CommandOrigin::Automation, action));
                if decision.allowed {
                    result.actions_executed += 1;
                }
            }
        }

        self.events
            .push(RuntimeEvent::new(RuntimeEventKind::AutomationTick {
                now_hh_mm: result.now_hh_mm.clone(),
                automations_triggered: result.automations_triggered,
                actions_executed: result.actions_executed,
                blocked: result.blocked.len(),
            }));
        result
    }

    fn automation_conditions_pass(&self, conditions: &[AutomationCondition]) -> bool {
        conditions.iter().all(|condition| match condition {
            AutomationCondition::EntityStateIs { entity_id, state } => self
                .graph
                .get(entity_id)
                .map(|entity| &entity.state == state)
                .unwrap_or(false),
        })
    }

    fn evaluate_for_execution(&self, command: &HomeCommand) -> SafetyDecision {
        if command.action.kind == HomeActionKind::ActivateScene {
            self.evaluate_scene_command(command)
        } else {
            self.evaluate(command)
        }
    }

    fn apply_state_change(&mut self, command: &HomeCommand) {
        if command.action.kind == HomeActionKind::ActivateScene {
            self.apply_scene_state_change(command);
            return;
        }
        self.apply_single_state_change(&command.action, command.origin);
    }

    fn apply_scene_state_change(&mut self, command: &HomeCommand) {
        let Some(scene) = self.scenes.get(&command.action.target.entity_id).cloned() else {
            return;
        };
        for action in &scene.actions {
            self.apply_single_state_change(action, command.origin);
        }
    }

    fn apply_single_state_change(&mut self, action: &HomeAction, origin: CommandOrigin) {
        let Some(current) = self.graph.get(&action.target.entity_id).cloned() else {
            return;
        };
        let old_state = current.state.clone();
        let next_state = match action.kind {
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
        if old_state != next_state {
            self.events
                .push(RuntimeEvent::new(RuntimeEventKind::StateChanged {
                    entity_id: action.target.entity_id.clone(),
                    old_state,
                    new_state: next_state.clone(),
                    origin,
                }));
        }
        self.graph.upsert(current.with_state(next_state));
    }

    fn evaluate_scene_command(&self, command: &HomeCommand) -> SafetyDecision {
        let scene_activation = self.evaluate(command);
        if !scene_activation.allowed {
            return scene_activation;
        }
        let Some(scene) = self.scenes.get(&command.action.target.entity_id) else {
            return SafetyDecision::block(
                SafetyReason::SceneDefinitionMissing,
                "scene entity exists but no scene definition is registered",
            );
        };
        for action in &scene.actions {
            let child = HomeCommand {
                origin: command.origin,
                action: action.clone(),
                confirmed: command.confirmed,
                reason: command.reason.clone(),
            };
            let decision = self.evaluate(&child);
            if !decision.allowed {
                return SafetyDecision::block(
                    SafetyReason::SceneActionBlocked,
                    format!("scene action blocked: {}", decision.message),
                );
            }
        }
        SafetyDecision::allow()
    }
}

fn automation_triggered(trigger: &AutomationTrigger, now_hh_mm: &str) -> bool {
    match trigger {
        AutomationTrigger::TimeOfDay { hh_mm } => hh_mm == now_hh_mm,
    }
}

pub fn demo_runtime() -> HomeRuntime {
    let mut runtime = HomeRuntime::with_default_policy();
    let kitchen_light = EntityId::new("light.kitchen").expect("valid demo entity id");
    let front_door = EntityId::new("lock.front_door").expect("valid demo entity id");
    let movie_scene = EntityId::new("scene.movie_night").expect("valid demo entity id");
    let kitchen_device = DeviceId::new("device.kitchen_light").expect("valid demo device id");
    let front_door_device = DeviceId::new("device.front_door_lock").expect("valid demo device id");
    runtime.upsert_device(
        Device::new(kitchen_device.clone(), "Kitchen Light Device")
            .with_manufacturer("GeniePod")
            .with_model("Demo Light")
            .with_area("kitchen"),
    );
    runtime.upsert_device(
        Device::new(front_door_device.clone(), "Front Door Lock Device")
            .with_manufacturer("GeniePod")
            .with_model("Demo Lock")
            .with_area("entry"),
    );
    runtime.upsert_entity(
        Entity::new(kitchen_light.clone(), "Kitchen Light")
            .with_area("kitchen")
            .with_device_id(kitchen_device)
            .with_state(EntityState::Off)
            .with_capability(Capability::Power),
    );
    runtime.upsert_entity(
        Entity::new(front_door, "Front Door")
            .with_area("entry")
            .with_device_id(front_door_device)
            .with_state(EntityState::Locked)
            .with_capability(Capability::Lock),
    );
    runtime.upsert_entity(
        Entity::new(movie_scene.clone(), "Movie Night")
            .with_area("living_room")
            .with_state(EntityState::Off)
            .with_capability(Capability::SceneActivation),
    );
    runtime.upsert_scene(
        Scene::new(movie_scene, "Movie Night").with_action(HomeAction {
            target: TargetSelector::exact(kitchen_light.clone()),
            kind: HomeActionKind::TurnOn,
            value: None,
        }),
    );
    runtime.upsert_automation(
        Automation::new(
            "automation.kitchen_lights_out",
            "Kitchen Lights Out",
            AutomationTrigger::TimeOfDay {
                hh_mm: "23:00".into(),
            },
        )
        .with_action(HomeAction {
            target: TargetSelector::exact(kitchen_light),
            kind: HomeActionKind::TurnOff,
            value: None,
        }),
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

    #[test]
    fn handle_audit_request_returns_recent_entries() {
        let mut runtime = demo_runtime();
        runtime.execute(demo_turn_on_kitchen_command());
        let response = runtime.handle_request(RuntimeRequest::Audit { limit: Some(5) });

        let RuntimeResponse::Audit { entries } = response else {
            panic!("expected audit response");
        };
        assert_eq!(entries.len(), 1);
        assert!(entries[0].decision.allowed);
    }

    #[test]
    fn audit_entry_uses_rfc3339_timestamp_json() {
        let mut runtime = demo_runtime();
        runtime.execute(demo_turn_on_kitchen_command());

        let json = serde_json::to_value(&runtime.audit()[0]).unwrap();
        assert!(json["ts"].as_str().unwrap().ends_with('Z'));

        let restored: AuditEntry = serde_json::from_value(json).unwrap();
        assert_eq!(restored.command.action.kind, HomeActionKind::TurnOn);
    }

    #[test]
    fn restores_persisted_audit_entries_without_replaying_actions() {
        let id = EntityId::new("light.kitchen").unwrap();
        let mut first = demo_runtime();
        first.execute(demo_turn_on_kitchen_command());

        let mut second = demo_runtime();
        second.restore_audit_entries(first.audit().to_vec());

        assert_eq!(second.audit().len(), 1);
        assert_eq!(second.graph().get(&id).unwrap().state, EntityState::Off);
    }

    #[test]
    fn applies_connectivity_report_to_entity_graph() {
        let mut runtime = HomeRuntime::with_default_policy();
        let report = ConnectivityReport::esp32c6_thread_demo().unwrap();
        let response = runtime.handle_request(RuntimeRequest::ApplyConnectivityReport { report });

        let RuntimeResponse::ConnectivityApplied { result } = response else {
            panic!("expected connectivity apply response");
        };
        assert_eq!(result.devices_seen, 1);
        assert_eq!(result.entities_upserted, 1);
        assert!(
            runtime
                .graph()
                .contains(&EntityId::new("light.thread_demo").unwrap())
        );
        assert_eq!(runtime.devices().count(), 1);
        assert!(
            runtime
                .graph()
                .get(&EntityId::new("light.thread_demo").unwrap())
                .unwrap()
                .device_id
                .is_some()
        );
    }

    #[test]
    fn list_devices_returns_registered_devices() {
        let mut runtime = demo_runtime();
        let response = runtime.handle_request(RuntimeRequest::ListDevices);

        let RuntimeResponse::Devices { devices } = response else {
            panic!("expected devices response");
        };
        assert_eq!(devices.len(), 2);
    }

    #[test]
    fn scene_activation_applies_registered_actions() {
        let id = EntityId::new("light.kitchen").unwrap();
        let mut runtime = demo_runtime();
        let scene_id = EntityId::new("scene.movie_night").unwrap();
        let decision = runtime.execute(HomeCommand::new(
            CommandOrigin::Dashboard,
            HomeAction {
                target: TargetSelector::exact(scene_id),
                kind: HomeActionKind::ActivateScene,
                value: None,
            },
        ));

        assert!(decision.allowed);
        assert_eq!(runtime.graph().get(&id).unwrap().state, EntityState::On);
    }

    #[test]
    fn scene_activation_does_not_bypass_nested_safety() {
        let lock_id = EntityId::new("lock.front_door").unwrap();
        let scene_id = EntityId::new("scene.unlock_home").unwrap();
        let mut runtime = demo_runtime();
        runtime.upsert_entity(
            Entity::new(scene_id.clone(), "Unsafe Unlock Scene")
                .with_state(EntityState::Off)
                .with_capability(Capability::SceneActivation),
        );
        runtime.upsert_scene(
            Scene::new(scene_id.clone(), "Unsafe Unlock Scene").with_action(HomeAction {
                target: TargetSelector::exact(lock_id.clone()),
                kind: HomeActionKind::Unlock,
                value: None,
            }),
        );

        let decision = runtime.execute(HomeCommand::new(
            CommandOrigin::Voice,
            HomeAction {
                target: TargetSelector::exact(scene_id),
                kind: HomeActionKind::ActivateScene,
                value: None,
            },
        ));

        assert_eq!(decision.reason, SafetyReason::SceneActionBlocked);
        assert_eq!(
            runtime.graph().get(&lock_id).unwrap().state,
            EntityState::Locked
        );
    }

    #[test]
    fn automation_tick_executes_matching_time_trigger() {
        let id = EntityId::new("light.kitchen").unwrap();
        let mut runtime = demo_runtime();
        runtime.execute(demo_turn_on_kitchen_command());

        let result = runtime.run_automation_tick("23:00".into());

        assert_eq!(result.automations_checked, 1);
        assert_eq!(result.automations_triggered, 1);
        assert_eq!(result.actions_executed, 1);
        assert_eq!(runtime.graph().get(&id).unwrap().state, EntityState::Off);
    }

    #[test]
    fn automation_tick_blocks_group_before_mutation() {
        let light_id = EntityId::new("light.kitchen").unwrap();
        let lock_id = EntityId::new("lock.front_door").unwrap();
        let mut runtime = demo_runtime();
        runtime.upsert_automation(
            Automation::new(
                "automation.unsafe",
                "Unsafe Automation",
                AutomationTrigger::TimeOfDay {
                    hh_mm: "12:00".into(),
                },
            )
            .with_action(HomeAction {
                target: TargetSelector::exact(light_id.clone()),
                kind: HomeActionKind::TurnOn,
                value: None,
            })
            .with_action(HomeAction {
                target: TargetSelector::exact(lock_id.clone()),
                kind: HomeActionKind::Unlock,
                value: None,
            }),
        );

        let result = runtime.run_automation_tick("12:00".into());

        assert_eq!(result.blocked.len(), 1);
        assert_eq!(
            runtime.graph().get(&light_id).unwrap().state,
            EntityState::Off
        );
        assert_eq!(
            runtime.graph().get(&lock_id).unwrap().state,
            EntityState::Locked
        );
    }

    #[test]
    fn service_call_executes_supported_domain_service() {
        let id = EntityId::new("light.kitchen").unwrap();
        let mut runtime = demo_runtime();
        let result = runtime
            .call_service(ServiceCall {
                domain: "light".into(),
                service: "turn_on".into(),
                target: crate::ServiceTarget {
                    entity_ids: vec![id.clone()],
                },
                data: serde_json::Value::Null,
                origin: CommandOrigin::LocalApi,
                confirmed: false,
            })
            .unwrap();

        assert_eq!(result.executed, 1);
        assert_eq!(runtime.graph().get(&id).unwrap().state, EntityState::On);
        assert!(
            runtime
                .events()
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::ServiceCalled { .. }))
        );
    }

    #[test]
    fn service_call_prevents_partial_multi_target_execution() {
        let light_id = EntityId::new("light.kitchen").unwrap();
        let lock_id = EntityId::new("lock.front_door").unwrap();
        let mut runtime = demo_runtime();
        let result = runtime
            .call_service(ServiceCall {
                domain: "lock".into(),
                service: "unlock".into(),
                target: crate::ServiceTarget {
                    entity_ids: vec![lock_id.clone()],
                },
                data: serde_json::Value::Null,
                origin: CommandOrigin::LocalApi,
                confirmed: false,
            })
            .unwrap();

        assert_eq!(result.executed, 0);
        assert!(!result.results[0].decision.allowed);
        assert_eq!(
            runtime.graph().get(&light_id).unwrap().state,
            EntityState::Off
        );
        assert_eq!(
            runtime.graph().get(&lock_id).unwrap().state,
            EntityState::Locked
        );
    }

    #[test]
    fn state_changes_are_emitted_as_runtime_events() {
        let mut runtime = demo_runtime();
        runtime.execute(demo_turn_on_kitchen_command());

        assert!(runtime.events().iter().any(|event| {
            matches!(
                &event.kind,
                RuntimeEventKind::StateChanged {
                    entity_id,
                    old_state: EntityState::Off,
                    new_state: EntityState::On,
                    origin: CommandOrigin::Voice,
                } if entity_id.as_str() == "light.kitchen"
            )
        }));
    }

    #[test]
    fn event_request_returns_recent_events() {
        let mut runtime = demo_runtime();
        runtime.execute(demo_turn_on_kitchen_command());

        let response = runtime.handle_request(RuntimeRequest::Events { limit: Some(10) });

        let RuntimeResponse::Events { events } = response else {
            panic!("expected events response");
        };
        assert_eq!(events.len(), 1);
    }
}
