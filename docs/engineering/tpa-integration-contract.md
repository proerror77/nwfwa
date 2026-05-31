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

### Score Claim

`POST /api/v1/claims/score`

Minimal stored-claim request:

```json
{
  "source_system": "tpa-demo",
  "claim_id": "CLM-0287"
}
```

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
    "latency_ms": 0
  },
  "top_reasons": ["..."],
  "evidence_refs": ["..."]
}
```

`model_score` exposes the L4 supervised model boundary for TPA panels and audit review: model key/version, runtime backend, score, explanations, and baseline FWA sub-probabilities. These fields remain assistive signals and do not make an automatic claim decision.

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

### Investigation Result Writeback

`POST /api/v1/investigations/results`

Request:

```json
{
  "claim_id": "CLM-0287",
  "investigation_id": "INV-DEMO-SMOKE",
  "outcome": "confirmed_fwa_review_needed",
  "confirmed_fwa": true,
  "financial_impact_type": "estimated_impact",
  "saving_amount": "8200.00",
  "currency": "CNY",
  "notes": "Evidence-backed manual review outcome.",
  "evidence_refs": [
    "audit:audit_...",
    "rule_runs:EARLY_CLAIM",
    "knowledge_cases:KC-1001"
  ]
}
```

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

Returns the claim-level audit timeline, including scoring, investigation, QA, and governed operations events where applicable.

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
