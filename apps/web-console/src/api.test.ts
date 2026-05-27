import { afterEach, describe, expect, it, vi } from "vitest";
import { approveRule, backtestRule, listModels, listRules, publishRule, submitRule } from "./api";

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

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/ops/rules/backtest",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify(payload),
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
});
