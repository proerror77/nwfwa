# Codex User Instructions

- Work autonomously on clear, reversible tasks.
- Ask only when the next step is destructive, irreversible, or genuinely ambiguous.
- Prefer evidence over assumption; verify before claiming completion.
- Preserve unrelated user changes and avoid broad cleanup unless requested.
- Use project-local `AGENTS.md` files when present.
- Make atomic commits for every coherent change. Each commit should contain one logical unit of work with a focused message.

# Skills

## karpathy-guidelines

Source: https://github.com/multica-ai/andrej-karpathy-skills

Use this skill when writing, reviewing, or refactoring code to reduce common LLM coding mistakes.

### Think Before Coding

- Do not make silent assumptions.
- Surface uncertainty, inconsistencies, and tradeoffs before implementation.
- If multiple interpretations are plausible, name them instead of silently choosing.
- Push back when a simpler or safer approach is more appropriate.
- Stop and ask only when the ambiguity blocks a correct next step.

### Simplicity First

- Write the minimum code that solves the requested problem.
- Do not add speculative features, abstractions, configuration, or flexibility.
- Avoid defensive handling for impossible or out-of-scope scenarios.
- If the solution is much larger than necessary, simplify before finalizing.

### Surgical Changes

- Touch only files and lines required by the task.
- Do not refactor, reformat, rewrite comments, or clean adjacent code unless requested.
- Match existing project style and patterns.
- Remove unused code only when it was made unused by the current change.
- Mention unrelated dead code or risks instead of editing them opportunistically.

### Goal-Driven Execution

- Convert non-trivial tasks into explicit success criteria.
- Prefer test-first or verification-first workflows for code changes.
- For multi-step work, state a short plan with a concrete verification check for each step.
- Continue looping until the success criteria are verified or a real blocker is identified.

# Rust Compile Rules

- Keep Rust CI commands locked to `Cargo.lock` with `--locked` for dependency reproducibility.
- Keep CI cold builds optimized for validation speed: Rust CI disables incremental compilation and debug info through `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, and `CARGO_PROFILE_TEST_DEBUG=0`.
- Keep the Rust dependency and target cache in CI unless a failing cache is proven to hide a real build issue.
- Do not add release-mode Rust builds to the default CI path unless release artifact validation is explicitly required; release builds trade correctness signal for longer feedback time.

# Collaboration

## Agent Teams

- If Agent Teams or equivalent multi-agent collaboration is available and suitable, use it by default for work that benefits from parallel review, domain specialization, or independent verification.
- Prefer Agent Teams for architecture review, security review, QA, research-heavy decisions, large refactors, and ambiguous product/engineering tradeoffs.
- Keep one agent responsible for final integration so the result remains coherent and minimal.
- Do not use Agent Teams when the task is trivial, when coordination would add more overhead than value, or when the user asks for a single-agent workflow.
