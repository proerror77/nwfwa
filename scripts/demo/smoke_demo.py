#!/usr/bin/env python3
import json
import os
import sys
import time
import urllib.error
import urllib.request


BASE_URL = os.environ.get("FWA_API_BASE_URL", "http://127.0.0.1:8080").rstrip("/")
API_KEY = os.environ.get("FWA_API_KEY", "dev-secret")
SOURCE_SYSTEM = os.environ.get("FWA_SOURCE_SYSTEM", "tpa-demo")
CLAIM_ID = os.environ.get("FWA_DEMO_CLAIM_ID", "CLM-0287")


def request(method, path, payload=None, retries=1):
    body = None
    headers = {"x-api-key": API_KEY}
    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        headers["content-type"] = "application/json"
    req = urllib.request.Request(
        f"{BASE_URL}{path}",
        data=body,
        headers=headers,
        method=method,
    )
    last_error = None
    for _ in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=5) as response:
                raw = response.read()
                return json.loads(raw.decode("utf-8")) if raw else {}
        except (urllib.error.URLError, TimeoutError) as error:
            last_error = error
            time.sleep(1)
    raise RuntimeError(f"{method} {path} failed: {last_error}")


def assert_true(condition, message):
    if not condition:
        raise AssertionError(message)


def agent_rag(score_rag):
    return {
        "Green": "GREEN",
        "Amber": "AMBER",
        "Red": "RED",
    }.get(score_rag, str(score_rag).upper())


def has_label(labels, **expected):
    return any(
        all(label.get(field) == value for field, value in expected.items())
        and label.get("evidence_refs")
        for label in labels
    )


