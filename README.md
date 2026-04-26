# Genie Home Runtime

`genie-home-runtime` is the Rust home automation runtime for the Genie
ecosystem.

It is the layer below `genie-claw` and above GenieOS/hardware. Its job is to
own the deterministic home-control plane:

- device graph and entity model
- state, scenes, automations, and schedules
- normalized Matter, Thread, Zigbee, BLE, Wi-Fi, UART, and bridge reports from
  GenieOS
- final physical-actuation safety checks
- audit logs and replayable runtime events
- local MCP/API surface for upper agent layers
- memory-constrained operation on Jetson-class devices

This repo does not try to be a Python Home Assistant fork. Home Assistant is
used as a local reference for domain coverage, integration behavior, and entity
semantics. The runtime itself is Rust-first, local-first, deterministic, and
designed for AI-native physical safety.

This project is inspired by Home Assistant, which is licensed under Apache
2.0. All code in this repository is independently implemented in Rust unless
explicitly stated otherwise.

## Ecosystem Position

```text
genie-claw
  voice, memory, skills, agent policy
      |
      v
genie-home-runtime
  entity graph, automations, final actuation safety, audit, MCP/API
      |
      v
genie-os + hardware
  Jetson L4T, drivers, ESP32-C6 connectivity, radios, audio, peripherals
```

## Current Status

This is the initial alpha scaffold.

Implemented now:

- core entity graph model
- basic scene model with nested action safety checks
- basic automation model with scheduler tick execution
- validated scene and automation configuration APIs
- Home Assistant-style domain service catalog and safety-gated service calls
- home domain support matrix for implemented, read-only, and planned domains
- expanded HA-style domain coverage for media players, vacuums, alarm panels,
  fan percentage, switch/fan toggles, and cover stop
- hardware/protocol inventory that explicitly separates runtime support from
  GenieOS driver requirements
- Home Assistant-style runtime event log for state/service/connectivity/automation events
- Home Assistant-style device registry with entity-to-device attribution
- self-validation report for runtime registry and automation invariants
- Home Assistant import plan that converts mappable states into Genie connectivity reports
- command and action model
- GenieOS state report path for hardware-originated entity updates
- deterministic safety policy
- in-memory runtime state
- appendable audit-entry model
- JSON request/response contract for status, entity listing, evaluate, and execute
- local Unix-socket JSON API for `genie-claw` and local tools
- SQLite registry persistence for devices, entities, scenes, and automations
- JSONL audit persistence for executed runtime decisions
- local JSON support bundle for field diagnostics
- Home Assistant states compatibility report for migration planning
- GenieOS connectivity report contract for discovered devices
- MCP-facing tool/resource manifest with permission labels for `genie-claw`
- deterministic mock hardware simulator with Thread, Matter, Zigbee, and BLE
  reference devices plus media/vacuum/alarm devices for hardware-free tests
- mock Home Assistant porting harness that converts simulated hardware to
  HA-style states, runs migration/import, and validates the Genie runtime graph
- reference systemd packaging for a production local socket service
- basic CLI demo/status binary
- Home Assistant reference checkout path ignored by git

Not implemented yet:

- real protocol/radio drivers in this repo
- full scheduler catch-up/missed-run engine
- MCP server
- direct Matter/Thread/BLE commissioning in this repo
- full migration/import tooling for users moving from Home Assistant

Hardware boundary:

- `genie-home-runtime` is ready to ingest normalized hardware/connectivity
  reports.
- GenieOS owns actual Jetson drivers, ESP32-C6 UART/SPI transport,
  ESP-Hosted-NG, Matter fabrics, Thread border router support, BLE scanning,
  and radio lifecycle.
- If an API reports `requires_genie_os_driver`, that is intentional; this repo
  must not pretend physical hardware is complete before hardware validation.

## Home Assistant Reference And Migration

The upstream Home Assistant clone belongs under:

```text
reference/home-assistant-core/
```

That path is intentionally ignored in `.gitignore`. Use it for compatibility
research only. Do not copy upstream code into the runtime unless a specific file
is reviewed for license, necessity, and clean integration boundaries.

