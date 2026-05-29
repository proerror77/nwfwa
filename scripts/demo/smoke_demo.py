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
MODEL_KEY = os.environ.get("FWA_DEMO_MODEL_KEY", "baseline_fwa")
RULE_ID = os.environ.get("FWA_DEMO_RULE_ID", "rule_early_claim")
CANDIDATE_RULE_ID = "candidate_early_high_amount"


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
        except urllib.error.HTTPError as error:
            last_error = f"HTTP {error.code}: {error.read().decode('utf-8', errors='replace')}"
            time.sleep(1)
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


def latest_completed_retraining_job():
    jobs = request("GET", f"/api/v1/ops/models/{MODEL_KEY}/retraining-jobs").get("jobs", [])
    completed_jobs = [
        job
        for job in jobs
        if job.get("status") == "completed"
        and job.get("candidate_model_version")
        and job.get("output_evaluation_id")
    ]
    assert_true(completed_jobs, "worker did not complete a retraining job")
    return completed_jobs[0]


def govern_retraining_candidate():
    completed_job = latest_completed_retraining_job()
    candidate_version = completed_job["candidate_model_version"]
    output_evaluation_id = completed_job["output_evaluation_id"]
    job_id = completed_job["job_id"]

    gates = request("GET", f"/api/v1/ops/models/{MODEL_KEY}/promotion-gates")
    assert_true(gates.get("model_version") == candidate_version, "promotion gates did not target latest candidate")
    assert_true(
        "approval missing" in gates.get("blockers", []),
        f"candidate gates should require approval before activation: {gates}",
    )
    active_gate = next(
        gate for gate in gates.get("gates", []) if gate.get("label") == "Active version"
    )
    assert_true(active_gate.get("passed") is False, "candidate should not be active before activation")

    review = request(
        "POST",
        f"/api/v1/ops/models/{MODEL_KEY}/promotion-reviews",
        {
            "decision": "approved",
            "reviewer": "model-governance-demo",
            "notes": "Demo smoke approves the retrained candidate after governance gates.",
            "evidence_refs": [
                f"model_versions:{MODEL_KEY}:{candidate_version}",
                f"model_retraining_jobs:{job_id}",
                f"model_evaluations:{output_evaluation_id}",
            ],
        },
    )
    assert_true(review.get("decision") == "approved", "promotion review was not approved")
    assert_true(review.get("model_version") == candidate_version, "promotion review target mismatch")

    gates_after_review = request("GET", f"/api/v1/ops/models/{MODEL_KEY}/promotion-gates")
    assert_true(
        gates_after_review.get("blockers") == ["model is not active"],
        f"candidate should only be blocked by activation status after approval: {gates_after_review}",
    )

    activated = request(
        "POST",
        f"/api/v1/ops/models/{MODEL_KEY}/activate",
        {
            "evidence_refs": [
                f"model_versions:{MODEL_KEY}:{candidate_version}",
                f"model_promotion_reviews:{MODEL_KEY}:{candidate_version}",
                f"model_retraining_jobs:{job_id}",
            ],
        },
    )
    assert_true(activated.get("status") == "active", "candidate model was not activated")
    assert_true(activated.get("version") == candidate_version, "activated model version mismatch")

    models = request("GET", "/api/v1/ops/models").get("models", [])
    assert_true(
        any(
            model.get("model_key") == MODEL_KEY
            and model.get("version") == candidate_version
            and model.get("status") == "active"
            for model in models
        ),
        "approved retraining candidate did not become active",
    )
    assert_true(
        any(
            model.get("model_key") == MODEL_KEY
            and model.get("version") == completed_job["model_version"]
            and model.get("status") == "approved"
            for model in models
        ),
        "previous active model was not moved to approved",
    )

    activated_gates = request("GET", f"/api/v1/ops/models/{MODEL_KEY}/promotion-gates")
    assert_true(
        activated_gates.get("decision") == "routing_allowed",
        f"activated candidate should pass routing gates: {activated_gates}",
    )

    print(
        json.dumps(
            {
                "status": "ok",
                "model_key": MODEL_KEY,
                "completed_retraining_job": job_id,
                "activated_version": candidate_version,
            },
            ensure_ascii=True,
        )
    )


