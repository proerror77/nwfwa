#!/usr/bin/env python3
"""Build a customer data governance evidence report from customer approvals."""

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
    "dataset_provenance_status",
    "label_provenance_status",
    "holdout_split_status",
    "shadow_traffic_plan_status",
)
REQUIRED_EVIDENCE_PREFIXES = (
    "dataset_provenance:",
    "label_provenance:",
    "holdout_split:",
    "shadow_traffic_plan:",
)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_customer_data_governance_report(source: dict) -> dict:
    evidence_refs = source.get("evidence_refs") or []
    blockers = []
    for field_name in REQUIRED_STATUS_FIELDS:
        if source.get(field_name) != "approved":
            blockers.append(f"{field_name}_not_approved")
    for field_name in ("approved_label_count", "holdout_claim_count"):
        if not isinstance(source.get(field_name), int) or source[field_name] <= 0:
            blockers.append(f"{field_name}_missing_or_zero")
    for prefix in REQUIRED_EVIDENCE_PREFIXES:
        if not any(isinstance(ref, str) and ref.startswith(prefix) for ref in evidence_refs):
            blockers.append(f"missing_{prefix.rstrip(':')}_evidence_ref")
    if any(isinstance(ref, str) and "local://template" in ref for ref in evidence_refs):
        blockers.append("template_evidence_refs_not_replaced")

    return {
        "artifact_kind": "customer_data_governance_report",
        "report_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "customer_scope_id": source.get("customer_scope_id", ""),
        "as_of_date": source.get("as_of_date", ""),
        "status": "approved" if not blockers else "blocked",
        "dataset_provenance_status": source.get("dataset_provenance_status", "missing"),
        "label_provenance_status": source.get("label_provenance_status", "missing"),
        "holdout_split_status": source.get("holdout_split_status", "missing"),
        "shadow_traffic_plan_status": source.get("shadow_traffic_plan_status", "missing"),
        "approved_label_count": source.get("approved_label_count", 0),
        "holdout_claim_count": source.get("holdout_claim_count", 0),
        "evidence_refs": evidence_refs,
        "blockers": blockers,
        "governance_boundary": (
            "customer data governance evidence only; must not score claims, assign labels, "
            "deny claims, activate models, or change routing policy"
        ),
    }


def build_from_file(source_uri: str, output_dir: Path) -> dict:
    report = build_customer_data_governance_report(load_json(Path(source_uri)))
    write_json(output_dir / "customer_data_governance_report.json", report)
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
