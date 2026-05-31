#!/usr/bin/env python3
import json
import os
import sys
import time
import urllib.error
import urllib.request
from decimal import Decimal


BASE_URL = os.environ.get("FWA_API_BASE_URL", "http://127.0.0.1:8080").rstrip("/")
API_KEY = os.environ.get("FWA_API_KEY", "dev-secret")
SOURCE_SYSTEM = os.environ.get("FWA_SOURCE_SYSTEM", "tpa-demo")
CLAIM_ID = os.environ.get("FWA_DEMO_CLAIM_ID", "CLM-0287")
MODEL_KEY = os.environ.get("FWA_DEMO_MODEL_KEY", "baseline_fwa")
RULE_ID = os.environ.get("FWA_DEMO_RULE_ID", "rule_early_claim")
CANDIDATE_RULE_ID = "candidate_early_high_amount"
ROUTING_POLICY_PREFIX = "demo_strict_prepay"


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


def decimal_value(value):
    return Decimal(str(value))


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


def run_routing_policy_governance():
    policies = request("GET", "/api/v1/ops/routing-policies").get("policies", [])
    assert_true(policies, "routing policy list returned no policies")
    assert_true(
        any(
            policy.get("review_mode") in ("pre_payment", "both")
            and policy.get("status") == "active"
            for policy in policies
        ),
        "routing policy list missing active pre-payment policy",
    )

    policy_id = f"{ROUTING_POLICY_PREFIX}_{int(time.time())}"
    evidence_ref = f"routing_policies:{policy_id}:v1:pre_payment"
    candidate = {
        "policy_id": policy_id,
        "version": 1,
        "review_mode": "pre_payment",
        "risk_thresholds": {
            "low_max": 0,
            "medium_min": 1,
            "high_min": 1,
            "critical_min": 90,
        },
        "confidence_thresholds": {
            "low_confidence_below": 60,
            "high_confidence_min": 80,
        },
        "provider_review_threshold": 70,
    }
    saved = request(
        "POST",
        "/api/v1/ops/routing-policies",
        {
            "owner": "policy-ops-demo",
            "policy": candidate,
        },
    )
    assert_true(saved.get("policy_id") == policy_id, "saved routing policy id mismatch")
    assert_true(saved.get("status") == "draft", "saved routing policy should be draft")
    assert_true(saved.get("owner") == "policy-ops-demo", "saved routing policy owner mismatch")

    draft_gates = request(
        "GET",
        f"/api/v1/ops/routing-policies/{policy_id}/pre_payment/1/promotion-gates",
    )
    assert_true(draft_gates.get("decision") == "activation_blocked", "draft routing gates should block activation")
    assert_true("approval missing" in draft_gates.get("blockers", []), "draft routing gates should require approval")

    lifecycle_payload = {"evidence_refs": [evidence_ref]}
    for action, expected_status in [
        ("submit", "submitted"),
        ("approve", "approved"),
    ]:
        result = request(
            "POST",
            f"/api/v1/ops/routing-policies/{policy_id}/pre_payment/1/{action}",
            lifecycle_payload,
        )
        assert_true(
            result.get("status") == expected_status,
            f"routing policy {action} did not reach {expected_status}",
        )

    gates = request(
        "GET",
        f"/api/v1/ops/routing-policies/{policy_id}/pre_payment/1/promotion-gates",
    )
    assert_true(gates.get("decision") == "activation_allowed", f"routing gates should allow activation: {gates}")
    assert_true(gates.get("passed_count") == gates.get("total_count"), "routing gates did not all pass")

    activated = request(
        "POST",
        f"/api/v1/ops/routing-policies/{policy_id}/pre_payment/1/activate",
        lifecycle_payload,
    )
    assert_true(activated.get("status") == "active", "routing policy was not activated")
    assert_true(activated.get("activated_at"), "activated routing policy missing activated_at")

    routed_score = request(
        "POST",
        "/api/v1/claims/score",
        {
            "source_system": SOURCE_SYSTEM,
            "review_mode": "pre_payment",
            "claim": {
                "external_claim_id": f"CLM-ROUTING-{policy_id}",
                "claim_amount": "8000",
                "currency": "CNY",
            },
        },
    )
    assert_true(
        routed_score.get("routing_policy", {}).get("policy_id") == policy_id,
        "activated routing policy did not control pre-payment scoring",
    )
    assert_true(
        routed_score.get("routing_policy", {}).get("risk_thresholds", {}).get("high_min") == 1,
        "activated routing policy thresholds were not used",
    )
    assert_true(routed_score.get("audit_id"), "routing policy controlled score missing audit id")
    return {
        "policy_id": policy_id,
        "status": activated["status"],
        "gates_decision": gates["decision"],
        "scoring_run_id": routed_score["run_id"],
    }


