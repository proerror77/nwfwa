# FWA Phase 4 Pilot Loop Design

## Scope

Phase 4 turns the demoable runtime into a pilot integration loop:

- TPA can write back investigation outcomes.
- TPA or QA can write back QA review results.
- Operators can query claim audit history.
- Operators can inspect audit-backed TPA API call records in Governance.
- Operators can inspect Provider/L6 graph-risk signals in Operations Studio.
- Operators can inspect Medical Review/L5 clinical issue and evidence-gap signals in Operations Studio.
- Operators can inspect Leads & Cases SLA governance, evidence sufficiency, and investigation writeback.
- Operators can inspect Audit Sampling control cohorts and missed-risk/false-positive calibration signals.
- Operators can inspect Dashboard seven-layer detection coverage for pilot scoring data.
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

### `GET /api/v1/ops/api-calls`

Returns audit-backed TPA API call records for Operations Studio governance.

Acceptance:

- Requires `x-api-key`.
- Derives records from scoring, investigation writeback, and QA writeback audit events.
- Includes endpoint, method, status code, result, source system, claim id, run id, audit id, event type, idempotency key, evidence refs, and observed time.
- Does not claim to be a full HTTP access log for failed requests or read-only calls without audit events.

## Data Model

Phase 4 adds:

- `investigation_results`
- `qa_reviews`

The existing `audit_events` table remains the authoritative timeline.

## Pilot Readiness

Implemented in this phase:

- Integration endpoints for outcome and QA writeback.
- Audit query endpoint for claim traceability.
- API call record endpoint and Governance Studio display for audit-backed TPA calls.
- Dataset catalog API from the Phase 3 branch as data integration groundwork.
- Leads & Cases Studio display for lead triage, case SLA status, evidence sufficiency, and investigation writeback audit output.
- Audit Sampling Studio display for deterministic QA samples, control cohorts, and calibration targets.
- Medical Review Studio display for L5 clinical issue types, evidence status, missing-evidence gaps, and selected-item evidence refs.
- Dashboard display for expected-versus-present seven-layer detection coverage.

Deferred to customer pilot setup:

- Real TPA credentials and network allowlists.
- Production RBAC policies.
- OpenTelemetry/Grafana deployment.
- Customer UAT scripts with real sample data.
- PII masking policy mapped to customer fields.

## Verification

- API tests cover investigation writeback, QA writeback, audit query, and API call records.
- OpenAPI includes the pilot loop paths.
- Full Rust/frontend/Python verification remains green.
