# TPA Integration Contract

This contract is the pilot-facing boundary for external TPA and claim administration systems.
The Operations Studio and internal ops APIs are separate surfaces.

## Transport

- Base URL: customer environment specific, for example `http://127.0.0.1:8080`.
- Authentication: every endpoint requires `x-api-key`.
- Content type: JSON request bodies must use `content-type: application/json`.
- OpenAPI: `GET /api/openapi.json`.
- Error shape: all documented errors return:

```json
{
  "code": "INVALID_API_KEY",
  "message": "invalid api key"
}
```

## Core Endpoints

### Normalize Raw Claim Inbox Payload

`POST /api/v1/inbox/claims/normalize`

Use this endpoint when the TPA or claim administration system sends a
customer-specific raw intake payload rather than the canonical scoring request.
The current pilot-shaped adapter supports an `AiClaim Core` envelope with
`systemCode`, `transNo`, and a nested `reportCase`.
For local correction checks, run the mock client in normalize-only mode against
a raw file:

```bash
python3 scripts/demo/tpa_mock_client.py \
  --base-url http://127.0.0.1:8080 \
  --api-key dev-secret \
  --inbox-payload-file /Users/proerror/Downloads/req.json \
  --normalize-only
```

The command prints the normalize response and exits non-zero when
`scoring_ready` is false, so the returned `validation_errors` are the concrete
input-box fields to fix before scoring. In normalize-only mode the mock client
also adds a local `correction_hints` array to the printed JSON. These hints mark
whether each finding blocks direct scoring and translate the remediation into a
short next action, for example matching the API key source system to
`systemCode = "AiClaim Core"` or mapping
`reportCase.policyList[0].coverageLimit` before scoring. For supported
blocking fields, the printed JSON also includes `correction_overlay_template`,
which is a minimal overlay scaffold that can be saved as the correction file.
The template covers policy coverage limits and policy/product/liability date
window fields such as `validateDate`, `expireDate`, and `claimValidateDate`.
To avoid copying the template by hand, add `--write-correction-template` in
normalize-only mode. The mock client writes the template to a local JSON file
without mutating the raw payload; existing files are preserved unless
`--overwrite-correction-template` is passed.

To test corrections without rewriting the raw customer payload, pass a local
JSON overlay with `--inbox-correction-file`. Objects are merged by key and
arrays are merged by index, so a coverage-limit correction can be as small as:

```json
{
  "reportCase": {
    "policyList": [
      {
        "coverageLimit": 20000
      }
    ]
  }
}
```

To write that scaffold directly:

```bash
python3 scripts/demo/tpa_mock_client.py \
  --base-url http://127.0.0.1:8080 \
  --api-key dev-secret \
  --inbox-payload-file /Users/proerror/Downloads/req.json \
  --write-correction-template /Users/proerror/Downloads/req-correction.json \
  --normalize-only
```

After replacing placeholder values in the generated overlay, rerun the
normalize check:

```bash
python3 scripts/demo/tpa_mock_client.py \
  --base-url http://127.0.0.1:8080 \
  --api-key dev-secret \
  --inbox-payload-file /Users/proerror/Downloads/req.json \
  --inbox-correction-file /Users/proerror/Downloads/req-correction.json \
  --normalize-only
```

