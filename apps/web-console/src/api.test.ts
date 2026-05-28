import { afterEach, describe, expect, it, vi } from "vitest";
import {
  approveRule,
  activateRoutingPolicy,
  backtestRule,
  claimNextModelRetrainingJob,
  completeModelRetrainingJob,
  createAuditSample,
  discoverRules,
  createModelRetrainingJob,
  getDashboardSummary,
  getClaimAuditHistory,
  getProviderRiskSummary,
  getRulePromotionGates,
  submitRulePromotionReview,
  investigateCase,
  listAgentRuns,
  listAuditSamples,
  listCases,
  listDatasets,
  listFactorReadiness,
  listFwaSchemes,
  listGovernanceChangeEvents,
  listLeads,
  listKnowledgeCases,
  listOpsAlerts,
  listOutcomeLabels,
  listQaFeedbackItems,
  listQaQueue,
  listQaQueueSummary,
  listWebhookEvents,
  listModelEvaluations,
  listModelRetrainingJobs,
  listModels,
  listRoutingPolicies,
  getModelPromotionGates,
  getModelRetrainingReadiness,
  approveRoutingPolicy,
  getRoutingPolicyPromotionGates,
  listAuditEvents,
  submitModelPromotionReview,
  updateModelRetrainingJobStatus,
  listRules,
  publishKnowledgeCase,
  publishRule,
  rollbackModel,
  rollbackRoutingPolicy,
  rollbackRule,
  saveRuleCandidate,
  saveRoutingPolicyCandidate,
  searchSimilarCases,
  submitAgentApproval,
  submitRoutingPolicy,
  submitRule,
  submitQaResult,
  submitWebhookDeliveryAttempt,
  triageLead,
} from "./api";

