from __future__ import annotations

import argparse
import json
from pathlib import Path

from .training import train_from_manifest


def main() -> None:
    parser = argparse.ArgumentParser(description="Train an FWA baseline model artifact.")
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--artifact-base-uri", required=True)
    parser.add_argument("--model-key", required=True)
    parser.add_argument("--base-model-version", required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--actor", required=True)
    parser.add_argument(
        "--algorithm",
        choices=["logistic_regression", "xgboost", "lightgbm"],
        help="Training algorithm. Defaults to manifest.algorithm or logistic_regression.",
    )
    args = parser.parse_args()

    payload = train_from_manifest(
        manifest_path=Path(args.manifest),
        artifact_base_uri=Path(args.artifact_base_uri),
        model_key=args.model_key,
        base_model_version=args.base_model_version,
        job_id=args.job_id,
        actor=args.actor,
        algorithm=args.algorithm,
    )
    print(json.dumps(payload, sort_keys=True))


if __name__ == "__main__":
    main()
