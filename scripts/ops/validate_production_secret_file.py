#!/usr/bin/env python3
"""Validate a customer-provided production Secret YAML before apply."""

from __future__ import annotations

import argparse
from pathlib import Path


REQUIRED_STRING_DATA_KEYS = {
    "FWA_API_KEY",
    "FWA_API_KEY_PRINCIPALS",
    "FWA_MODEL_SIGNATURE_KEY",
    "FWA_ALERT_RECEIVER_TOKEN",
    "FWA_ALERT_RECEIVER_SIGNING_SECRET",
    "FWA_MLOPS_ALERT_ROUTER_TOKEN",
    "DATABASE_URL",
    "POSTGRES_USER",
    "POSTGRES_PASSWORD",
    "POSTGRES_DB",
    "MINIO_ROOT_USER",
    "MINIO_ROOT_PASSWORD",
}

SENSITIVE_KEYS = {
    "FWA_API_KEY",
    "FWA_MODEL_SIGNATURE_KEY",
    "FWA_ALERT_RECEIVER_TOKEN",
    "FWA_ALERT_RECEIVER_SIGNING_SECRET",
    "FWA_MLOPS_ALERT_ROUTER_TOKEN",
    "POSTGRES_PASSWORD",
    "MINIO_ROOT_PASSWORD",
}

FORBIDDEN_MARKERS = (
    "replace-with-",
    "changeme",
    "dev-secret",
    "staging-",
    "demo-",
    "localhost",
    "example.invalid",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def parse_string_data(text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    in_string_data = False
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped == "stringData:":
            in_string_data = True
            continue
        if in_string_data and line and not line.startswith((" ", "\t")):
            break
        if in_string_data and ":" in stripped:
            key, value = stripped.split(":", 1)
            values[key] = value.strip().strip("\"'")
    return values


def validate_secret_file(path: Path, namespace: str) -> None:
    text = path.read_text(encoding="utf-8")
    require("kind: Secret" in text, "secret file must be a Kubernetes Secret")
    require(f"name: {namespace}-secrets" in text, f"secret name must be {namespace}-secrets")
    require("type: Opaque" in text, "secret type must be Opaque")
    values = parse_string_data(text)
    missing = sorted(REQUIRED_STRING_DATA_KEYS - set(values))
    require(not missing, f"secret file missing stringData keys: {', '.join(missing)}")
    empty = sorted(key for key in REQUIRED_STRING_DATA_KEYS if not values.get(key))
    require(not empty, f"secret file has empty stringData values: {', '.join(empty)}")
    weak = sorted(key for key in SENSITIVE_KEYS if len(values.get(key, "")) < 16)
    require(not weak, f"secret file has weak production secret values: {', '.join(weak)}")
    duplicate_sensitive_values = {
        value
        for value in (values[key] for key in SENSITIVE_KEYS if key in values)
        if list(values[key] for key in SENSITIVE_KEYS if key in values).count(value) > 1
    }
    require(not duplicate_sensitive_values, "secret file reuses sensitive values across keys")
    principals = values.get("FWA_API_KEY_PRINCIPALS", "")
    require(principals.count("|") >= 5, "FWA_API_KEY_PRINCIPALS must include key, actor, role, tenant, scope, and permissions")
    database_url = values.get("DATABASE_URL", "")
    require(database_url.startswith(("postgres://", "postgresql://")), "DATABASE_URL must be a PostgreSQL URL")
    lowered = text.lower()
    found = [marker for marker in FORBIDDEN_MARKERS if marker in lowered]
    require(not found, f"production secret file contains forbidden marker: {', '.join(found)}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--secret-file", required=True)
    parser.add_argument("--namespace", default="nwfwa-production")
    args = parser.parse_args()

    validate_secret_file(Path(args.secret_file), args.namespace)
    print("production secret file validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"production secret file validation failed: {exc}")
        raise SystemExit(1)
