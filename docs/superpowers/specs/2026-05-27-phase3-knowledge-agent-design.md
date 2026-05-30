# FWA Phase 3 Knowledge And Agent Design

## Scope

Phase 3 adds the first production-shaped slice for:

- FWA Knowledge Base case listing and similar-case search.
- Agent Case Investigator deterministic evidence package.
- Agent audit trail records that keep every generated conclusion traceable.
- Agent guardrail summaries for PII context redaction, tool policy checks, tool status, and human approval gates.

This phase stays inside the Rust modular monolith. It does not introduce external LLM providers, pgvector, Temporal, or autonomous claim decisions.

## Non-Goals

- No automatic fraud or denial decision.
- No free-form database access by agents.
- No shell/tool execution by agents.
- No embedding service or vector database dependency.
- No long-running workflow engine.

## Infrastructure Alignment

This phase is the deterministic, production-shaped slice of the broader
agentic operating model. Its constraints are deliberate:

- similar-case search uses structured fields and evidence refs instead of a
  vector database;
- agent investigation is deterministic and testable without external LLM calls;
- agent output is assistive only and cannot change claim adjudication state;
- `agent_runs` and `agent_steps` provide the first audit trail, not the full
  future agent workspace model.

Future agent-native infrastructure should follow
`docs/engineering/infrastructure-architecture.md`: agent run records, step
records, context snapshots, workspace artifacts, approval gates, retrieval audit,
and optional `pgvector` or specialist vector search only after the evidence
workflow needs it.

## API Surface

### `GET /api/v1/ops/knowledge/cases`

Returns curated confirmed/suspected FWA cases for Operations Studio.

Acceptance:

- Requires the same `x-api-key` authentication as existing operations APIs.
- Returns stable `case_id`, FWA type, diagnosis, provider profile, summary, outcome, tags, and evidence refs.

### `POST /api/v1/knowledge/search-similar`

Returns similar historical knowledge cases for a TPA or Studio claim context.

Acceptance:

- Requires `x-api-key`.
- Scores similarity using deterministic structured fields and keyword/tag overlap.
- Returns `similarity_score`, `matched_signals`, and `evidence_refs`.

### `POST /api/v1/agent/cases/investigate`

Generates a deterministic agent investigation package.

Acceptance:

- Requires `x-api-key`.
- Returns `agent_run_id`, `risk_summary`, `investigation_checklist`, `similar_cases`, `qa_opinion_draft`, and `evidence_refs`.
- Every conclusion has evidence references.
- The response explicitly states `decision_boundary = "assistive_only"`.
- Saves an agent run record and audit event.
- Governance surfaces summarize each agent run's assistive boundary, PII context status, tool policy status, tool status, and human approval gate.
- Does not modify `scoring_runs.recommended_action` or any claim adjudication state.

## Data Model

Phase 3 adds:

- `knowledge_cases`
- `agent_runs`
- `agent_steps`

`knowledge_cases` stores structured case metadata plus evidence refs in JSONB. `agent_runs` stores deterministic agent outputs, related claim/run identifiers, status, and evidence refs. `agent_steps` stores evidence-backed investigation steps for audit replay.

## Rust Boundaries

- `apps/api-server/src/routes/knowledge.rs`: Knowledge Base list/search handlers.
- `apps/api-server/src/routes/agent.rs`: Agent investigation handler.
- `apps/api-server/src/repository.rs`: Repository records and persistence methods.
- `crates/fwa-agent`: Deterministic investigator domain logic and response contracts.

The API layer handles auth, request/response shaping, and persistence orchestration. Agent business logic must remain deterministic and testable without network calls.

## UI Surface

Operations Studio adds real pages for:

- Knowledge Base: list curated cases, inspect tag/evidence provenance, run similar-case search, and see source trace completeness.
- Agent Investigator: submit case signals and view evidence-backed summary/checklist.

Both pages use the existing API helper and navigation pattern.

## Verification

- API tests cover knowledge list, similar search, and agent investigation output.
- OpenAPI test covers the new Phase 3 paths.
- Frontend API helper tests cover the new helper functions.
- `cargo test --workspace`, frontend tests/build, and repository health check must pass before PR.