def demo_rule_backtest_payload():
    return {
        "rule": {
            "rule_id": RULE_ID,
            "version": 1,
            "name": "Early claim after policy start",
            "conditions": [
                {
                    "field": "days_since_policy_start",
                    "operator": "<=",
                    "value": 7,
                }
            ],
            "action": {
                "score": 25,
                "alert_code": "EARLY_CLAIM",
                "recommended_action": "ManualReview",
                "reason": "Policy-start early claim requires manual review.",
            },
        },
        "samples": [
            {
                "external_claim_id": "CLM-RULE-BT-TP-1",
                "claim_amount": "8000",
                "currency": "CNY",
                "service_date": "2026-01-06",
                "confirmed_fwa": True,
                "policy": {
                    "external_policy_id": "POL-RULE-BT-TP-1",
                    "coverage_start_date": "2026-01-01",
                    "coverage_end_date": "2026-12-31",
                    "coverage_limit": "10000",
                },
            },
            {
                "external_claim_id": "CLM-RULE-BT-TP-2",
                "claim_amount": "7000",
                "currency": "CNY",
                "service_date": "2026-01-07",
                "confirmed_fwa": True,
                "policy": {
                    "external_policy_id": "POL-RULE-BT-TP-2",
                    "coverage_start_date": "2026-01-01",
                    "coverage_end_date": "2026-12-31",
                    "coverage_limit": "10000",
                },
            },
            {
                "external_claim_id": "CLM-RULE-BT-TN",
                "claim_amount": "500",
                "currency": "CNY",
                "service_date": "2026-03-01",
                "confirmed_fwa": False,
                "policy": {
                    "external_policy_id": "POL-RULE-BT-TN",
                    "coverage_start_date": "2026-01-01",
                    "coverage_end_date": "2026-12-31",
                    "coverage_limit": "10000",
                },
            },
        ],
        "expected_review_capacity": 5,
    }


def demo_rule_discovery_samples():
    return [
        {
            "external_claim_id": "CLM-RULE-DISC-TP",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-05",
            "confirmed_fwa": True,
            "policy": {
                "external_policy_id": "POL-RULE-DISC-TP",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
            },
        },
        {
            "external_claim_id": "CLM-RULE-DISC-TN-LOW",
            "claim_amount": "500",
            "currency": "CNY",
            "service_date": "2026-03-01",
            "confirmed_fwa": False,
            "policy": {
                "external_policy_id": "POL-RULE-DISC-TN-LOW",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
            },
        },
        {
            "external_claim_id": "CLM-RULE-DISC-TN-HIGH",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-03-01",
            "confirmed_fwa": False,
            "policy": {
                "external_policy_id": "POL-RULE-DISC-TN-HIGH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000",
            },
        },
    ]


def run_rule_discovery_candidate_lifecycle():
    samples = demo_rule_discovery_samples()
    discovery = request(
        "POST",
        "/api/v1/ops/rules/discover",
        {"min_support": 1, "samples": samples},
    )
    assert_true(discovery.get("sample_count") == len(samples), "rule discovery sample count mismatch")
    assert_true(discovery.get("positive_count") == 1, "rule discovery positive count mismatch")
    candidates = discovery.get("candidates", [])
    assert_true(candidates, "rule discovery returned no candidates")
    candidate = candidates[0]
    assert_true(
        candidate.get("rule", {}).get("rule_id") == CANDIDATE_RULE_ID,
        f"unexpected top discovered rule candidate: {candidate}",
    )
    assert_true(candidate.get("precision", 0) >= 0.70, "discovered rule precision below threshold")
    assert_true(candidate.get("lift", 0) > 1.0, "discovered rule should enrich FWA labels")
    assert_true(candidate.get("estimated_saving") != "0.00", "discovered rule missing saving estimate")
    assert_true(candidate.get("explanation"), "discovered rule missing explanation")

    saved = request(
        "POST",
        "/api/v1/ops/rules/candidates",
        {
            "owner": "rule-discovery-demo",
            "rule": candidate["rule"],
        },
    )
    assert_true(
        saved.get("summary", {}).get("rule_id") == CANDIDATE_RULE_ID,
        "saved candidate rule id mismatch",
    )
    assert_true(saved.get("summary", {}).get("status") == "draft", "saved candidate should stay draft")
    assert_true(saved.get("summary", {}).get("owner") == "rule-discovery-demo", "saved candidate owner mismatch")

    backtest = request(
        "POST",
        "/api/v1/ops/rules/backtest",
        {
            "rule": candidate["rule"],
            "samples": samples,
            "expected_review_capacity": 3,
        },
    )
    assert_true(
        backtest.get("promotion_recommendation") == "eligible_for_review",
        f"discovered candidate backtest should be eligible: {backtest}",
    )

    gates = request("GET", f"/api/v1/ops/rules/{CANDIDATE_RULE_ID}/promotion-gates")
    assert_true(
        "backtest evidence missing" not in gates.get("blockers", []),
        f"candidate promotion gates did not consume discovery backtest evidence: {gates}",
    )
    assert_true(
        "approval missing" in gates.get("blockers", []),
        "candidate should still require approval before routing",
    )
    return {
        "rule_id": CANDIDATE_RULE_ID,
        "support": candidate["support"],
        "precision": candidate["precision"],
        "lift": candidate["lift"],
    }


