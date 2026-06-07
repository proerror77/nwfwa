#!/usr/bin/env python3
"""Validate a customer-provided staging Secret YAML before apply."""

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

PLACEHOLDER_MARKERS = (
    "replace-with-",
    "changeme",
    "dev-secret",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def parse_string_data_keys(text: str) -> set[str]:
    keys: set[str] = set()
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
            keys.add(stripped.split(":", 1)[0])
    return keys


def validate_secret_file(path: Path, allow_placeholders: bool = False) -> None:
    text = path.read_text(encoding="utf-8")
    require("kind: Secret" in text, "secret file must be a Kubernetes Secret")
    require(
        "name: nwfwa-staging-secrets" in text,
        "secret name must be nwfwa-staging-secrets",
    )
    require("type: Opaque" in text, "secret type must be Opaque")
    keys = parse_string_data_keys(text)
    missing = sorted(REQUIRED_STRING_DATA_KEYS - keys)
    require(not missing, f"secret file missing stringData keys: {', '.join(missing)}")
    if not allow_placeholders:
        lowered = text.lower()
        found = [marker for marker in PLACEHOLDER_MARKERS if marker in lowered]
        require(not found, f"secret file still contains placeholders: {', '.join(found)}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--secret-file", required=True)
    parser.add_argument("--allow-placeholders", action="store_true")
    args = parser.parse_args()

    validate_secret_file(Path(args.secret_file), args.allow_placeholders)
    print("staging secret file validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"staging secret file validation failed: {exc}")
        raise SystemExit(1)
