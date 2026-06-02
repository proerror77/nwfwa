# API Reference

All local and pilot APIs use JSON. Except for `GET /api/v1/health` and
`GET /api/openapi.json`, callers should send `x-api-key`.

`GET /api/openapi.json` is an application route that returns the API contract.
It is not itself listed as an OpenAPI `paths` item.

Local demo API key:

```text
dev-secret
```

## API Groups

- Health and contract
- Inbound claim inbox
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

`/api/v1/health` includes `api_key_configuration`,
`source_system_configuration`, `database_configuration`,
`model_service_configuration`, `object_storage_configuration`,
`customer_scope_configuration`, `retention_policy_configuration`, and
`backup_restore_configuration`, `pii_masking_configuration`, and
`key_rotation_configuration`, `network_allowlist_configuration`, and
`alert_routing_configuration`, and `observability_exporter_configuration`
and `agent_policy_configuration` checks. They report `local_dev_key` when the
API is still using the local `dev-secret` key,
`local_demo_source` when the API is still using the local `tpa-demo` source
system, `local_dev_database` when the API is still using the local development
database URL, `local_dev_model_service` when the API is still using the local
development model service URL, `heuristic_model_scorer` when the API is using
the heuristic fallback scorer, `local_demo_object_storage` when the API is still
using local demo artifact storage, `local_demo_customer_scope` when the API is
still using the local demo customer scope, `local_demo_retention_policy` when
the API is still using the local demo retention policy id,
`local_demo_backup_restore` when the API is still using the local demo backup
and restore plan id, `local_demo_pii_masking` when the API is still using the
local demo PII masking policy id, `local_demo_key_rotation` when the API is
still using the local demo key rotation policy id,
`local_demo_network_allowlist` when the API is still using the local demo
network allowlist id, `local_demo_alert_routing` when the API is still using
the local demo alert routing policy id, `local_demo_observability_exporter`
when the API is still using the local demo observability exporter endpoint, and
`local_demo_agent_policy` when the API is still using the local demo Agent tool
policy id, and `configured` after non-default values are configured. The
response never exposes the configured key, source-system value, database URL,
model service URL, object storage URI, customer scope id, retention policy id,
backup restore plan id, PII masking policy id, key rotation policy id, network
allowlist id, alert routing policy id, observability exporter endpoint, or
Agent policy id.

The response also includes `pilot_readiness.status`. It is `ready` only when
all customer pilot configuration checks report `configured`; otherwise it is
`not_ready` and `pilot_readiness.blocking_checks` lists the non-secret check
names and statuses that still block customer pilot traffic.

