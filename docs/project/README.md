# Project Documentation

This folder is the detailed project handbook for `nwfwa`.

Use it when you need to understand what the system does, which APIs exist, how
the technical stack fits together, and what is ready for demo or pilot use.

## Documents

- [Architecture](architecture.md): product boundary, runtime topology, modules,
  workflow map, and deployment shape.
- [Technology Stack](technology-stack.md): Rust, Python, Yew, PostgreSQL,
  Docker, CI, and development tooling.
- [API Reference](api-reference.md): every API route, method, purpose, auth
  requirement, main inputs, outputs, and side effects.
- [Data Model](data-model.md): PostgreSQL schema groups, relationships, and
  table responsibilities.
- [Operations Guide](operations-guide.md): local demo, verification gates, CI,
  pilot readiness, and known production boundaries.

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