The endpoint validates the envelope, checks source-system identity, normalizes
epoch-millisecond dates, masks PII-bearing values, maps claim-header,
medical-record, invoice, provider, product, and liability fields into a
canonical claim context, and returns data-quality signals such as identity
mismatch, date inconsistency, missing coverage limit, coverage-window mismatch,
missing claim amount, and policy-liability mismatch. It also raises
`document_invoice_mismatch` for the matching invoice path when any structured
invoice diagnosis list does not align with medical-record diagnoses, including
non-primary invoices.
Canonical claim headers preserve normalized service, receive, and accident
dates for claim-timing features.
Identity mismatch compares accident person, policy insured person, every
invoice person, and every medical-record patient name when those fields are
present.
Canonical member snapshots expose masked member and certificate identifiers,
certificate type, gender, birth date, source timezone, and top-level raw
epoch-millisecond date metadata for member, policy, coverage, and liability
windows: `source_timezone`, `member_birth_date_raw_epoch_ms`,
`policy_first_apply_date_raw_epoch_ms`, `coverage_start_date_raw_epoch_ms`,
`coverage_end_date_raw_epoch_ms`, `liability_start_date_raw_epoch_ms`,
`liability_claim_start_date_raw_epoch_ms`, and
`liability_end_date_raw_epoch_ms`. Raw `insuredNo` and `certNo` must not be
persisted in API-visible canonical outputs.
Canonical document evidence preserves every `medicalRecordInfoList` entry as a
separate document with claim nature, medical-record type, chief complaint,
current medical history, past history, extracted diagnosis, procedure,
prescription, normalized visit date, first-happen date, operation-start date,
and source refs. Structured free-text fields are normalized and redacted before
they appear in API-visible canonical output.
Medical-record text normalization converts literal `/n` separators to line
breaks, drops BOM/replacement-character OCR artifacts, normalizes full-width or
non-breaking spaces, collapses repeated line-internal whitespace, and removes
empty lines before PII redaction and extraction.
Canonical bill lines preserve fee details from every source invoice across all
source policies. Each line keeps its source invoice id, invoice bill type,
invoice document type,
social-insurance type, department, medical type, invoice claim nature, invoice
start/end dates, diagnosis list, invoice-level payment totals, fee-group
amount, fee-group other amount, social-insurance amount, Medicare prorated
percentage, invoice-level provider code/name/class/type/city/province/network
flags, `source_path` for
`reportCase.policyList[p].invoiceList[i].feeList[f].feeDetailList[d]`, and
`invoice:{invoiceNo}:fee_detail:{detailId}` evidence ref.
Canonical claim header `total_amount` is the sum of all source invoice
`feeAmount` values across all policies; it is not limited to the primary
policy or primary invoice. When
`reportCase.claimAmount` is missing but invoice totals are available, the inbox
returns a `missing_claim_amount` warning instead of overwriting the raw payload.
Claim-level date checks compare `claimReceiveDate` with `accidentDate`; an
accident date after receive date returns `date_inconsistency` on
`reportCase.accidentDate`.
Invoice date checks compare `claimReceiveDate` with every invoice `startDate`;
non-primary invoice dates after receive date return `date_inconsistency` on the
matching `reportCase.policyList[n].invoiceList[m].startDate` path.
Invoice date windows also validate every invoice `endDate`; an end date earlier
than `startDate` returns `date_inconsistency` on the matching
`reportCase.policyList[n].invoiceList[m].endDate` path.
Medical-record date checks compare `claimReceiveDate` with every
`medicalRecordInfoList[n].visitDate`; visits after receive date return
`date_inconsistency` on the matching medical-record path.
They also compare every `medicalRecordInfoList[n].firstHappenDate` with
`claimReceiveDate`; first-happen dates after receive date return
`date_inconsistency` on the matching first-happen-date path.
They also compare every `medicalRecordInfoList[n].operationStartDate` with
`claimReceiveDate`; operation dates after receive date return
`date_inconsistency` on the matching operation-date path.
Canonical document evidence preserves `source_path` for each
`reportCase.medicalRecordInfoList[n]` record so QA, Agent investigation, and
audit review can trace normalized and redacted text back to the exact source
record without exposing raw PII.
Diagnosis-item support checks run per invoice. If any invoice contains fee
details without structured diagnosis context, the endpoint returns a
field-level `diagnosis_item_mismatch` warning for that invoice.
Canonical policy snapshots preserve all source product/liability windows across
source policies in `member_policy_snapshot.product_liabilities`; primary
`product_code` and `liability_code` remain convenience fields for first-pass
routing only. Each product-liability entry preserves source `policy_id`,
`source_path` for `reportCase.policyList[n].productList[m]`, `main_liability`
from source `mainLiab`, and parses `isSeriousDiseaseLiability` `Y`/`N` values
into booleans. Liability-level rows also preserve `liability_source_path` for
`reportCase.policyList[n].productList[m].claimLiabilityList[k]`.
Products without a source `claimLiabilityList` are emitted as product-only
entries in the same array with `liability_*` fields set to `null`, so downstream
reviewers can distinguish absent liability data from a dropped product; their
`liability_source_path` is `null`.
They also expose `policy_first_apply_date` and
`insured_with_social_insurance` for policy-tenure, waiting-period, and coverage
constraint features.
Coverage readiness validation scans every policy and every product/liability
entry. A missing policy coverage limit, or a non-primary policy, product, or
liability that does not cover the service date, produces a structured warning
on the matching source path, for example
`reportCase.policyList[n].coverageLimit` or
`reportCase.policyList[n].validateDate`, and keeps `scoring_ready = false`
until the customer adapter or reviewer resolves the coverage context.
Canonical bill lines include fee amount, self-pay, own-expense, other-payment,
and social-insurance amount mapped from the source invoice, fee group, and fee
detail levels without collapsing those levels into one amount.
Each request writes a PII-safe audit event and API call record with source
trace metadata. The audit payload stores raw payload refs, mapping version,
validation results, data-quality signals, and a PII-safe `source_paths` summary
for normalized evidence rows, not the full raw medical or identity payload.

