# Home Assistant Reference Usage

Home Assistant is cloned locally for research, compatibility mapping, and
migration planning:

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
- migration inputs for existing installations

## What Not To Borrow Blindly

- Python runtime structure
- dynamic integration loading model
- broad cloud integration surface
- implicit trust in service calls
- plugin behavior without strong permissioning
- permanent bridge dependency as the default product path

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
- Which entities, areas, scenes, and simple automations can be imported with
  minimum user effort?
- Which integrations should become native Genie adapters, and which should be
  reported as unsupported during migration?

## Migration Direction

The preferred path is not "run Home Assistant forever behind Genie." The
preferred path is:

1. Read a user's existing Home Assistant configuration/state where available.
2. Generate a Genie-native device graph and compatibility report.
3. Import low-risk scenes and simple automations only when they can be mapped
   deterministically.
4. Leave unsupported or risky automations disabled until the user reviews them.
5. Execute all future physical actions through Genie Home Runtime safety policy.
