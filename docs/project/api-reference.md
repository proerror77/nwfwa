# API Reference

All local and pilot APIs use JSON. Except for `GET /api/v1/health` and
`GET /api/openapi.json`, callers should send `x-api-key`.

Local demo API key:

```text
dev-secret
```

## API Groups

- Health and contract
- Runtime scoring
- Operations dashboard and governance
- Lead, case, investigation, QA, and medical review workflow
- Rules and routing policies
- Datasets, features, and model evaluation lineage
- Model operations
- Provider, member, and scheme intelligence
- Knowledge and agent workflows
- Audit history

## Health And Contract

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/health` | Return API health status. | No | None |
| GET | `/api/openapi.json` | Return OpenAPI contract. | No | None |

## Runtime Scoring

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/claims/score` | Score a claim through the FWA runtime. | Yes | Persists scoring run, feature values, rule runs, model scores, audit events, API call record, and possible lead. |

Main request modes:

- stored demo claim by `source_system` and `claim_id`
- submitted claim payload with member, policy, provider, diagnosis, procedure,
  amount, dates, and context fields

Main response fields:

- `run_id`
- `audit_id`
- risk score and RAG band
- recommended action
- score layers
- alerts
- top reasons
- evidence refs
- routing and review metadata

## Dashboard And Operational Summary

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/dashboard/summary` | Return executive and operations rollups. | Yes | None |
| GET | `/api/v1/ops/alerts` | List operational alerts from scoring, cases, QA, and governance. | Yes | None |
| GET | `/api/v1/ops/webhook-events` | List webhook-style event records. | Yes | None |
| POST | `/api/v1/ops/webhook-events/{event_id}/delivery-attempts` | Record a webhook delivery attempt. | Yes | Appends delivery attempt evidence. |

Dashboard rollups include suspected claims, risk amount, RAG distribution, rule
hits, model scores, seven-layer coverage, QA signals, investigation writebacks,
case SLA status, and saving attribution.

## Leads And Cases

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/leads` | List generated FWA leads. | Yes | None |
| POST | `/api/v1/ops/leads/{lead_id}/triage` | Triage a lead into open, merge, close, or review outcome. | Yes | Updates lead, may create case, records audit event. |
| GET | `/api/v1/ops/cases` | List investigation cases. | Yes | None |
| POST | `/api/v1/ops/cases/{case_id}/status` | Update case status, assignment, reviewer, priority, or notes. | Yes | Updates case and appends audit event. |

Lead triage is the bridge from scoring to case workflow. Case evidence packages
preserve claim, rule, model, anomaly, document, and similar-case references.

## Investigation And QA Writeback

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/investigations/results` | Accept investigation result writeback from TPA or UI. | Yes | Appends investigation result, audit event, labels, and saving attribution. |
| POST | `/api/v1/qa/results` | Accept QA result writeback. | Yes | Appends QA review, audit event, and feedback labels. |
| GET | `/api/v1/ops/qa/feedback-items` | List QA feedback items for governance. | Yes | None |
| POST | `/api/v1/ops/qa/feedback-items/{feedback_id}/status` | Update feedback item status. | Yes | Updates feedback status and audit metadata. |
| GET | `/api/v1/ops/qa/queue` | List demo QA queue records. | Yes | None |
| GET | `/api/v1/ops/qa/queue-summary` | Return QA queue rollups. | Yes | None |
| GET | `/api/v1/ops/labels` | List structured outcome labels. | Yes | None |

Writebacks append evidence. They do not adjudicate claims.

## Medical Review

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/medical-review/queue` | List claims needing clinical evidence review. | Yes | None |
| POST | `/api/v1/ops/medical-review/results` | Submit medical review result. | Yes | Appends audit event and medical-review feedback label. |

Medical review focuses on clinical evidence gaps, medical necessity, diagnosis
and procedure consistency, and reviewer feedback.

