#!/usr/bin/env python3
"""Generate and stream demo TPA case drops from the AiClaim sample payload."""

import argparse
import copy
import json
import os
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from simulate_tpa_case_drop import (  # noqa: E402
    DEFAULT_PAYLOAD_FILE,
    observe_fwa_response,
)
from tpa_mock_client import load_json_file, merge_json_overlay  # noqa: E402


DEFAULT_CASES = [
    "low_risk_request_evidence",
    "request_evidence_dental",
    "manual_review_high_amount",
    "intake_block_missing_coverage",
    "intake_reject_source_mismatch",
]


def set_path(payload, path, value):
    cursor = payload
    for key in path[:-1]:
        cursor = cursor[key]
    cursor[path[-1]] = value


def first_invoice(payload):
    return payload["reportCase"]["policyList"][0]["invoiceList"][0]


def first_fee_details(payload):
    return [
        detail
        for fee in first_invoice(payload).get("feeList", [])
        for detail in fee.get("feeDetailList", [])
    ]


def apply_identity_consistency(payload, member_name="LEE, Peter"):
    report_case = payload["reportCase"]
    first_invoice(payload)["accidentPersonName"] = member_name
    if report_case.get("medicalRecordInfoList"):
        report_case["medicalRecordInfoList"][0]["patientName"] = member_name


def apply_case_identity(payload, case_kind, sequence):
    report_case = payload["reportCase"]
    safe_kind = case_kind.upper().replace("_", "-")
    report_case["id"] = 100000 + sequence
    report_case["reportNo"] = f"TPA-DEMO-{safe_kind}-{sequence:05d}"
    payload["transNo"] = f"TPA-DEMO-TXN-{safe_kind}-{sequence:05d}-{int(time.time())}"
    first_invoice(payload)["invoiceNo"] = f"TPA-DEMO-INV-{sequence:05d}"
    first_invoice(payload)["invoiceBusinessId"] = f"TPA-DEMO-INV-{sequence:05d}-BIZ"


def apply_amount(payload, amount):
    invoice = first_invoice(payload)
    invoice["feeAmount"] = amount
    invoice["selfPayAmount"] = round(amount * 0.28, 2)
    invoice["medicareAmount"] = round(amount * 0.34, 2)
    details = first_fee_details(payload)
    if not details:
        return
    if len(details) == 1:
        details[0]["amount"] = amount
        details[0]["selfPayAmount"] = round(amount * 0.28, 2)
        return
    first_amount = round(amount * 0.75, 2)
    second_amount = round(amount - first_amount, 2)
    details[0]["amount"] = first_amount
    details[0]["selfPayAmount"] = round(first_amount * 0.28, 2)
    details[1]["amount"] = second_amount
    details[1]["selfPayAmount"] = round(second_amount * 0.28, 2)
    fee_groups = invoice.get("feeList", [])
    if len(fee_groups) >= 2:
        fee_groups[0]["feeAmount"] = first_amount
        fee_groups[0]["selfPayAmount"] = round(first_amount * 0.28, 2)
        fee_groups[1]["feeAmount"] = second_amount
        fee_groups[1]["selfPayAmount"] = round(second_amount * 0.28, 2)


def apply_routine_low_risk_items(payload, amount):
    invoice = first_invoice(payload)
    invoice["feeAmount"] = amount
    invoice["selfPayAmount"] = round(amount * 0.2, 2)
    invoice["medicareAmount"] = round(amount * 0.5, 2)
    item_amount = round(amount / 3, 2)
    invoice["feeList"] = [
        {
            "id": 2001,
            "feeAmount": amount,
            "feeCategory": "consultation",
            "medicareAmount": round(amount * 0.5, 2),
            "otherAmount": 0,
            "ownExpenseAmount": 0,
            "selfPayAmount": round(amount * 0.2, 2),
            "feeDetailList": [
                {
                    "id": 2002,
                    "medicalCategory": "routine",
                    "name": "普通门诊挂号费",
                    "amount": item_amount,
                    "ownExpenseAmount": 0,
                    "selfPayAmount": round(item_amount * 0.2, 2),
                },
                {
                    "id": 2003,
                    "medicalCategory": "routine",
                    "name": "常规口腔检查",
                    "amount": item_amount,
                    "ownExpenseAmount": 0,
                    "selfPayAmount": round(item_amount * 0.2, 2),
                },
                {
                    "id": 2004,
                    "medicalCategory": "routine",
                    "name": "普通护理处置",
                    "amount": round(amount - item_amount * 2, 2),
                    "ownExpenseAmount": 0,
                    "selfPayAmount": round((amount - item_amount * 2) * 0.2, 2),
                },
            ],
        }
    ]


