# Home Assistant Reference Usage

Home Assistant is cloned locally for research and compatibility mapping:

```text
reference/home-assistant-core/
```

The directory is ignored by git.

## What To Borrow

Borrow concepts, not architecture:

- entity domains
- state naming conventions
- common service/action names
- integration edge cases
- device and area organization ideas
- user expectations around scenes and automations

## What Not To Borrow Blindly

- Python runtime structure
- dynamic integration loading model
- broad cloud integration surface
- implicit trust in service calls
- plugin behavior without strong permissioning

## Clean-Room Rule

When adapting an idea:

1. Write the behavior we need in Genie terms.
2. Implement it in Rust with Genie safety and audit constraints.
3. Add tests for the Genie behavior.
4. Keep the upstream checkout out of commits.

## Initial Compatibility Questions

- Which Home Assistant domains map to first-party Genie domains?
- Which service calls are inherently high risk?
- Which state fields are required for deterministic safety?
- Which integrations should be bridges versus native Genie adapters?