## Audit And Governance

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/audit-events` | List audit events across workflows. | Yes | None |
| GET | `/api/v1/ops/audit-samples` | List deterministic audit sample records. | Yes | None |
| POST | `/api/v1/ops/audit-samples` | Create an audit sample for QA or control review. | Yes | Persists sample record and audit context. |
| GET | `/api/v1/ops/api-calls` | List audited API call records. | Yes | None |
| GET | `/api/v1/audit/claims/{claim_id}` | Return claim-level audit timeline. | Yes | None |
| GET | `/api/v1/ops/agent-runs` | List agent run records. | Yes | None |
| POST | `/api/v1/ops/agent-runs/{agent_run_id}/approvals` | Submit human approval decision for an agent run. | Yes | Records approval and audit event. |

Governance endpoints are read-heavy and audit-first. Mutating governance actions
record human context and evidence refs.

## Rules

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/rules` | List FWA rules. | Yes | None |
| GET | `/api/v1/ops/rules/{rule_id}` | Return one rule and its versions. | Yes | None |
| POST | `/api/v1/ops/rules/backtest` | Run deterministic rule backtest. | Yes | Records backtest result evidence. |
| GET | `/api/v1/ops/rules/performance` | Return rule performance rollups. | Yes | None |
| GET | `/api/v1/ops/rules/{rule_id}/promotion-gates` | Evaluate rule promotion readiness. | Yes | None |
| POST | `/api/v1/ops/rules/{rule_id}/promotion-reviews` | Submit human promotion review. | Yes | Records review evidence. |
| POST | `/api/v1/ops/rules/candidates` | Save a candidate rule. | Yes | Creates or updates candidate rule evidence. |
| POST | `/api/v1/ops/rules/discover` | Discover candidate rules from observed signals. | Yes | Records discovery provenance. |
| POST | `/api/v1/ops/rules/{rule_id}/submit` | Submit rule for governance. | Yes | Updates lifecycle status and audit trail. |
| POST | `/api/v1/ops/rules/{rule_id}/approve` | Approve submitted rule. | Yes | Updates lifecycle status and audit trail. |
| POST | `/api/v1/ops/rules/{rule_id}/publish` | Publish approved rule. | Yes | Updates lifecycle status and audit trail. |
| POST | `/api/v1/ops/rules/{rule_id}/rollback` | Roll back active rule version. | Yes | Restores governed version and records audit trail. |

Rule APIs support deterministic controls. They should not silently change active
customer behavior without lifecycle and audit evidence.

## Routing Policies

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/routing-policies` | List routing policies. | Yes | None |
| POST | `/api/v1/ops/routing-policies` | Save routing policy candidate. | Yes | Creates or updates candidate policy. |
| POST | `/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/submit` | Submit routing policy version. | Yes | Updates lifecycle status. |
| GET | `/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/promotion-gates` | Evaluate routing policy promotion gates. | Yes | None |
| POST | `/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/approve` | Approve routing policy version. | Yes | Updates lifecycle status. |
| POST | `/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/activate` | Activate routing policy version. | Yes | Changes active routing policy. |
| POST | `/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/rollback` | Roll back routing policy version. | Yes | Restores governed version. |

Routing policies govern review mode and route thresholds. They are separate from
claim adjudication.

## Datasets, Features, And Model Evaluation Lineage

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/datasets` | List external dataset versions. | Yes | None |
| POST | `/api/v1/ops/datasets` | Register external dataset metadata. | Yes | Persists dataset catalog, splits, and schema fields. |
| GET | `/api/v1/ops/datasets/{dataset_id}` | Return dataset detail. | Yes | None |
| POST | `/api/v1/ops/datasets/{dataset_id}/mappings` | Add source-to-canonical field mapping. | Yes | Persists field mapping. |
| GET | `/api/v1/ops/factors/readiness` | Return factor readiness by profiled fields. | Yes | None |
| POST | `/api/v1/ops/feature-sets` | Register feature-set version. | Yes | Persists feature-set lineage. |
| POST | `/api/v1/ops/model-datasets` | Register model dataset built from a feature set. | Yes | Persists model dataset lineage. |
| GET | `/api/v1/ops/model-evaluations` | List model evaluation runs and lineage. | Yes | None |
| POST | `/api/v1/ops/model-evaluations` | Register model evaluation metrics and artifacts. | Yes | Persists evaluation and lineage audit event. |
| GET | `/api/v1/ops/model-evaluations/{evaluation_run_id}` | Return model evaluation detail. | Yes | None |

