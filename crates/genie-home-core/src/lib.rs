//! Core types and deterministic policy for Genie Home Runtime.
//!
//! This crate intentionally keeps the first alpha small. The goal is to define
//! the stable seams: entity graph, command model, safety policy, runtime state,
//! and audit entries.

pub mod command;
pub mod entity;
pub mod runtime;
pub mod safety;

pub use command::{CommandOrigin, HomeAction, HomeActionKind, HomeCommand, TargetSelector};
pub use entity::{Capability, Entity, EntityGraph, EntityId, EntityState};
pub use runtime::{AuditEntry, HomeRuntime, RuntimeStatus};
pub use safety::{SafetyDecision, SafetyPolicy, SafetyReason};
