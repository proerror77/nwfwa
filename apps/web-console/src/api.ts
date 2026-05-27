async function requestJson(path: string, apiKey: string, init: RequestInit = {}) {
  const response = await fetch(path, {
    ...init,
    headers: {
      "content-type": "application/json",
      "x-api-key": apiKey,
      ...init.headers,
    },
  });

  const body = await response.json().catch(() => ({}));
  if (!response.ok) {
    if (body && typeof body === "object" && "message" in body) {
      throw new Error(String((body as { message: unknown }).message));
    }
    throw new Error(`HTTP ${response.status}`);
  }
  return body;
}

export async function scoreClaim(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/claims/score", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listRules(apiKey: string) {
  return requestJson("/api/v1/ops/rules", apiKey);
}

export async function getRule(ruleId: string, apiKey: string) {
  return requestJson(`/api/v1/ops/rules/${encodeURIComponent(ruleId)}`, apiKey);
}

export async function getRulePromotionGates(ruleId: string, apiKey: string) {
  return requestJson(
    `/api/v1/ops/rules/${encodeURIComponent(ruleId)}/promotion-gates`,
    apiKey,
  );
}

export async function submitRulePromotionReview(
  ruleId: string,
  payload: unknown,
  apiKey: string,
) {
  return requestJson(
    `/api/v1/ops/rules/${encodeURIComponent(ruleId)}/promotion-reviews`,
    apiKey,
    {
      method: "POST",
      body: JSON.stringify(payload),
    },
  );
}

export async function backtestRule(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/ops/rules/backtest", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function discoverRules(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/ops/rules/discover", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function saveRuleCandidate(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/ops/rules/candidates", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function submitRule(ruleId: string, apiKey: string) {
  return requestJson(`/api/v1/ops/rules/${encodeURIComponent(ruleId)}/submit`, apiKey, {
    method: "POST",
    body: "{}",
  });
}

export async function approveRule(ruleId: string, apiKey: string) {
  return requestJson(`/api/v1/ops/rules/${encodeURIComponent(ruleId)}/approve`, apiKey, {
    method: "POST",
    body: "{}",
  });
}

export async function publishRule(ruleId: string, apiKey: string) {
  return requestJson(`/api/v1/ops/rules/${encodeURIComponent(ruleId)}/publish`, apiKey, {
    method: "POST",
    body: "{}",
  });
}

export async function listModels(apiKey: string) {
  return requestJson("/api/v1/ops/models", apiKey);
}

export async function getModelPerformance(modelKey: string, apiKey: string) {
  return requestJson(
    `/api/v1/ops/models/${encodeURIComponent(modelKey)}/performance`,
    apiKey,
  );
}

export async function getModelPromotionGates(modelKey: string, apiKey: string) {
  return requestJson(
    `/api/v1/ops/models/${encodeURIComponent(modelKey)}/promotion-gates`,
    apiKey,
  );
}

export async function submitModelPromotionReview(
  modelKey: string,
  payload: unknown,
  apiKey: string,
) {
  return requestJson(
    `/api/v1/ops/models/${encodeURIComponent(modelKey)}/promotion-reviews`,
    apiKey,
    {
      method: "POST",
      body: JSON.stringify(payload),
    },
  );
}

export async function listDatasets(apiKey: string) {
  return requestJson("/api/v1/ops/datasets", apiKey);
}

export async function listModelEvaluations(apiKey: string) {
  return requestJson("/api/v1/ops/model-evaluations", apiKey);
}

export async function getDashboardSummary(apiKey: string) {
  return requestJson("/api/v1/ops/dashboard/summary", apiKey);
}

export async function getClaimAuditHistory(claimId: string, apiKey: string) {
  return requestJson(`/api/v1/audit/claims/${encodeURIComponent(claimId)}`, apiKey);
}

export async function listLeads(apiKey: string) {
  return requestJson("/api/v1/ops/leads", apiKey);
}

export async function triageLead(leadId: string, payload: unknown, apiKey: string) {
  return requestJson(`/api/v1/ops/leads/${encodeURIComponent(leadId)}/triage`, apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listCases(apiKey: string) {
  return requestJson("/api/v1/ops/cases", apiKey);
}

export async function listAuditSamples(apiKey: string) {
  return requestJson("/api/v1/ops/audit-samples", apiKey);
}

export async function createAuditSample(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/ops/audit-samples", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listKnowledgeCases(apiKey: string) {
  return requestJson("/api/v1/ops/knowledge/cases", apiKey);
}

export async function searchSimilarCases(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/knowledge/search-similar", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function investigateCase(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/agent/cases/investigate", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function submitQaResult(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/qa/results", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}
