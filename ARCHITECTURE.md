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
- require confirmation for sensitive indirect actions from agents,
  automations, schedules, bridges, and local APIs
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
- `validate`: runtime invariant checks for devices, entities, scenes, and
  automations
- `list_devices`: current device registry snapshots
- `list_entities`: current entity snapshots
- `list_services`: supported Home Assistant-style domain services
- `list_domains`: implemented, read-only, and planned domain support
- `hardware_inventory`: runtime hardware/protocol boundaries and GenieOS
  driver requirements
- `list_scenes`: registered scene definitions
- `list_automations`: registered automation definitions
- `audit`: recent runtime decisions
- `events`: recent state/service/connectivity/automation events
- `evaluate`: return a safety decision without mutating state
- `execute`: apply the command only if the deterministic safety policy allows it
- `call_service`: translate an HA-style domain service call into safety-gated
  Genie actions
- `run_automation_tick`: evaluate local automations for a scheduler tick

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

Device, entity, scene, and automation snapshots are persisted in SQLite by the
runtime process so a restart does not reset known home state. The development
default is:

```text
/tmp/genie-home-runtime-state.sqlite3
```

The runtime binary can also emit a local JSON support bundle from persisted
state and audit files. This is intentionally a CLI path first, not a network
endpoint, so field diagnostics do not expand the physical-control attack
surface.

Reference systemd packaging serves the production socket from:

```text
/run/geniepod/home-runtime.sock
```

and stores durable runtime files under:

```text
/var/lib/geniepod/
```

Home Assistant migration support is intentionally report-first. The runtime can
analyze an HA-style states JSON dump and classify entities as `mappable`,
`manual_review`, or `unsupported`, but it does not become a permanent bridge or
delegate physical safety to Home Assistant.

For migration execution planning, the runtime can convert mappable HA state
records into a Genie connectivity report. That report can then be inspected and
applied through the existing `apply_connectivity_report` path. Manual-review
and unsupported records are kept out of the apply payload.

GenieOS connectivity is modeled as a structured report boundary. GenieOS can
publish discovered Matter, Thread, Zigbee, BLE, Wi-Fi, UART, or ESP32-C6-backed
devices through `apply_connectivity_report`; the runtime translates those into
entities and still owns safety checks for later actuation.

The runtime exposes a hardware inventory API so upper layers can be truthful
about support. UART and ESP32-C6 are runtime-boundary ready because the runtime
can ingest their normalized reports. Actual serial/SPI drivers, ESP32 firmware,
ESP-Hosted-NG, Matter fabric management, Thread border-router behavior, BLE
GATT transport, Zigbee coordinators, Z-Wave controllers, and Wi-Fi lifecycle
belong to GenieOS and must be validated on hardware there.

The first MCP-facing surface is a manifest of tools, resources, and required
permissions, not a full server. This keeps the tool names stable for
`genie-claw` while the lower local socket API remains the only execution path
for physical actions. Permission labels distinguish read-only operations from
evaluation, actuation, audit access, connectivity writes, automation runs, and
support diagnostics.

Scenes are modeled as registered action groups. Scene activation still passes
through the safety layer, and every nested scene action is evaluated before any
state mutation is applied. A scene cannot be used to bypass lock, cover, HVAC,
or other sensitive action checks.

Automations are modeled as enabled rules with explicit triggers, conditions,
and actions. The first scheduler boundary is an HH:MM tick. Matching
automations evaluate all actions before mutating state, so a blocked nested
action prevents partial execution.

Domain services mirror the parts of Home Assistant that are useful for
migration and user familiarity: `light.turn_on`, `lock.unlock`,
`cover.open_cover`, `scene.turn_on`, and similar calls. They are translated into
Genie `HomeCommand`s and then evaluated by the same safety layer. A multi-target
service call is all-or-nothing; if any target is blocked, none of the targets
are mutated.

The domain support matrix is intentionally explicit. It distinguishes domains
with safety-gated actuation from read-only state domains and planned domains
such as alarm panels, media players, and vacuums. This prevents the agent layer
from hallucinating unsupported physical control.

Runtime events provide the Home Assistant-style observability plane. State
changes, service calls, connectivity reports, and automation ticks are emitted
as structured JSON events and persisted separately from the actuation audit log.

Devices and entities are separate. A physical device may expose multiple
entities, and entities can carry a `device_id` pointer. This matches the useful
part of Home Assistant's registry model while keeping execution in Genie-owned
policy and state.

Validation is a first-class runtime operation. It checks registry consistency
before field deployment: entity device references, scene backing entities,
scene action targets, automation trigger shape, and automation action targets.

## Next Alpha Targets

- local MCP server transport
- richer scheduler persistence and catch-up policy
- support-bundle integration with `genie-ctl`
