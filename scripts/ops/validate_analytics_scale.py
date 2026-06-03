#!/usr/bin/env python3
"""Static checks for analytics-scale ClickHouse and export contracts."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = ROOT / "analytics" / "clickhouse" / "schema.sql"
QUERIES_PATH = ROOT / "analytics" / "clickhouse" / "dashboard_queries.sql"
EXPORT_SCRIPT = ROOT / "scripts" / "ops" / "build_analytics_export.py"

REQUIRED_TABLES = [
    "analytics_scoring_events",
    "analytics_rule_events",
    "analytics_model_events",
    "analytics_case_sla_events",
    "analytics_value_events",
    "analytics_reviewer_capacity_events",
    "analytics_provider_graph_snapshots",
]

REQUIRED_QUERY_BLOCKS = [
    "scoring_volume_daily",
    "rule_drift_daily",
    "model_drift_daily",
    "sla_reporting_daily",
    "roi_reporting_daily",
    "reviewer_capacity_daily",
    "false_positive_cost_daily",
    "provider_graph_snapshots",
]

REQUIRED_EXPORT_JOBS = [
    "scoring_events_export",
    "rule_events_export",
    "model_events_export",
    "case_sla_events_export",
    "value_events_export",
    "reviewer_capacity_events_export",
    "provider_graph_snapshots_export",
]

FORBIDDEN_ANALYTICS_TERMS = [
    "member_name",
    "patient_name",
    "provider_name",
    "street_address",
    "phone_number",
    "email_address",
    "ssn",
    "date_of_birth",
    "raw_payload",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise AssertionError(f"missing required file: {path}") from exc


def validate_schema(schema: str) -> None:
    require("CREATE DATABASE IF NOT EXISTS fwa_analytics" in schema, "schema must create database")
    require("MergeTree" in schema, "schema must use ClickHouse MergeTree")
    require("PARTITION BY toYYYYMM" in schema, "schema must partition by month")
    require("customer_scope_id" in schema, "schema must carry customer_scope_id")
    require("evidence_refs Array(String)" in schema, "schema must carry evidence refs")
    for table in REQUIRED_TABLES:
        require(
            f"CREATE TABLE IF NOT EXISTS fwa_analytics.{table}" in schema,
            f"schema missing table {table}",
        )
    lower_schema = schema.lower()
    for forbidden in FORBIDDEN_ANALYTICS_TERMS:
        require(forbidden not in lower_schema, f"schema contains raw PII term {forbidden}")


def validate_queries(queries: str) -> None:
    for block in REQUIRED_QUERY_BLOCKS:
        require(f"-- {block}" in queries, f"dashboard queries missing block {block}")
    for table in REQUIRED_TABLES:
        require(table in queries, f"dashboard queries missing table {table}")
    for snippet in [
        "false_positive_cost",
        "sla_breach_rate",
        "roi_ratio",
        "capacity_utilization",
        "avg_shadow_delta",
        "suspicious_cluster_score",
    ]:
        require(snippet in queries, f"dashboard queries missing metric {snippet}")


def validate_export_script() -> None:
    require(EXPORT_SCRIPT.is_file(), "missing build_analytics_export.py")
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run(
            [
                sys.executable,
                str(EXPORT_SCRIPT),
                "--output-dir",
                tmp,
                "--object-storage-uri",
                "s3://nwfwa-staging-artifacts",
                "--clickhouse-url",
                "http://clickhouse:8123",
                "--customer-scope-id",
                "staging-customer",
            ],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        output_dir = Path(tmp)
        manifest_path = output_dir / "analytics_export_manifest.json"
        scheduled_path = output_dir / "scheduled_exports.json"
        require(manifest_path.is_file(), "export script did not write manifest")
        require(scheduled_path.is_file(), "export script did not write scheduled exports")
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        require(manifest["derived_analytics_store"] == "ClickHouse", "manifest must target ClickHouse")
        jobs = {job["job_kind"]: job for job in manifest["scheduled_exports"]}
        for job_kind in REQUIRED_EXPORT_JOBS:
            require(job_kind in jobs, f"manifest missing export job {job_kind}")
            require(jobs[job_kind]["sink_table"].startswith("fwa_analytics."), f"{job_kind} sink must be ClickHouse")
            require(jobs[job_kind]["pii_boundary"].startswith("masked identifiers"), f"{job_kind} missing PII boundary")
        for artifact in ["schema.sql", "dashboard_queries.sql", "index.json"]:
            require((output_dir / artifact).is_file(), f"export script did not write {artifact}")


def main() -> int:
    validate_schema(read(SCHEMA_PATH))
    validate_queries(read(QUERIES_PATH))
    validate_export_script()
    print("analytics scale validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"analytics scale validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
