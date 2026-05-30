# FWA Phase 2 Operations Design

Date: 2026-05-27

Status: approved by continuation request

## Goal

Phase 2 turns the Phase 1 scoring runtime into an operations product slice for rule operators and model operators.

The deliverable is not a full no-code rule builder or complete MLOps platform. It is the smallest useful operations layer that can:

- show the active rule library and rule versions,
- backtest candidate rules against deterministic sample claims,
- move rules through a simple approval lifecycle,
- show model versions and basic model performance,
- expose these capabilities through Operations Studio pages backed by real APIs.

## Scope

### Included

Rules:

- `GET /api/v1/ops/rules`
- `GET /api/v1/ops/rules/{rule_id}`
- `POST /api/v1/ops/rules/backtest`
- `POST /api/v1/ops/rules/{rule_id}/submit`
- `POST /api/v1/ops/rules/{rule_id}/approve`
- `POST /api/v1/ops/rules/{rule_id}/publish`

Models:

- `GET /api/v1/ops/models`
- `GET /api/v1/ops/models/{model_key}/performance`

Studio:

- navigation can switch between pages,
- Rules page displays rule library, selected rule details, lifecycle controls, discovery provenance, and JSON backtest panels,
- Models page displays model versions, deployment boundary metadata, and selected performance metrics,
- Runtime Scoring remains unchanged except for shared API helpers.

Persistence:

- reuse existing `rules`, `rule_versions`, `model_versions`, `model_scores`, and `scoring_runs` tables,
- add only narrow columns or seed data if tests prove it is required,
- keep Postgres repository as the production adapter and in-memory repository as the test adapter.

Audit:

- lifecycle actions write audit events where the repository has enough context,
- Phase 2 API responses include enough IDs and statuses to trace rule/model state.

### Deferred

- drag-and-drop visual rule builder,
- full historical backtest over a customer data warehouse,
- rule ROI attribution,
- model retraining workflows,
- model drift alerting jobs,
- model registry artifact upload,
- production RBAC,
- QA-driven rule/model feedback automation.

## Architecture

The Phase 2 APIs stay in `apps/api-server`. Domain logic remains in Rust crates:

- `fwa-rules` owns rule lifecycle DTOs and deterministic backtest logic.
- `fwa-ml-runtime` continues to own model runtime DTOs.
- `apps/api-server::repository` owns persistence queries for operations APIs.
- `apps/web-console` owns the lightweight Operations Studio pages.

The scoring endpoint may continue using the deterministic demo rule set until rule publishing is wired into the active scoring path. Phase 2 operations APIs must still expose a real rule lifecycle and backtest surface, because that is the prerequisite for replacing hard-coded active rules in a later task.

## Rule Backtest Semantics

Backtest input:

```json
{
  "rule": {
    "rule_id": "candidate_early_high_amount",
    "version": 1,
    "name": "Early high amount",
    "conditions": [
      {
        "field": "days_since_policy_start",
        "operator": "<=",
        "value": 7
      }
    ],
    "action": {
      "score": 25,
      "alert_code": "EARLY_HIGH_AMOUNT",
      "recommended_action": "ManualReview",
      "reason": "保单生效后 7 天内发生高额理赔"
    }
  },
  "samples": [
    {
      "external_claim_id": "CLM-1",
      "claim_amount": "8000",
      "currency": "CNY",
      "service_date": "2026-01-06",
      "policy": {
        "external_policy_id": "POL-1",
        "coverage_start_date": "2026-01-01",
        "coverage_end_date": "2026-12-31",
        "coverage_limit": "10000"
      }
    }
  ]
}
```

Backtest output:

```json
{
  "sample_count": 1,
  "matched_count": 1,
  "match_rate": 1.0,
  "average_score_contribution": 25.0,
  "estimated_saving": "800.00",
  "matched_claim_ids": ["CLM-1"]
}
```

The first implementation uses `estimated_saving = matched claim amount * 0.10`, rounded to cents. This is intentionally simple and explicit.

## Model Performance Semantics

Model list returns known model versions from persistence or a deterministic baseline fallback:

- `baseline_fwa`
- version `0.1.0`
- runtime kind `python_http`
- execution provider `cpu`
- status `active`

Performance aggregates from `model_scores` and `scoring_runs` when rows exist. With no rows, the API returns zeros and `data_status = "empty"`, not fake precision/recall.

Metrics:

- scored runs,
- average score,
- high risk score count,
- latest scored time when available,
- data status.

## Testing

Required tests:

- rules list API returns seeded/default rules,
- rule detail API returns versions and status,
- backtest API returns expected match count and saving for deterministic samples,
- lifecycle actions change status in the in-memory repository,
- model list API returns baseline model,
- model performance API returns empty metrics when no scores exist,
- frontend lint/build continues passing.

## Acceptance Criteria

- Rule operators can inspect rule IDs, status, version, owner, score, and action.
- Rule operators can run a candidate JSON rule against sample payloads and see match rate and estimated saving.
- Rule operators can see whether discovered candidates came from labeled samples, which deterministic discovery mode produced them, and which governance path saves them into the rule lifecycle.
- Rule operators can submit, approve, and publish a rule through API calls.
- Model operators can inspect model versions, deployment boundary metadata, and basic performance metrics.
- Operations Studio Rules and Models pages are backed by API calls, not placeholder text.
- CI passes for repository health, Rust fmt/clippy/tests, Python tests, frontend lint/test/build, and migrations.
