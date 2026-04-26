//! Core types and deterministic policy for Genie Home Runtime.
//!
//! This crate intentionally keeps the first alpha small. The goal is to define
//! the stable seams: entity graph, command model, safety policy, runtime state,
//! and audit entries.

pub mod automation;
pub mod command;
pub mod connectivity;
pub mod device;
pub mod entity;
pub mod event;
pub mod ha_port;
pub mod hardware;
pub mod mcp;
pub mod migration;
pub mod mock_hardware;
pub mod protocol;
pub mod runtime;
pub mod safety;
pub mod scene;
pub mod scheduler;
pub mod service;
pub mod validation;

pub use automation::{
    Automation, AutomationBlock, AutomationCondition, AutomationTickResult, AutomationTrigger,
};
pub use command::{CommandOrigin, HomeAction, HomeActionKind, HomeCommand, TargetSelector};
pub use connectivity::{
    ConnectivityApplyResult, ConnectivityDevice, ConnectivityEntity, ConnectivityProtocol,
    ConnectivityReport, EntityStateUpdate, StateApplyResult, StateReport,
};
pub use device::{Device, DeviceId, DeviceIdError, DeviceRegistry};
pub use entity::{Capability, Entity, EntityGraph, EntityId, EntityState, SafetyClass};
pub use event::{RuntimeEvent, RuntimeEventKind};
pub use ha_port::{
    MockHomeAssistantPortResult, mock_hardware_to_home_assistant_states,
    run_mock_home_assistant_port,
};
pub use hardware::{
    HardwareAdapterState, HardwareAdapterStatus, HardwareCapability, HardwareInventory,
    HardwareSupportLevel, default_hardware_inventory,
};
pub use mcp::{McpPermission, McpResourceSpec, McpSurface, McpToolSpec, default_mcp_surface};
pub use migration::{
    HomeAssistantEntityRecord, MigrationCandidate, MigrationCompatibility, MigrationCounts,
    MigrationImportPlan, MigrationReport, build_home_assistant_import_plan,
    build_home_assistant_migration_report, parse_home_assistant_entities_json,
};
pub use mock_hardware::{
    HardwareInterface, MockHardwareBus, MockHardwareCommandResult, MockIoTEntity,
    mock_turn_on_thread_lamp_command,
};
pub use protocol::{
    CommandResponse, ConfigChangeResult, ConfigResource, DeviceSnapshot, EntitySnapshot,
    ExecuteCommandRequest, RuntimeRequest, RuntimeResponse,
};
pub use runtime::{
    AuditEntry, HomeRuntime, RuntimeStatus, demo_runtime, demo_turn_on_kitchen_command,
};
pub use safety::{SafetyDecision, SafetyPolicy, SafetyReason};
pub use scene::Scene;
pub use scheduler::{
    SchedulerCatchUpMode, SchedulerCatchUpPolicy, SchedulerRunResult, SchedulerWindow,
    SchedulerWindowError, enumerate_hh_mm_window,
};
pub use service::{
    DomainSupport, DomainSupportLevel, ServiceActionResult, ServiceCall, ServiceCallError,
    ServiceCallResult, ServiceSpec, ServiceTarget, domain_support_matrix, service_call_to_commands,
    service_specs,
};
pub use validation::{ValidationIssue, ValidationReport, ValidationSeverity, validate_runtime};