def build_demo_case(base_payload, case_kind, sequence):
    payload = copy.deepcopy(base_payload)
    apply_case_identity(payload, case_kind, sequence)
    payload["systemCode"] = "AiClaim Core"
    payload["reportCase"]["calculateRisk"] = "Y"
    payload["reportCase"].pop("claimAmount", None)

    policy = payload["reportCase"]["policyList"][0]
    policy["coverageLimit"] = 20000

    if case_kind == "low_risk_request_evidence":
        apply_identity_consistency(payload)
        apply_routine_low_risk_items(payload, 180.0)
        policy["coverageLimit"] = 100000
        payload["reportCase"]["claimAmount"] = 180.0
        payload["reportCase"]["accidentReason"] = "routine_outpatient"
    elif case_kind == "request_evidence_dental":
        apply_amount(payload, 397.06)
        policy["coverageLimit"] = 20000
        payload["reportCase"]["calculateRisk"] = "N"
    elif case_kind == "manual_review_high_amount":
        apply_identity_consistency(payload)
        apply_amount(payload, 18600.0)
        policy["coverageLimit"] = 20000
        payload["reportCase"]["claimAmount"] = 18600.0
        first_invoice(payload)["redFlag"] = "Y"
    elif case_kind == "intake_block_missing_coverage":
        apply_amount(payload, 397.06)
        policy.pop("coverageLimit", None)
        payload["reportCase"]["calculateRisk"] = "N"
    elif case_kind == "intake_reject_source_mismatch":
        apply_identity_consistency(payload)
        apply_amount(payload, 397.06)
        payload["systemCode"] = "Wrong TPA"
    else:
        raise ValueError(f"unknown demo case kind: {case_kind}")

    return payload


def actual_bucket(summary):
    response = summary.get("fwa_response")
    if response in {"rejected_at_intake", "idempotency_conflict"}:
        return response
    if response == "accepted_for_audit_but_not_ready_for_scoring":
        return "intake_blocked"
    score = summary.get("score") or {}
    action = score.get("recommended_action")
    if action == "StandardProcessing":
        return "straight_through"
    if action == "RequestEvidence":
        return "request_evidence"
    if action in {"ManualReview", "EscalateInvestigation", "ProviderReview"}:
        return "manual_review"
    if action == "QaSample":
        return "qa_sample"
    return response or "unknown"


def expected_bucket(case_kind):
    return {
        "low_risk_request_evidence": "request_evidence",
        "request_evidence_dental": "request_evidence",
        "manual_review_high_amount": "manual_review",
        "intake_block_missing_coverage": "intake_blocked",
        "intake_reject_source_mismatch": "rejected_at_intake",
    }[case_kind]


def load_base_payload(payload_file, correction_file=None):
    payload = load_json_file(payload_file)
    if correction_file:
        payload = merge_json_overlay(payload, load_json_file(correction_file))
    return payload


def emit_payloads(output_dir, cases):
    output = Path(output_dir)
    output.mkdir(parents=True, exist_ok=True)
    written = []
    for case_kind, payload in cases:
        path = output / f"{case_kind}.json"
        path.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        written.append(str(path))
    return written


def run_stream(args):
    base_payload = load_base_payload(args.payload_file, args.inbox_correction_file)
    case_kinds = args.case or DEFAULT_CASES
    sequence = args.sequence_start
    results = []

    for iteration in range(args.iterations):
        generated_cases = [
            (case_kind, build_demo_case(base_payload, case_kind, sequence + index))
            for index, case_kind in enumerate(case_kinds)
        ]
        sequence += len(generated_cases)

        if args.emit_dir:
            emit_payloads(args.emit_dir, generated_cases)

        for case_kind, payload in generated_cases:
            if args.dry_run:
                summary = {
                    "case_kind": case_kind,
                    "expected_bucket": expected_bucket(case_kind),
                    "payload_brief": {
                        "systemCode": payload.get("systemCode"),
                        "reportNo": payload.get("reportCase", {}).get("reportNo"),
                        "transNo": payload.get("transNo"),
                        "coverageLimit": payload.get("reportCase", {})
                        .get("policyList", [{}])[0]
                        .get("coverageLimit"),
                        "feeAmount": first_invoice(payload).get("feeAmount"),
                    },
                }
            else:
                response = observe_fwa_response(
                    args.base_url,
                    args.api_key,
                    payload,
                    include_audit=not args.skip_audit,
                )
                summary = {
                    "case_kind": case_kind,
                    "expected_bucket": expected_bucket(case_kind),
                    "actual_bucket": actual_bucket(response),
                    "claim_id": response.get("input", {}).get("report_no"),
                    "fwa_response": response.get("fwa_response"),
                    "normalize": response.get("normalize"),
                    "score": response.get("score"),
                    "audit": response.get("audit"),
                }
            results.append(summary)
            print(json.dumps(summary, ensure_ascii=False, sort_keys=True), flush=True)

            if args.interval_seconds > 0:
                time.sleep(args.interval_seconds)

    return results


def main():
    parser = argparse.ArgumentParser(
        description="Generate raw TPA demo cases from req.json and stream them into FWA."
    )
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--api-key", default=os.environ.get("FWA_API_KEY", "dev-secret"))
    parser.add_argument("--payload-file", default=DEFAULT_PAYLOAD_FILE)
    parser.add_argument(
        "--inbox-correction-file",
        help="Optional overlay applied to the base payload before variant generation.",
    )
    parser.add_argument(
        "--case",
        action="append",
        choices=DEFAULT_CASES,
        help="Case kind to stream. Can be repeated. Defaults to all demo kinds.",
    )
    parser.add_argument("--iterations", type=int, default=1)
    parser.add_argument("--interval-seconds", type=float, default=0.0)
    parser.add_argument("--sequence-start", type=int, default=1)
    parser.add_argument("--emit-dir", help="Optional directory for generated raw payload JSON.")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--skip-audit", action="store_true")
    args = parser.parse_args()

    if args.iterations < 1:
        parser.error("--iterations must be >= 1")
    if args.interval_seconds < 0:
        parser.error("--interval-seconds must be >= 0")

    run_stream(args)
    return 0


if __name__ == "__main__":
    sys.exit(main())
