# Claude Project Instructions

Follow the project rules in `AGENTS.md`.

## Local Rust Verification Cadence

Use cadence B by default:

- During development, run only one cargo command at a time to avoid `target` build directory lock contention.
- For each small Rust change, prefer `cargo check --locked -p <crate>` and do not run tests by default.
- Before an atomic commit, run formatting, affected-crate cargo check, and the key tests for the feature group.
- After push, rely on GitHub CI for full validation, then fix failures with targeted local commands.