`calculateRisk = N` is treated only as a source-system hint. It does not bypass
FWA scoring unless a customer-specific config explicitly permits that behavior.

The response includes:

- `external_message_id`: `systemCode + transNo + reportNo` source identity
  returned to the caller for correlation.
- `audit_id` and `run_id`: trace handles for Governance audit search. They use
  a stable external-message fingerprint and do not expose raw source transaction
  ids.
- `idempotency_key`: stable inbox normalization key derived from a SHA-256
  fingerprint of the external message id.
- `mapping_version`: adapter mapping version used for audit replay.
- audit `source_paths`: PII-safe source-path summary available in the persisted
  inbox audit payload for normalized document, bill-line, product, and liability
  evidence rows.
- `validation_result`: `accepted`, `accepted_with_warnings`, or `rejected`.
- `scoring_ready`: whether the normalized context can proceed directly to
  scoring.
- `validation_errors`: field-level findings with `field_path`, `severity`, and
  `remediation`.
- `canonical_claim_context`: normalized claim header, member/policy snapshot,
  provider snapshot, itemized bill lines, and document evidence.
- `data_quality_signals` and `evidence_refs`.

Documented errors:

- `400` rejected payload with structured `validation_errors`.
- `401` missing or invalid API key.

### Score Claim

`POST /api/v1/claims/score`

Minimal stored-claim request:

```json
{
  "source_system": "tpa-demo",
  "claim_id": "CLM-0287"
}
```

Normalized inbox context request:

```json
{
  "source_system": "tpa-demo",
  "canonical_claim_context": {
    "claim_header": {
      "external_claim_id": "CLM-0287",
      "total_amount": 8800,
      "currency": "CNY",
      "service_date": "2026-01-06"
    },
    "member_policy_snapshot": {
      "masked_member_id": "masked-member-1",
      "policy_id": "POL-0287",
      "product_code": "MED",
      "coverage_start_date": "2026-01-01",
      "coverage_end_date": "2026-12-31",
      "coverage_limit": 10000
    },
    "provider_snapshot": {
      "provider_code": "PRV-0287",
      "name": "Northwind Hospital",
      "city": "SH"
    },
    "itemized_bill_lines": [],
    "document_evidence": []
  }
}
```

Use this mode after `POST /api/v1/inbox/claims/normalize` returns
`scoring_ready = true` or a reviewer resolves blocking validation findings.
`claim_id`, full claim payload fields, and `canonical_claim_context` are
mutually exclusive request modes. Canonical source paths and evidence refs from
bill lines and document evidence are preserved in the scoring response and
audit event. For normalized inbox scoring, the `scoring.completed` audit
payload includes `canonical_claim_context_trace` with `input_mode`,
`evidence_refs`, and `source_refs` so QA and Agent summaries can trace the
score back to normalized bill-line and document sources without exposing raw
PII.

The response is audit-backed and must be treated as assistive risk routing, not an automatic denial:

