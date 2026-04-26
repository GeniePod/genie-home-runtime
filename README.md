# Genie Home Runtime

`genie-home-runtime` is the Rust home automation runtime for the Genie
ecosystem.

It is the layer below `genie-claw` and above GenieOS/hardware. Its job is to
own the deterministic home-control plane:

- device graph and entity model
- state, scenes, automations, and schedules
- Matter, Thread, Zigbee, BLE, Wi-Fi, and bridge integrations
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
- command and action model
- deterministic safety policy
- in-memory runtime state
- appendable audit-entry model
- JSON request/response contract for status, entity listing, evaluate, and execute
- local Unix-socket JSON API for `genie-claw` and local tools
- basic CLI demo/status binary
- Home Assistant reference checkout path ignored by git

Not implemented yet:

- real protocol adapters
- persisted state database
- scheduler/automation engine
- MCP server
- Matter/Thread/BLE integrations
- migration/import tooling for users moving from Home Assistant

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
cargo run -p genie-home-runtime -- demo
cargo run -p genie-home-runtime -- entities
echo '{"origin":"voice","action":{"target":{"entity_id":"light.kitchen","confidence":1.0},"kind":"turn_on","value":null},"confirmed":false,"reason":null}' \
  | cargo run -p genie-home-runtime -- evaluate
```

Run the local runtime socket API:

```bash
cargo run -p genie-home-runtime -- serve /tmp/genie-home-runtime.sock
```

Send a request from another terminal:

```bash
echo '{"type":"status"}' \
  | cargo run -p genie-home-runtime -- request /tmp/genie-home-runtime.sock
```
