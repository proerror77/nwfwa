#!/usr/bin/env python3
"""Render supported production evidence reports from package source templates."""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.ops.build_customer_data_governance_report import (
    build_from_file as build_customer_data_governance_report,
)
from scripts.ops.build_model_serving_slo_report import (
    build_from_file as build_model_serving_slo_report,
)
from scripts.ops.build_ocr_vector_analytics_execution_report import (
    build_from_file as build_ocr_vector_analytics_execution_report,
)
from scripts.ops.build_retention_legal_hold_report import (
    build_from_file as build_retention_legal_hold_report,
)


DEFAULT_PACKAGE_DIR = Path("artifacts/production-evidence-package")
SUPPORTED_RENDERERS = (
    {
        "gate_id": "customer_data_governance",
        "source": "customer-data-governance-source.json",
        "artifact": "customer_data_governance_report.json",
        "builder": build_customer_data_governance_report,
    },
    {
        "gate_id": "retention_legal_hold",
        "source": "retention-legal-hold-source.json",
        "artifact": "retention_legal_hold_report.json",
        "builder": build_retention_legal_hold_report,
    },
    {
        "gate_id": "model_serving_slo",
        "source": "model-serving-slo-source.json",
        "artifact": "model_serving_slo_report.json",
        "builder": build_model_serving_slo_report,
    },
    {
        "gate_id": "ocr_vector_analytics_execution",
        "source": "ocr-vector-analytics-source.json",
        "artifact": "ocr_vector_analytics_execution_report.json",
        "builder": build_ocr_vector_analytics_execution_report,
    },
)


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def render_package(package_dir: Path) -> dict:
    source_dir = package_dir / "sources"
    evidence_dir = package_dir / "evidence"
    rendered = []
    missing_sources = []
    blocked_count = 0
    for renderer in SUPPORTED_RENDERERS:
        source_uri = source_dir / renderer["source"]
        if not source_uri.exists():
            missing_sources.append(str(source_uri))
            continue
        report = renderer["builder"](str(source_uri), evidence_dir)
        blockers = report.get("blockers") or []
        if blockers:
            blocked_count += 1
        rendered.append(
            {
                "gate_id": renderer["gate_id"],
                "source": f"sources/{renderer['source']}",
                "artifact": f"evidence/{renderer['artifact']}",
                "status": report.get("status", "unknown"),
                "blocker_count": len(blockers),
            }
        )
    summary = {
        "artifact_kind": "production_evidence_package_render_summary",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "package_dir": str(package_dir),
        "rendered_count": len(rendered),
        "missing_source_count": len(missing_sources),
        "blocked_report_count": blocked_count,
        "status": (
            "rendered_with_blockers"
            if blocked_count or missing_sources
            else "rendered_without_blockers"
        ),
        "readiness_claim": False,
        "rendered_reports": rendered,
        "missing_sources": missing_sources,
        "boundary": (
            "Rendering source templates only creates evidence report artifacts. "
            "Production readiness still requires validate_production_readiness_contract.py "
            "to pass against all required evidence."
        ),
    }
    write_json(package_dir / "render_summary.json", summary)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", default=str(DEFAULT_PACKAGE_DIR))
    args = parser.parse_args()
    print(json.dumps(render_package(Path(args.package_dir)), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
