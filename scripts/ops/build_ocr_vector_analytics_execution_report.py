#!/usr/bin/env python3
"""Build OCR/vector/analytics execution evidence from customer run results."""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


DEFAULT_OUTPUT_DIR = Path("artifacts/production-evidence-package/evidence")
REQUIRED_STATUS_FIELDS = (
    "ocr_execution_status",
    "embedding_vector_status",
    "retrieval_ranking_status",
    "clickhouse_export_status",
    "dashboard_access_status",
    "analytics_retention_backup_status",
)
REQUIRED_COUNT_FIELDS = (
    "document_count",
    "embedding_job_count",
    "retrieval_audit_count",
    "analytics_export_job_count",
)
REQUIRED_EVIDENCE_PREFIXES = (
    "ai_evidence_execution:",
    "ocr_outputs:",
    "embedding_jobs:",
    "retrieval_audits:",
    "analytics_exports:",
    "clickhouse_dashboard:",
)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def evidence_ref_is_non_production(value: object) -> bool:
    return isinstance(value, str) and (
        "local://" in value or "file://" in value or "{" in value or "}" in value
    )


def build_ocr_vector_analytics_execution_report(source: dict) -> dict:
    evidence_refs = source.get("evidence_refs") or []
    blockers = []
    for field_name in REQUIRED_STATUS_FIELDS:
        if source.get(field_name) != "completed":
            blockers.append(f"{field_name}_not_completed")
    for field_name in REQUIRED_COUNT_FIELDS:
        if not isinstance(source.get(field_name), int) or source[field_name] <= 0:
            blockers.append(f"{field_name}_missing_or_zero")
    if source.get("raw_phi_exported") is not False:
        blockers.append("raw_phi_exported_not_false")
    for prefix in REQUIRED_EVIDENCE_PREFIXES:
        if not any(isinstance(ref, str) and ref.startswith(prefix) for ref in evidence_refs):
            blockers.append(f"missing_{prefix.rstrip(':')}_evidence_ref")
    if any(isinstance(ref, str) and "local://template" in ref for ref in evidence_refs):
        blockers.append("template_evidence_refs_not_replaced")
    if any(evidence_ref_is_non_production(ref) for ref in evidence_refs):
        blockers.append("non_production_evidence_refs")

    return {
        "artifact_kind": "ocr_vector_analytics_execution_report",
        "report_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "status": "completed" if not blockers else "blocked",
        "ocr_execution_status": source.get("ocr_execution_status", "missing"),
        "embedding_vector_status": source.get("embedding_vector_status", "missing"),
        "retrieval_ranking_status": source.get("retrieval_ranking_status", "missing"),
        "clickhouse_export_status": source.get("clickhouse_export_status", "missing"),
        "dashboard_access_status": source.get("dashboard_access_status", "missing"),
        "analytics_retention_backup_status": source.get(
            "analytics_retention_backup_status", "missing"
        ),
        "document_count": source.get("document_count", 0),
        "embedding_job_count": source.get("embedding_job_count", 0),
        "retrieval_audit_count": source.get("retrieval_audit_count", 0),
        "analytics_export_job_count": source.get("analytics_export_job_count", 0),
        "raw_phi_exported": source.get("raw_phi_exported"),
        "evidence_refs": evidence_refs,
        "blockers": blockers,
        "governance_boundary": (
            "OCR/vector/analytics execution evidence only; must not export raw PHI "
            "to vectors or analytics tables"
        ),
    }


def build_from_file(source_uri: str, output_dir: Path) -> dict:
    report = build_ocr_vector_analytics_execution_report(load_json(Path(source_uri)))
    write_json(output_dir / "ocr_vector_analytics_execution_report.json", report)
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-uri", required=True)
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    args = parser.parse_args()
    print(json.dumps(build_from_file(args.source_uri, Path(args.output_dir)), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
