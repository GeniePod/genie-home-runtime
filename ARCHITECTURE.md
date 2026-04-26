# Genie Home Runtime Architecture

## Purpose

`genie-home-runtime` is the deterministic home-control runtime for the Genie
stack.

The agent layer may understand intent. This runtime decides whether physical
execution is allowed.

The system prompt is not a safety boundary. The home runtime is.

## Design Principles

- Local-first: no cloud dependency for core device graph, scenes, schedules, or
  actuation.
- Deterministic safety: every physical action passes runtime checks.
- Replayable state: device events, commands, and actuation decisions should be
  auditable and eventually replayable.
- Integration isolation: protocol adapters do not own core policy.
- Memory constrained: default deployment target remains Jetson-class hardware.
- AI-native boundary: expose stable structured APIs to `genie-claw`, not raw
  Home Assistant internals.

## Internal Layers

```text
API / MCP boundary
    |
Command router
    |
Safety layer
    |
Entity graph + state store
    |
Integration adapters
    |
GenieOS / radios / bridges / devices
```

## Safety Model

Safety is inspired by autonomous-runtime discipline:

- classify command risk
- resolve target confidence
- check current device availability/state
- require confirmation for high-risk actions
- block ambiguous multi-target sensitive actions
- record every decision

The first alpha scaffold implements this as a pure Rust policy in
`genie-home-core::safety`.

## Home Assistant Relationship

Home Assistant is a reference and bridge target, not the architecture owner.

Use it to study:

- domain names and common entity capabilities
- integration behavior
- state shape and service semantics
- compatibility expectations users already understand

Do not copy its Python runtime architecture into this project. Genie Home
Runtime should remain Rust-first, smaller, more deterministic, and optimized
for local AI actuation.

## Crates

- `genie-home-core`: entity graph, command model, safety policy, runtime state.
- `genie-home-runtime`: executable runtime shell and future API/MCP process.

## Next Alpha Targets

- persisted SQLite state/audit log
- HTTP or Unix-socket API for `genie-claw`
- Home Assistant bridge adapter interface
- initial MCP tool/resource surface
- ESP32-C6 Thread/Matter capability boundary with GenieOS
- support-bundle diagnostics compatible with `genie-ctl`
