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
  third-party platform internals.

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

Home Assistant is a reference and migration source, not a bridge target and not
the architecture owner.

Use it to study:

- domain names and common entity capabilities
- integration behavior
- state shape and service semantics
- compatibility expectations users already understand
- migration needs for existing smart-home users

Do not copy its Python runtime architecture into this project. Genie Home
Runtime should remain Rust-first, smaller, more deterministic, and optimized
for local AI actuation.

The intended path for Home Assistant users is low-effort migration, not
permanent dependency:

- import entity names, domains, areas, and device metadata
- map common scenes and simple automations into Genie-native models
- report unsupported integrations clearly
- keep physical execution under Genie runtime safety policy
- avoid long-term architecture decisions that require Home Assistant to remain
  installed

## Crates

- `genie-home-core`: entity graph, command model, safety policy, runtime state.
- `genie-home-runtime`: executable runtime shell and future API/MCP process.

## Next Alpha Targets

- persisted SQLite state/audit log
- HTTP or Unix-socket API for `genie-claw`
- Home Assistant migration compatibility report
- initial MCP tool/resource surface
- ESP32-C6 Thread/Matter capability boundary with GenieOS
- support-bundle diagnostics compatible with `genie-ctl`
