#!/usr/bin/env python3
"""Static checks for staging container packaging."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]

REQUIRED_FILES = {
    ".dockerignore": [
        "target",
        "apps/web-console/node_modules",
        "apps/web-console/dist",
        "**/.venv",
        "**/*.egg-info",
    ],
    "apps/api-server/Dockerfile": [
        "FROM rust:1.96-bookworm AS builder",
        "CARGO_INCREMENTAL=0",
        "cargo build --locked -p api-server",
        "target/debug/api-server",
        "COPY apps ./apps",
        "COPY crates ./crates",
        "CMD [\"api-server\"]",
    ],
    "apps/worker/Dockerfile": [
        "FROM rust:1.96-bookworm AS builder",
        "cargo build --release --locked -p worker",
        "COPY apps ./apps",
        "COPY crates ./crates",
        "CMD [\"worker\", \"health\"]",
    ],
    "apps/web-console/Dockerfile": [
        "FROM rust:1.96-bookworm AS builder",
        "rustup target add wasm32-unknown-unknown",
        "cargo install trunk --version 0.21.14 --locked",
        "CARGO_INCREMENTAL=0",
        "NO_COLOR=false trunk build --release --locked",
        "COPY apps/web-console/nginx.conf /etc/nginx/conf.d/default.conf",
        "nginx:1.27-alpine",
    ],
    "apps/web-console/nginx.conf": [
        "listen 8081;",
        "location = /",
        "try_files /index.html =404;",
        "try_files $uri /index.html;",
        "proxy_pass http://api-server:8080/api/;",
    ],
    "infra/docker-compose.yml": [
        "migrate-seed:",
        "api-server:",
        "FWA_BIND_ADDR: 0.0.0.0:8080",
        "FWA_MODEL_SERVICE_URL: http://ml-service:8001",
        "web-console:",
        "dockerfile: apps/web-console/Dockerfile",
        '"5173:8081"',
        "object-storage:",
        "clickhouse:",
        "clickhouse/clickhouse-server:24.8",
        "../analytics/clickhouse/schema.sql:/docker-entrypoint-initdb.d/001_schema.sql:ro",
    ],
    "infra/dockerfiles/Dockerfile.ops": [
        "FROM postgres:16",
        "COPY migrations ./migrations",
        "COPY scripts/demo/seed_demo.sql ./scripts/demo/seed_demo.sql",
        "COPY scripts/demo/assert_demo_persistence.sql ./scripts/demo/assert_demo_persistence.sql",
    ],
    "infra/k8s/staging/database-jobs.yaml": [
        "name: database-migrate",
        "name: demo-seed",
        "ghcr.io/replace-me/nwfwa-ops:staging",
        "/app/migrations/0001_initial.sql",
        "/app/scripts/demo/seed_demo.sql",
    ],
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def main() -> int:
    for relative_path, snippets in REQUIRED_FILES.items():
        path = ROOT / relative_path
        require(path.is_file(), f"missing required packaging file: {relative_path}")
        text = path.read_text(encoding="utf-8")
        for snippet in snippets:
            require(snippet in text, f"{relative_path} missing snippet: {snippet}")

    kustomization = (ROOT / "infra/k8s/staging/kustomization.yaml").read_text(encoding="utf-8")
    require("database-jobs.yaml" in kustomization, "database Jobs must be part of staging kustomization")

    print("container packaging validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"container packaging validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
