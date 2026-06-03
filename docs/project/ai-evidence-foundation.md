# AI Evidence Foundation

This document describes the evidence and retrieval infrastructure contract now
present in the repository. It is a staging-ready schema and proof contract, not
a customer production OCR or vector-search deployment.

## Purpose

The AI evidence foundation keeps agent and retrieval workflows auditable without
putting raw PII or raw document text into prompts, logs, vectors, or free text.
PostgreSQL stores metadata, checksums, redaction status, retention policy,
source refs, and evidence refs. Raw files and extracted text stay in
customer-approved object storage or data-lake locations.

Implemented coverage:

- document registry;
- document chunk registry;
- OCR output metadata;
- redaction review metadata;
- embedding job registry;
- retrieval audit trail;
- agent run steps, context snapshots, tool calls, policy checks, approvals;
- agent workspace artifact registry.

## Schema

The schema lives in `migrations/0001_initial.sql`.

| Table | Purpose |
| --- | --- |
| `evidence_documents` | Source document registry with storage URI, checksum, redaction status, and retention policy |
| `evidence_document_chunks` | Chunk registry with chunking version, offsets, checksum, and redaction status |
| `evidence_ocr_outputs` | OCR engine version, output URI, checksum, confidence, and quality status |
| `evidence_redaction_reviews` | Redaction policy review for document or chunk evidence |
| `evidence_embedding_jobs` | Embedding model/version, target refs, vector store refs, checksum, and status |
| `evidence_retrieval_audit_events` | Query checksum, retrieval method, source refs, result refs, and redaction status |
| `agent_workspace_artifacts` | Agent-produced object-storage artifact refs, checksum, retention, and evidence refs |

Existing agent tables complete the approval boundary:

- `agent_runs`
- `agent_steps`
- `agent_context_snapshots`
- `tool_calls`
- `tool_results`
- `agent_policy_checks`
- `agent_approvals`

## Runtime API

The governed metadata API is implemented under `/api/v1/ops/evidence/*`:

| Method | Path | Purpose |
| --- | --- | --- |
| GET/POST | `/api/v1/ops/evidence/documents` | List or register document metadata, storage URI, checksum, redaction status, and retention policy |
| GET | `/api/v1/ops/evidence/documents/{document_id}` | Read one document metadata record within the authenticated customer scope |
| GET/POST | `/api/v1/ops/evidence/documents/{document_id}/chunks` | List or register chunk metadata, offsets, checksum, token count, and redaction status |
| GET/POST | `/api/v1/ops/evidence/documents/{document_id}/ocr-outputs` | List or register OCR output metadata, engine version, output URI, checksum, confidence, and quality status |
| GET/POST | `/api/v1/ops/evidence/embedding-jobs` | List or register embedding job metadata, target refs, vector store refs, checksum, and status |
| GET/POST | `/api/v1/ops/evidence/retrieval-audit-events` | List or record retrieval audit metadata with query checksum, source refs, result refs, and redaction status |

The API derives `customer_scope_id`, `actor_id`, `actor_role`, and
`source_system` from the authenticated API key. Callers do not supply tenant
scope. Payloads intentionally carry URIs, checksums, redaction status, and
evidence refs instead of raw document text, raw OCR text, embedding vectors, or
raw query text.

## Proof

Run:

```bash
python3 scripts/ops/validate_ai_evidence_foundation.py
python3 scripts/ops/build_ai_evidence_foundation.py \
  --output-dir artifacts/ai-evidence-foundation \
  --object-storage-uri s3://nwfwa-staging-artifacts \
  --customer-scope-id staging-customer
```

The output includes:

- `ai_evidence_foundation_manifest.json`
- `index.json`

The manifest records the schema checksum, table responsibilities, required
workflows, and PII boundary. It does not require customer data.

## Production Boundary

Still required for customer production:

- customer-approved OCR worker and output storage;
- embedding worker and `pgvector` or managed vector store deployment;
- retrieval ranking evaluation and access policy;
- document/object retention, legal hold, and destruction automation;
- live audit dashboards for retrieval and agent workspace artifacts;
- customer-approved prompt/log/vector masking policy.