def create_and_review_qa_audit_sample(score):
    sample = request(
        "POST",
        "/api/v1/ops/audit-samples",
        {
            "sample_mode": "qa_calibration",
            "population_definition": "Demo high-risk FWA claims sampled for QA calibration.",
            "inclusion_criteria": {
                "min_risk_score": 70,
                "review_mode": "pre_payment",
            },
            "deterministic_seed": f"demo-qa-calibration-{score['run_id']}",
            "sample_size": 1,
            "reviewer": "qa-sampling-demo",
            "assignment_queue": "QA Review",
        },
    )
    assert_true(sample.get("sample_id"), "audit sample missing sample_id")
    assert_true(sample.get("sample_mode") == "qa_calibration", "audit sample mode mismatch")
    assert_true(
        sample.get("selection_method") == "reviewer_consistency_rotation",
        "audit sample selection method mismatch",
    )
    assert_true(sample.get("reviewer") == "qa-sampling-demo", "audit sample reviewer mismatch")
    assert_true(sample.get("assignment_queue") == "QA Review", "audit sample queue mismatch")
    selected_leads = sample.get("selected_leads", [])
    assert_true(len(selected_leads) == 1, "audit sample should select exactly one lead")
    selected_lead = selected_leads[0]
    assert_true(selected_lead.get("claim_id"), "sampled lead missing claim_id")
    assert_true(selected_lead.get("risk_score", 0) >= 70, "sampled lead should be high risk")
    assert_true(selected_lead.get("evidence_refs"), "sampled lead missing evidence refs")
    assert_true(sample.get("outcome_distribution", {}).get("selected_count") == 1, "sample selected count mismatch")
    assert_true(sample.get("outcome_distribution", {}).get("open_count") == 1, "sample should start open")

    queue_items = request("GET", "/api/v1/ops/qa/queue").get("items", [])
    queue_item = next(
        (
            item
            for item in queue_items
            if item.get("sample_id") == sample["sample_id"]
            and item.get("lead_id") == selected_lead["lead_id"]
        ),
        None,
    )
    assert_true(queue_item is not None, "QA queue missing sampled audit item")
    assert_true(queue_item.get("status") == "open", "sampled QA queue item should start open")
    assert_true(queue_item.get("reviewer") == "qa-sampling-demo", "sampled QA queue reviewer mismatch")
    assert_true(queue_item.get("assignment_queue") == "QA Review", "sampled QA queue mismatch")
    assert_true(queue_item.get("risk_score") == selected_lead["risk_score"], "sampled QA queue risk mismatch")
    assert_true(queue_item.get("evidence_refs"), "sampled QA queue item missing evidence refs")

    qa_case_id = queue_item["qa_case_id"]
    qa_result = request(
        "POST",
        "/api/v1/qa/results",
        {
            "qa_case_id": qa_case_id,
            "claim_id": queue_item["claim_id"],
            "qa_conclusion": "pass",
            "issue_type": "qa_review_completed",
            "feedback_target": "workflow",
            "notes": "Demo smoke completed the sampled QA calibration review.",
            "evidence_refs": [
                f"qa_queue:{qa_case_id}",
                f"audit_samples:{sample['sample_id']}",
                f"leads:{selected_lead['lead_id']}",
            ],
        },
    )
    assert_true(qa_result.get("event_type") == "qa.result.received", "sampled QA result was not audited")
    assert_true(qa_result.get("audit_id"), "sampled QA result missing audit id")

    reviewed_queue_items = request("GET", "/api/v1/ops/qa/queue").get("items", [])
    reviewed_item = next(
        (
            item
            for item in reviewed_queue_items
            if item.get("qa_case_id") == qa_case_id
        ),
        None,
    )
    assert_true(reviewed_item is not None, "reviewed sampled QA queue item missing")
    assert_true(reviewed_item.get("status") == "reviewed", "sampled QA queue item was not reviewed")
    assert_true(reviewed_item.get("qa_conclusion") == "pass", "sampled QA queue conclusion mismatch")
    assert_true(reviewed_item.get("issue_type") == "qa_review_completed", "sampled QA queue issue mismatch")

    samples = request("GET", "/api/v1/ops/audit-samples").get("samples", [])
    listed_sample = next(
        (item for item in samples if item.get("sample_id") == sample["sample_id"]),
        None,
    )
    assert_true(listed_sample is not None, "audit sample list missing demo sample")
    distribution = listed_sample.get("outcome_distribution", {})
    assert_true(distribution.get("selected_count") == 1, "listed sample selected count mismatch")
    assert_true(distribution.get("reviewed_count") == 1, "listed sample reviewed count mismatch")
    assert_true(distribution.get("open_count") == 0, "listed sample should have no open cases")
    assert_true(
        distribution.get("qa_conclusions", {}).get("pass") == 1,
        "listed sample missing pass conclusion count",
    )
    assert_true(
        distribution.get("feedback_targets", {}).get("workflow") == 1,
        "listed sample missing workflow feedback count",
    )

    sample_audit = request(
        "GET",
        f"/api/v1/ops/audit-events?sample_id={sample['sample_id']}&limit=10",
    ).get("events", [])
    assert_true(
        any(event.get("event_type") == "audit_sample.created" for event in sample_audit),
        "audit sample creation was not present in governance audit log",
    )
    return {
        "sample_id": sample["sample_id"],
        "qa_case_id": qa_case_id,
        "claim_id": queue_item["claim_id"],
        "reviewed_count": distribution["reviewed_count"],
        "audit_id": qa_result["audit_id"],
    }


def resolve_demo_qa_feedback(qa):
    feedback_id = "qa_feedback_QA-DEMO-SMOKE"
    update = request(
        "POST",
        f"/api/v1/ops/qa/feedback-items/{feedback_id}/status",
        {
            "status": "resolved",
            "actor_id": "rule-ops-demo",
            "notes": "Demo smoke confirms rule operations reviewed and closed the QA feedback.",
            "evidence_refs": [
                f"qa_feedback:{feedback_id}",
                f"audit:{qa['audit_id']}",
                f"rules:{RULE_ID}:v1",
            ],
        },
    )
    assert_true(update.get("audit_id"), "QA feedback status update missing audit_id")
    item = update.get("item", {})
    assert_true(item.get("feedback_id") == feedback_id, "QA feedback status update id mismatch")
    assert_true(item.get("status") == "resolved", "QA feedback item was not resolved")
    assert_true(item.get("status_updated_by") == "rule-ops-demo", "QA feedback status actor mismatch")
    assert_true(item.get("status_audit_id") == update["audit_id"], "QA feedback status audit mismatch")
    assert_true(item.get("status_evidence_refs"), "QA feedback status update missing evidence refs")

    resolved = request(
        "GET",
        "/api/v1/ops/qa/feedback-items?feedback_target=rules&status=resolved",
    ).get("items", [])
    assert_true(
        any(entry.get("feedback_id") == feedback_id for entry in resolved),
        "resolved QA feedback list missing demo feedback item",
    )
    return {
        "feedback_id": feedback_id,
        "status": item["status"],
        "audit_id": update["audit_id"],
    }


