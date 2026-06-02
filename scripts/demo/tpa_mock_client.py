#!/usr/bin/env python3
import argparse
import json
import sys
import time
import urllib.error
import urllib.request


def request(base_url, api_key, method, path, payload=None, allow_http_error=False):
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
        if allow_http_error:
            try:
                return json.loads(raw) if raw else {"code": f"HTTP_{error.code}"}
            except json.JSONDecodeError:
                return {"code": f"HTTP_{error.code}", "message": raw}
        raise RuntimeError(f"{method} {path} returned HTTP {error.code}: {raw}") from error


def require(value, message):
    if not value:
        raise AssertionError(message)
    return value


def load_json_file(path):
    with open(path, encoding="utf-8") as file:
        return json.load(file)


def merge_json_overlay(base, overlay):
    if isinstance(base, dict) and isinstance(overlay, dict):
        merged = dict(base)
        for key, value in overlay.items():
            merged[key] = merge_json_overlay(merged.get(key), value)
        return merged
    if isinstance(base, list) and isinstance(overlay, list):
        merged = list(base)
        for index, value in enumerate(overlay):
            if index < len(merged):
                merged[index] = merge_json_overlay(merged[index], value)
            else:
                merged.append(value)
        return merged
    return overlay


def generated_inbox_payload(source_system, claim_id, suffix):
    return {
        "systemCode": source_system,
        "transNo": f"MOCK-INBOX-{suffix}",
        "reportCase": {
            "reportNo": claim_id,
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
    }


def is_direct_scoring_blocker(field_path, severity):
    if severity == "error":
        return True
    if severity != "warning":
        return False
    if not field_path.startswith("reportCase.policyList["):
        return False
    if ".invoiceList[" in field_path:
        return False
    if ".productList[" in field_path:
        return field_path.rsplit(".", 1)[-1] in {
            "validateDate",
            "claimValidateDate",
            "expireDate",
        }
    return field_path.rsplit(".", 1)[-1] in {
        "coverageLimit",
        "validateDate",
        "expireDate",
    }


def next_action_for_validation_error(error):
    field_path = error.get("field_path", "")
    remediation = error.get("remediation", "")
    if field_path == "systemCode":
        return "use an API key/source-system config that matches the payload systemCode"
    if field_path.endswith(".coverageLimit"):
        return "map the policy or liability coverage limit before direct scoring"
    if field_path.endswith((".validateDate", ".expireDate", ".claimValidateDate")):
        return "fix or reviewer-resolve the policy/product/liability date window before scoring"
    if field_path == "reportCase.calculateRisk":
        return "keep the payload in the FWA audit path unless customer config explicitly allows bypass"
    return remediation or "review this field before scoring"


def add_correction_hints(normalize_response):
    if normalize_response.get("scoring_ready") is True:
        return normalize_response

    hints = []
    for error in normalize_response.get("validation_errors", []):
        field_path = error.get("field_path", "")
        severity = error.get("severity", "")
        hints.append(
            {
                "field_path": field_path,
                "severity": severity,
                "blocks_scoring": is_direct_scoring_blocker(field_path, severity),
                "next_action": next_action_for_validation_error(error),
            }
        )

    if normalize_response.get("code") == "SOURCE_SYSTEM_MISMATCH" and not hints:
        hints.append(
            {
                "field_path": "systemCode",
                "severity": "error",
                "blocks_scoring": True,
                "next_action": "use an API key/source-system config that matches the payload systemCode",
            }
        )

    if hints:
        normalize_response = dict(normalize_response)
        normalize_response["correction_hints"] = hints
    return normalize_response


def main():
    parser = argparse.ArgumentParser(description="Run the pilot TPA integration flow.")
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--api-key", default="dev-secret")
    parser.add_argument("--source-system")
    parser.add_argument("--claim-id", default="CLM-0287")
    parser.add_argument("--member-id", default="MBR-0287")
    parser.add_argument(
        "--inbox-payload-file",
        help="Path to a raw TPA inbox JSON payload to normalize before scoring.",
    )
    parser.add_argument(
        "--inbox-correction-file",
        help="Path to a local JSON overlay merged into the inbox payload before normalization.",
    )
    parser.add_argument(
        "--normalize-only",
        action="store_true",
        help="Only call /api/v1/inbox/claims/normalize and print the normalization response.",
    )
    args = parser.parse_args()

    suffix = str(int(time.time()))
    raw_inbox_payload = (
        load_json_file(args.inbox_payload_file)
        if args.inbox_payload_file
        else generated_inbox_payload(args.source_system or "tpa-demo", args.claim_id, suffix)
    )
    if args.inbox_correction_file:
        raw_inbox_payload = merge_json_overlay(
            raw_inbox_payload,
            load_json_file(args.inbox_correction_file),
        )
    source_system = args.source_system or raw_inbox_payload.get("systemCode") or "tpa-demo"

    inbox = request(
        args.base_url,
        args.api_key,
        "POST",
        "/api/v1/inbox/claims/normalize",
        raw_inbox_payload,
        allow_http_error=args.normalize_only,
    )
    if args.normalize_only:
        inbox = add_correction_hints(inbox)
        print(json.dumps(inbox, ensure_ascii=False, indent=2, sort_keys=True))
        return 0 if inbox.get("scoring_ready") is True else 2

    require(inbox.get("idempotency_key"), "inbox normalize missing idempotency_key")
    require(inbox.get("scoring_ready") is True, f"inbox context not scoring ready: {inbox}")
    canonical_claim_context = require(
        inbox.get("canonical_claim_context"),
        "inbox normalize missing canonical_claim_context",
    )
    effective_claim_id = canonical_claim_context.get("claim_header", {}).get(
        "external_claim_id",
        args.claim_id,
    )

    score = request(
        args.base_url,
        args.api_key,
        "POST",
        "/api/v1/claims/score",
        {
            "source_system": source_system,
            "canonical_claim_context": canonical_claim_context,
        },
    )
    require(score.get("claim_id") == effective_claim_id, "canonical score claim_id mismatch")
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
            "claim_id": effective_claim_id,
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
            "claim_id": effective_claim_id,
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
            "claim_id": effective_claim_id,
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
        f"/api/v1/audit/claims/{effective_claim_id}",
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
                "claim_id": effective_claim_id,
                "inbox_idempotency_key": inbox.get("idempotency_key"),
                "inbox_run_id": inbox.get("run_id"),
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