Genie Home Runtime does not require or target a Home Assistant bridge as a core
architecture path. The goal is native Genie device graph, automations, and
actuation safety. For users who already have Home Assistant, we should plan
low-effort migration paths: entity import, area/scene mapping, automation
translation where safe, and clear compatibility reports.

Clone/update reference:

```bash
mkdir -p reference
git clone --depth 1 https://github.com/home-assistant/core.git reference/home-assistant-core
```

## Commands

```bash
cargo test --workspace
cargo run -p genie-home-runtime -- status
cargo run -p genie-home-runtime -- validate
cargo run -p genie-home-runtime -- demo
cargo run -p genie-home-runtime -- devices
cargo run -p genie-home-runtime -- entities
cargo run -p genie-home-runtime -- services
cargo run -p genie-home-runtime -- domains
cargo run -p genie-home-runtime -- hardware
cargo run -p genie-home-runtime -- mock-hardware-demo
cargo run -p genie-home-runtime -- ha-mock-port-demo
cargo run -p genie-home-runtime -- events
cargo run -p genie-home-runtime -- scenes
cargo run -p genie-home-runtime -- automations
cargo run -p genie-home-runtime -- automation-tick 23:00
echo '{"origin":"voice","action":{"target":{"entity_id":"light.kitchen","confidence":1.0},"kind":"turn_on","value":null},"confirmed":false,"reason":null}' \
  | cargo run -p genie-home-runtime -- evaluate
echo '{"domain":"light","service":"turn_on","target":{"entity_ids":["light.kitchen"]},"data":{},"origin":"local_api","confirmed":false}' \
  | cargo run -p genie-home-runtime -- call-service
echo '{"id":"scene.bedtime","display_name":"Bedtime","actions":[{"target":{"entity_id":"light.kitchen","confidence":1.0},"kind":"turn_off","value":null}]}' \
  | cargo run -p genie-home-runtime -- upsert-scene
echo '{"id":"automation.bedtime","display_name":"Bedtime","enabled":true,"trigger":{"time_of_day":{"hh_mm":"23:00"}},"conditions":[],"actions":[{"target":{"entity_id":"light.kitchen","confidence":1.0},"kind":"turn_off","value":null}]}' \
  | cargo run -p genie-home-runtime -- upsert-automation
echo '{"source":"genie-os-test","updates":[{"entity_id":"light.kitchen","state":"on","attributes":{}}]}' \
  | cargo run -p genie-home-runtime -- apply-state-report
```

Run the local runtime socket API:

```bash
cargo run -p genie-home-runtime -- serve \
  /tmp/genie-home-runtime.sock \
  /tmp/genie-home-runtime-audit.jsonl \
  /tmp/genie-home-runtime-state.sqlite3 \
  /tmp/genie-home-runtime-events.jsonl
```

Send a request from another terminal:

```bash
echo '{"type":"status"}' \
  | cargo run -p genie-home-runtime -- request /tmp/genie-home-runtime.sock
```

Generate a sample GenieOS connectivity report request:

```bash
cargo run -p genie-home-runtime -- connectivity-demo
```

Run a deterministic mock hardware flow:

```bash
cargo run -p genie-home-runtime -- mock-hardware-demo
```

Run mock hardware through the Home Assistant migration/import path:

```bash
cargo run -p genie-home-runtime -- ha-mock-port-demo
```

Print the MCP-facing tool/resource manifest:

```bash
cargo run -p genie-home-runtime -- mcp-manifest
```

Inspect recent audit entries:

```bash
echo '{"type":"audit","limit":10}' \
  | cargo run -p genie-home-runtime -- request /tmp/genie-home-runtime.sock
```

Generate a local support bundle:

```bash
cargo run -p genie-home-runtime -- support-bundle \
  /tmp/genie-home-runtime-audit.jsonl \
  /tmp/genie-home-runtime-state.sqlite3 \
  /tmp/genie-home-runtime-events.jsonl
```

Generate a Home Assistant migration compatibility report from a states JSON
dump:

```bash
curl -s http://homeassistant.local:8123/api/states > ha-states.json
cargo run -p genie-home-runtime -- ha-compat-report ha-states.json
cargo run -p genie-home-runtime -- ha-import-plan ha-states.json
```

Production-style systemd reference files live in:

```text
packaging/systemd/
```