def publish_demo_knowledge_case(investigation, qa):
    knowledge = request(
        "POST",
        "/api/v1/ops/knowledge/cases",
        {
            "case_id": "KC-DEMO-SMOKE",
            "title": "Demo confirmed early high amount claim",
            "fwa_type": "Abuse",
            "scheme_family": "diagnosis_procedure_mismatch",
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "provider_type": "hospital",
            "summary": "Confirmed early high amount respiratory claim with weak procedure support.",
            "outcome": "Confirmed FWA review outcome added to the governed knowledge base.",
            "tags": ["demo_confirmed_fwa", "early_claim", "high_amount"],
            "evidence_refs": [
                "investigation_results:INV-DEMO-SMOKE",
                "qa_reviews:QA-DEMO-SMOKE",
                f"audit:{investigation['audit_id']}",
                f"audit:{qa['audit_id']}",
            ],
            "source_claim_id": CLAIM_ID,
        },
    )
    assert_true(knowledge.get("audit_id"), "knowledge publish missing audit_id")
    case = knowledge.get("case", {})
    assert_true(case.get("case_id") == "KC-DEMO-SMOKE", "published knowledge case id mismatch")
    assert_true(case.get("evidence_refs"), "published knowledge case missing evidence refs")

    similar = request(
        "POST",
        "/api/v1/knowledge/search-similar",
        {
            "claim_id": CLAIM_ID,
            "diagnosis_code": "J10",
            "provider_region": "Shanghai",
            "tags": ["demo_confirmed_fwa"],
        },
    )
    results = similar.get("results", [])
    assert_true(results, "published knowledge case search returned no results")
    assert_true(
        results[0].get("case_id") == "KC-DEMO-SMOKE",
        f"published knowledge case was not top similar result: {results[:2]}",
    )
    assert_true(results[0].get("evidence_refs"), "published similar case missing evidence refs")
    assert_true(
        "knowledge_cases:KC-DEMO-SMOKE" in results[0].get("provenance_refs", []),
        "published similar case missing provenance ref",
    )
    return {
        "case_id": case["case_id"],
        "audit_id": knowledge["audit_id"],
        "similarity_score": results[0]["similarity_score"],
    }


def assert_factor_factory_readiness():
    readiness = request("GET", "/api/v1/ops/factors/readiness")
    assert_true(readiness.get("dataset_count", 0) >= 1, "factor readiness missing datasets")
    assert_true(readiness.get("factor_count", 0) >= 8, "factor readiness missing seeded factor cards")
    assert_true(
        readiness.get("online_ready_count", 0) >= 4,
        "factor readiness missing online-ready factors",
    )
    assert_true(
        readiness.get("rule_convertible_count", 0) >= 4,
        "factor readiness missing rule-convertible factors",
    )
    assert_true(
        readiness.get("ready_factor_count", 0) >= 4,
        "factor readiness missing ready factor cards",
    )
    cards = readiness.get("factor_cards", [])
    amount_ratio = next(
        (
            card
            for card in cards
            if card.get("factor_name") == "claim_amount_to_limit_ratio"
        ),
        None,
    )
    assert_true(amount_ratio is not None, "factor card missing claim_amount_to_limit_ratio")
    assert_true(amount_ratio.get("readiness_status") == "ready", "amount ratio factor not ready")
    assert_true(amount_ratio.get("owner") == "feature-ops", "amount ratio factor owner mismatch")
    assert_true(amount_ratio.get("online_available") is True, "amount ratio factor is not online available")
    assert_true(amount_ratio.get("rule_convertible") is True, "amount ratio factor is not rule convertible")
    assert_true(amount_ratio.get("business_meaning"), "amount ratio factor missing business meaning")
    assert_true(amount_ratio.get("calculation_logic"), "amount ratio factor missing calculation logic")
    assert_true(amount_ratio.get("source_fields"), "amount ratio factor missing source fields")
    assert_true(amount_ratio.get("evidence_refs"), "amount ratio factor missing evidence refs")

    confirmed_label = next(
        (
            card
            for card in cards
            if card.get("factor_name") == "confirmed_fwa"
        ),
        None,
    )
    assert_true(confirmed_label is not None, "factor card missing confirmed_fwa label")
    assert_true(confirmed_label.get("is_label") is True, "confirmed_fwa should be marked as label")
    assert_true(
        "label_field" in confirmed_label.get("readiness_issues", []),
        "confirmed_fwa label should be excluded from online factor readiness",
    )
    return {
        "factor_count": readiness["factor_count"],
        "ready_factor_count": readiness["ready_factor_count"],
        "rule_convertible_count": readiness["rule_convertible_count"],
    }


def assert_dataset_model_lineage():
    datasets = request("GET", "/api/v1/ops/datasets")
    dataset = next(
        (
            item
            for item in datasets.get("datasets", [])
            if item.get("dataset_key") == "demo_claims_fwa"
            and item.get("dataset_version") == "2026-05-demo"
        ),
        None,
    )
    assert_true(dataset is not None, "demo_claims_fwa dataset is missing")
    assert_true(dataset.get("sample_grain") == "claim", "demo dataset sample grain mismatch")
    assert_true(dataset.get("label_column") == "confirmed_fwa", "demo dataset label mismatch")
    assert_true(dataset.get("storage_format") == "parquet", "demo dataset storage format mismatch")
    assert_true(dataset.get("row_count") == 25000, "demo dataset row count mismatch")
    assert_true(
        set(dataset.get("entity_keys", [])) >= {"claim_id", "member_id", "provider_id"},
        "demo dataset missing required entity keys",
    )

    health = next(
        (
            item
            for item in datasets.get("health", [])
            if item.get("dataset_key") == "demo_claims_fwa"
            and item.get("dataset_version") == "2026-05-demo"
        ),
        None,
    )
    assert_true(health is not None, "demo_claims_fwa health row is missing")
    assert_true(health.get("data_quality_status") == "ready", "demo dataset should be ready")
    assert_true(health.get("field_count", 0) >= 8, "demo dataset missing profiled fields")
    assert_true(health.get("label_count", 0) >= 1, "demo dataset missing label field")
    assert_true(health.get("entity_key_count", 0) >= 3, "demo dataset missing entity key fields")
    assert_true(health.get("online_ready_count", 0) >= 6, "demo dataset missing online-ready fields")

    evaluations = request("GET", "/api/v1/ops/model-evaluations")
    evaluation = next(
        (
            item
            for item in evaluations.get("evaluations", [])
            if item.get("evaluation_run_id") == "eval-baseline-fwa-2026-05-demo"
        ),
        None,
    )
    assert_true(evaluation is not None, "baseline model evaluation is missing")
    assert_true(evaluation.get("model_key") == MODEL_KEY, "baseline evaluation model key mismatch")
    assert_true(evaluation.get("model_version") == "0.1.0", "baseline evaluation version mismatch")
    assert_true(decimal_value(evaluation.get("auc")) >= Decimal("0.8"), "baseline evaluation AUC too low")
    assert_true(
        decimal_value(evaluation.get("precision")) >= Decimal("0.7"),
        "baseline evaluation precision too low",
    )
    assert_true(
        decimal_value(evaluation.get("recall")) >= Decimal("0.6"),
        "baseline evaluation recall too low",
    )
    assert_true(
        decimal_value(evaluation.get("metrics_json", {}).get("psi")) <= Decimal("0.1"),
        "baseline evaluation PSI should be within demo governance threshold",
    )

    lineage = next(
        (
            item
            for item in evaluations.get("lineage", [])
            if item.get("evaluation_run_id") == "eval-baseline-fwa-2026-05-demo"
        ),
        None,
    )
    assert_true(lineage is not None, "baseline evaluation lineage is missing")
    assert_true(lineage.get("source_dataset_key") == "demo_claims_fwa", "lineage dataset key mismatch")
    assert_true(
        lineage.get("source_dataset_version") == "2026-05-demo",
        "lineage dataset version mismatch",
    )
    assert_true(
        lineage.get("source_data_quality_status") == "ready",
        "lineage source dataset should be ready",
    )

    detail = request("GET", "/api/v1/ops/model-evaluations/eval-baseline-fwa-2026-05-demo")
    assert_true(
        detail.get("evaluation", {}).get("evaluation_run_id") == "eval-baseline-fwa-2026-05-demo",
        "baseline evaluation detail endpoint mismatch",
    )
    assert_true(
        detail.get("evaluation", {}).get("confusion_matrix_json"),
        "baseline evaluation detail missing confusion matrix",
    )

    return {
        "dataset_key": dataset["dataset_key"],
        "dataset_version": dataset["dataset_version"],
        "data_quality_status": health["data_quality_status"],
        "evaluation_run_id": evaluation["evaluation_run_id"],
        "auc": evaluation["auc"],
    }


