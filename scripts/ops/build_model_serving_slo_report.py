#!/usr/bin/env python3
"""Build model serving SLO evidence from customer production measurements."""

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
REQUIRED_EVIDENCE_PREFIXES = (
    "model_serving:",
    "model_artifact:",
    "probability_calibration_reports:",
    "probability_calibration_input:",
    "calibration_labels:",
)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_model_serving_slo_report(source: dict) -> dict:
    evidence_refs = source.get("evidence_refs") or []
    blockers = []
    for field_name in ("model_key", "model_version"):
        if not isinstance(source.get(field_name), str) or not source[field_name].strip():
            blockers.append(f"{field_name}_missing")
    latency_slo_ms = source.get("latency_slo_ms")
    p95_latency_ms = source.get("p95_latency_ms")
    if not isinstance(latency_slo_ms, (int, float)) or latency_slo_ms <= 0:
        blockers.append("latency_slo_ms_missing_or_invalid")
    if not isinstance(p95_latency_ms, (int, float)) or (
        isinstance(latency_slo_ms, (int, float)) and p95_latency_ms > latency_slo_ms
    ):
        blockers.append("p95_latency_ms_outside_slo")
    error_rate_slo = source.get("error_rate_slo")
    error_rate = source.get("error_rate")
    if not isinstance(error_rate_slo, (int, float)) or not 0 <= error_rate_slo <= 1:
        blockers.append("error_rate_slo_missing_or_invalid")
    if not isinstance(error_rate, (int, float)) or (
        isinstance(error_rate_slo, (int, float)) and error_rate > error_rate_slo
    ):
        blockers.append("error_rate_outside_slo")
    for field_name in (
        "checksum_verified",
        "signature_verified",
        "rollback_ready",
        "calibrated_probability_serving_active",
    ):
        if source.get(field_name) is not True:
            blockers.append(f"{field_name}_not_true")
    if source.get("fallback_status") != "healthy":
        blockers.append("fallback_status_not_healthy")
    if source.get("probability_calibration_status") != "calibrated":
        blockers.append("probability_calibration_status_not_calibrated")
    for prefix in REQUIRED_EVIDENCE_PREFIXES:
        if not any(isinstance(ref, str) and ref.startswith(prefix) for ref in evidence_refs):
            blockers.append(f"missing_{prefix.rstrip(':')}_evidence_ref")
    if any(isinstance(ref, str) and "local://template" in ref for ref in evidence_refs):
        blockers.append("template_evidence_refs_not_replaced")

    return {
        "artifact_kind": "model_serving_slo_report",
        "report_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "status": "passed" if not blockers else "blocked",
        "model_key": source.get("model_key", ""),
        "model_version": source.get("model_version", ""),
        "latency_slo_ms": source.get("latency_slo_ms"),
        "p95_latency_ms": source.get("p95_latency_ms"),
        "error_rate_slo": source.get("error_rate_slo"),
        "error_rate": source.get("error_rate"),
        "checksum_verified": source.get("checksum_verified", False),
        "signature_verified": source.get("signature_verified", False),
        "fallback_status": source.get("fallback_status", "missing"),
        "rollback_ready": source.get("rollback_ready", False),
        "probability_calibration_status": source.get("probability_calibration_status", "missing"),
        "calibrated_probability_serving_active": source.get(
            "calibrated_probability_serving_active", False
        ),
        "evidence_refs": evidence_refs,
        "blockers": blockers,
        "governance_boundary": (
            "model serving SLO evidence only; must not activate models, change thresholds, "
            "score claims, assign labels, deny claims, or change routing policy"
        ),
    }


def build_from_file(source_uri: str, output_dir: Path) -> dict:
    report = build_model_serving_slo_report(load_json(Path(source_uri)))
    write_json(output_dir / "model_serving_slo_report.json", report)
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