function mockFetch(body: unknown, ok = true) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok,
    json: () => Promise.resolve(body),
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("ops API helpers", () => {
  it("calls rule operations endpoints with API key", async () => {
    const fetchMock = mockFetch({ rules: [] });

    await listRules("dev-secret");
    await getRulePromotionGates("rule_early_claim", "dev-secret");
    await submitRulePromotionReview(
      "rule_early_claim",
      { decision: "approved", reviewer: "rule-governance", notes: "limited rollout" },
      "dev-secret",
    );
    await submitRule("rule_early_claim", "dev-secret");
    await approveRule("rule_early_claim", "dev-secret");
    await publishRule("rule_early_claim", "dev-secret");
    await rollbackRule("rule_early_claim", "dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/ops/rules",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/rules/rule_early_claim/promotion-gates",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/ops/rules/rule_early_claim/promotion-reviews",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          decision: "approved",
          reviewer: "rule-governance",
          notes: "limited rollout",
        }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "/api/v1/ops/rules/rule_early_claim/submit",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      5,
      "/api/v1/ops/rules/rule_early_claim/approve",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      6,
      "/api/v1/ops/rules/rule_early_claim/publish",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      7,
      "/api/v1/ops/rules/rule_early_claim/rollback",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("calls routing policy lifecycle endpoints", async () => {
    const fetchMock = mockFetch({ policies: [] });
    const policy = {
      policy_id: "fwa_risk_fusion_routing",
      review_mode: "pre_payment",
      version: 2,
    };
    const candidate = {
      owner: "policy-ops",
      policy: {
        ...policy,
        risk_thresholds: {
          low_max: 24,
          medium_min: 25,
          high_min: 65,
          critical_min: 88,
        },
        confidence_thresholds: {
          low_confidence_below: 55,
          high_confidence_min: 85,
        },
        provider_review_threshold: 72,
      },
    };

    await listRoutingPolicies("dev-secret");
    await saveRoutingPolicyCandidate(candidate, "dev-secret");
    await getRoutingPolicyPromotionGates(policy, "dev-secret");
    await submitRoutingPolicy(policy, "dev-secret");
    await approveRoutingPolicy(policy, "dev-secret");
    await activateRoutingPolicy(policy, "dev-secret");
    await rollbackRoutingPolicy(policy, "dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/ops/routing-policies",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/routing-policies",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(candidate),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/ops/routing-policies/fwa_risk_fusion_routing/pre_payment/2/promotion-gates",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "/api/v1/ops/routing-policies/fwa_risk_fusion_routing/pre_payment/2/submit",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      5,
      "/api/v1/ops/routing-policies/fwa_risk_fusion_routing/pre_payment/2/approve",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      6,
      "/api/v1/ops/routing-policies/fwa_risk_fusion_routing/pre_payment/2/activate",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      7,
      "/api/v1/ops/routing-policies/fwa_risk_fusion_routing/pre_payment/2/rollback",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("posts rule backtest payload", async () => {
    const fetchMock = mockFetch({ sample_count: 0 });
    const payload = { rule: { rule_id: "candidate" }, samples: [] };

    await backtestRule(payload, "dev-secret");
    await discoverRules({ samples: [] }, "dev-secret");
    await saveRuleCandidate({ rule: { rule_id: "candidate" } }, "dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/ops/rules/backtest",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/rules/discover",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ samples: [] }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/ops/rules/candidates",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ rule: { rule_id: "candidate" } }),
      }),
    );
  });

  it("calls model operations endpoints", async () => {
    const fetchMock = mockFetch({ models: [] });

    await listModels("dev-secret");
    await getModelPromotionGates("baseline_fwa", "dev-secret");
    await getModelRetrainingReadiness("baseline_fwa", "dev-secret");
    await listModelRetrainingJobs("baseline_fwa", "dev-secret");
    await createModelRetrainingJob(
      "baseline_fwa",
      { requested_by: "model-ops", notes: "drift" },
      "dev-secret",
    );
    await updateModelRetrainingJobStatus(
      "job_1",
      { status: "running", actor: "trainer-worker", notes: "started" },
      "dev-secret",
    );
    await claimNextModelRetrainingJob(
      { actor: "trainer-worker", model_key: "baseline_fwa", notes: "claim next" },
      "dev-secret",
    );
    await completeModelRetrainingJob(
      "job_1",
      {
        actor: "trainer-worker",
        notes: "done",
        candidate_model_version: "0.2.0-candidate",
        artifact_uri: "s3://models/baseline_fwa/0.2.0-candidate/model.onnx",
        validation_report_uri: "s3://models/baseline_fwa/0.2.0-candidate/validation.json",
        evaluation_run_id: "eval_candidate",
        confusion_matrix_json: {},
        metrics_json: {},
      },
      "dev-secret",
    );
    await submitModelPromotionReview(
      "baseline_fwa",
      { decision: "approved", reviewer: "model-governance", notes: "shadow only" },
      "dev-secret",
    );
    await rollbackModel("baseline_fwa", "dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/models",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/models/baseline_fwa/promotion-gates",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/models/baseline_fwa/retraining-readiness",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/models/baseline_fwa/retraining-jobs",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/models/baseline_fwa/retraining-jobs",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ requested_by: "model-ops", notes: "drift" }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/model-retraining-jobs/job_1/status",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          status: "running",
          actor: "trainer-worker",
          notes: "started",
        }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/model-retraining-jobs/claim-next",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          actor: "trainer-worker",
          model_key: "baseline_fwa",
          notes: "claim next",
        }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/model-retraining-jobs/job_1/output",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          actor: "trainer-worker",
          notes: "done",
          candidate_model_version: "0.2.0-candidate",
          artifact_uri: "s3://models/baseline_fwa/0.2.0-candidate/model.onnx",
          validation_report_uri: "s3://models/baseline_fwa/0.2.0-candidate/validation.json",
          evaluation_run_id: "eval_candidate",
          confusion_matrix_json: {},
          metrics_json: {},
        }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/models/baseline_fwa/promotion-reviews",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          decision: "approved",
          reviewer: "model-governance",
          notes: "shadow only",
        }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/models/baseline_fwa/rollback",
      expect.objectContaining({
        method: "POST",
        body: "{}",
      }),
    );
  });

  it("calls dataset lineage endpoints", async () => {
    const fetchMock = mockFetch({ datasets: [], evaluations: [] });

    await listDatasets("dev-secret");
    await listFactorReadiness("dev-secret");
    await listModelEvaluations("dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/ops/datasets",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/factors/readiness",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/ops/model-evaluations",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("calls webhook delivery endpoints", async () => {
    const fetchMock = mockFetch({ events: [] });

    await listWebhookEvents("dev-secret");
    await submitWebhookDeliveryAttempt(
      "webhook_audit_1",
      {
        delivery_status: "failed",
        response_status_code: 503,
        error_message: "TPA unavailable",
      },
      "dev-secret",
    );

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/webhook-events",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/webhook-events/webhook_audit_1/delivery-attempts",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          delivery_status: "failed",
          response_status_code: 503,
          error_message: "TPA unavailable",
        }),
      }),
    );
  });

  it("calls dashboard summary endpoint", async () => {
    const fetchMock = mockFetch({ suspected_claims: 0 });

    await getDashboardSummary("dev-secret");
    await getProviderRiskSummary("dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/ops/dashboard/summary",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/providers/risk-summary",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("calls claim audit history endpoint", async () => {
    const fetchMock = mockFetch({ claim_id: "CLM-0287", events: [], runs: [] });

    await getClaimAuditHistory("CLM-0287", "dev-secret");
    await listAuditEvents("dev-secret", {
      limit: 25,
      event_type: "qa.result.received",
      actor_id: "qa_reviewer",
      run_id: "pilot_qa_QA-1",
      claim_id: "CLM-1",
      rule_id: "rule_early_claim",
      rule_version: 1,
      model_key: "baseline_fwa",
      model_version: "0.1.0",
      routing_policy_id: "fwa_risk_fusion_routing",
      routing_policy_version: 2,
      review_mode: "pre_payment",
    });
    await listGovernanceChangeEvents("dev-secret", 100);
    await listAgentRuns("dev-secret");
    await listOpsAlerts("dev-secret");
    await listFwaSchemes("dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/audit/claims/CLM-0287",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/audit-events?limit=25&event_type=qa.result.received&actor_id=qa_reviewer&run_id=pilot_qa_QA-1&claim_id=CLM-1&rule_id=rule_early_claim&rule_version=1&model_key=baseline_fwa&model_version=0.1.0&routing_policy_id=fwa_risk_fusion_routing&routing_policy_version=2&review_mode=pre_payment",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/ops/audit-events?limit=100&event_group=governance",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "/api/v1/ops/agent-runs",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      5,
      "/api/v1/ops/alerts",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      6,
      "/api/v1/ops/fwa-schemes",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("submits agent approval decisions", async () => {
    const fetchMock = mockFetch({ approval: {}, audit_id: "aud_1" });
    const payload = {
      decision: "approved",
      approver: "qa-lead",
      reason: "Evidence package is sufficient for manual review routing.",
      evidence_refs: ["agent_run:agent_CLM-0287"],
    };

    await submitAgentApproval("agent_CLM-0287", payload, "dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/agent-runs/agent_CLM-0287/approvals",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    );
  });

  it("calls lead and case lifecycle endpoints", async () => {
    const fetchMock = mockFetch({ leads: [], cases: [], case: {}, audit_id: "aud_1" });
    const triagePayload = {
      decision: "open_case",
      assignee: "siu-reviewer-1",
      reviewer: "medical-reviewer-1",
      priority: "high",
      notes: "Open investigation from high-risk FWA lead.",
    };

    await listLeads("dev-secret");
    await triageLead("lead_CLM-0287", triagePayload, "dev-secret");
    await listCases("dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/ops/leads",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/leads/lead_CLM-0287/triage",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(triagePayload),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/ops/cases",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("calls audit sampling endpoints", async () => {
    const fetchMock = mockFetch({ samples: [] });
    const payload = {
      sample_mode: "risk_ranked",
      population_definition: "RED and high risk leads for weekly QA",
      inclusion_criteria: { min_risk_score: 70 },
      deterministic_seed: "pilot-week-1",
      sample_size: 2,
      reviewer: "qa-reviewer-1",
      assignment_queue: "QA Review",
    };

    await listAuditSamples("dev-secret");
    await createAuditSample(payload, "dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/ops/audit-samples",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/audit-samples",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    );
  });

  it("posts QA review results", async () => {
    const fetchMock = mockFetch({ event_status: "accepted" });
    const payload = {
      qa_case_id: "QA-9001",
      claim_id: "CLM-0287",
      qa_conclusion: "issue_found_escalate",
      issue_type: "alert_handling_incomplete",
      feedback_target: "rules",
      notes: "Reviewer should attach provider history evidence.",
      evidence_refs: ["audit:scoring.completed"],
    };

    await submitQaResult(payload, "dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/qa/results",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(payload),
      }),
    );
  });

  it("lists QA feedback items", async () => {
    const fetchMock = mockFetch({ items: [] });

    await listQaFeedbackItems("dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/qa/feedback-items",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("lists QA review queue items", async () => {
    const fetchMock = mockFetch({ items: [] });

    await listQaQueue("dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/qa/queue",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("summarizes the QA feedback queue", async () => {
    const fetchMock = mockFetch({ open_count: 0 });

    await listQaQueueSummary("dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/qa/queue-summary",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("lists governed outcome labels", async () => {
    const fetchMock = mockFetch({ labels: [] });

    await listOutcomeLabels("dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/labels",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("calls knowledge and agent endpoints", async () => {
    const fetchMock = mockFetch({ cases: [], results: [], evidence_refs: [] });
    const searchPayload = {
      diagnosis_code: "J10",
      provider_region: "Shanghai",
      tags: ["early_claim"],
    };
    const investigationPayload = {
      claim_id: "CLM-0287",
      risk_score: 87,
      rag: "RED",
      top_reasons: ["金额高于同病种同地区 P99"],
      similar_case_query: searchPayload,
    };

    await listKnowledgeCases("dev-secret");
    await publishKnowledgeCase({ case_id: "KC-PUBLISHED-1" }, "dev-secret");
    await searchSimilarCases(searchPayload, "dev-secret");
    await investigateCase(investigationPayload, "dev-secret");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/ops/knowledge/cases",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/ops/knowledge/cases",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ case_id: "KC-PUBLISHED-1" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/knowledge/search-similar",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(searchPayload),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "/api/v1/agent/cases/investigate",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(investigationPayload),
      }),
    );
  });
});