## Inbound Claim Inbox

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/inbox/claims/normalize` | Validate and normalize a raw TPA or claim-system payload before scoring. | Yes | Persists a PII-safe inbox audit event and API call record with raw payload ref, mapping version, canonical context summary, validation findings, data-quality signals, and evidence refs. |

The inbox endpoint is the boundary for customer-specific payloads such as an
`AiClaim Core` envelope with `systemCode`, `transNo`, and nested `reportCase`.
It uses a stable SHA-256 fingerprint of `systemCode + transNo + reportNo` for
internal `audit_id`, `run_id`, `raw_payload_ref`, and `idempotency_key` handles
so governance logs do not expose raw source transaction ids. It does not
directly score the claim. Callers should resolve blocking
validation errors and required scoring fields before submitting the canonical
context to `/api/v1/claims/score`.
The audit event stores source trace metadata and validation outcomes, not the
full raw PII-bearing payload. Its payload includes a PII-safe `source_paths`
summary gathered from normalized document, bill-line, product, and liability
evidence paths.
`canonical_claim_context.claim_header` preserves service, receive, and accident
dates for timing and waiting-period features. Epoch-millisecond source dates are
normalized with the source business timezone for the adapter. The current
`AiClaim Core` mapping uses `Asia/Shanghai`, so China-business midnight values
do not shift to the previous UTC date. Claim header, bill-line invoice dates,
and medical-record evidence dates retain the original raw epoch-millisecond
values beside the normalized date plus `source_timezone` so QA, Agent summaries,
and audit review can reconstruct the source-system interpretation.
Claim-level date checks report `date_inconsistency` on
`reportCase.accidentDate` when the accident date is later than
`claimReceiveDate`.
When `reportCase.claimAmount` is missing but source invoice totals are
available, `claim_header.total_amount` is derived from those invoices and
`data_quality_signals` includes `missing_claim_amount`.
Identity consistency checks compare accident person, policy insured person,
invoice person, and medical-record patient name when present. Mismatches are
reported through `data_quality_signals` as `identity_mismatch`.
`canonical_claim_context.member_policy_snapshot` exposes only masked member and
certificate identifiers, certificate type, gender, birth date, first-apply date,
source timezone, raw epoch-millisecond member/policy/coverage/liability date
fields, and social-insurance participation fields needed for routing and
feature calculation.
`canonical_claim_context.document_evidence` contains one normalized document
entry per source `medicalRecordInfoList` record, including claim nature,
medical-record type, chief complaint, current medical history, past history,
visit dates, source timezone, raw epoch-millisecond date fields, `source_path`
such as `reportCase.medicalRecordInfoList[n]`, and its own source refs.
Structured free-text fields are redacted before they leave the inbox boundary.
Before redaction, medical-record text normalization converts literal `/n`
separators to line breaks, removes BOM/replacement-character OCR artifacts,
normalizes full-width or non-breaking spaces, collapses repeated line-internal
whitespace, and drops empty lines.
`canonical_claim_context.itemized_bill_lines` preserves fee-detail lines from
every source invoice across all source policies, not only the primary policy or
primary invoice. Each line also carries invoice-level bill type, document type,
social-insurance type, department, medical type, claim nature, start/end dates,
invoice payment totals, fee-group amount, fee-group other amount, Medicare
prorated percentage, invoice-level provider context, source timezone, raw
epoch-millisecond invoice date fields when those fields exist in the raw
payload, and `source_path` such as
`reportCase.policyList[p].invoiceList[i].feeList[f].feeDetailList[d]`.
Invoice-level diagnosis gaps are reported as warnings on the matching
`reportCase.policyList[n].invoiceList[m].feeList` path.
For policy coverage, `member_policy_snapshot.product_liabilities` preserves
every source product and claim-liability window across all source policies,
including source policy id, waiting-period candidate dates, serious-disease
liability markers, main-liability markers, source timezone, raw
epoch-millisecond product/liability window dates, raw source paths, and evidence
refs.
Each entry carries `source_path` for the source
`reportCase.policyList[n].productList[m]`; liability rows also carry
`liability_source_path` for
`reportCase.policyList[n].productList[m].claimLiabilityList[k]`. The top-level
`product_code` and `liability_code` fields are primary values for compatibility,
not the complete coverage list.
Products without a source `claimLiabilityList` are still preserved in the same
array as product-only entries with `liability_*` fields and
`liability_source_path` set to `null`.
Coverage readiness validation scans every source policy and the same product
list. Missing policy limits, non-primary product or liability mismatches, and
policy-level window mismatches are returned as field-level warnings such as
`reportCase.policyList[n].coverageLimit`,
`reportCase.policyList[n].validateDate`, or
`reportCase.policyList[n].productList[m].validateDate` and block direct scoring
through `scoring_ready = false`.

## Runtime Scoring

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| POST | `/api/v1/claims/score` | Score a claim through the FWA runtime. | Yes | Persists scoring run, feature values, rule runs, model scores, audit events, API call record, and possible lead. |

Main request modes:

- stored demo claim by `source_system` and `claim_id`
- submitted claim payload with member, policy, provider, diagnosis, procedure,
  amount, dates, and context fields
- normalized inbox context by `source_system` and `canonical_claim_context`,
  using the `canonical_claim_context` returned from
  `/api/v1/inbox/claims/normalize`

These request modes are mutually exclusive. Callers should not combine
`claim_id`, full payload fields, and `canonical_claim_context` in the same
scoring request.

Request fields that affect policy selection:

- `review_mode`: separates `pre_payment` and `post_payment` behavior.
- `source_system`: scopes stored-claim lookup and audit source.
- `claim_id`: identifies stored demo or pilot claim records.
- `customer_scope_id`: not accepted from the request body. It is derived from
  the authenticated API key configuration and persisted into scoring audit
  payloads for tenant/customer-scope traceability.
- `canonical_claim_context`: carries normalized claim header, member/policy,
  provider, bill-line, and document evidence from the inbox boundary into
  runtime scoring.
  Scoring audit events persist `canonical_claim_context_trace` with normalized
  evidence refs and source refs for QA, Agent summaries, and audit review.

`review_mode` participates in routing policy, active model, and threshold
selection. It does not change the assistive-only decision boundary.

Main response fields:

- `run_id`
- `audit_id`
- risk score and RAG band
- recommended action
- score layers, each with layer-level `evidence_refs`
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
case SLA status, saving attribution, and `audit_coverage` for normalized inbox
trace governance. `audit_coverage.scoring_runs` counts successful
`scoring.completed` audit events, `canonical_trace_runs` counts events with
`canonical_claim_context_trace`, and `canonical_trace_coverage` is the ratio
used by Operations Studio.
`value_measurement` separates observed `prevented_payment` and
`recovered_amount` from estimated `avoided_future_exposure`,
`deterrence_estimate`, and aggregate `estimated_impact`, with review cost,
false-positive operational cost, reviewer capacity hours, net value, and an
evidence caveat.
Each `saving_attributions` item carries `financial_impact_type` so source-level
rule, model, and agent ROI can distinguish prevented payment, recovered amount,
avoided exposure, deterrence estimate, and other estimated impact instead of
mixing them into one source total.

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

Writebacks append evidence. They do not adjudicate claims. QA queue items carry
`canonical_source_refs` and `canonical_evidence_refs` when a prior normalized
scoring audit event exists for the claim, allowing reviewers to trace sampled
cases back to source bill lines and documents. Investigation result writeback merges
the latest successful normalized scoring trace's canonical evidence refs into
the persisted investigation result, response, outcome labels, and
`investigation.result.received` audit event. QA result writeback merges the
latest successful normalized scoring trace's canonical evidence refs into the
persisted QA review, response, and `qa.result.received` audit event.

## Medical Review

| Method | Path | Purpose | Auth | Side effects |
| --- | --- | --- | --- | --- |
| GET | `/api/v1/ops/medical-review/queue` | List claims needing clinical evidence review. | Yes | None |
| POST | `/api/v1/ops/medical-review/results` | Submit medical review result. | Yes | Appends audit event and medical-review feedback label. |

Medical review focuses on clinical evidence gaps, medical necessity, diagnosis
and procedure consistency, and reviewer feedback. Queue items expose
`canonical_source_refs` and `canonical_evidence_refs` when the scoring audit
came from a normalized inbox context. Medical review result writeback merges
canonical evidence refs from the referenced scoring audit into the persisted
review, response, and `medical.review.recorded` audit event. Writeback may also
include `clinical_outcomes` such as `documentation_issue`,
`medical_necessity_review_required`, or `insufficient_evidence`; when omitted,
the platform derives a compatible controlled outcome from `decision` for label
governance.

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
record human context and evidence refs. `/api/v1/ops/audit-events` supports
operational filters for event type, event group, actor, claim, run, rule,
model, routing policy, review mode, QA, Agent, data lineage, and
`has_canonical_trace=true` to isolate normalized inbox scoring trace events.

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
| POST | `/api/v1/ops/rules/discover` | Discover candidate rules from labeled sample claims. | Yes | Records discovery provenance and candidate metrics. |
| POST | `/api/v1/ops/rules/{rule_id}/submit` | Submit rule for governance. | Yes | Updates lifecycle status and audit trail. |
| POST | `/api/v1/ops/rules/{rule_id}/approve` | Mark rule approved with reviewer evidence. | Yes | Updates lifecycle status and audit trail. |
| POST | `/api/v1/ops/rules/{rule_id}/publish` | Publish approved rule. | Yes | Updates lifecycle status and audit trail. |
| POST | `/api/v1/ops/rules/{rule_id}/rollback` | Move active rule back to approved status. | Yes | Records rollback audit evidence. |

Rule APIs support deterministic controls. They should not silently change active
customer behavior without lifecycle and audit evidence.
Rule promotion gates treat a stored backtest as usable routing evidence only
when its own blockers are cleared; underpowered samples, weak precision/recall,
excess false positives, or review-capacity overflow keep the deterministic
backtest gate blocked.

Rule lifecycle caveat: `approve` currently writes approved status with evidence
refs. `rollback` moves an active rule back to approved status; it does not select
an arbitrary older version.

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

Model evaluation registration should include the FWA `scheme_family` dimension
so performance, drift, and promotion gates can be interpreted by FWA pattern.
Promotion-ready evaluations should also include time/group split evidence in
`metrics_json`: `time_group_split_status = "passed"`, a non-empty
`time_split_field`, and non-empty `group_split_fields`. Without those fields,
model promotion gates keep routing blocked because FWA validation must not rely
on random train/test splits. Promotion-ready evaluations must also carry
`pilot_validation_status = "passed"` or `customer_validation_status = "passed"`;
public or Kaggle-inspired offline research datasets remain research inputs and
cannot serve as production promotion evidence.

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
| POST | `/api/v1/ops/model-retraining-jobs/{job_id}/output` | Register validation output for a retraining job. | Yes | Requires validation state, creates candidate model and evaluation evidence. |
| POST | `/api/v1/ops/models/{model_key}/promotion-reviews` | Submit model promotion review. | Yes | Records human review evidence. |
| POST | `/api/v1/ops/models/{model_key}/activate` | Activate the latest governed candidate that passes gates. | Yes | Demotes previous active model, activates target, and records audit trail. |
| POST | `/api/v1/ops/models/{model_key}/rollback` | Roll back active model to recorded previous active version. | Yes | Restores approved previous active model and records audit trail. |

Model APIs govern the demo and pilot model lifecycle. The local ML service now
produces a production-style baseline bundle with artifact checksum/signature,
serving manifest, feature materialization manifest, shadow comparison report,
drift report, and segment fairness report. External scheduler, serving image
registry, secrets manager, observability dashboards, and customer environment
deployment remain outside this repository.

Model rollback audit payloads separate the restored and replaced versions:
`previous_active_version` is the approved historical version restored to
`active`, while `replaced_active_version` is the version moved from `active`
back to `approved`.

Governed retraining boundary: retraining jobs model the offline worker contract,
artifact evidence, validation metrics, MLOps reports, and candidate
registration. They do not represent automatic production promotion or automatic
customer-environment deployment.

Promotion gates should be read as the policy checklist for activation. They
cover data quality, label provenance, drift, promotion review evidence, feature
reproducibility, explanation artifacts, and validation quality.

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
evidence packages for human review, not autonomous decisions. When a prior
`scoring.completed` audit event for the claim contains
`canonical_claim_context_trace`, the persisted Agent context snapshot carries
the trace and source refs so reviewers can connect the summary back to
normalized inbox evidence.

Human approval decisions for Agent runs must include
`agent_run:{agent_run_id}` in request `evidence_refs`. The platform appends
`policy:{FWA_AGENT_POLICY_ID}` to the persisted approval and
`agent.approval.decided` audit event so the approval gate remains tied to the
configured Agent governance policy.

Knowledge case publish requires confirmed review evidence such as
`investigation_results:*` or `qa_reviews:*`. When `source_claim_id` has a prior
canonical scoring trace, knowledge case publish preserves canonical evidence
refs from that trace in the saved case, similar-case search result, and
`knowledge.case.published` audit event.

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
