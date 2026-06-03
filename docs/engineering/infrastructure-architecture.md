# Infrastructure Architecture

Date: 2026-05-28

Status: baseline architecture for staged implementation

## Purpose

This document defines the infrastructure baseline for the FWA Risk and
Operations Platform. It complements the PRD and implementation specs by
separating the core platform foundation from optional accelerators such as
ClickHouse, Redis, graph databases, vector databases, and external agent
frameworks.

The goal is not to add every modern database to the MVP. The goal is to make
the system ready for pilot data, evidence workflows, agent-assisted
investigation, analytics growth, and production governance without losing the
current modular-monolith discipline.

## Architecture Principles

- PostgreSQL remains the transactional source of truth for claims, providers,
  policies, rules, models, cases, labels, audit events, and job state.
- Object storage is the durable artifact layer for Parquet datasets, documents,
  OCR output, model artifacts, feature profiles, backtest outputs, and evidence
  packages.
- Async jobs are a first-class runtime surface, not an afterthought. The worker
  owns long-running jobs such as backtests, embeddings, document processing,
  graph projections, and export jobs.
- Agent workflows must be auditable, resumable, and approval-gated. Agents can
  prepare evidence and propose actions, but they do not autonomously adjudicate
  claims or promote production rules/models.
- Optional specialist stores are introduced only when the product has a clear
  workload that PostgreSQL and object storage cannot handle well enough.
- No raw PII should be placed in prompts, vector stores, logs, or third-party
  tools unless a customer-approved governance path explicitly allows it.

## Runtime Topology

The baseline runtime remains a small set of processes:

```text
web-console
  -> api-server
       -> PostgreSQL
       -> ml-service
       -> object storage
       -> worker job queue

worker
  -> PostgreSQL
  -> object storage
  -> ml-service or offline model tools
  -> optional search, vector, graph, or analytics stores

ml-service
  -> model artifacts
  -> explanation artifacts
```

Process responsibilities:

- `api-server`: synchronous TPA APIs, Operations Studio APIs, auth boundary,
  request validation, scoring orchestration, and audit writes.
- `worker`: async jobs, batch imports, rule backtests, model evaluations,
  dataset profiling, embedding creation, graph projections, exports, and
  agent-run continuation.
- `ml-service`: Python baseline scoring, experimentation support, and early
  explanation work. Rust remains the long-term production inference gateway.
- `web-console`: operator workflow, governance review, evidence inspection,
  and pilot operations.

## Data Plane

### PostgreSQL

PostgreSQL stores authoritative operational records:

- claim, member, policy, provider, claim item, and source-system records;
- scoring runs, rule hits, model scores, anomaly signals, and evidence refs;
- lead, case, investigation, QA, label, and outcome records;
- rule, feature-set, dataset, model, threshold, and promotion metadata;
- audit events and agent-run metadata;
- job queue, job attempts, retries, locks, and completion state for MVP/pilot.

PostgreSQL can also carry early graph and vector workloads:

- graph projection tables such as `relationship_edges` and
  `relationship_metrics`;
- `pgvector` for similar-case and document-chunk retrieval when the first
  retrieval workload is still modest;
- JSONB payloads for governed extension fields, with typed columns for fields
  used in filtering, joins, or audit queries.

### Object Storage

Use S3-compatible object storage in production and MinIO or a local equivalent
for development when document and artifact flows become active.

Stored artifacts include:

- immutable Parquet datasets and feature matrices;
- dataset profiles, split manifests, and feature reproducibility hashes;
- rule backtest outputs and model evaluation reports;
- model binaries, explanation artifacts, and threshold manifests;
- document originals, OCR output, extracted tables, and redacted text;
- agent-generated evidence packages and investigation workspaces.

The database stores artifact URIs, checksums, owners, retention class, and
evidence refs. It does not store large binary objects inline.

### Relationship Graph

Neo4j is not required for the baseline. The first graph layer should use
PostgreSQL projection tables and Rust graph jobs:

- `provider -> member`
- `provider -> provider`
- `provider -> facility`
- `provider -> ordering/referring provider`
- `member -> policy`
- `claim -> claim`
- `document -> claim/case`

The worker can compute connected components, centrality, community candidates,
temporal bursts, referral concentration, and suspicious dense subgraphs using
Rust libraries such as `petgraph` or equivalent graph tooling.

Introduce a dedicated graph database only when interactive graph traversal,
graph visualization, or temporal graph workloads become central enough to
justify another operational store.

### Vector And Search

Vector retrieval is useful for evidence and similar-case workflows, not for the
source of truth.

Recommended path:

1. Start with PostgreSQL full-text search and structured filters.
2. Add `pgvector` for similar cases, document chunks, and evidence snippets when
   retrieval becomes part of the pilot workflow.
3. Move to Qdrant, OpenSearch vector search, LanceDB, or another specialist
   store only when retrieval volume, latency, or isolation requirements exceed
   what the primary database should carry.

Every vector record must link back to authoritative records and artifact URIs.
Embeddings must have model version, chunking version, source checksum, redaction
status, and retention metadata.

### Analytics Store

ClickHouse is not required for MVP scoring or case workflow. It becomes useful
when the platform needs high-volume analytical queries over event streams:

- scoring events;
- rule runs;
- model scores;
- feature values;
- graph metrics;
- lead lifecycle events;
- case SLA events;
- audit and operations metrics.

Until volume proves the need, PostgreSQL materialized views and exports are
enough. If ClickHouse is introduced, PostgreSQL remains the operational source
of truth and ClickHouse is a derived analytical store.

The repository now includes a derived analytical event store contract under
`analytics/clickhouse`. It defines ClickHouse tables for scoring, rule, model,
case SLA, value, reviewer capacity, and provider graph snapshot events, plus
dashboard queries for rule/model drift, SLA, ROI, capacity, false-positive cost,
and graph risk reporting. The staging shape includes a ClickHouse service and a
worker CronJob that emits the scheduled analytics export plan. This is a
staging proof contract; customer production still needs live scheduler
credentials, retention settings, and data movement approvals.

## Job And Agent Plane

### Job Execution

MVP and pilot can use PostgreSQL-backed jobs with explicit states:

```text
queued -> running -> succeeded
       -> retry_wait -> failed
       -> canceled
```

Required job metadata:

- job type, input artifact refs, actor, tenant/customer scope, and priority;
- idempotency key, retry count, lock owner, heartbeat, and timeout;
- started/completed timestamps, output artifact refs, and failure reason;
- audit event ids for job start, completion, failure, and cancellation.

Redis, NATS, Kafka, or a managed queue can be introduced later for higher
throughput, delayed jobs, fanout, streaming, or stricter isolation. They should
not replace the audit trail or durable job outcome records.

### Agent-Native Operating Model

Agentic behavior should be implemented as an operating model over the existing
domain APIs, not as an opaque side service.

Required agent infrastructure:

- `agent_runs`: objective, actor, tenant/customer scope, status, model/provider
  metadata, started/completed timestamps, and final disposition.
- `agent_steps`: tool calls, inputs, outputs, evidence refs, error state,
  token/cost metadata where available, and audit event ids.
- `agent_context_snapshots`: bounded context sent to the agent, redaction
  status, source refs, and checksum.
- `agent_workspace_artifacts`: investigation drafts, evidence packages,
  summaries, and exported files stored in object storage.
- `agent_approvals`: proposed action, approver, decision, reason, and linked
  audit events.

Agent tools should be primitives over business capabilities:

- read claim context;
- read provider profile;
- search similar cases;
- create evidence package draft;
- propose case update;
- propose rule candidate;
- register dataset artifact;
- summarize document evidence;
- request missing evidence checklist.

Agents should not have direct tools for autonomous claim denial, payment hold,
rule publication, model promotion, deletion of audit events, or irreversible
data export. Those actions require human approval and customer-controlled
workflow.

## Control Plane

### Configuration And Secrets

Local development can use `.env` files and documented environment variables.
Pilot and production should use a secret manager or platform-native secret
store.

Required configuration classes:

- database and migration connection strings;
- object storage bucket names and credentials;
- API keys, webhook secrets, and SSO/RBAC configuration;
- model service URL, model artifact locations, and inference timeouts;
- customer data retention, masking, and export policies.

### Migrations And Release

Database migrations must be versioned and applied before dependent application
code is promoted.

Release gates:

- repository health check;
- Rust format, clippy, and tests;
- Python tests;
- frontend lint, tests, and build;
- migration dry run against a representative database;
- rollback notes for database, model, and rule promotion changes.

### Observability

Minimum pilot observability:

- structured logs with request id, run id, audit id, actor, source system,
  customer scope, endpoint, status, duration, and error code;
- health endpoints for API, worker, ML service, database, and object storage;
- counters for scoring requests, failures, rule hits, model failures, job state,
  case lifecycle changes, and audit append rate;
- traces for scoring and agent investigation flows.

Production observability should add OpenTelemetry collection, dashboards,
alerts, log retention policy, and incident runbooks.

## Security, Privacy, And Compliance

Required controls:

