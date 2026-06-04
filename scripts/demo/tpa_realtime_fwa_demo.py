#!/usr/bin/env python3
"""Run a business-facing real-time TPA FWA demo chain."""

import argparse
import json
import os
import sys
import time
from decimal import Decimal
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from simulate_tpa_case_drop import observe_fwa_response  # noqa: E402
from tpa_mock_client import load_json_file, merge_json_overlay, request  # noqa: E402


ROOT_DIR = Path(__file__).resolve().parents[2]
DEFAULT_PAYLOAD_FILE = (
    ROOT_DIR / "data/tpa-rule-funnel-demo/inbox_payloads/manual_review_high_amount.json"
)


def claim_amount_from_payload(payload):
    report_case = payload.get("reportCase") or {}
    if report_case.get("claimAmount") is not None:
        return Decimal(str(report_case["claimAmount"]))
    total = Decimal("0")
    for policy in report_case.get("policyList") or []:
        for invoice in policy.get("invoiceList") or []:
            if invoice.get("feeAmount") is not None:
                total += Decimal(str(invoice["feeAmount"]))
    return total


def load_payload(payload_file, correction_file):
    payload = load_json_file(payload_file)
    if correction_file:
        payload = merge_json_overlay(payload, load_json_file(correction_file))
    return payload


def make_repeatable_payload(payload, suffix):
    updated = json.loads(json.dumps(payload))
    report_case = updated.setdefault("reportCase", {})
    base_report_no = report_case.get("reportNo") or report_case.get("id") or "TPA-DEMO"
    report_case["reportNo"] = f"{base_report_no}-RT-{suffix}"
    updated["transNo"] = f"{updated.get('transNo') or 'TPA-DEMO-TXN'}-RT-{suffix}"
    return updated


def latest_lead_for_claim(base_url, api_key, claim_id, score_run_id):
    leads = request(base_url, api_key, "GET", "/api/v1/ops/leads").get("leads", [])
    for lead in leads:
        if lead.get("claim_id") == claim_id and lead.get("run_id") == score_run_id:
            return lead
    for lead in leads:
        if lead.get("claim_id") == claim_id:
            return lead
    return None


def open_case_from_lead(base_url, api_key, lead):
    payload = {
        "decision": "open_case",
        "merge_target_lead_id": None,
        "assignee": "demo-investigator",
        "reviewer": "demo-reviewer",
        "priority": "high",
        "notes": "Real-time TPA demo opens a governed FWA investigation case.",
        "evidence_refs": lead.get("evidence_refs") or [f"leads:{lead['lead_id']}"],
    }
    return request(
        base_url,
        api_key,
        "POST",
        f"/api/v1/ops/leads/{lead['lead_id']}/triage",
        payload,
    )


def update_case_to_investigating(base_url, api_key, case_id, triage_audit_id):
    return request(
        base_url,
        api_key,
        "POST",
        f"/api/v1/ops/cases/{case_id}/status",
        {
            "status": "investigating",
            "actor_id": "demo-investigator",
            "notes": "Real-time TPA demo investigation started from the triaged lead.",
            "evidence_refs": [f"investigation_cases:{case_id}", f"audit:{triage_audit_id}"],
        },
    )


def record_confirmed_prevented_payment(base_url, api_key, case_id, claim_id, score, amount, suffix):
    evidence_refs = [
        f"investigation_cases:{case_id}",
        f"audit:{score['audit_id']}",
    ]
    evidence_refs.extend(score.get("evidence_refs") or [])
    return request(
        base_url,
        api_key,
        "POST",
        "/api/v1/investigations/results",
        {
            "case_id": case_id,
            "claim_id": claim_id,
            "investigation_id": f"INV-TPA-REALTIME-{suffix}",
            "outcome": "confirmed_fwa_prevented_payment",
            "confirmed_fwa": True,
            "financial_impact_type": "prevented_payment",
            "saving_amount": f"{amount:.2f}",
            "currency": "CNY",
            "notes": "Demo reviewer confirmed the pre-payment FWA intervention and prevented payment.",
            "evidence_refs": evidence_refs,
        },
    )


