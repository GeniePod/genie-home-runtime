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
- Home Assistant local reference workflow under ignored `reference/`, framed for
  compatibility research and migration planning rather than a required bridge.
