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
- Home Assistant local reference workflow under ignored `reference/`, framed for
  compatibility research and migration planning rather than a required bridge.
