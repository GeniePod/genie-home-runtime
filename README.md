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
- basic CLI demo/status binary
- Home Assistant reference checkout path ignored by git

Not implemented yet:

- real protocol adapters
- persisted state database
- scheduler/automation engine
- MCP server
- Matter/Thread/BLE integrations
- Home Assistant bridge adapter

## Home Assistant Reference

The upstream Home Assistant clone belongs under:

```text
reference/home-assistant-core/
```

That path is intentionally ignored in `.gitignore`. Use it for compatibility
research only. Do not copy upstream code into the runtime unless a specific file
is reviewed for license, necessity, and clean integration boundaries.

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
```