```json
{
  "run_id": "run_...",
  "audit_id": "audit_...",
  "claim_id": "CLM-0287",
  "risk_score": 87,
  "rag": "Red",
  "recommended_action": "ManualReview",
  "model_score": {
    "model_key": "baseline_fwa",
    "model_version": "0.1.0",
    "runtime_kind": "python_http",
    "execution_provider": "cpu",
    "score": 83,
    "label": "HIGH_RISK",
    "explanations": [
      {
        "feature": "claim_amount_to_limit_ratio",
        "direction": "increases_risk",
        "contribution": 0.82,
        "reason": "理赔金额占保障额度比例较高"
      }
    ],
    "metadata": {
      "fraud_probability": 0.83,
      "abuse_probability": 0.61,
      "waste_probability": 0.47
    },
    "latency_ms": 12
  },
  "top_reasons": ["..."],
  "evidence_refs": [
    "model_scores:baseline_fwa",
    "model_versions:baseline_fwa:0.1.0"
  ]
}
```

`model_score` exposes the L4 supervised model boundary for TPA panels and audit review: model key/version, runtime backend, score, explanations, and baseline FWA sub-probabilities. `evidence_refs` also carries the exact `model_versions:{model_key}:{model_version}` reference used for scoring so audit replay can bind the score to a governed model version. These fields remain assistive signals and do not make an automatic claim decision.

Allowed `recommended_action` values:

- `StandardProcessing`
- `QaSample`
- `ManualReview`
- `RequestEvidence`
- `EscalateInvestigation`
- `PostPaymentAudit`
- `ProviderReview`
- `RecoveryReview`

Documented errors:

- `400` invalid or ambiguous scoring request.
- `401` missing or invalid API key.
- `404` stored claim id was not found.
- `502` model service failure.

### Member Profile Summary

`GET /api/v1/members/{member_id}/profile-summary`

Returns a compact policy and claim history profile for TPA panels.

Documented errors:

- `401` missing or invalid API key.
- `404` member id was not found.

### Similar Knowledge Cases

`POST /api/v1/knowledge/search-similar`

Request:

```json
{
  "claim_id": "CLM-0287",
  "diagnosis_code": "J10",
  "provider_region": "Shanghai",
  "tags": ["early_claim", "high_amount"]
}
```

Response:

```json
{
  "results": [
    {
      "case_id": "KC-1001",
      "similarity": 0.92,
      "summary": "...",
      "evidence_refs": ["knowledge_cases:KC-1001"]
    }
  ]
}
```

Documented errors:

- `400` invalid query, including blank diagnosis, region, or tags.
- `401` missing or invalid API key.

Similarity results return the saved knowledge case evidence refs. If the case
was published with `source_claim_id` and that claim has a prior
`canonical_claim_context_trace`, those refs include the canonical invoice,
document, or line-item evidence preserved from the scoring audit.

### Investigation Result Writeback

`POST /api/v1/investigations/results`

Request:

```json
{
  "case_id": "case_CLM-0287",
  "claim_id": "CLM-0287",
  "investigation_id": "INV-DEMO-SMOKE",
  "outcome": "confirmed_fwa_review_needed",
  "confirmed_fwa": true,
  "financial_impact_type": "estimated_impact",
  "saving_amount": "8200.00",
  "currency": "CNY",
  "notes": "Evidence-backed manual review outcome.",
  "evidence_refs": [
    "investigation_cases:case_CLM-0287",
    "audit:audit_...",
    "rule_runs:EARLY_CLAIM",
    "knowledge_cases:KC-1001"
  ]
}
```

`case_id` is optional for claim-level TPA writebacks. When provided, it must
match an existing FWA case for the same claim and the platform projects the
final outcome, reviewer notes, and `investigation_id` onto that case for the
Operations Studio case list.
For claims scored from normalized inbox context, investigation writeback appends
canonical evidence refs from the latest successful scoring trace to the saved
investigation result, response, outcome label, saving attribution evidence, and
`investigation.result.received` audit event.

Response:

```json
{
  "claim_id": "CLM-0287",
  "event_type": "investigation.result.received",
  "event_status": "succeeded",
  "audit_id": "audit_investigation_INV-DEMO-SMOKE",
  "run_id": "pilot_investigation_INV-DEMO-SMOKE",
  "idempotency_key": "tpa-writeback:investigation.result.received:audit_investigation_INV-DEMO-SMOKE",
  "evidence_refs": ["audit:audit_..."]
}
```

