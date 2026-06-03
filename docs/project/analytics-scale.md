# Analytics Scale

This document describes the analytics-scale proof now present in the repository.
It is a staging contract for high-volume reporting, not a customer production
deployment.

## Purpose

PostgreSQL remains the operational source of truth for claims, scoring, cases,
rules, model governance, QA, and audit. ClickHouse is introduced as a derived
analytical event store for reports that would become expensive or awkward in the
operational database.

The implemented coverage is:

- derived ClickHouse event store;
- scheduled PostgreSQL-to-object-storage-to-ClickHouse export contract;
- rule and model drift queries;
- SLA, ROI, reviewer capacity, and false-positive cost queries;
- provider graph snapshot query.

## Files

- `analytics/clickhouse/schema.sql`: ClickHouse database and MergeTree tables.
- `analytics/clickhouse/dashboard_queries.sql`: dashboard query set.
- `scripts/ops/build_analytics_export.py`: local proof artifact generator.
- `scripts/ops/validate_analytics_scale.py`: static validation gate.
- `infra/k8s/staging/clickhouse.yaml`: staging ClickHouse service.
- `infra/k8s/staging/worker-cronjobs.yaml`: scheduled export-plan CronJob.

## Derived Tables

| Table | Purpose |
| --- | --- |
| `analytics_scoring_events` | scoring volume, RAG mix, action mix, model/rule-pack attribution |
| `analytics_rule_events` | rule hits, drift, false-positive labels, saving and false-positive cost |
| `analytics_model_events` | model score distribution, shadow delta, calibration, outcome labels |
| `analytics_case_sla_events` | queue SLA, triage time, closure time, breach rate |
| `analytics_value_events` | prevented, recovered, avoided exposure, gross saving, net value, ROI |
| `analytics_reviewer_capacity_events` | assigned/closed cases, utilization, precision at capacity |
| `analytics_provider_graph_snapshots` | provider relationship risk, concentration, cluster snapshots |

The tables use masked identifiers such as `claim_hash`, `member_hash`,
`provider_hash`, and `reviewer_id_hash`. Raw names, addresses, member numbers,
and raw payloads stay out of the analytics store.

## Local Proof

Run:

```bash
python3 scripts/ops/validate_analytics_scale.py
python3 scripts/ops/build_analytics_export.py \
  --output-dir artifacts/analytics-export \
  --object-storage-uri s3://nwfwa-staging-artifacts \
  --clickhouse-url http://clickhouse:8123 \
  --customer-scope-id staging-customer
```

The output includes:

- `analytics_export_manifest.json`
- `scheduled_exports.json`
- `schema.sql`
- `dashboard_queries.sql`
- `index.json`

## Local Runtime

Start ClickHouse with the demo stack:

```bash
docker compose -f infra/docker-compose.yml up -d clickhouse
```

The Compose service mounts `analytics/clickhouse/schema.sql` into
`/docker-entrypoint-initdb.d/001_schema.sql` so a fresh ClickHouse volume creates
the derived tables automatically.

## Kubernetes Staging

Staging includes:

- `clickhouse` StatefulSet and Service;
- `analytics-export-plan` worker CronJob;
- static validation through `scripts/ops/validate_k8s_staging.py`.

Render:

```bash
kubectl kustomize infra/k8s/staging
```

The CronJob emits the portable scheduled export plan. A customer environment can
replace that plan generator with the approved scheduler that performs the actual
PostgreSQL extraction, object-storage write, and ClickHouse load.

## Production Boundary

Still required for customer production:

- customer-approved data movement credentials;
- ClickHouse retention, backup, and access policy;
- live export job runner and retry/dead-letter handling;
- dashboard hosting and alert routing;
- validation against customer labels, holdouts, and shadow traffic.