def assert_model_governance_controls(dashboard):
    performance = request("GET", f"/api/v1/ops/models/{MODEL_KEY}/performance")
    assert_true(performance.get("model_key") == MODEL_KEY, "model performance key mismatch")
    assert_true(performance.get("scored_runs", 0) >= 1, "model performance missing scored runs")
    assert_true(performance.get("drift_status") == "stable", "baseline model drift should be stable")
    assert_true(
        decimal_value(performance.get("score_psi")) == Decimal("0.08"),
        "baseline model PSI mismatch",
    )

    gates = request("GET", f"/api/v1/ops/models/{MODEL_KEY}/promotion-gates")
    assert_true(gates.get("model_key") == MODEL_KEY, "model promotion gate key mismatch")
    assert_true(gates.get("model_version") == "0.1.0", "model promotion gate version mismatch")
    assert_true(gates.get("decision") == "routing_allowed", f"model gates should allow routing: {gates}")
    assert_true(gates.get("blockers") == [], f"model gates should not have blockers: {gates}")
    assert_true(gates.get("passed_count") == gates.get("total_count"), "not all model gates passed")
    assert_true(
        gates.get("latest_evaluation_id") == "eval-baseline-fwa-2026-05-demo",
        "model gates missing baseline evaluation evidence",
    )
    assert_true(gates.get("source_data_quality_status") == "ready", "model gates source data not ready")
    assert_true(gates.get("approved_label_count", 0) >= 1, "model gates missing approved labels")
    gate_rows = {gate.get("label"): gate for gate in gates.get("gates", [])}
    for label in [
        "Immutable dataset",
        "Holdout metrics",
        "Out-of-time evidence",
        "Review-capacity threshold",
        "Explanation artifact",
        "Leakage check",
        "Shadow comparison",
        "Source data quality",
        "Feature reproducibility",
        "Label provenance",
        "Drift status",
        "Model QA feedback closure",
        "Label governance",
        "Approval",
        "Active version",
    ]:
        assert_true(label in gate_rows, f"model promotion gates missing {label}")
        assert_true(gate_rows[label].get("passed") is True, f"model gate {label} did not pass")
        assert_true(
            gate_rows[label].get("evidence_source") != "missing",
            f"model gate {label} missing evidence source",
        )

    governance = dashboard.get("model_governance", {})
    assert_true(governance.get("total_models", 0) >= 1, "dashboard model governance missing models")
    assert_true(
        governance.get("evaluated_models", 0) >= 1,
        "dashboard model governance missing evaluation coverage",
    )
    assert_true(
        governance.get("drift_watch_count", 0) == 0,
        "dashboard model governance should not show drift watch",
    )
    assert_true(
        governance.get("drift_detected_count", 0) == 0,
        "dashboard model governance should not show detected drift",
    )
    assert_true(
        decimal_value(governance.get("average_precision")) >= Decimal("0.70"),
        "dashboard model governance precision too low",
    )
    assert_true(
        decimal_value(governance.get("average_recall")) >= Decimal("0.60"),
        "dashboard model governance recall too low",
    )

    return {
        "promotion_decision": gates["decision"],
        "passed_count": gates["passed_count"],
        "latest_evaluation_id": gates["latest_evaluation_id"],
        "drift_status": performance["drift_status"],
    }


