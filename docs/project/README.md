# Project Documentation

This folder is the detailed project handbook for `nwfwa`.

Use it when you need to understand what the system does, which APIs exist, how
the technical stack fits together, and what is ready for demo or pilot use.

## Documents

- [Architecture](architecture.md): product boundary, runtime topology, modules,
  workflow map, and deployment shape.
- [Technology Stack](technology-stack.md): Rust, Python, Yew, PostgreSQL,
  Docker, Kubernetes staging, CI, and development tooling.
- [API Reference](api-reference.md): every API route, method, purpose, auth
  requirement, main inputs, outputs, and side effects.
- [PRD Coverage](prd-coverage.md): PRD capability matrix, repository evidence,
  and customer-data/customer-environment boundaries.
- [Data Model](data-model.md): PostgreSQL schema groups, relationships, and
  table responsibilities.
- [ML Algorithm Strategy](ml-algorithm-strategy.md): researched model plan,
  current baseline boundaries, validation gates, and production ML roadmap.
- [Public Data MVP Pack](public-data-mvp.md): CMS/OIG public-data boundary,
  manifest generation commands, and production interpretation.
- [AI Evidence Foundation](ai-evidence-foundation.md): document registry,
  chunks, OCR/redaction metadata, embedding jobs, retrieval audit, and agent
  workspace artifacts.
- [ML Pipeline Runbook](ml-pipeline-runbook.md): operating workflow for dataset
  intake, training, registration, review, promotion, serving, monitoring, and
  rollback.
- [Analytics Scale](analytics-scale.md): ClickHouse derived event store,
  scheduled export contract, dashboard queries, and production boundary.
- [Operations Guide](operations-guide.md): local demo, Kubernetes staging,
  verification gates, CI, pilot readiness, and known production boundaries.

## Source Of Truth

These documents summarize the current repository implementation and should be
read together with:

- [Product PRD](../product/fwa-risk-operations-prd.md)
- [Infrastructure Architecture](../engineering/infrastructure-architecture.md)
- [TPA Integration Contract](../engineering/tpa-integration-contract.md)
- [Pilot Demo Runbook](../engineering/demo-runbook.md)
- [Pilot Readiness](../engineering/pilot-readiness.md)
- [CI/CD](../engineering/ci-cd.md)

When there is a conflict, use code and OpenAPI tests as the contract truth, then
update these documents.
