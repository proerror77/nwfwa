#!/usr/bin/env python3
import argparse
import json
import time
import urllib.error
import urllib.request


def request(base_url, api_key, method, path, payload=None):
    body = None
    headers = {"x-api-key": api_key}
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
        with urllib.request.urlopen(req, timeout=10) as response:
            raw = response.read().decode("utf-8")
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as error:
        raw = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {path} returned HTTP {error.code}: {raw}") from error


def require(value, message):
    if not value:
        raise AssertionError(message)
    return value


def main():
    parser = argparse.ArgumentParser(description="Run the pilot TPA integration flow.")
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--api-key", default="dev-secret")
    parser.add_argument("--source-system", default="tpa-demo")
    parser.add_argument("--claim-id", default="CLM-0287")
    parser.add_argument("--member-id", default="MBR-0287")
    args = parser.parse_args()

    suffix = str(int(time.time()))

    inbox = request(
        args.base_url,
        args.api_key,
        "POST",
        "/api/v1/inbox/claims/normalize",
        {
            "systemCode": args.source_system,
            "transNo": f"MOCK-INBOX-{suffix}",
            "reportCase": {
                "reportNo": args.claim_id,
                "claimReceiveDate": 1779811200000,
                "calculateRisk": "Y",
                "policyList": [
                    {
                        "policyNo": "POL-MOCK",
                        "insuredName": "Mock Member",
                        "coverageLimit": 20000,
                        "invoiceList": [
                            {
                                "invoiceNo": f"INV-MOCK-{suffix}",
                                "feeAmount": 397.06,
                                "startDate": 1766678400000,
                                "hospitalName": "Mock Hospital",
                                "diagnosisList": [
                                    {
                                        "detailCode": "K05.300",
                                        "detailName": "慢性牙周炎",
                                    }
                                ],
                                "feeList": [
                                    {
                                        "feeCategory": "westernMedicineFee",
                                        "medicareAmount": 21.55,
                                        "feeDetailList": [
                                            {
                                                "name": "双氯芬酸二乙胺乳胶剂",
                                                "amount": 51.51,
                                            }
                                        ],
                                    }
                                ],
                            }
                        ],
                    }
                ],
            },
        },
    )
    require(inbox.get("idempotency_key"), "inbox normalize missing idempotency_key")

    score = request(
        args.base_url,
        args.api_key,
        "POST",
        "/api/v1/claims/score",
        {"source_system": args.source_system, "claim_id": args.claim_id},
    )
    audit_id = require(score.get("audit_id"), "score response missing audit_id")

    member = request(
        args.base_url,
        args.api_key,
        "GET",
        f"/api/v1/members/{args.member_id}/profile-summary",
    )
    require(member.get("member_id") == args.member_id, "member profile mismatch")

    similar = request(
        args.base_url,
        args.api_key,
        "POST",
        "/api/v1/knowledge/search-similar",
        {
            "claim_id": args.claim_id,
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim", "high_amount"],
        },
    )
    require(similar.get("results"), "similar case search returned no results")

    investigation = request(
        args.base_url,
        args.api_key,
        "POST",
        "/api/v1/investigations/results",
        {
            "claim_id": args.claim_id,
            "investigation_id": f"INV-MOCK-{suffix}",
            "outcome": "confirmed_fwa_review_needed",
            "confirmed_fwa": True,
            "financial_impact_type": "estimated_impact",
            "saving_amount": "8200.00",
            "currency": "CNY",
            "notes": "Mock TPA investigation writeback with evidence references.",
            "evidence_refs": [
                f"audit:{audit_id}",
                "rule_runs:EARLY_CLAIM",
                "knowledge_cases:KC-1001",
            ],
        },
    )
    require(investigation.get("idempotency_key"), "investigation missing idempotency_key")

    qa = request(
        args.base_url,
        args.api_key,
        "POST",
        "/api/v1/qa/results",
        {
            "qa_case_id": f"QA-MOCK-{suffix}",
            "claim_id": args.claim_id,
            "qa_conclusion": "issue_found_escalate",
            "issue_type": "alert_handling_incomplete",
            "feedback_target": "rules",
            "notes": "Mock TPA QA writeback with evidence references.",
            "evidence_refs": [f"audit:{audit_id}", "rule_runs:EARLY_CLAIM"],
        },
    )
    require(qa.get("idempotency_key"), "QA writeback missing idempotency_key")

    audit = request(
        args.base_url,
        args.api_key,
        "GET",
        f"/api/v1/audit/claims/{args.claim_id}",
    )
    event_types = [event.get("event_type") for event in audit.get("events", [])]
    for expected in [
        "scoring.completed",
        "investigation.result.received",
        "qa.result.received",
    ]:
        require(expected in event_types, f"audit history missing {expected}")

    print(
        json.dumps(
            {
                "claim_id": args.claim_id,
                "inbox_idempotency_key": inbox.get("idempotency_key"),
                "score": {
                    "run_id": score.get("run_id"),
                    "audit_id": audit_id,
                    "rag": score.get("rag"),
                    "risk_score": score.get("risk_score"),
                },
                "member_id": member.get("member_id"),
                "similar_case_count": len(similar.get("results", [])),
                "investigation_idempotency_key": investigation.get("idempotency_key"),
                "qa_idempotency_key": qa.get("idempotency_key"),
                "audit_event_types": event_types,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
