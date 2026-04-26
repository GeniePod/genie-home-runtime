//! Core types and deterministic policy for Genie Home Runtime.
//!
//! This crate intentionally keeps the first alpha small. The goal is to define
//! the stable seams: entity graph, command model, safety policy, runtime state,
//! and audit entries.

pub mod automation;
pub mod command;
pub mod connectivity;
pub mod entity;
pub mod mcp;
pub mod migration;
pub mod protocol;
pub mod runtime;
pub mod safety;
pub mod scene;
pub mod service;

pub use automation::{
    Automation, AutomationBlock, AutomationCondition, AutomationTickResult, AutomationTrigger,
};
pub use command::{CommandOrigin, HomeAction, HomeActionKind, HomeCommand, TargetSelector};
pub use connectivity::{
    ConnectivityApplyResult, ConnectivityDevice, ConnectivityEntity, ConnectivityProtocol,
    ConnectivityReport,
};
pub use entity::{Capability, Entity, EntityGraph, EntityId, EntityState};
pub use mcp::{McpPermission, McpResourceSpec, McpSurface, McpToolSpec, default_mcp_surface};
pub use migration::{
    HomeAssistantEntityRecord, MigrationCandidate, MigrationCompatibility, MigrationCounts,
    MigrationReport, build_home_assistant_migration_report, parse_home_assistant_entities_json,
};
pub use protocol::{
    CommandResponse, EntitySnapshot, ExecuteCommandRequest, RuntimeRequest, RuntimeResponse,
};
pub use runtime::{
    AuditEntry, HomeRuntime, RuntimeStatus, demo_runtime, demo_turn_on_kitchen_command,
};
pub use safety::{SafetyDecision, SafetyPolicy, SafetyReason};
pub use scene::Scene;
pub use service::{
    ServiceActionResult, ServiceCall, ServiceCallError, ServiceCallResult, ServiceSpec,
    ServiceTarget, service_call_to_commands, service_specs,
};
