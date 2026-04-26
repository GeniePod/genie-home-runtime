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

## Agent Boundary Contract

The first stable boundary is JSON-shaped and intentionally small:

- `status`: runtime health and safety policy summary
- `list_entities`: current entity snapshots
- `audit`: recent runtime decisions
- `evaluate`: return a safety decision without mutating state
- `execute`: apply the command only if the deterministic safety policy allows it

`evaluate` is the preferred first call from `genie-claw` when it needs to ask
"can this physical action happen?" before presenting confirmation UI or sending
an execution request.

The first process boundary is a local Unix socket. This intentionally avoids a
LAN-exposed HTTP surface for physical control. The default development socket
path is:

```text
/tmp/genie-home-runtime.sock
```

Production packaging can move this to `/run/geniepod/home-runtime.sock` with
systemd-owned permissions.

Executed command decisions are appended to JSONL audit storage by the runtime
process. The development default is:

```text
/tmp/genie-home-runtime-audit.jsonl
```

Entity snapshots are persisted in SQLite by the runtime process so a restart
does not reset known device state. The development default is:

```text
/tmp/genie-home-runtime-state.sqlite3
```

The runtime binary can also emit a local JSON support bundle from persisted
state and audit files. This is intentionally a CLI path first, not a network
endpoint, so field diagnostics do not expand the physical-control attack
surface.

## Next Alpha Targets

- production systemd unit for the Unix-socket API
- Home Assistant migration compatibility report
- initial MCP tool/resource surface
- ESP32-C6 Thread/Matter capability boundary with GenieOS
- support-bundle integration with `genie-ctl`
