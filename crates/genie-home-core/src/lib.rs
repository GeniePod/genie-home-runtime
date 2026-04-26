//! Core types and deterministic policy for Genie Home Runtime.
//!
//! This crate intentionally keeps the first alpha small. The goal is to define
//! the stable seams: entity graph, command model, safety policy, runtime state,
//! and audit entries.

pub mod command;
pub mod entity;
pub mod protocol;
pub mod runtime;
pub mod safety;

pub use command::{CommandOrigin, HomeAction, HomeActionKind, HomeCommand, TargetSelector};
pub use entity::{Capability, Entity, EntityGraph, EntityId, EntityState};
pub use protocol::{
    CommandResponse, EntitySnapshot, ExecuteCommandRequest, RuntimeRequest, RuntimeResponse,
};
pub use runtime::{
    AuditEntry, HomeRuntime, RuntimeStatus, demo_runtime, demo_turn_on_kitchen_command,
};
pub use safety::{SafetyDecision, SafetyPolicy, SafetyReason};