def assert_governance_audit_trail(agent_governance, rule_release, routing_policy, qa_audit_sample, qa_feedback_update):
    governance_events = request("GET", "/api/v1/ops/audit-events?event_group=governance&limit=200").get("events", [])
    assert_true(governance_events, "governance audit trail returned no events")
    expected_event_types = {
        "rule.candidate.saved",
        "rule.status.changed",
        "rule.promotion.reviewed",
        "routing_policy.candidate.saved",
        "routing_policy.status.changed",
        "routing_policy.activation.completed",
        "agent.approval.decided",
        "audit_sample.created",
        "qa.feedback.status.updated",
    }
    observed_event_types = {event.get("event_type") for event in governance_events}
    missing_event_types = expected_event_types - observed_event_types
    assert_true(
        not missing_event_types,
        f"governance audit trail missing event types: {sorted(missing_event_types)}",
    )
    for event_type in expected_event_types:
        event = next(event for event in governance_events if event.get("event_type") == event_type)
        assert_true(event.get("evidence_refs"), f"governance event {event_type} missing evidence refs")

    rule_events = request(
        "GET",
        f"/api/v1/ops/audit-events?event_group=governance&rule_id={rule_release['rule_id']}&rule_version=1&limit=50",
    ).get("events", [])
    assert_true(
        any(event.get("event_type") == "rule.promotion.reviewed" for event in rule_events),
        "rule governance audit filter missing promotion review",
    )
    assert_true(
        any(
            event.get("event_type") == "rule.status.changed"
            and event.get("payload", {}).get("to_status") == "active"
            for event in rule_events
        ),
        "rule governance audit filter missing active lifecycle event",
    )

    routing_events = request(
        "GET",
        "/api/v1/ops/audit-events"
        f"?event_group=governance&routing_policy_id={routing_policy['policy_id']}"
        "&routing_policy_version=1&review_mode=pre_payment&limit=50",
    ).get("events", [])
    for event_type in [
        "routing_policy.candidate.saved",
        "routing_policy.status.changed",
        "routing_policy.activation.completed",
    ]:
        assert_true(
            any(event.get("event_type") == event_type for event in routing_events),
            f"routing governance audit filter missing {event_type}",
        )

    agent_events = request(
        "GET",
        f"/api/v1/ops/audit-events?event_group=governance&agent_run_id={agent_governance['agent_run_id']}&limit=20",
    ).get("events", [])
    assert_true(
        any(event.get("event_type") == "agent.approval.decided" for event in agent_events),
        "agent governance audit filter missing approval decision",
    )

    sample_events = request(
        "GET",
        f"/api/v1/ops/audit-events?event_group=governance&sample_id={qa_audit_sample['sample_id']}&limit=20",
    ).get("events", [])
    assert_true(
        any(event.get("event_type") == "audit_sample.created" for event in sample_events),
        "QA sample governance audit filter missing sample creation",
    )

    feedback_events = request(
        "GET",
        f"/api/v1/ops/audit-events?event_group=governance&feedback_id={qa_feedback_update['feedback_id']}&limit=20",
    ).get("events", [])
    assert_true(
        any(event.get("event_type") == "qa.feedback.status.updated" for event in feedback_events),
        "QA feedback governance audit filter missing status update",
    )

    return {
        "event_type_count": len(observed_event_types),
        "rule_event_count": len(rule_events),
        "routing_event_count": len(routing_events),
        "agent_event_count": len(agent_events),
        "sample_event_count": len(sample_events),
        "feedback_event_count": len(feedback_events),
    }


def assert_tpa_api_call_observability(score, investigation, qa):
    api_calls = request("GET", "/api/v1/ops/api-calls?limit=50").get("calls", [])
    expected = {
        "scoring.completed": {
            "endpoint": "/api/v1/claims/score",
            "audit_id": score["audit_id"],
            "run_id": score["run_id"],
            "idempotency_key": None,
        },
        "investigation.result.received": {
            "endpoint": "/api/v1/investigations/results",
            "audit_id": investigation["audit_id"],
            "run_id": investigation["run_id"],
            "idempotency_key": investigation["idempotency_key"],
        },
        "qa.result.received": {
            "endpoint": "/api/v1/qa/results",
            "audit_id": qa["audit_id"],
            "run_id": qa["run_id"],
            "idempotency_key": qa["idempotency_key"],
        },
    }
    observed = {}
    for event_type, contract in expected.items():
        observed[event_type] = next(
            (
                call
                for call in api_calls
                if call.get("event_type") == event_type and call.get("audit_id") == contract["audit_id"]
            ),
            None,
        )
    missing = {event_type for event_type, call in observed.items() if call is None}
    assert_true(not missing, f"API call observability missing event types: {sorted(missing)}")

    for event_type, contract in expected.items():
        call = observed[event_type]
        assert_true(call.get("endpoint") == contract["endpoint"], f"{event_type} endpoint mismatch")
        assert_true(call.get("method") == "POST", f"{event_type} method mismatch")
        assert_true(call.get("status_code") == 200, f"{event_type} status code mismatch")
        assert_true(call.get("result") == "succeeded", f"{event_type} result mismatch")
        assert_true(call.get("claim_id") == CLAIM_ID, f"{event_type} claim id mismatch")
        assert_true(call.get("audit_id") == contract["audit_id"], f"{event_type} audit id mismatch")
        assert_true(call.get("run_id") == contract["run_id"], f"{event_type} run id mismatch")
        assert_true(
            call.get("idempotency_key") == contract["idempotency_key"],
            f"{event_type} idempotency key mismatch",
        )
        assert_true(call.get("evidence_refs"), f"{event_type} evidence refs missing")

    return {
        "api_call_count": len(api_calls),
        "observed_event_types": sorted(expected),
    }


def assert_tpa_webhook_delivery(score):
    webhooks = request("GET", "/api/v1/ops/webhook-events").get("events", [])
    expected_event_types = {
        "fwa.score.completed",
        "fwa.case.routed",
        "fwa.investigation.closed",
        "fwa.qa.reviewed",
        "fwa.medical.reviewed",
    }
    claim_events = [
        event
        for event in webhooks
        if event.get("claim_id") == CLAIM_ID and event.get("event_type") in expected_event_types
    ]
    observed_event_types = {event.get("event_type") for event in claim_events}
    missing_event_types = expected_event_types - observed_event_types
    assert_true(
        not missing_event_types,
        f"webhook event list missing expected TPA events: {sorted(missing_event_types)}",
    )
    for event in claim_events:
        assert_true(event.get("event_id"), "webhook event missing event_id")
        assert_true(event.get("source_audit_id"), "webhook event missing source audit id")
        assert_true(event.get("idempotency_key", "").startswith("fwa-webhook:"), "webhook idempotency key invalid")
        assert_true(event.get("signature_key_id"), "webhook event missing signature key id")
        assert_true(event.get("signature_algorithm") == "hmac-sha256", "webhook signature algorithm mismatch")
        assert_true(event.get("evidence_refs"), "webhook event missing evidence refs")

    score_event = next(
        (
            event
            for event in claim_events
            if event.get("event_type") == "fwa.score.completed"
            and event.get("source_audit_id") == score["audit_id"]
        ),
        None,
    )
    assert_true(score_event is not None, "webhook event list missing current scoring audit event")
    assert_true(score_event.get("delivery_status") == "pending", "new score webhook should be pending")
    assert_true(score_event.get("retry_count") == 0, "new score webhook should not have retries")

    attempt = request(
        "POST",
        f"/api/v1/ops/webhook-events/{score_event['event_id']}/delivery-attempts",
        {
            "delivery_status": "failed",
            "response_status_code": 503,
            "error_message": "TPA webhook endpoint unavailable",
        },
    )
    assert_true(attempt.get("event_id") == score_event["event_id"], "webhook attempt event id mismatch")
    assert_true(attempt.get("attempt_number") == 1, "first webhook attempt should be attempt 1")
    assert_true(attempt.get("delivery_status") == "failed", "webhook attempt status mismatch")
    assert_true(attempt.get("response_status_code") == 503, "webhook attempt response status mismatch")
    assert_true(attempt.get("next_attempt_at"), "failed webhook attempt should schedule retry")

    updated_events = request("GET", "/api/v1/ops/webhook-events").get("events", [])
    updated_score_event = next(
        (
            event
            for event in updated_events
            if event.get("event_id") == score_event["event_id"]
        ),
        None,
    )
    assert_true(updated_score_event is not None, "updated webhook score event missing")
    assert_true(
        updated_score_event.get("delivery_status") == "retry_wait",
        "failed webhook event should move to retry_wait",
    )
    assert_true(updated_score_event.get("retry_count") == 1, "webhook retry count mismatch")
    assert_true(
        updated_score_event.get("last_response_status_code") == 503,
        "webhook last response status mismatch",
    )
    assert_true(
        updated_score_event.get("last_error_message") == "TPA webhook endpoint unavailable",
        "webhook last error message mismatch",
    )
    assert_true(updated_score_event.get("next_attempt_at"), "retry_wait webhook event missing next attempt")
    return {
        "event_id": score_event["event_id"],
        "event_type": score_event["event_type"],
        "delivery_status": updated_score_event["delivery_status"],
        "retry_count": updated_score_event["retry_count"],
    }