Dataset APIs store metadata and URIs. They do not store full Parquet rows in
PostgreSQL.

## Models

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/models` | List model versions. | Yes | None |
| GET | `/api/v1/ops/models/{model_key}/performance` | Return model performance and drift summary. | Yes | None |
| GET | `/api/v1/ops/models/{model_key}/promotion-gates` | Evaluate model promotion readiness. | Yes | None |
| GET | `/api/v1/ops/models/{model_key}/retraining-readiness` | Evaluate retraining readiness. | Yes | None |
| GET | `/api/v1/ops/models/{model_key}/retraining-jobs` | List retraining jobs for a model. | Yes | None |
| POST | `/api/v1/ops/models/{model_key}/retraining-jobs` | Create retraining job. | Yes | Creates job and audit evidence. |
| POST | `/api/v1/ops/model-retraining-jobs/{job_id}/status` | Update retraining job status. | Yes | Updates job status and audit evidence. |
| POST | `/api/v1/ops/model-retraining-jobs/claim-next` | Claim next queued retraining job. | Yes | Assigns job to worker actor. |
| POST | `/api/v1/ops/model-retraining-jobs/{job_id}/output` | Complete retraining job with output artifacts. | Yes | Persists candidate model and artifact evidence. |
| POST | `/api/v1/ops/models/{model_key}/promotion-reviews` | Submit model promotion review. | Yes | Records human review evidence. |
| POST | `/api/v1/ops/models/{model_key}/activate` | Activate approved model version. | Yes | Changes active model and records audit trail. |
| POST | `/api/v1/ops/models/{model_key}/rollback` | Roll back active model to governed target. | Yes | Restores active model version and records audit trail. |

Model APIs govern the demo and pilot model lifecycle. They are not a complete
production model training system.

## Provider, Member, And Scheme Intelligence

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/members/{member_id}/profile-summary` | Return member profile risk summary. | Yes | None |
| GET | `/api/v1/ops/providers/risk-summary` | Return provider risk, peer, graph, and history signals. | Yes | None |
| GET | `/api/v1/ops/fwa-schemes` | List FWA scheme taxonomy and governance metadata. | Yes | None |

These endpoints help explain why a claim is suspicious and where review should
be routed.

## Knowledge And Agent Workflows

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/knowledge/cases` | List confirmed knowledge cases. | Yes | None |
| POST | `/api/v1/ops/knowledge/cases` | Publish a confirmed knowledge case. | Yes | Persists case and audit evidence. |
| POST | `/api/v1/knowledge/search-similar` | Search similar FWA knowledge cases. | Yes | Records retrieval evidence where applicable. |
| POST | `/api/v1/agent/cases/investigate` | Generate assistive investigation package. | Yes | Persists agent run and audit evidence. |

Agent responses must include `decision_boundary: assistive_only`. They are
evidence packages for human review, not autonomous decisions.

## Common Error Shape

The API uses a simple JSON error shape for contract-facing endpoints:

```json
{
  "code": "ERROR_CODE",
  "message": "Human-readable message"
}
```

## Idempotency And Evidence

Writeback-style endpoints should send stable identifiers and evidence refs.
Evidence refs should be structured pointers, for example:

- `rule_runs:EARLY_CLAIM`
- `model_scores:baseline_fwa`
- `knowledge_cases:KC-1001`
- `agent_run:agent_CLM-0287`
- `audit:audit-id`

Do not place PII in `notes`, `summary`, or `evidence_refs`.
