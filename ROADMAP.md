# Genie Home Runtime Roadmap

## Alpha 0: Runtime Skeleton

- Entity graph
- Command model
- Deterministic safety policy
- In-memory runtime state
- Audit-entry model
- Home Assistant reference checkout workflow

## Alpha 1: Agent Boundary

- Local HTTP or Unix-socket API for `genie-claw`
- Stable command request/response schema
- Runtime status endpoint
- Actuation audit persistence
- Support bundle output

## Alpha 2: State Persistence

- SQLite state store
- Entity registry persistence
- Event log persistence
- Replayable audit log
- Basic scene model

## Alpha 3: Bridge Adapters

- Home Assistant bridge adapter
- GenieOS connectivity capability adapter
- Mock integration test harness
- Domain compatibility matrix

## Alpha 4: Automation Core

- Scheduler
- Rule/condition/action model
- Safe scene execution
- Confirmation handoff surface for app/agent

## Alpha 5: MCP And Product Boundary

- Local MCP server for `genie-claw`
- Tool/resource exposure with permissions
- App/dashboard API surface
- Field diagnostics

## Non-Goals

- Do not become a Python Home Assistant fork.
- Do not let protocol adapters own safety policy.
- Do not rely on LLM prompts for physical safety.
- Do not add cloud dependencies to core control paths.
