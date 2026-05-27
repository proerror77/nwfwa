# FWA Phase 4 Pilot Loop Design

## Scope

Phase 4 turns the demoable runtime into a pilot integration loop:

- TPA can write back investigation outcomes.
- TPA or QA can write back QA review results.
- Operators can query claim audit history.
- Pilot readiness is documented with what is implemented and what remains operational.

This phase stays inside the Rust modular monolith and uses the same API key boundary as the earlier APIs.

## Non-Goals

- No customer-specific connector.
- No full RBAC product.
- No external observability stack deployment.
- No automatic claim adjudication.
- No PII export or raw document processing.

## API Surface

### `POST /api/v1/investigations/results`

Writes investigation outcome from TPA/investigator systems.

Acceptance:

- Requires `x-api-key`.
- Persists `claim_id`, `outcome`, `confirmed_fwa`, `saving_amount`, `currency`, `notes`, `evidence_refs`.
- Writes `audit_events` with `event_type = "investigation.result.received"`.
- Does not modify scoring recommendation.

### `POST /api/v1/qa/results`

Writes QA review result.

Acceptance:

- Requires `x-api-key`.
- Persists `qa_case_id`, `claim_id`, `qa_conclusion`, `issue_type`, `feedback_target`, `notes`, `evidence_refs`.
- Writes `audit_events` with `event_type = "qa.result.received"`.

### `GET /api/v1/audit/claims/{claim_id}`

Returns claim audit history.

Acceptance:

- Requires `x-api-key`.
- Returns audit events ordered by creation.
- Includes `audit_id`, `run_id`, `event_type`, `event_status`, `summary`, `evidence_refs`, and payload.

## Data Model

Phase 4 adds:

- `investigation_results`
- `qa_reviews`

The existing `audit_events` table remains the authoritative timeline.

## Pilot Readiness

Implemented in this phase:

- Integration endpoints for outcome and QA writeback.
- Audit query endpoint for claim traceability.
- Dataset catalog API from the Phase 3 branch as data integration groundwork.

Deferred to customer pilot setup:

- Real TPA credentials and network allowlists.
- Production RBAC policies.
- OpenTelemetry/Grafana deployment.
- Customer UAT scripts with real sample data.
- PII masking policy mapped to customer fields.

## Verification

- API tests cover investigation writeback, QA writeback, and audit query.
- OpenAPI includes the three pilot loop paths.
- Full Rust/frontend/Python verification remains green.