def assert_member_profile_summary():
    profile = request("GET", "/api/v1/members/MBR-0287/profile-summary")
    assert_true(profile.get("member_id") == "MBR-0287", "member profile summary id mismatch")
    assert_true(profile.get("claim_count", 0) >= 1, "member profile missing claim history")
    assert_true(profile.get("policy_count", 0) >= 1, "member profile missing policy history")
    assert_true(profile.get("currency") == "CNY", "member profile currency mismatch")
    assert_true(
        decimal_value(profile.get("total_claim_amount", "0")) >= Decimal("8000"),
        "member profile total claim amount below seeded demo claim",
    )
    assert_true(
        profile.get("high_risk_claim_count", 0) >= 1,
        "member profile missing high-risk scoring history",
    )
    assert_true(profile.get("latest_claim_id") == CLAIM_ID, "member profile latest claim mismatch")
    assert_true(profile.get("risk_level_summary"), "member profile missing risk level summary")
    assert_true("历史理赔" in profile.get("profile_summary", ""), "member profile missing Chinese summary")
    assert_true(
        "members:MBR-0287" in profile.get("evidence_refs", []),
        "member profile missing member evidence ref",
    )
    return {
        "member_id": profile["member_id"],
        "claim_count": profile["claim_count"],
        "high_risk_claim_count": profile["high_risk_claim_count"],
        "latest_claim_id": profile["latest_claim_id"],
    }


def assert_provider_risk_summary():
    summary = request("GET", "/api/v1/ops/providers/risk-summary")
    assert_true(summary.get("provider_count", 0) >= 1, "provider risk summary missing providers")
    assert_true(
        summary.get("review_required_count", 0) >= 1,
        "provider risk summary missing review-required providers",
    )
    assert_true(summary.get("high_risk_count", 0) >= 1, "provider risk summary missing high-risk providers")
    provider = next(
        (
            item
            for item in summary.get("providers", [])
            if item.get("provider_id") == "PRV-0287"
        ),
        None,
    )
    assert_true(provider is not None, "provider risk summary missing demo provider")
    assert_true(provider.get("risk_score", 0) >= 70, "demo provider risk score should be high")
    assert_true(provider.get("risk_tier") == "high", "demo provider risk tier mismatch")
    assert_true(provider.get("review_required") is True, "demo provider should require review")
    assert_true(provider.get("review_route") == "provider_review", "demo provider review route mismatch")
    assert_true(provider.get("claim_count", 0) >= 1, "demo provider missing claim count")
    assert_true(provider.get("latest_claim_id") == CLAIM_ID, "demo provider latest claim mismatch")
    assert_true(
        "providers:PRV-0287" in provider.get("evidence_refs", []),
        "demo provider missing provider evidence ref",
    )
    return {
        "provider_id": provider["provider_id"],
        "risk_score": provider["risk_score"],
        "review_route": provider["review_route"],
        "latest_claim_id": provider["latest_claim_id"],
    }


def assert_fwa_scheme_taxonomy():
    taxonomy = request("GET", "/api/v1/ops/fwa-schemes")
    schemes = taxonomy.get("schemes", [])
    families = {scheme.get("scheme_family") for scheme in schemes}
    assert_true(taxonomy.get("scheme_count", 0) >= 15, "FWA scheme taxonomy missing scheme families")
    for family in [
        "duplicate_billing",
        "upcoding",
        "unbundling",
        "medically_unnecessary_service",
        "excessive_utilization",
        "diagnosis_procedure_mismatch",
        "laboratory_testing_abuse",
        "telehealth_abuse",
        "genetic_testing_abuse",
        "pharmacy_controlled_substance_abuse",
        "dme_home_health_hospice_rehab_risk",
        "provider_peer_outlier",
        "relationship_concentration",
    ]:
        assert_true(family in families, f"FWA scheme taxonomy missing {family}")

    provider_scheme = next(
        scheme for scheme in schemes if scheme.get("scheme_family") == "provider_peer_outlier"
    )
    assert_true(
        provider_scheme.get("default_review_route") == "provider_review",
        "provider scheme review route mismatch",
    )
    assert_true(
        "L6_PROVIDER_GRAPH_RISK" in provider_scheme.get("primary_layers", []),
        "provider scheme missing L6 layer",
    )
    assert_true(
        "peer_group_definition" in provider_scheme.get("minimum_evidence", []),
        "provider scheme missing peer evidence requirement",
    )

    clinical_scheme = next(
        scheme for scheme in schemes if scheme.get("scheme_family") == "diagnosis_procedure_mismatch"
    )
    assert_true(
        clinical_scheme.get("default_review_route") == "medical_review",
        "diagnosis-procedure scheme review route mismatch",
    )
    assert_true(
        "L5_MEDICAL_REASONABLENESS" in clinical_scheme.get("primary_layers", []),
        "diagnosis-procedure scheme missing L5 layer",
    )

    relationship_scheme = next(
        scheme for scheme in schemes if scheme.get("scheme_family") == "relationship_concentration"
    )
    assert_true(
        relationship_scheme.get("default_review_route") == "investigation",
        "relationship concentration scheme review route mismatch",
    )
    return {
        "scheme_count": taxonomy["scheme_count"],
        "families": sorted(families),
    }


