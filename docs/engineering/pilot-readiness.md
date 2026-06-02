# Pilot Readiness

This document defines the minimum operational checklist for a customer pilot of the FWA platform.

## Integration Surface

Pilot API endpoints:

- `POST /api/v1/claims/score`
- `POST /api/v1/investigations/results`
- `POST /api/v1/qa/results`
- `GET /api/v1/audit/claims/{claim_id}`
- `POST /api/v1/knowledge/search-similar`
- `POST /api/v1/agent/cases/investigate`
- `GET /api/v1/ops/medical-review/queue`
- `POST /api/v1/ops/medical-review/results`
- `GET /api/v1/ops/api-calls`

All endpoints require `x-api-key`. Customer-specific credentials, network allowlists, and key rotation are configured outside the repository before pilot start.

## Writeback Contract

Investigation writeback captures:

- `claim_id`
- `investigation_id`
- `outcome`
- `confirmed_fwa`
- `saving_amount`
- `currency`
- `notes`
- `evidence_refs`

QA writeback captures:

- `qa_case_id`
- `claim_id`
- `qa_conclusion`
- `issue_type`
- `feedback_target`
- `notes`
- `evidence_refs`

Medical review writeback captures:

- `claim_id`
- `scoring_audit_id`
- `reviewer`
- `decision`
- `notes`
- `evidence_refs`

The writeback APIs append audit events and do not alter scoring recommendations or adjudication state.
Medical review results also produce governed outcome labels for model and workflow feedback.

## Monitoring

Minimum pilot monitoring:

- API health: `GET /api/v1/health`
- API key readiness: `/api/v1/health` check `api_key_configuration` must be
  `configured`, not `local_dev_key`, before customer pilot traffic.
- Source-system readiness: `/api/v1/health` check
  `source_system_configuration` must be `configured`, not
  `local_demo_source`, before customer pilot traffic.
- Worker health: `cargo run --locked -p worker -- health`
- ML service health: `GET /health`
- CI health: GitHub Actions `repository-health`, `migrations`, `rust`, `python`, `frontend`
- Runtime logs: request path, status, run id, audit id, event type, source system
- API call records: audit-backed scoring, investigation, and QA writeback calls in Governance
- Database checks: migration success and audit event append rate

OpenTelemetry, Grafana, Loki, and alert routing are production setup tasks after pilot environment selection.

## PII Handling

Pilot payloads should use customer-approved masked identifiers where possible.

Do not place PII in:

- `notes`
- `summary`
- `evidence_refs`
- free-text agent output

Evidence references should point to structured objects, for example `rule_runs:EARLY_CLAIM`, `agent_run:agent_CLM-0287`, or `knowledge_cases:KC-1001`.

## UAT Checklist

- Score a representative claim through `/api/v1/claims/score`.
- Confirm the response includes `run_id`, `audit_id`, RAG, scores, alerts, and evidence refs.
- Search similar cases through `/api/v1/knowledge/search-similar`.
- Generate an assistive investigation package through `/api/v1/agent/cases/investigate`.
- Write back an investigation result.
- Write back a QA result.
- Query `/api/v1/audit/claims/{claim_id}` and verify the timeline contains scoring, investigation, and QA events where applicable.
- Confirm high-risk outputs are assistive only and do not directly reject claims.
- Confirm customer pilot data is registered as Parquet dataset metadata before model training or evaluation use.