def main():
    health = request("GET", "/api/v1/health", retries=180)
    assert_true(health.get("status") == "ok", "health endpoint did not return ok")

    score = request(
        "POST",
        "/api/v1/claims/score",
        {"source_system": SOURCE_SYSTEM, "claim_id": CLAIM_ID},
    )
    assert_true(score.get("claim_id") == CLAIM_ID, "score response claim_id mismatch")
    assert_true(score.get("run_id"), "score response missing run_id")
    assert_true(score.get("audit_id"), "score response missing audit_id")
    assert_true(isinstance(score.get("risk_score"), int), "score response missing risk_score")
    assert_true(len(score.get("layers", [])) == 7, "score response must include 7 layers")
    assert_true(score.get("top_reasons"), "score response missing top_reasons")
    assert_true(score.get("evidence_refs"), "score response missing evidence_refs")

    medical_queue = request("GET", "/api/v1/ops/medical-review/queue?limit=20")
    medical_items = medical_queue.get("items", [])
    medical_item = next(
        (
            item
            for item in medical_items
            if item.get("claim_id") == CLAIM_ID and item.get("audit_id") == score["audit_id"]
        ),
        None,
    )
    assert_true(medical_item is not None, "medical review queue missing scored claim")
    assert_true(
        medical_item.get("review_route") == "medical_review",
        "medical review queue item missing medical_review route",
    )
    assert_true(
        medical_item.get("evidence_refs"),
        "medical review queue item missing evidence_refs",
    )

    medical_review = request(
        "POST",
        "/api/v1/ops/medical-review/results",
        {
            "claim_id": CLAIM_ID,
            "scoring_audit_id": score["audit_id"],
            "reviewer": "medical-reviewer-demo",
            "decision": "request_more_evidence",
            "notes": "Demo smoke medical review requests supporting medical record evidence.",
            "evidence_refs": [f"audit:{score['audit_id']}", "claim_items:PROC-001"],
        },
    )
    assert_true(
        medical_review.get("event_type") == "medical.review.recorded",
        "medical review writeback was not audited",
    )
    assert_true(
        medical_review.get("review_status") == "pending_evidence",
        "medical review status should request evidence",
    )

    similar = request(
        "POST",
        "/api/v1/knowledge/search-similar",
        {
            "claim_id": CLAIM_ID,
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["early_claim", "high_amount"],
        },
    )
    results = similar.get("results", [])
    assert_true(results, "similar-case search returned no results")
    assert_true(results[0].get("case_id") == "KC-1001", "expected KC-1001 as top similar case")
    assert_true(results[0].get("evidence_refs"), "similar case missing evidence_refs")

    agent = request(
        "POST",
        "/api/v1/agent/cases/investigate",
        {
            "claim_id": CLAIM_ID,
            "risk_score": score["risk_score"],
            "rag": agent_rag(score["rag"]),
            "scheme_family": "diagnosis_procedure_mismatch",
            "top_reasons": score["top_reasons"],
            "similar_case_query": {
                "diagnosis_code": "J10",
                "provider_region": "Shanghai",
                "tags": ["early_claim", "high_amount"],
            },
        },
    )
    assert_true(agent.get("decision_boundary") == "assistive_only", "agent must be assistive only")
    assert_true(agent.get("agent_run_id", "").startswith("agent_"), "agent response missing run id")
    assert_true(agent.get("findings"), "agent response missing findings")
    assert_true(agent.get("investigation_checklist"), "agent response missing checklist")
    assert_true(agent.get("evidence_refs"), "agent response missing evidence refs")
    assert_true(
        agent.get("similar_cases", [{}])[0].get("case_id") == "KC-1001",
        "agent response missing expected similar case",
    )

    investigation = request(
        "POST",
        "/api/v1/investigations/results",
        {
            "claim_id": CLAIM_ID,
            "investigation_id": "INV-DEMO-SMOKE",
            "outcome": "confirmed_fwa_review_needed",
            "confirmed_fwa": True,
            "financial_impact_type": "estimated_impact",
            "saving_amount": "8200.00",
            "currency": "CNY",
            "notes": "Demo smoke investigation records evidence-backed manual review outcome.",
            "evidence_refs": [
                f"agent_run:{agent['agent_run_id']}",
                f"audit:{score['audit_id']}",
                "knowledge_cases:KC-1001",
            ],
        },
    )
    assert_true(
        investigation.get("event_type") == "investigation.result.received",
        "investigation writeback was not audited",
    )
    assert_true(investigation.get("audit_id"), "investigation writeback missing audit_id")

    qa = request(
        "POST",
        "/api/v1/qa/results",
        {
            "qa_case_id": "QA-DEMO-SMOKE",
            "claim_id": CLAIM_ID,
            "qa_conclusion": "issue_found_escalate",
            "issue_type": "alert_handling_incomplete",
            "feedback_target": "rules",
            "notes": "Demo smoke review confirms alert handling evidence was reviewed.",
            "evidence_refs": [f"audit:{score['audit_id']}", "rule_runs:EARLY_CLAIM"],
        },
    )
    assert_true(qa.get("event_type") == "qa.result.received", "QA writeback was not audited")
    assert_true(qa.get("audit_id"), "QA writeback missing audit_id")

    audit = request("GET", f"/api/v1/audit/claims/{CLAIM_ID}")
    event_types = [event.get("event_type") for event in audit.get("events", [])]
    assert_true("scoring.completed" in event_types, "audit history missing scoring.completed")
    assert_true(
        "medical.review.recorded" in event_types,
        "audit history missing medical.review.recorded",
    )
    assert_true(
        "investigation.result.received" in event_types,
        "audit history missing investigation.result.received",
    )
    assert_true("qa.result.received" in event_types, "audit history missing qa.result.received")

    labels = request("GET", "/api/v1/ops/labels").get("labels", [])
    assert_true(
        any(label.get("claim_id") == CLAIM_ID for label in labels),
        "outcome labels missing scored claim",
    )
    assert_true(
        has_label(
            labels,
            claim_id=CLAIM_ID,
            source_type="medical_review",
            label_name="insufficient_evidence",
            label_value="true",
            governance_status="needs_review",
            feedback_target="workflow",
        ),
        "outcome labels missing evidence-backed medical review label",
    )
    assert_true(
        has_label(
            labels,
            claim_id=CLAIM_ID,
            source_type="investigation_result",
            label_name="confirmed_fwa",
            label_value="true",
            governance_status="approved_for_training",
            feedback_target="models",
        ),
        "outcome labels missing investigation confirmed_fwa label",
    )
    assert_true(
        has_label(
            labels,
            claim_id=CLAIM_ID,
            source_type="qa_review",
            label_name="alert_handling_incomplete",
            label_value="true",
            governance_status="needs_review",
            feedback_target="rules",
        ),
        "outcome labels missing QA feedback label",
    )

    dashboard = request("GET", "/api/v1/ops/dashboard/summary")
    assert_true(dashboard.get("suspected_claims", 0) >= 1, "dashboard missing suspected claims")
    assert_true(dashboard.get("investigation_results", 0) >= 1, "dashboard missing investigations")
    assert_true(dashboard.get("qa_reviews", 0) >= 1, "dashboard missing QA reviews")
    assert_true(dashboard.get("confirmed_fwa", 0) >= 1, "dashboard missing confirmed FWA")
    assert_true(dashboard.get("rule_hits", 0) >= 1, "dashboard missing rule hits")
    assert_true(
        dashboard.get("model_scores", {})
        .get("baseline_fwa", {})
        .get("scored_runs", 0)
        >= 1,
        "dashboard missing baseline_fwa model scores",
    )
    assert_true(
        dashboard.get("layer_scores", {})
        .get("L7_RISK_FUSION_ROUTING", {})
        .get("scored_runs", 0)
        >= 1,
        "dashboard missing L7 risk fusion scores",
    )
    label_pool = dashboard.get("label_pool", {})
    assert_true(label_pool.get("total_labels", 0) >= 1, "dashboard missing labels")
    assert_true(
        label_pool.get("medical_review_labels", 0) >= 1,
        "dashboard missing medical review labels",
    )
    assert_true(
        label_pool.get("evidence_backed_labels", 0) >= 1,
        "dashboard missing evidence-backed labels",
    )

    print(
        json.dumps(
            {
                "status": "ok",
                "claim_id": CLAIM_ID,
                "run_id": score["run_id"],
                "audit_id": score["audit_id"],
                "risk_score": score["risk_score"],
                "similar_case": results[0]["case_id"],
                "medical_review_audit_id": medical_review["audit_id"],
                "agent_run_id": agent["agent_run_id"],
                "investigation_audit_id": investigation["audit_id"],
                "outcome_labels": label_pool["total_labels"],
            },
            ensure_ascii=True,
        )
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"demo smoke failed: {error}", file=sys.stderr)
        sys.exit(1)
