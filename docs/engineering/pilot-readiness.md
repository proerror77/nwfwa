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
- Pilot readiness gate: `/api/v1/health` field `pilot_readiness.status` must be
  `ready` before customer pilot traffic. When it is `not_ready`,
  `pilot_readiness.blocking_checks` lists the non-secret configuration check
  names and statuses that still need remediation.
- API key readiness: `/api/v1/health` check `api_key_configuration` must be
  `configured`, not `local_dev_key`, before customer pilot traffic.
- Source-system readiness: `/api/v1/health` check
  `source_system_configuration` must be `configured`, not
  `local_demo_source`, before customer pilot traffic.
- Database readiness: `/api/v1/health` check `database_configuration` must be
  `configured`, not `local_dev_database`, before customer pilot traffic.
- Model service readiness: `/api/v1/health` check
  `model_service_configuration` must be `configured`, not
  `local_dev_model_service` or `heuristic_model_scorer`, before customer pilot
  traffic.
- Object storage readiness: `/api/v1/health` check
  `object_storage_configuration` must be `configured`, not
  `local_demo_object_storage`, before customer pilot traffic.
- Customer scope readiness: `/api/v1/health` check
  `customer_scope_configuration` must be `configured`, not
  `local_demo_customer_scope`, before customer pilot traffic.
  The customer scope is derived from the authenticated API key configuration,
  not from caller-supplied claim payloads. Inbox normalization, scoring, TPA
  writeback, case workflow, and governance audit payloads include
  `customer_scope_id` for tenant/customer traceability.
- Retention policy readiness: `/api/v1/health` check
  `retention_policy_configuration` must be `configured`, not
  `local_demo_retention_policy`, before customer pilot traffic.
- Backup and restore readiness: `/api/v1/health` check
  `backup_restore_configuration` must be `configured`, not
  `local_demo_backup_restore`, before customer pilot traffic.
- PII masking readiness: `/api/v1/health` check
  `pii_masking_configuration` must be `configured`, not
  `local_demo_pii_masking`, before customer pilot traffic.
- Key rotation readiness: `/api/v1/health` check
  `key_rotation_configuration` must be `configured`, not
  `local_demo_key_rotation`, before customer pilot traffic.
- Network allowlist readiness: `/api/v1/health` check
  `network_allowlist_configuration` must be `configured`, not
  `local_demo_network_allowlist`, before customer pilot traffic.
- Alert routing readiness: `/api/v1/health` check
  `alert_routing_configuration` must be `configured`, not
  `local_demo_alert_routing`, before customer pilot traffic.
- Observability exporter readiness: `/api/v1/health` check
  `observability_exporter_configuration` must be `configured`, not
  `local_demo_observability_exporter`, before customer pilot traffic.
- Agent policy readiness: `/api/v1/health` check
  `agent_policy_configuration` must be `configured`, not
  `local_demo_agent_policy`, before customer pilot traffic.
- Worker health: `cargo run --locked -p worker -- health`
- ML service health: `GET /health`
- CI health: GitHub Actions `repository-health`, `migrations`, `rust`, `python`, `frontend`
- Runtime logs: request path, status, run id, audit id, event type, source system
- API call records: audit-backed scoring, investigation, and QA writeback calls
  in Governance with `actor_role` and `customer_scope_id` for role and tenant
  traceability
- Database checks: migration success and audit event append rate

Grafana and Loki dashboards are production setup tasks after pilot environment selection.

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
