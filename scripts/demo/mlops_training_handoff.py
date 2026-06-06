#!/usr/bin/env python3
import argparse
import json
import sys
import time
from pathlib import Path
import urllib.error
import urllib.parse
import urllib.request


def request_json(base_url, method, path, payload=None, api_key=None):
    body = None
    headers = {}
    if api_key:
        headers["x-api-key"] = api_key
    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        headers["content-type"] = "application/json"
    req = urllib.request.Request(
        f"{base_url.rstrip('/')}{path}",
        data=body,
        headers=headers,
        method=method,
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            raw = response.read().decode("utf-8")
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as error:
        raw = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {path} returned HTTP {error.code}: {raw}") from error


def train_provider_output(
    ml_service_url,
    manifest_path,
    artifact_base_uri,
    model_key,
    base_model_version,
    job_id,
    actor,
    algorithm,
    poll_interval_seconds=1.0,
    timeout_seconds=300.0,
):
    payload = {
        "manifest_path": manifest_path,
        "artifact_base_uri": artifact_base_uri,
        "model_key": model_key,
        "base_model_version": base_model_version,
        "job_id": job_id,
        "actor": actor,
    }
    if algorithm:
        payload["algorithm"] = algorithm
    training_job = request_json(ml_service_url, "POST", "/training-jobs", payload)
    deadline = time.monotonic() + timeout_seconds
    while training_job.get("status") in {"queued", "running"}:
        if time.monotonic() >= deadline:
            raise TimeoutError(
                f"training job did not complete before timeout: {training_job.get('status')}"
            )
        time.sleep(poll_interval_seconds)
        training_job = request_json(
            ml_service_url,
            "GET",
            f"/training-jobs/{urllib.parse.quote(job_id, safe='')}",
        )
    if training_job.get("status") != "completed":
        raise ValueError(
            f"training job did not complete: {training_job.get('status', 'unknown')}"
        )
    output = training_job.get("provider_output")
    if not isinstance(output, dict):
        raise ValueError("training job response missing provider_output")
    validate_provider_output(output)
    return output


def register_provider_output(api_url, api_key, job_id, provider_output):
    if not api_key:
        raise ValueError("--api-key is required when --register is set")
    return request_json(
        api_url,
        "POST",
        f"/api/v1/ops/model-retraining-jobs/{urllib.parse.quote(job_id, safe='')}/output",
        provider_output,
        api_key=api_key,
    )


def validate_provider_output(output):
    required_fields = [
        "candidate_model_version",
        "artifact_uri",
        "artifact_sha256",
        "validation_report_uri",
        "evaluation_run_id",
        "metrics_json",
        "evidence_refs",
    ]
    missing = [field for field in required_fields if not output.get(field)]
    if missing:
        raise ValueError(f"training output missing required fields: {', '.join(missing)}")
    if not isinstance(output["evidence_refs"], list):
        raise ValueError("training output evidence_refs must be a list")
    if (
        output.get("mined_rule_candidates")
        and output.get("mined_rule_owner") != "external-training-platform"
    ):
        raise ValueError(
            "mined rule candidates must be owned by external-training-platform"
        )


def write_json_file(path, payload):
    output_path = Path(path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as file:
        json.dump(payload, file, ensure_ascii=False, indent=2, sort_keys=True)
        file.write("\n")


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description=(
            "Run the independent ML training service and optionally register its "
            "completed provider output into the FWA retraining job API."
        )
    )
    parser.add_argument("--ml-service-url", default="http://127.0.0.1:8001")
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--artifact-base-uri", required=True)
    parser.add_argument("--model-key", default="baseline_fwa")
    parser.add_argument("--base-model-version", required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--actor", default="external-training-platform")
    parser.add_argument(
        "--algorithm",
        choices=["logistic_regression", "xgboost", "lightgbm"],
        help="Training algorithm. Defaults to the manifest or logistic regression.",
    )
    parser.add_argument("--write-provider-output")
    parser.add_argument("--poll-interval-seconds", type=float, default=1.0)
    parser.add_argument("--timeout-seconds", type=float, default=300.0)
    parser.add_argument(
        "--register",
        action="store_true",
        help="POST the completed provider output to the FWA API after training.",
    )
    parser.add_argument("--api-url", default="http://127.0.0.1:8080")
    parser.add_argument("--api-key")
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        provider_output = train_provider_output(
            args.ml_service_url,
            args.manifest,
            args.artifact_base_uri,
            args.model_key,
            args.base_model_version,
            args.job_id,
            args.actor,
            args.algorithm,
            poll_interval_seconds=args.poll_interval_seconds,
            timeout_seconds=args.timeout_seconds,
        )
        result = {
            "handoff_status": "provider_output_ready",
            "job_id": args.job_id,
            "candidate_model_version": provider_output["candidate_model_version"],
            "provider_output": provider_output,
        }
        if args.write_provider_output:
            write_json_file(args.write_provider_output, provider_output)
            result["provider_output_path"] = args.write_provider_output
        if args.register:
            registration = register_provider_output(
                args.api_url,
                args.api_key,
                args.job_id,
                provider_output,
            )
            result["handoff_status"] = "registered_with_fwa"
            result["registration_response"] = registration
        print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
        return 0
    except (RuntimeError, OSError, TimeoutError, ValueError) as error:
        print(json.dumps({"error": str(error)}, ensure_ascii=False), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