def run_rule_backtest_and_publish(score, investigation):
    backtest = request("POST", "/api/v1/ops/rules/backtest", demo_rule_backtest_payload())
    assert_true(
        backtest.get("promotion_recommendation") == "eligible_for_review",
        f"rule backtest should be eligible for review: {backtest}",
    )
    assert_true(backtest.get("precision", 0) >= 0.70, "rule backtest precision below threshold")
    assert_true(backtest.get("recall", 0) >= 0.60, "rule backtest recall below threshold")
    assert_true(backtest.get("lift", 0) > 1.0, "rule backtest lift should show enrichment")
    assert_true(backtest.get("estimated_saving") != "0.00", "rule backtest missing saving estimate")

    gates = request("GET", f"/api/v1/ops/rules/{RULE_ID}/promotion-gates")
    assert_true(
        "backtest evidence missing" not in gates.get("blockers", []),
        f"rule promotion gates did not consume backtest evidence: {gates}",
    )
    assert_true(
        "shadow rollout missing" not in gates.get("blockers", []),
        f"rule promotion gates missing runtime rollout evidence: {gates}",
    )

    review = request(
        "POST",
        f"/api/v1/ops/rules/{RULE_ID}/promotion-reviews",
        {
            "decision": "approved",
            "reviewer": "rule-governance-demo",
            "notes": "Demo smoke approves rule release after deterministic backtest.",
            "evidence_refs": [
                f"rules:{RULE_ID}:v1",
                f"audit:{score['audit_id']}",
                f"audit:{investigation['audit_id']}",
            ],
        },
    )
    assert_true(review.get("decision") == "approved", "rule promotion review was not approved")

    lifecycle_payload = {
        "evidence_refs": [
            f"rules:{RULE_ID}:v1",
            f"rule_backtest_runs:{RULE_ID}:v1",
            f"rule_promotion_reviews:{RULE_ID}:v1",
        ]
    }
    for path, expected_status in [
        (f"/api/v1/ops/rules/{RULE_ID}/submit", "submitted"),
        (f"/api/v1/ops/rules/{RULE_ID}/approve", "approved"),
        (f"/api/v1/ops/rules/{RULE_ID}/publish", "active"),
    ]:
        result = request("POST", path, lifecycle_payload)
        assert_true(
            result.get("status") == expected_status,
            f"rule lifecycle {path} did not reach {expected_status}",
        )

    published = request("GET", f"/api/v1/ops/rules/{RULE_ID}")
    assert_true(published.get("summary", {}).get("status") == "active", "rule was not published")
    assert_true(
        any(
            event.get("event_type") == "rule.status.changed"
            and event.get("payload", {}).get("to_status") == "active"
            for event in published.get("audit_events", [])
        ),
        "published rule missing lifecycle audit event",
    )
    return {
        "rule_id": RULE_ID,
        "matched_count": backtest["matched_count"],
        "precision": backtest["precision"],
        "lift": backtest["lift"],
    }


