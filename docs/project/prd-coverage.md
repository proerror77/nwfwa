# PRD Coverage

This document maps the product PRD to the current repository evidence. It is
intended to answer what is complete in code, what is only a staging proof, and
what still requires customer data or a customer environment.

## Summary

The repository has implemented the demo and pilot-contract surfaces that can be
completed without real customer data:

- inbound claim inbox, normalization, correction templates, and canonical trace;
- deterministic scoring, FWA rule pack, review modes, rule lifecycle, and
  promotion controls;
- customer-approved deterministic adjudication-policy boundary for hard-deny,
  straight-through, pending-evidence, manual-review, and score-only rules;
- lead, case, investigation, QA, medical review, label, and audit workflows;
- model registry, evaluation gates, retraining jobs, Rust artifact scoring,
  external training handoff, and scheduled MLOps monitoring-plan contracts;
- dataset, feature-set, public-data MVP, and label-governance contracts;
- knowledge search, assistive agent investigation, and AI evidence metadata;
- ClickHouse analytics-scale schema, dashboard queries, export contracts, and
  provider graph snapshots;
- Kubernetes staging manifests, container packaging, GitHub Environment staging
  package workflow, and pilot-foundation proof artifacts;
- Yew Operations Studio demo surface.

The remaining production boundary is not another local module. It is customer
validation and environment execution:

- real customer labels and label provenance;
- customer holdout validation and live shadow traffic;
- customer-approved production deployment, secrets, retention, observability,
  OCR/vector workers, analytics execution, and network controls;
- customer-executed live restore, rollback, alert, and operational drills beyond
  the staging `operational_drill_proof.json` contract.

## Machine-Checkable Proof

Generate the local proof artifact:

```bash
python3 scripts/ops/build_prd_coverage.py --output-dir artifacts/prd-coverage
```

The command writes:

- `prd_coverage_summary.json`
- `index.json`

CI runs the same script in the `staging-proof` job. The script fails when an
expected code, document, workflow, or proof artifact is missing.

## Coverage Matrix

| PRD capability | Current status | Repository evidence | Remaining boundary |
| --- | --- | --- | --- |
| Decision boundary | Implemented | `docs/product/fwa-risk-operations-prd.md`, `docs/project/ml-algorithm-strategy.md`, `apps/api-server/src/routes/agent.rs`, `apps/api-server/tests/knowledge_agent.rs` | Customer governance must still approve production adjudication policy and automatic denial or straight-through rule authority. |
| Inbound claim inbox and canonical trace | Implemented | `apps/api-server/src/routes/inbox.rs`, `apps/api-server/src/routes/claims.rs`, `scripts/demo/tpa_mock_client.py` | Customer-specific raw payload adapters and PII policy remain environment-specific. |
| Core scoring, rules, and review modes | Implemented with customer-validation boundary | `crates/fwa-scoring`, `crates/fwa-rules`, `apps/api-server/src/routes/ops_rules.rs`, demo seed/smoke, `docs/product/fwa-risk-operations-prd.md` | Customer-specific approved rule lists, production threshold refs, appeal/override routes, and live routing-impact evidence must be supplied by the customer environment before production adjudication. |
| Lead, case, QA, medical, and feedback loop | Implemented | `apps/api-server/src/routes/ops_cases.rs`, `apps/api-server/src/routes/ops_medical.rs`, `apps/api-server/src/routes/pilot_loop.rs` | Real reviewer workflow tools and customer operating procedure remain external. |
| Model operations and MLOps pipeline | Implemented with customer validation boundary | `apps/api-server/src/routes/ops_models.rs`, `crates/fwa-ml-runtime`, `apps/worker`, `docs/project/ml-pipeline-runbook.md` | Real labels, live shadow traffic, customer holdout, and production drift evidence. |
| Dataset, feature, and label governance | Implemented with customer validation boundary | `apps/api-server/src/routes/ops_datasets.rs`, `scripts/data/build_public_data_mvp.py`, `docs/project/public-data-mvp.md` | Real customer dataset intake, source data quality, and label provenance. |
| Knowledge, agent, and AI evidence foundation | Staging proof | `apps/api-server/src/routes/knowledge.rs`, `apps/api-server/src/routes/ops_evidence.rs`, `scripts/ops/build_ai_evidence_foundation.py` | Customer OCR, embedding/vector store, retrieval ranking, masking, and retention execution. |
| Analytics scale | Staging proof | `analytics/clickhouse/schema.sql`, `analytics/clickhouse/dashboard_queries.sql`, `scripts/ops/build_analytics_export.py` | Live scheduler credentials, ClickHouse retention/backup/access policy, dashboard hosting. |
| Pilot foundation and staging deployment | Staging proof | `infra/k8s/staging`, `.github/workflows/deploy-staging.yml`, `scripts/ops/build_staging_evidence.py`, `scripts/ops/validate_staging_deployment_package.py` | Customer cluster credentials, secrets, allowlists, observability receiver, restore drill. |
| Web console operations studio | Implemented | `apps/web-console/src/main.rs`, `apps/web-console/src/styles.css`, `scripts/demo/smoke_web_console.mjs` | Customer UAT and role-specific UX refinements. |

## Practical Reading

For progress accounting, count repository-owned PRD work as complete for demo
and pilot-contract purposes except for customer data and customer environment
execution. Do not claim production model effectiveness, ROI accuracy, or live
drift safety until customer labels, holdout validation, and shadow traffic exist.
