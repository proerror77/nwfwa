import { afterEach, describe, expect, it, vi } from "vitest";
import {
  approveRule,
  backtestRule,
  createAuditSample,
  discoverRules,
  getDashboardSummary,
  getClaimAuditHistory,
  getRulePromotionGates,
  submitRulePromotionReview,
  investigateCase,
  listAuditSamples,
  listCases,
  listDatasets,
  listLeads,
  listKnowledgeCases,
  listQaFeedbackItems,
  listModelEvaluations,
  listModels,
  getModelPromotionGates,
  submitModelPromotionReview,
  listRules,
  publishRule,
  saveRuleCandidate,
  searchSimilarCases,
  submitRule,
  submitQaResult,
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
    await submitModelPromotionReview(
      "baseline_fwa",
      { decision: "approved", reviewer: "model-governance", notes: "shadow only" },
      "dev-secret",
    );

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
  });

  it("calls dataset lineage endpoints", async () => {
    const fetchMock = mockFetch({ datasets: [], evaluations: [] });

    await listDatasets("dev-secret");
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
      "/api/v1/ops/model-evaluations",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("calls dashboard summary endpoint", async () => {
    const fetchMock = mockFetch({ suspected_claims: 0 });

    await getDashboardSummary("dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/dashboard/summary",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
      }),
    );
  });

  it("calls claim audit history endpoint", async () => {
    const fetchMock = mockFetch({ claim_id: "CLM-0287", events: [] });

    await getClaimAuditHistory("CLM-0287", "dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/audit/claims/CLM-0287",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
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
      "/api/v1/knowledge/search-similar",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(searchPayload),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/agent/cases/investigate",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(investigationPayload),
      }),
    );
  });
});
