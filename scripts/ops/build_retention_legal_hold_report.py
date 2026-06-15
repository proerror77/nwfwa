#!/usr/bin/env python3
"""Build retention/legal-hold production evidence from customer controls."""

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
REQUIRED_EVIDENCE_PREFIXES = ("retention_policy:", "legal_hold_policy:")


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_retention_legal_hold_report(source: dict) -> dict:
    evidence_refs = source.get("evidence_refs") or []
    blockers = []
    if not isinstance(source.get("retention_years"), int) or source["retention_years"] < 6:
        blockers.append("retention_years_below_six")
    for field_name in ("retention_policy_id", "legal_hold_policy_id", "archive_storage_uri"):
        if not isinstance(source.get(field_name), str) or not source[field_name].strip():
            blockers.append(f"{field_name}_missing")
    if source.get("legal_hold_reconciliation_status") != "completed":
        blockers.append("legal_hold_reconciliation_not_completed")
    if source.get("destruction_workflow") != "human_approval_required_before_destroy":
        blockers.append("destruction_workflow_not_human_approved")
    if source.get("automated_destruction_enabled") is not False:
        blockers.append("automated_destruction_not_disabled")
    for prefix in REQUIRED_EVIDENCE_PREFIXES:
        if not any(isinstance(ref, str) and ref.startswith(prefix) for ref in evidence_refs):
            blockers.append(f"missing_{prefix.rstrip(':')}_evidence_ref")
    if any(isinstance(ref, str) and "local://template" in ref for ref in evidence_refs):
        blockers.append("template_evidence_refs_not_replaced")

    return {
        "artifact_kind": "retention_legal_hold_report",
        "report_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "status": "configured" if not blockers else "blocked",
        "retention_years": source.get("retention_years", 0),
        "retention_policy_id": source.get("retention_policy_id", ""),
        "legal_hold_policy_id": source.get("legal_hold_policy_id", ""),
        "archive_storage_uri": source.get("archive_storage_uri", ""),
        "legal_hold_reconciliation_status": source.get(
            "legal_hold_reconciliation_status", "missing"
        ),
        "destruction_workflow": source.get("destruction_workflow", "missing"),
        "automated_destruction_enabled": source.get("automated_destruction_enabled", True),
        "evidence_refs": evidence_refs,
        "blockers": blockers,
        "governance_boundary": (
            "retention/legal-hold evidence only; must not archive, destroy, or mutate "
            "customer records without a separate customer-approved execution workflow"
        ),
    }


def build_from_file(source_uri: str, output_dir: Path) -> dict:
    report = build_retention_legal_hold_report(load_json(Path(source_uri)))
    write_json(output_dir / "retention_legal_hold_report.json", report)
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