- tenant/customer scoping on every externally reachable API and async job;
- least-privilege service accounts;
- API keys for MVP, then SSO/RBAC for pilot or enterprise use;
- network allowlists or private connectivity for customer integrations;
- encryption in transit and at rest;
- PII classification and masking rules before logs, prompts, vectors, and
  exported artifacts;
- append-only audit semantics for scoring, case, rule, model, data, and agent
  events;
- backup and restore runbooks for PostgreSQL and object storage;
- retention and legal hold policies for evidence artifacts and audit events.

## Staged Implementation

### Phase 0: Local MVP Foundation

Current foundation:

- PostgreSQL;
- Rust `api-server`;
- Rust `worker`;
- Python `ml-service`;
- Yew `web-console`;
- migrations, seed scripts, and CI checks.

Keep this phase simple. Do not introduce Redis, ClickHouse, vector DB, graph DB,
Kubernetes, or production observability unless a specific MVP task requires it.

### Phase 1: Pilot Foundation

Add the infrastructure that makes customer pilot data safe and reproducible:

- object storage for datasets, documents, model artifacts, and evidence
  packages;
- durable PostgreSQL job table and worker heartbeats;
- backup and restore procedure;
- structured logging and minimum metrics;
- environment and secret management;
- customer-specific retention, masking, and allowlist configuration.

### Phase 2: AI Evidence Foundation

Add retrieval and agent infrastructure only after evidence workflows are active:

- document registry, chunk registry, OCR output, and redaction status;
- embedding jobs and retrieval audit;
- `pgvector` first, specialist vector store later if necessary;
- agent run, step, context snapshot, workspace artifact, and approval tables;
- human approval gates for case, rule, model, and export actions.

The repository now includes the PostgreSQL metadata contract for this phase:
`evidence_documents`, `evidence_document_chunks`, `evidence_ocr_outputs`,
`evidence_redaction_reviews`, `evidence_embedding_jobs`,
`evidence_retrieval_audit_events`, and `agent_workspace_artifacts`. The
contract is validated by `scripts/ops/validate_ai_evidence_foundation.py` and
documented in `docs/project/ai-evidence-foundation.md`. Production still needs
customer-approved OCR workers, embedding/vector storage, retrieval ranking, and
retention/access controls.

### Phase 3: Analytics Scale

Add analytical infrastructure when operational volume proves the need:

- derived analytical event store or ClickHouse: implemented as
  `analytics/clickhouse/schema.sql`;
- scheduled exports from PostgreSQL to object storage and analytics store:
  implemented as worker/export-plan contracts and
  `scripts/ops/build_analytics_export.py`;
- rule/model drift dashboards: implemented as ClickHouse dashboard queries;
- SLA, ROI, reviewer capacity, and false-positive cost reporting: implemented
  as ClickHouse dashboard queries;
- graph metrics snapshots for provider and relationship risk: implemented as
  `analytics_provider_graph_snapshots`.

### Phase 4: Production Hardening

Add production deployment and governance controls:

- infrastructure as code;
- managed secrets;
- SSO/RBAC;
- network isolation;
- OpenTelemetry-based monitoring and alerting;
- disaster recovery drills;
- release promotion, rollback, and incident runbooks;
- formal data retention and evidence destruction workflows.

The repository now includes the staging contract for part of this phase:
`build-governance-ops-plan` emits the backup manifest, restore-drill,
retention-scan, legal-hold, and destruction-review job graph. Customer
production still needs the chosen environment to execute those jobs with
approved retention windows, legal holds, and human approval before destruction.
The staging evidence pack also emits `operational_drill_proof.json` so restore,
rollback, alert-route, pilot readiness, and incident tabletop drills have a
machine-checkable evidence contract before live customer execution.

## Non-Goals

- No Kubernetes requirement for MVP.
- No ClickHouse requirement for MVP.
- No Redis requirement for MVP.
- No Neo4j requirement for MVP graph discovery.
- No standalone vector database requirement before evidence retrieval volume is
  proven.
- No raw PII in LLM prompts, vector stores, or logs without customer-approved
  governance.
- No autonomous agent action that changes adjudication, payment, rule
  publication, or model promotion state.

## Acceptance Criteria

- The platform has one authoritative transactional store and every derived store
  links back to it.
- Large artifacts are stored outside PostgreSQL with checksums, retention class,
  and evidence refs.
- Async jobs have durable state, retries, idempotency, audit events, and output
  artifact refs.
- Agent workflows have run records, step records, context snapshots, tool-call
  audit, workspace artifacts, and approval gates.
- Optional stores such as Redis, ClickHouse, graph DB, vector DB, and
  OpenSearch have explicit workload triggers before adoption.
- Pilot environments can explain where data lives, how it is masked, how it is
  backed up, and how it is restored.