def value_snapshot(base_url, api_key):
    dashboard = request(base_url, api_key, "GET", "/api/v1/ops/dashboard/summary")
    value = dashboard.get("value_measurement") or {}
    return {
        "suspected_claims": dashboard.get("suspected_claims"),
        "confirmed_fwa": dashboard.get("confirmed_fwa"),
        "saving_amount": dashboard.get("saving_amount"),
        "prevented_payment": value.get("prevented_payment"),
        "recovered_amount": value.get("recovered_amount"),
        "estimated_impact": value.get("estimated_impact"),
        "net_value": value.get("net_value"),
        "evidence_caveat": value.get("evidence_caveat"),
    }


def run_demo(args):
    suffix = str(int(time.time()))
    raw_payload = load_payload(args.payload_file, args.inbox_correction_file)
    if args.unique_message:
        raw_payload = make_repeatable_payload(raw_payload, suffix)

    claim_amount = claim_amount_from_payload(raw_payload)
    before_value = value_snapshot(args.base_url, args.api_key)
    observed = observe_fwa_response(args.base_url, args.api_key, raw_payload, include_audit=True)
    if observed.get("fwa_response") != "scored_and_routed":
        return {
            "status": "stopped_before_scoring",
            "claim_amount": f"{claim_amount:.2f}",
            "observed_response": observed,
            "dashboard_before": before_value,
        }

    score = observed["score"]
    claim_id = score["claim_id"]
    lead = latest_lead_for_claim(args.base_url, args.api_key, claim_id, score["run_id"])
    if not lead:
        raise RuntimeError(f"no generated FWA lead found for claim {claim_id}")

    triage = open_case_from_lead(args.base_url, args.api_key, lead)
    case = triage.get("case")
    if not case:
        raise RuntimeError(f"lead {lead['lead_id']} did not open a case")
    case_update = update_case_to_investigating(
        args.base_url,
        args.api_key,
        case["case_id"],
        triage["audit_id"],
    )
    investigation = record_confirmed_prevented_payment(
        args.base_url,
        args.api_key,
        case["case_id"],
        claim_id,
        score,
        claim_amount,
        suffix,
    )
    after_value = value_snapshot(args.base_url, args.api_key)

    return {
        "status": "completed",
        "business_story": {
            "message": "TPA claim was normalized, scored, routed to investigation, confirmed, written back, and counted as prevented payment.",
            "claim_amount": f"{claim_amount:.2f}",
            "financial_impact_type": "prevented_payment",
        },
        "claim": {
            "claim_id": claim_id,
            "risk_score": score.get("risk_score"),
            "rag": score.get("rag"),
            "recommended_action": score.get("recommended_action"),
            "decision_outcome": score.get("decision_outcome"),
        },
        "workflow": {
            "inbox_run_id": observed["normalize"].get("run_id"),
            "score_run_id": score["run_id"],
            "score_audit_id": score["audit_id"],
            "lead_id": lead["lead_id"],
            "case_id": case["case_id"],
            "case_status": case_update.get("case", {}).get("status"),
            "investigation_audit_id": investigation.get("audit_id"),
            "writeback_idempotency_key": investigation.get("idempotency_key"),
        },
        "dashboard_before": before_value,
        "dashboard_after": after_value,
        "ui_targets": {
            "intake": f"{args.web_url.rstrip('/')}/#intake-ops",
            "cases": f"{args.web_url.rstrip('/')}/#leads-cases",
            "dashboard": f"{args.web_url.rstrip('/')}/#dashboard",
        },
    }


def main():
    parser = argparse.ArgumentParser(
        description="Run the real-time TPA FWA demo from raw packet to prevented-payment value."
    )
    parser.add_argument("--base-url", default=os.environ.get("FWA_API_BASE_URL", "http://127.0.0.1:8080"))
    parser.add_argument("--web-url", default=os.environ.get("FWA_WEB_URL", "http://127.0.0.1:5173"))
    parser.add_argument("--api-key", default=os.environ.get("FWA_API_KEY", "dev-secret"))
    parser.add_argument("--payload-file", default=str(DEFAULT_PAYLOAD_FILE))
    parser.add_argument("--inbox-correction-file")
    parser.add_argument(
        "--reuse-message",
        dest="unique_message",
        action="store_false",
        help="Reuse the payload's source transaction identifiers instead of appending a demo suffix.",
    )
    parser.set_defaults(unique_message=True)
    args = parser.parse_args()

    print(json.dumps(run_demo(args), ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