def assert_dashboard_roi(dashboard, agent, lead):
    value = dashboard.get("value_measurement", {})
    assert_true(
        decimal_value(value.get("estimated_impact", "0")) >= Decimal("8200"),
        "dashboard value measurement missing estimated impact",
    )
    assert_true(
        decimal_value(value.get("net_value", "0")) > Decimal("0"),
        "dashboard value measurement missing positive net value",
    )
    assert_true(value.get("currency") == "CNY", "dashboard value measurement currency mismatch")
    assert_true(
        "estimated" in value.get("evidence_caveat", ""),
        "dashboard value measurement missing evidence caveat",
    )

    attributions = dashboard.get("saving_attributions", [])
    assert_true(
        any(
            item.get("source_type") == "agent"
            and item.get("source_id") == agent["agent_run_id"]
            and decimal_value(item.get("saving_amount", "0")) > Decimal("0")
            and f"agent_run:{agent['agent_run_id']}" in item.get("evidence_refs", [])
            for item in attributions
        ),
        "dashboard saving attribution missing agent source",
    )
    assert_true(
        any(
            item.get("source_type") == "rule"
            and item.get("source_id") == "EARLY_CLAIM"
            and decimal_value(item.get("saving_amount", "0")) > Decimal("0")
            and "rule_runs:EARLY_CLAIM" in item.get("evidence_refs", [])
            for item in attributions
        ),
        "dashboard saving attribution missing rule source",
    )

    segments = dashboard.get("saving_segments", [])
    assert_true(
        any(
            item.get("segment_type") == "provider"
            and item.get("segment_id") == "PRV-0287"
            and decimal_value(item.get("saving_amount", "0")) >= Decimal("8200")
            and item.get("roi", 0) > 0
            for item in segments
        ),
        "dashboard segment ROI missing provider attribution",
    )
    assert_true(
        any(
            item.get("segment_type") == "scheme"
            and item.get("segment_id") == lead["scheme_family"]
            and decimal_value(item.get("saving_amount", "0")) >= Decimal("8200")
            for item in segments
        ),
        "dashboard segment ROI missing scheme attribution",
    )
    assert_true(
        any(
            item.get("segment_type") == "campaign"
            and item.get("segment_id") == "prepay-fwa-sprint-q1"
            and item.get("roi", 0) > 0
            for item in segments
        ),
        "dashboard segment ROI missing campaign attribution",
    )
    return {
        "net_value": value["net_value"],
        "estimated_impact": value["estimated_impact"],
        "attribution_count": len(attributions),
        "segment_count": len(segments),
    }


