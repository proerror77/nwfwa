import { afterEach, describe, expect, it, vi } from "vitest";
import {
  approveRule,
  backtestRule,
  discoverRules,
  getDashboardSummary,
  investigateCase,
  listDatasets,
  listKnowledgeCases,
  listModelEvaluations,
  listModels,
  listRules,
  publishRule,
  searchSimilarCases,
  submitRule,
  submitQaResult,
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
      "/api/v1/ops/rules/rule_early_claim/submit",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/v1/ops/rules/rule_early_claim/approve",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "/api/v1/ops/rules/rule_early_claim/publish",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("posts rule backtest payload", async () => {
    const fetchMock = mockFetch({ sample_count: 0 });
    const payload = { rule: { rule_id: "candidate" }, samples: [] };

    await backtestRule(payload, "dev-secret");
    await discoverRules({ samples: [] }, "dev-secret");

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
  });

  it("calls model operations endpoints", async () => {
    const fetchMock = mockFetch({ models: [] });

    await listModels("dev-secret");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/models",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-api-key": "dev-secret" }),
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