def main():
    health = request("GET", "/api/v1/health", retries=180)
    assert_true(health.get("status") == "ok", "health endpoint did not return ok")
    assert_true(health.get("service") == "api-server", "health endpoint missing service metadata")
    assert_true(health.get("version"), "health endpoint missing version metadata")
    health_checks = health.get("checks", [])
    assert_true(
        any(check.get("name") == "http_router" and check.get("status") == "ok" for check in health_checks),
        "health endpoint missing http_router check",
    )

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

    leads = request("GET", "/api/v1/ops/leads").get("leads", [])
    lead = next(
        (
            item
            for item in leads
            if item.get("claim_id") == CLAIM_ID and item.get("run_id") == score["run_id"]
        ),
        None,
    )
    assert_true(lead is not None, "high-risk scoring did not generate a lead")
    assert_true(lead.get("lead_source") == "scoring_run", "lead source should be scoring_run")
    assert_true(lead.get("status") == "new", "new lead should be pending triage")
    assert_true(
        lead.get("disposition") == "pending_triage",
        "new lead should have pending_triage disposition",
    )
    assert_true(lead.get("scheme_family"), "lead missing scheme family")
    assert_true(lead.get("evidence_refs"), "lead missing evidence_refs")
    lead_id = lead["lead_id"]

    triage = request(
        "POST",
        f"/api/v1/ops/leads/{lead_id}/triage",
        {
            "decision": "open_case",
            "assignee": "siu-reviewer-demo",
            "reviewer": "medical-reviewer-demo",
            "priority": "high",
            "notes": "Demo smoke opens a governed investigation case from the scored FWA lead.",
        },
    )
    assert_true(triage.get("audit_id"), "lead triage missing audit_id")
    case = triage.get("case") or {}
    assert_true(case.get("lead_id") == lead_id, "triage did not open a case for the lead")
    assert_true(case.get("claim_id") == CLAIM_ID, "case claim_id mismatch")
    assert_true(case.get("status") == "triage", "new case should start in triage")
    assert_true(case.get("scheme_family") == lead["scheme_family"], "case scheme family mismatch")
    assert_true(
        case.get("evidence_package", {})
        .get("evidence_sufficiency", {})
        .get("minimum_evidence"),
        "case missing evidence sufficiency package",
    )
    case_id = case["case_id"]

    case_status = request(
        "POST",
        f"/api/v1/ops/cases/{case_id}/status",
        {
            "status": "investigating",
            "actor_id": "siu-reviewer-demo",
            "notes": "Demo smoke investigation started from triaged FWA lead.",
            "evidence_refs": [f"leads:{lead_id}", f"audit:{triage['audit_id']}"],
        },
    )
    assert_true(case_status.get("audit_id"), "case status update missing audit_id")
    assert_true(
        case_status.get("case", {}).get("status") == "investigating",
        "case status did not update to investigating",
    )

    cases = request("GET", "/api/v1/ops/cases").get("cases", [])
    assert_true(
        any(
            item.get("case_id") == case_id and item.get("status") == "investigating"
            for item in cases
        ),
        "case list missing investigating case",
    )

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

    discovered_rule = run_rule_discovery_candidate_lifecycle()
    rule_release = run_rule_backtest_and_publish(score, investigation)

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
    assert_true("lead.triaged" in event_types, "audit history missing lead.triaged")
    assert_true(
        "case.status.updated" in event_types,
        "audit history missing case.status.updated",
    )
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
    case_sla = dashboard.get("case_sla", {})
    assert_true(case_sla.get("total_cases", 0) >= 1, "dashboard missing investigation cases")
    assert_true(case_sla.get("open_cases", 0) >= 1, "dashboard missing open case SLA rollup")

    readiness = request("GET", f"/api/v1/ops/models/{MODEL_KEY}/retraining-readiness")
    assert_true(
        readiness.get("recommendation") == "prepare_retraining",
        f"baseline model should be ready for governed retraining: {readiness}",
    )
    assert_true(
        readiness.get("approved_label_count", 0) >= 1,
        "retraining readiness missing approved model labels",
    )
    assert_true(
        readiness.get("blockers") == [],
        "retraining readiness should not have blockers",
    )

    retraining_job = request(
        "POST",
        f"/api/v1/ops/models/{MODEL_KEY}/retraining-jobs",
        {
            "requested_by": "model-ops-demo",
            "notes": "Demo smoke queues retraining from readiness triggers.",
        },
    )
    assert_true(retraining_job.get("status") == "queued", "retraining job was not queued")
    assert_true(retraining_job.get("job_id"), "retraining job missing job_id")
    assert_true(
        retraining_job.get("readiness_recommendation") == "prepare_retraining",
        "retraining job missing readiness recommendation",
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
                "case_id": case_id,
                "outcome_labels": label_pool["total_labels"],
                "retraining_job_id": retraining_job["job_id"],
                "discovered_rule": discovered_rule,
                "rule_release": rule_release,
            },
            ensure_ascii=True,
        )
    )


if __name__ == "__main__":
    try:
        if len(sys.argv) == 2 and sys.argv[1] == "--govern-retraining-candidate":
            govern_retraining_candidate()
        elif len(sys.argv) == 1:
            main()
        else:
            raise RuntimeError("usage: smoke_demo.py [--govern-retraining-candidate]")
    except Exception as error:
        print(f"demo smoke failed: {error}", file=sys.stderr)
        sys.exit(1)
