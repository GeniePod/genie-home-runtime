# Changelog

## 0.1.0-alpha.0 - Unreleased

Initial scaffold for Genie Home Runtime.

### Added

- Rust workspace with `genie-home-core` and `genie-home-runtime`.
- Entity graph, command model, safety policy, runtime state, and audit entries.
- Deterministic safety checks for unknown targets, low target confidence,
  unavailable state, unsupported capabilities, critical targets, and sensitive
  voice/agent commands.
- CLI status/demo binary.
- JSON runtime request/response contract for status, entity listing, command
  evaluation, and command execution.
- Local Unix-socket JSON API plus `request` CLI command for local agent/runtime
  calls without exposing a network port.
- SQLite entity snapshot persistence for runtime restarts.
- Recent audit query support and JSONL audit persistence for executed runtime
  decisions.
- Local JSON support bundle generation from persisted state and audit files.
- Reference systemd unit and tmpfiles configuration for the local socket
  service.
- Home Assistant states compatibility report for migration planning without a
  permanent bridge dependency.
- GenieOS connectivity report contract and runtime application path for
  discovered devices.
- MCP-facing tool/resource manifest for the upper `genie-claw` agent layer.
- Basic scene model with nested action safety evaluation before scene execution.
- Basic automation model with HH:MM scheduler tick execution and action-group
  safety evaluation.
- Sensitive unlock/open actions now require confirmation for automation,
  schedule, bridge, local API, voice, and agent origins.
- MCP-facing manifest now labels tools and resources with required permission
  classes.
- Home Assistant-style service catalog and safety-gated service-call execution.
- Runtime event log for state changes, service calls, connectivity reports, and
  automation ticks.
- Device registry with entity-to-device attribution and SQLite persistence.
- Runtime validation report for registry, scene, and automation consistency.
- Home Assistant import plan generation into Genie connectivity reports.
- SQLite persistence for scene and automation registries.
- Home Assistant local reference workflow under ignored `reference/`, framed for
  compatibility research and migration planning rather than a required bridge.
- Runtime hardware/protocol inventory API that marks UART/ESP32-C6 as
  report-boundary ready and radio drivers as GenieOS responsibilities.
- Domain support matrix API for implemented safety-gated domains, read-only
  state domains, and planned domains.
- Validated scene and automation configuration APIs, including delete paths and
  persistence triggers.
- GenieOS state report API for applying hardware-originated state updates to
  already-discovered entities.
- Deterministic mock hardware interface with reference Thread, Matter, Zigbee,
  and BLE-like devices for hardware-free runtime tests.
- Mock Home Assistant porting harness that converts simulated devices to
  HA-style states, runs migration/import planning, and validates the imported
  runtime graph.
- Expanded HA-style service/domain coverage for media players, vacuums, alarm
  panels, fan percentage, switch/fan toggles, and cover stop.
- Bounded scheduler catch-up window for deterministic missed-tick replay after
  runtime downtime or restart.
- Versioned runtime snapshot export/import with validation before restore.
- Local MCP-style stdio JSON-RPC bridge for manifest discovery and `tools/call`
  execution against runtime requests.
- Normalized GenieOS adapter message boundary for heartbeat, adapter status,
  connectivity discovery, and hardware-originated state reports.