def govern_agent_run(agent):
    agent_run_id = agent["agent_run_id"]
    runs = request("GET", "/api/v1/ops/agent-runs").get("runs", [])
    run = next((item for item in runs if item.get("agent_run_id") == agent_run_id), None)
    assert_true(run is not None, "agent run log missing demo agent run")
    assert_true(run.get("claim_id") == CLAIM_ID, "agent run log claim_id mismatch")
    assert_true(run.get("status") == "succeeded", "agent run log status mismatch")
    assert_true(run.get("decision_boundary") == "assistive_only", "agent run must stay assistive only")
    assert_true(run.get("steps"), "agent run log missing steps")
    assert_true(run.get("context_snapshots"), "agent run log missing redacted context snapshot")
    context_snapshot = run["context_snapshots"][0]
    assert_true(
        context_snapshot.get("redaction_status") == "pii_masked",
        "agent context snapshot was not PII masked",
    )
    assert_true(context_snapshot.get("checksum", "").startswith("snapshot:"), "agent context snapshot missing checksum")
    assert_true(run.get("tool_calls"), "agent run log missing tool call audit")
    tool_call = next(
        (
            call
            for call in run["tool_calls"]
            if call.get("tool_name") == "knowledge.search_similar"
        ),
        None,
    )
    assert_true(tool_call is not None, "agent run log missing knowledge search tool call")
    assert_true(tool_call.get("status") == "succeeded", "agent tool call did not succeed")
    assert_true(tool_call.get("evidence_refs"), "agent tool call missing evidence refs")
    policy_check = next(
        (
            check
            for check in run.get("policy_checks", [])
            if check.get("tool_call_id") == tool_call.get("tool_call_id")
        ),
        None,
    )
    assert_true(policy_check is not None, "agent run log missing tool policy check")
    assert_true(policy_check.get("decision") == "allowed", "agent tool policy check did not allow tool")
    assert_true(
        policy_check.get("policy_name") == "agent_tool_allowlist",
        "agent tool policy check policy mismatch",
    )
    tool_result = next(
        (
            result
            for result in run.get("tool_results", [])
            if result.get("tool_call_id") == tool_call.get("tool_call_id")
        ),
        None,
    )
    assert_true(tool_result is not None, "agent run log missing tool result")
    assert_true(tool_result.get("status") == "succeeded", "agent tool result did not succeed")
    assert_true(tool_result.get("output_json", {}).get("result_count", 0) > 0, "agent tool result missing matches")
    pending_approval = next(
        (
            approval
            for approval in run.get("approvals", [])
            if approval.get("proposed_action") == "manual_review_required"
        ),
        None,
    )
    assert_true(pending_approval is not None, "agent run missing pending human approval")
    assert_true(pending_approval.get("decision") == "pending", "agent approval should start pending")

    alerts = request("GET", "/api/v1/ops/alerts").get("alerts", [])
    assert_true(
        any(
            alert.get("alert_type") == "agent_approval_pending"
            and alert.get("claim_id") == CLAIM_ID
            and f"agent_run:{agent_run_id}" in alert.get("evidence_refs", [])
            for alert in alerts
        ),
        "ops alerts missing pending agent approval",
    )

    approval = request(
        "POST",
        f"/api/v1/ops/agent-runs/{agent_run_id}/approvals",
        {
            "decision": "approved",
            "approver": "agent-governance-demo",
            "reason": "Evidence package is sufficient for manual review routing.",
            "evidence_refs": [
                f"agent_run:{agent_run_id}",
                "agent_approval:manual_review_required",
            ],
        },
    )
    assert_true(approval.get("audit_id"), "agent approval response missing audit id")
    approval_record = approval.get("approval", {})
    assert_true(approval_record.get("agent_run_id") == agent_run_id, "agent approval run id mismatch")
    assert_true(approval_record.get("decision") == "approved", "agent approval was not approved")
    assert_true(
        approval_record.get("approver") == "agent-governance-demo",
        "agent approval approver mismatch",
    )
    assert_true(
        f"agent_run:{agent_run_id}" in approval_record.get("evidence_refs", []),
        "agent approval missing run evidence ref",
    )

    approved_runs = request("GET", "/api/v1/ops/agent-runs").get("runs", [])
    approved_run = next((item for item in approved_runs if item.get("agent_run_id") == agent_run_id), None)
    assert_true(approved_run is not None, "approved agent run log missing")
    approved_approval = next(
        (
            item
            for item in approved_run.get("approvals", [])
            if item.get("proposed_action") == "manual_review_required"
        ),
        None,
    )
    assert_true(approved_approval is not None, "approved agent run missing approval record")
    assert_true(approved_approval.get("decision") == "approved", "agent run approval log not approved")

    resolved_alerts = request("GET", "/api/v1/ops/alerts").get("alerts", [])
    assert_true(
        not any(
            alert.get("alert_type") == "agent_approval_pending"
            and alert.get("claim_id") == CLAIM_ID
            for alert in resolved_alerts
        ),
        "pending agent approval alert remained after approval",
    )

    audit_events = request(
        "GET",
        f"/api/v1/ops/audit-events?agent_run_id={agent_run_id}&limit=10",
    ).get("events", [])
    assert_true(
        any(event.get("event_type") == "agent.investigation.completed" for event in audit_events),
        "agent audit log missing investigation completion",
    )
    assert_true(
        any(event.get("event_type") == "agent.approval.decided" for event in audit_events),
        "agent audit log missing approval decision",
    )
    return {
        "agent_run_id": agent_run_id,
        "approval_id": approval_record["approval_id"],
        "approval_decision": approval_record["decision"],
        "audit_id": approval["audit_id"],
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
    scheme_taxonomy = assert_fwa_scheme_taxonomy()

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
    member_profile = assert_member_profile_summary()
    provider_risk = assert_provider_risk_summary()

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
    assert_true(
        lead.get("scheme_family") in scheme_taxonomy["families"],
        "lead scheme family must be governed by FWA taxonomy",
    )
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
    qa_audit_sample = create_and_review_qa_audit_sample(score)

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
    agent_governance = govern_agent_run(agent)

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
                "rule_runs:EARLY_CLAIM",
                "knowledge_cases:KC-1001",
                "campaigns:prepay-fwa-sprint-q1",
            ],
        },
    )
    assert_true(
        investigation.get("event_type") == "investigation.result.received",
        "investigation writeback was not audited",
    )
    assert_true(investigation.get("audit_id"), "investigation writeback missing audit_id")
    assert_true(
        investigation.get("idempotency_key", "").startswith("tpa-writeback:investigation.result.received:"),
        "investigation writeback missing idempotency key",
    )

    discovered_rule = run_rule_discovery_candidate_lifecycle()
    rule_release = run_rule_backtest_and_publish(score, investigation)
    routing_policy = run_routing_policy_governance()

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
    assert_true(
        qa.get("idempotency_key", "").startswith("tpa-writeback:qa.result.received:"),
        "QA writeback missing idempotency key",
    )
    qa_feedback_update = resolve_demo_qa_feedback(qa)
    knowledge_publish = publish_demo_knowledge_case(investigation, qa)

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
        "agent.investigation.completed" in event_types,
        "audit history missing agent.investigation.completed",
    )
    assert_true(
        "agent.approval.decided" in event_types,
        "audit history missing agent.approval.decided",
    )
    assert_true(
        "investigation.result.received" in event_types,
        "audit history missing investigation.result.received",
    )
    assert_true("qa.result.received" in event_types, "audit history missing qa.result.received")
    assert_true(
        "qa.feedback.status.updated" in event_types,
        "audit history missing qa.feedback.status.updated",
    )
    assert_true(
        "knowledge.case.published" in event_types,
        "audit history missing knowledge.case.published",
    )
    api_call_observability = assert_tpa_api_call_observability(score, investigation, qa)
    governance_audit = assert_governance_audit_trail(
        agent_governance,
        rule_release,
        routing_policy,
        qa_audit_sample,
        qa_feedback_update,
    )
    webhook_delivery = assert_tpa_webhook_delivery(score)

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
    assert_true(
        dashboard.get("qa_queue", {}).get("sampled_cases", 0) >= 1,
        "dashboard missing sampled QA cases",
    )
    assert_true(
        dashboard.get("qa_queue", {}).get("reviewed_cases", 0) >= 1,
        "dashboard missing reviewed sampled QA cases",
    )
    assert_true(
        dashboard.get("qa_queue", {}).get("feedback_resolved_count", 0) >= 1,
        "dashboard missing resolved QA feedback count",
    )
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
    agent_governance_summary = dashboard.get("agent_governance", {})
    assert_true(
        agent_governance_summary.get("total_runs", 0) >= 1,
        "dashboard missing agent governance runs",
    )
    assert_true(
        agent_governance_summary.get("approved_approvals", 0) >= 1,
        "dashboard missing approved agent approvals",
    )
    roi_summary = assert_dashboard_roi(dashboard, agent, lead)
    model_governance = assert_model_governance_controls(dashboard)

    dataset_model_lineage = assert_dataset_model_lineage()
    factor_readiness = assert_factor_factory_readiness()

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
                "scheme_taxonomy": {"scheme_count": scheme_taxonomy["scheme_count"]},
                "member_profile": member_profile,
                "provider_risk": provider_risk,
                "similar_case": results[0]["case_id"],
                "medical_review_audit_id": medical_review["audit_id"],
                "agent_run_id": agent["agent_run_id"],
                "agent_governance": agent_governance,
                "investigation_audit_id": investigation["audit_id"],
                "case_id": case_id,
                "qa_audit_sample": qa_audit_sample,
                "outcome_labels": label_pool["total_labels"],
                "roi_summary": roi_summary,
                "model_governance": model_governance,
                "retraining_job_id": retraining_job["job_id"],
                "discovered_rule": discovered_rule,
                "qa_feedback_update": qa_feedback_update,
                "knowledge_publish": knowledge_publish,
                "api_call_observability": api_call_observability,
                "governance_audit": governance_audit,
                "dataset_model_lineage": dataset_model_lineage,
                "factor_readiness": factor_readiness,
                "webhook_delivery": webhook_delivery,
                "rule_release": rule_release,
                "routing_policy": routing_policy,
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
