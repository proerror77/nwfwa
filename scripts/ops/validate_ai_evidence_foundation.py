#!/usr/bin/env python3
"""Static checks for the AI evidence foundation schema and proof contract."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MIGRATION_PATH = ROOT / "migrations" / "0001_initial.sql"
BUILD_SCRIPT = ROOT / "scripts" / "ops" / "build_ai_evidence_foundation.py"

REQUIRED_TABLES = [
    "evidence_documents",
    "evidence_document_chunks",
    "evidence_ocr_outputs",
    "evidence_redaction_reviews",
    "evidence_embedding_jobs",
    "evidence_retrieval_audit_events",
    "agent_steps",
    "agent_context_snapshots",
    "agent_approvals",
    "agent_workspace_artifacts",
]

REQUIRED_SNIPPETS = [
    "customer_scope_id TEXT NOT NULL",
    "redaction_status TEXT NOT NULL",
    "retention_policy_id TEXT NOT NULL",
    "content_checksum TEXT NOT NULL",
    "chunking_version TEXT NOT NULL",
    "ocr_engine_version TEXT NOT NULL",
    "embedding_model_version TEXT NOT NULL",
    "vector_store_kind TEXT NOT NULL",
    "query_checksum TEXT NOT NULL",
    "result_refs JSONB NOT NULL DEFAULT '[]'::jsonb",
    "artifact_checksum TEXT NOT NULL",
]

REQUIRED_INDEXES = [
    "idx_evidence_documents_customer_scope",
    "idx_evidence_documents_claim_id",
    "idx_evidence_document_chunks_document_id",
    "idx_evidence_ocr_outputs_document_id",
    "idx_evidence_embedding_jobs_target",
    "idx_evidence_retrieval_audit_customer_scope",
    "idx_agent_workspace_artifacts_run",
]

FORBIDDEN_TERMS = [
    "member_name",
    "patient_name",
    "street_address",
    "email_address",
    "phone_number",
    "raw_document_text",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise AssertionError(f"missing required file: {path}") from exc


def validate_migration(sql: str) -> None:
    for table in REQUIRED_TABLES:
        require(f"CREATE TABLE IF NOT EXISTS {table}" in sql, f"migration missing table {table}")
    for snippet in REQUIRED_SNIPPETS:
        require(snippet in sql, f"migration missing snippet: {snippet}")
    for index_name in REQUIRED_INDEXES:
        require(index_name in sql, f"migration missing index {index_name}")
    lower_sql = sql.lower()
    for forbidden in FORBIDDEN_TERMS:
        require(forbidden not in lower_sql, f"migration contains raw PII field {forbidden}")


def validate_build_script() -> None:
    require(BUILD_SCRIPT.is_file(), "missing build_ai_evidence_foundation.py")
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run(
            [
                sys.executable,
                str(BUILD_SCRIPT),
                "--output-dir",
                tmp,
                "--object-storage-uri",
                "s3://nwfwa-staging-artifacts",
                "--customer-scope-id",
                "staging-customer",
            ],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        output_dir = Path(tmp)
        manifest_path = output_dir / "ai_evidence_foundation_manifest.json"
        require(manifest_path.is_file(), "proof script did not write manifest")
        require((output_dir / "index.json").is_file(), "proof script did not write index")
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        tables = {entry["table"] for entry in manifest["foundation_tables"]}
        for table in REQUIRED_TABLES:
            if table.startswith("agent_") and table != "agent_workspace_artifacts":
                continue
            require(table in tables, f"manifest missing foundation table {table}")
        for workflow in [
            "document_registry",
            "chunk_registry",
            "ocr_output_registry",
            "redaction_review",
            "embedding_job_registry",
            "retrieval_audit",
            "agent_workspace_artifacts",
            "human_approval_gate",
        ]:
            require(workflow in manifest["required_workflows"], f"manifest missing workflow {workflow}")
        require("raw document text" in manifest["pii_boundary"], "manifest must state PII boundary")


def main() -> int:
    validate_migration(read(MIGRATION_PATH))
    validate_build_script()
    print("ai evidence foundation validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"ai evidence foundation validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