Documented errors:

- `400` invalid writeback, unsupported financial impact type, negative saving amount, missing evidence, or PII in notes/evidence refs.
- `401` missing or invalid API key.
- `404` `case_id` was provided but no matching case exists for the claim.

### QA Result Writeback

`POST /api/v1/qa/results`

Request:

```json
{
  "qa_case_id": "QA-DEMO-SMOKE",
  "claim_id": "CLM-0287",
  "qa_conclusion": "issue_found_escalate",
  "issue_type": "alert_handling_incomplete",
  "feedback_target": "rules",
  "notes": "Alert handling evidence was reviewed.",
  "evidence_refs": ["audit:audit_...", "rule_runs:EARLY_CLAIM"]
}
```

Allowed `issue_type` values include the PRD-governed QA labels:

- `confirmed_fwa`
- `false_positive`
- `improper_payment`
- `insufficient_evidence`
- `abuse_not_fraud`
- `documentation_issue`
- `medical_necessity_issue`
- `policy_exclusion`

Response:

```json
{
  "claim_id": "CLM-0287",
  "event_type": "qa.result.received",
  "event_status": "succeeded",
  "audit_id": "audit_qa_QA-DEMO-SMOKE",
  "run_id": "pilot_qa_QA-DEMO-SMOKE",
  "idempotency_key": "tpa-writeback:qa.result.received:audit_qa_QA-DEMO-SMOKE",
  "evidence_refs": ["audit:audit_..."]
}
```

Documented errors:

- `400` invalid conclusion, issue type, feedback target, missing evidence, or PII in notes/evidence refs.
- `401` missing or invalid API key.

### Claim Audit History

`GET /api/v1/audit/claims/{claim_id}`

Returns the claim-level audit timeline, including scoring, investigation, QA,
and governed operations events where applicable. Normalized inbox scoring events
include `canonical_claim_context_trace` in the event payload.
Operations users can call `/api/v1/ops/audit-events?has_canonical_trace=true`
to list normalized inbox scoring events that carry this trace.
Agent investigation runs reuse the latest successful scoring trace for the
same claim in their persisted context snapshot, including source refs from
normalized bill lines and documents.
QA queue items also expose canonical source refs and canonical evidence refs
from the latest successful normalized scoring trace for reviewer grounding.
When a QA result is written back for the same claim, the platform appends those
canonical evidence refs to the saved QA review, writeback response, and
`qa.result.received` audit event. This keeps `/Users/proerror/Downloads/req.json`
style inbox payloads traceable through normalization, scoring, QA, and audit
without requiring the reviewer to manually copy every invoice or document ref.
Medical Review queue items follow the same trace contract for L5 medical
reasonableness: they expose canonical source/evidence refs from the scoring
audit, and `POST /api/v1/ops/medical-review/results` appends canonical evidence
refs from the referenced scoring audit to the review response and
`medical.review.recorded` audit event.

Documented errors:

- `401` missing or invalid API key.

## Idempotency

Writeback response `idempotency_key` is stable for the same business identifier:

- Investigation: `investigation_id`.
- QA: `qa_case_id`.

TPA clients may retry the same writeback with the same identifier. The platform upserts the corresponding audit event instead of creating duplicate timeline entries.

## PII Rules

Do not put PII in:

- `notes`
- `summary`
- `evidence_refs`
- free-text Agent or QA output

Use structured references such as `audit:*`, `rule_runs:*`, `agent_run:*`, `knowledge_cases:*`, `investigation_results:*`, and `qa_reviews:*`.

## Mock Client

Run the pilot mock client after the API server, Postgres seed, and ML service are running:

```bash
python3 scripts/demo/tpa_mock_client.py \
  --base-url http://127.0.0.1:8080 \
  --api-key dev-secret \
  --claim-id CLM-0287 \
  --member-id MBR-0287
```

The client exercises the six core endpoints and prints a compact summary containing `run_id`, `audit_id`, writeback idempotency keys, and audit event types.
