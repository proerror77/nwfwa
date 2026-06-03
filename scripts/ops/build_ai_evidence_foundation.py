#!/usr/bin/env python3
"""Build AI evidence foundation proof artifacts without customer data."""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT_DIR = Path("artifacts/ai-evidence-foundation")
MIGRATION_PATH = ROOT / "migrations" / "0001_initial.sql"

FOUNDATION_TABLES = [
    {
        "table": "evidence_documents",
        "purpose": "document registry with source refs, storage URI, checksum, redaction and retention status",
    },
    {
        "table": "evidence_document_chunks",
        "purpose": "chunk registry with chunking version, offsets, checksum, and redaction status",
    },
    {
        "table": "evidence_ocr_outputs",
        "purpose": "OCR output registry with engine version, output URI, checksum, and quality status",
    },
    {
        "table": "evidence_redaction_reviews",
        "purpose": "document or chunk redaction review audit with policy and checksums",
    },
    {
        "table": "evidence_embedding_jobs",
        "purpose": "embedding job registry for document, chunk, and knowledge-case vectors",
    },
    {
        "table": "evidence_retrieval_audit_events",
        "purpose": "retrieval audit trail with masked query checksum, source refs, result refs, and redaction status",
    },
    {
        "table": "agent_workspace_artifacts",
        "purpose": "object-storage-backed agent workspace artifact registry",
    },
]


def sha256_file(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_manifest(output_dir: Path, object_storage_uri: str, customer_scope_id: str) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    artifact_root = object_storage_uri.rstrip("/") + f"/ai-evidence/{customer_scope_id}"
    manifest = {
        "artifact_kind": "ai_evidence_foundation_manifest",
        "generated_at": generated_at,
        "customer_scope_id": customer_scope_id,
        "migration_path": "migrations/0001_initial.sql",
        "migration_checksum": sha256_file(MIGRATION_PATH),
        "source_of_truth": "PostgreSQL operational evidence tables",
        "object_storage_root": artifact_root,
        "foundation_tables": FOUNDATION_TABLES,
        "required_workflows": [
            "document_registry",
            "chunk_registry",
            "ocr_output_registry",
            "redaction_review",
            "embedding_job_registry",
            "retrieval_audit",
            "agent_workspace_artifacts",
            "human_approval_gate",
        ],
        "pii_boundary": "raw document text and raw payloads stay in customer-approved storage; database rows carry checksums, masked refs, redaction status, and evidence refs",
        "boundary": "staging proof contract only; customer production still needs OCR/vector workers, pgvector or managed vector storage, retention policy, and access controls",
    }
    write_json(output_dir / "ai_evidence_foundation_manifest.json", manifest)
    write_json(
        output_dir / "index.json",
        {
            "artifact_kind": "ai_evidence_foundation_index",
            "generated_at": generated_at,
            "artifacts": ["ai_evidence_foundation_manifest.json"],
            "customer_data_required": False,
        },
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    parser.add_argument("--object-storage-uri", default="s3://nwfwa-staging-artifacts")
    parser.add_argument("--customer-scope-id", default="staging-customer")
    args = parser.parse_args()

    manifest = build_manifest(Path(args.output_dir), args.object_storage_uri, args.customer_scope_id)
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
