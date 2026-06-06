#!/usr/bin/env python3
"""Small concurrent smoke test for the TPA intake normalize endpoint."""

import argparse
import json
import os
import statistics
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from stream_tpa_demo_cases import build_demo_case, load_base_payload  # noqa: E402
from tpa_mock_client import request  # noqa: E402


ROOT_DIR = Path(__file__).resolve().parents[2]
DEFAULT_PAYLOAD_FILE = (
    ROOT_DIR / "data/tpa-rule-funnel-demo/inbox_payloads/manual_review_high_amount.json"
)


def normalize_one(base_url, api_key, base_payload, sequence):
    payload = build_demo_case(base_payload, "manual_review_high_amount", sequence)
    started = time.perf_counter()
    try:
        response = request(
            base_url,
            api_key,
            "POST",
            "/api/v1/inbox/claims/normalize",
            payload,
            allow_http_error=True,
        )
    except Exception as error:
        response = {
            "code": "REQUEST_FAILED",
            "message": str(error),
        }
    elapsed_ms = (time.perf_counter() - started) * 1000
    return {
        "sequence": sequence,
        "elapsed_ms": round(elapsed_ms, 2),
        "code": response.get("code"),
        "message": response.get("message"),
        "validation_result": response.get("validation_result"),
        "scoring_ready": response.get("scoring_ready"),
        "run_id": response.get("run_id"),
    }


def percentile(values, percent):
    if not values:
        return None
    ordered = sorted(values)
    index = min(len(ordered) - 1, round((percent / 100) * (len(ordered) - 1)))
    return round(ordered[index], 2)


def run_smoke(args):
    base_payload = load_base_payload(args.payload_file, args.inbox_correction_file)
    started = time.perf_counter()
    results = []
    with ThreadPoolExecutor(max_workers=args.concurrency) as executor:
        futures = [
            executor.submit(
                normalize_one,
                args.base_url,
                args.api_key,
                base_payload,
                args.sequence_start + index,
            )
            for index in range(args.requests)
        ]
        for future in as_completed(futures):
            results.append(future.result())
    elapsed_seconds = time.perf_counter() - started
    latencies = [result["elapsed_ms"] for result in results]
    failures = [
        result
        for result in results
        if result.get("code") or result.get("validation_result") not in {"accepted", "warning"}
    ]
    return {
        "requests": args.requests,
        "concurrency": args.concurrency,
        "elapsed_seconds": round(elapsed_seconds, 2),
        "throughput_rps": round(args.requests / elapsed_seconds, 2)
        if elapsed_seconds > 0
        else None,
        "latency_ms": {
            "min": round(min(latencies), 2) if latencies else None,
            "mean": round(statistics.fmean(latencies), 2) if latencies else None,
            "p50": percentile(latencies, 50),
            "p95": percentile(latencies, 95),
            "max": round(max(latencies), 2) if latencies else None,
        },
        "success_count": len(results) - len(failures),
        "failure_count": len(failures),
        "sample_failures": failures[:5],
    }


def main():
    parser = argparse.ArgumentParser(
        description="Run a bounded local concurrency smoke against TPA intake normalize."
    )
    parser.add_argument("--base-url", default=os.environ.get("FWA_API_BASE_URL", "http://127.0.0.1:8080"))
    parser.add_argument("--api-key", default=os.environ.get("FWA_API_KEY", "aiclaim-demo-key"))
    parser.add_argument("--payload-file", default=str(DEFAULT_PAYLOAD_FILE))
    parser.add_argument("--inbox-correction-file")
    parser.add_argument("--requests", type=int, default=20)
    parser.add_argument("--concurrency", type=int, default=4)
    parser.add_argument("--sequence-start", type=int, default=50000)
    args = parser.parse_args()

    if args.requests < 1:
        parser.error("--requests must be >= 1")
    if args.concurrency < 1:
        parser.error("--concurrency must be >= 1")
    if args.concurrency > args.requests:
        args.concurrency = args.requests

    print(json.dumps(run_smoke(args), ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
