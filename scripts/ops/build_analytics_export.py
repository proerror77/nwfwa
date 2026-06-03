#!/usr/bin/env python3
"""Build analytics-scale export proof artifacts without customer data."""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT_DIR = Path("artifacts/analytics-export")
SCHEMA_PATH = ROOT / "analytics" / "clickhouse" / "schema.sql"
QUERIES_PATH = ROOT / "analytics" / "clickhouse" / "dashboard_queries.sql"


EXPORT_JOBS = [
    {
        "job_kind": "scoring_events_export",
        "source_tables": ["scoring_runs", "claims"],
        "sink_table": "fwa_analytics.analytics_scoring_events",
        "dashboard_coverage": ["risk_volume", "rag_band_mix"],
    },
    {
        "job_kind": "rule_events_export",
        "source_tables": ["rule_runs", "scoring_runs", "qa_reviews"],
        "sink_table": "fwa_analytics.analytics_rule_events",
        "dashboard_coverage": ["rule_drift", "false_positive_rate"],
    },
    {
        "job_kind": "model_events_export",
        "source_tables": ["model_scores", "model_evaluation_runs", "scoring_runs"],
        "sink_table": "fwa_analytics.analytics_model_events",
        "dashboard_coverage": ["model_drift", "shadow_delta", "calibration_signal"],
    },
    {
        "job_kind": "case_sla_events_export",
        "source_tables": ["investigation_cases", "audit_events"],
        "sink_table": "fwa_analytics.analytics_case_sla_events",
        "dashboard_coverage": ["sla_reporting"],
    },
    {
        "job_kind": "value_events_export",
        "source_tables": ["saving_attributions", "investigation_cases", "qa_reviews"],
        "sink_table": "fwa_analytics.analytics_value_events",
        "dashboard_coverage": ["roi_reporting", "false_positive_cost"],
    },
    {
        "job_kind": "reviewer_capacity_events_export",
        "source_tables": ["investigation_cases", "qa_reviews"],
        "sink_table": "fwa_analytics.analytics_reviewer_capacity_events",
        "dashboard_coverage": ["reviewer_capacity", "precision_at_capacity"],
    },
    {
        "job_kind": "provider_graph_snapshots_export",
        "source_tables": ["providers", "claims", "rule_runs"],
        "sink_table": "fwa_analytics.analytics_provider_graph_snapshots",
        "dashboard_coverage": ["provider_graph_snapshots", "risk_concentration"],
    },
]


def sha256_file(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_export_manifest(
    output_dir: Path,
    object_storage_uri: str,
    clickhouse_url: str,
    customer_scope_id: str,
    cron: str,
) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    export_root = object_storage_uri.rstrip("/") + f"/analytics-exports/{customer_scope_id}"
    jobs = []
    for job in EXPORT_JOBS:
        job_dir = job["job_kind"].removesuffix("_export")
        jobs.append(
            {
                **job,
                "schedule": cron,
                "incremental_watermark": "event_time or snapshot_time",
                "idempotency_key": "customer_scope_id + export_window_start + sink_table",
                "staging_object_uri": f"{export_root}/{job_dir}/{{window_start}}.ndjson",
                "load_mode": "append-only window load into ClickHouse MergeTree",
                "pii_boundary": "masked identifiers only; raw names, addresses, member numbers, and payloads stay out of analytics store",
            }
        )

    manifest = {
        "artifact_kind": "analytics_scale_export_manifest",
        "generated_at": generated_at,
        "customer_scope_id": customer_scope_id,
        "source_of_truth": "PostgreSQL operational tables",
        "derived_analytics_store": "ClickHouse",
        "clickhouse_url": clickhouse_url,
        "object_storage_uri": object_storage_uri,
        "schema_path": "analytics/clickhouse/schema.sql",
        "schema_checksum": sha256_file(SCHEMA_PATH),
        "dashboard_queries_path": "analytics/clickhouse/dashboard_queries.sql",
        "dashboard_queries_checksum": sha256_file(QUERIES_PATH),
        "scheduled_exports": jobs,
        "required_dashboards": [
            "rule_drift",
            "model_drift",
            "sla_reporting",
            "roi_reporting",
            "reviewer_capacity",
            "false_positive_cost",
            "provider_graph_snapshots",
        ],
        "boundary": "proof contract only; customer production requires live scheduler credentials, object-storage policy, and ClickHouse retention settings",
    }

    write_json(output_dir / "analytics_export_manifest.json", manifest)
    write_json(output_dir / "scheduled_exports.json", {"scheduled_exports": jobs})
    (output_dir / "schema.sql").write_text(SCHEMA_PATH.read_text(encoding="utf-8"), encoding="utf-8")
    (output_dir / "dashboard_queries.sql").write_text(
        QUERIES_PATH.read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    write_json(
        output_dir / "index.json",
        {
            "artifact_kind": "analytics_scale_evidence_index",
            "generated_at": generated_at,
            "artifacts": [
                "analytics_export_manifest.json",
                "scheduled_exports.json",
                "schema.sql",
                "dashboard_queries.sql",
            ],
            "customer_data_required": False,
        },
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    parser.add_argument("--object-storage-uri", default="s3://nwfwa-staging-artifacts")
    parser.add_argument("--clickhouse-url", default="http://clickhouse:8123")
    parser.add_argument("--customer-scope-id", default="staging-customer")
    parser.add_argument("--cron", default="15 * * * *")
    args = parser.parse_args()

    manifest = build_export_manifest(
        Path(args.output_dir),
        args.object_storage_uri,
        args.clickhouse_url,
        args.customer_scope_id,
        args.cron,
    )
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
