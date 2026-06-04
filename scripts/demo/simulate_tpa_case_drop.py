#!/usr/bin/env python3
"""Simulate one raw TPA case drop and summarize the FWA response."""

import argparse
import json
import os
import sys
import time
from decimal import Decimal
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from tpa_mock_client import (  # noqa: E402
    add_correction_hints,
    load_json_file,
    merge_json_overlay,
    request,
)


DEFAULT_PAYLOAD_FILE = "/Users/proerror/Downloads/req.json"


def payload_brief(payload):
    report_case = payload.get("reportCase") or {}
    policies = report_case.get("policyList") or []
    invoices = [
        invoice
        for policy in policies
        for invoice in (policy.get("invoiceList") or [])
    ]
    medical_records = report_case.get("medicalRecordInfoList") or []
    fee_amounts = [
        Decimal(str(invoice.get("feeAmount")))
        for invoice in invoices
        if invoice.get("feeAmount") is not None
    ]
    first_record = medical_records[0] if medical_records else {}
    first_invoice = invoices[0] if invoices else {}

    return {
        "system_code": payload.get("systemCode"),
        "trans_no": payload.get("transNo"),
        "report_no": report_case.get("reportNo"),
        "report_id": report_case.get("id"),
        "calculate_risk": report_case.get("calculateRisk"),
        "policy_count": len(policies),
        "medical_record_count": len(medical_records),
        "invoice_count": len(invoices),
        "invoice_total_amount": float(sum(fee_amounts, Decimal("0"))),
        "first_medical_record": compact_dict(
            first_record,
            ["hospitalName", "departmentName", "diagnosisName", "medicalType"],
        ),
        "first_invoice": compact_dict(
            first_invoice,
            ["invoiceNo", "hospitalName", "feeAmount", "medicalType"],
        ),
    }


def compact_dict(source, keys):
    return {key: source.get(key) for key in keys if source.get(key) is not None}


def summarize_validation_errors(normalize_response):
    errors = normalize_response.get("validation_errors") or []
    hints_by_field = {
        hint.get("field_path"): hint
        for hint in normalize_response.get("correction_hints", [])
    }
    return [
        {
            "field_path": error.get("field_path"),
            "severity": error.get("severity"),
            "blocks_scoring": hints_by_field.get(error.get("field_path"), {}).get(
                "blocks_scoring"
            ),
            "next_action": hints_by_field.get(error.get("field_path"), {}).get(
                "next_action",
                error.get("remediation"),
            ),
        }
        for error in errors
    ]


def summarize_normalize(normalize_response):
    return {
        "code": normalize_response.get("code"),
        "message": normalize_response.get("message"),
        "validation_result": normalize_response.get("validation_result"),
        "scoring_ready": normalize_response.get("scoring_ready"),
        "run_id": normalize_response.get("run_id"),
        "audit_id": normalize_response.get("audit_id"),
        "idempotency_key": normalize_response.get("idempotency_key"),
        "data_quality_signals": normalize_response.get("data_quality_signals", []),
        "validation_errors": summarize_validation_errors(normalize_response),
        "correction_overlay_template": normalize_response.get(
            "correction_overlay_template"
        ),
    }


def summarize_score(score_response):
    if not score_response:
        return None
    return {
        "run_id": score_response.get("run_id"),
        "audit_id": score_response.get("audit_id"),
        "claim_id": score_response.get("claim_id"),
        "risk_score": score_response.get("risk_score"),
        "rag": score_response.get("rag"),
        "recommended_action": score_response.get("recommended_action"),
        "decision_outcome": score_response.get("decision_outcome"),
        "decision_authority": score_response.get("decision_authority"),
        "top_reasons": score_response.get("top_reasons", []),
        "evidence_refs": score_response.get("evidence_refs", []),
        "agent_investigation_prefill": score_response.get(
            "agent_investigation_prefill"
        ),
    }


def summarize_audit(audit_response):
    if not audit_response:
        return None
    return {
        "claim_id": audit_response.get("claim_id"),
        "event_types": [
            event.get("event_type")
            for event in audit_response.get("events", [])
            if event.get("event_type")
        ],
    }


def observe_fwa_response(base_url, api_key, raw_payload, include_audit):
    normalize_response = request(
        base_url,
        api_key,
        "POST",
        "/api/v1/inbox/claims/normalize",
        raw_payload,
        allow_http_error=True,
    )
    normalize_response = add_correction_hints(normalize_response)
    summary = {
        "input": payload_brief(raw_payload),
        "normalize": summarize_normalize(normalize_response),
        "score": None,
        "audit": None,
    }

    if normalize_response.get("scoring_ready") is not True:
        summary["fwa_response"] = normalize_stop_reason(normalize_response)
        return summary

    run_id = normalize_response.get("run_id")
    source_system = raw_payload.get("systemCode")
    if not run_id or not source_system:
        summary["fwa_response"] = "accepted_but_missing_scoring_handoff"
        return summary

    score_response = request(
        base_url,
        api_key,
        "POST",
        "/api/v1/claims/score",
        {
            "source_system": source_system,
            "inbox_run_id": run_id,
        },
    )
    summary["score"] = summarize_score(score_response)
    summary["fwa_response"] = "scored_and_routed"

    claim_id = score_response.get("claim_id")
    if include_audit and claim_id:
        audit_response = request(
            base_url,
            api_key,
            "GET",
            f"/api/v1/audit/claims/{claim_id}",
        )
        summary["audit"] = summarize_audit(audit_response)

    return summary


def load_payload(payload_file, correction_file):
    raw_payload = load_json_file(payload_file)
    if correction_file:
        raw_payload = merge_json_overlay(raw_payload, load_json_file(correction_file))
    return raw_payload


def normalize_stop_reason(normalize_response):
    if normalize_response.get("code") == "INBOX_IDEMPOTENCY_CONFLICT":
        return "idempotency_conflict"
    if (
        normalize_response.get("code")
        or normalize_response.get("validation_result") == "rejected"
    ):
        return "rejected_at_intake"
    return "accepted_for_audit_but_not_ready_for_scoring"


def with_unique_trans_no(raw_payload):
    payload = json.loads(json.dumps(raw_payload))
    base_trans_no = payload.get("transNo") or "missing-trans-no"
    payload["transNo"] = f"{base_trans_no}-sim-{int(time.time())}"
    return payload


def main():
    parser = argparse.ArgumentParser(
        description="Drop one raw TPA case into FWA and print the observed response."
    )
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--api-key", default=os.environ.get("FWA_API_KEY", "dev-secret"))
    parser.add_argument("--payload-file", default=DEFAULT_PAYLOAD_FILE)
    parser.add_argument(
        "--inbox-correction-file",
        help="Optional local overlay merged before normalize; raw payload is not modified.",
    )
    parser.add_argument(
        "--skip-audit",
        action="store_true",
        help="Do not fetch the claim audit timeline after scoring.",
    )
    parser.add_argument(
        "--unique-trans-no",
        action="store_true",
        help="Append a runtime suffix to transNo in memory for repeat demos.",
    )
    args = parser.parse_args()

    raw_payload = load_payload(args.payload_file, args.inbox_correction_file)
    if args.unique_trans_no:
        raw_payload = with_unique_trans_no(raw_payload)
    summary = observe_fwa_response(
        args.base_url,
        args.api_key,
        raw_payload,
        include_audit=not args.skip_audit,
    )
    print(json.dumps(summary, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
