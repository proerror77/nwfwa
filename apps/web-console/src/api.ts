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

function ruleLifecycleBody(ruleId: string, version = 1) {
  return JSON.stringify({
    evidence_refs: [`rules:${ruleId}:v${version}`],
  });
}

export async function submitRule(ruleId: string, apiKey: string, version = 1) {
  return requestJson(`/api/v1/ops/rules/${encodeURIComponent(ruleId)}/submit`, apiKey, {
    method: "POST",
    body: ruleLifecycleBody(ruleId, version),
  });
}

export async function approveRule(ruleId: string, apiKey: string, version = 1) {
  return requestJson(`/api/v1/ops/rules/${encodeURIComponent(ruleId)}/approve`, apiKey, {
    method: "POST",
    body: ruleLifecycleBody(ruleId, version),
  });
}

export async function publishRule(ruleId: string, apiKey: string, version = 1) {
  return requestJson(`/api/v1/ops/rules/${encodeURIComponent(ruleId)}/publish`, apiKey, {
    method: "POST",
    body: ruleLifecycleBody(ruleId, version),
  });
}

export async function rollbackRule(ruleId: string, apiKey: string, version = 1) {
  return requestJson(`/api/v1/ops/rules/${encodeURIComponent(ruleId)}/rollback`, apiKey, {
    method: "POST",
    body: ruleLifecycleBody(ruleId, version),
  });
}

export async function listModels(apiKey: string) {
  return requestJson("/api/v1/ops/models", apiKey);
}

type RoutingPolicyRef = {
  policy_id: string;
  review_mode: string;
  version: number;
};

function routingPolicyLifecyclePath(policy: RoutingPolicyRef, action: string) {
  return `/api/v1/ops/routing-policies/${encodeURIComponent(policy.policy_id)}/${encodeURIComponent(policy.review_mode)}/${encodeURIComponent(policy.version)}/${action}`;
}

function routingPolicyLifecycleBody(policy: RoutingPolicyRef) {
  return JSON.stringify({
    evidence_refs: [
      `routing_policies:${policy.policy_id}:v${policy.version}:${policy.review_mode}`,
    ],
  });
}

export async function listRoutingPolicies(apiKey: string) {
  return requestJson("/api/v1/ops/routing-policies", apiKey);
}

export async function saveRoutingPolicyCandidate(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/ops/routing-policies", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function getRoutingPolicyPromotionGates(
  policy: RoutingPolicyRef,
  apiKey: string,
) {
  return requestJson(routingPolicyLifecyclePath(policy, "promotion-gates"), apiKey);
}

export async function submitRoutingPolicy(policy: RoutingPolicyRef, apiKey: string) {
  return requestJson(routingPolicyLifecyclePath(policy, "submit"), apiKey, {
    method: "POST",
    body: routingPolicyLifecycleBody(policy),
  });
}

export async function approveRoutingPolicy(policy: RoutingPolicyRef, apiKey: string) {
  return requestJson(routingPolicyLifecyclePath(policy, "approve"), apiKey, {
    method: "POST",
    body: routingPolicyLifecycleBody(policy),
  });
}

export async function activateRoutingPolicy(policy: RoutingPolicyRef, apiKey: string) {
  return requestJson(routingPolicyLifecyclePath(policy, "activate"), apiKey, {
    method: "POST",
    body: routingPolicyLifecycleBody(policy),
  });
}

export async function rollbackRoutingPolicy(policy: RoutingPolicyRef, apiKey: string) {
  return requestJson(routingPolicyLifecyclePath(policy, "rollback"), apiKey, {
    method: "POST",
    body: routingPolicyLifecycleBody(policy),
  });
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

export async function getModelRetrainingReadiness(modelKey: string, apiKey: string) {
  return requestJson(
    `/api/v1/ops/models/${encodeURIComponent(modelKey)}/retraining-readiness`,
    apiKey,
  );
}

export async function listModelRetrainingJobs(modelKey: string, apiKey: string) {
  return requestJson(
    `/api/v1/ops/models/${encodeURIComponent(modelKey)}/retraining-jobs`,
    apiKey,
  );
}

export async function createModelRetrainingJob(
  modelKey: string,
  payload: unknown,
  apiKey: string,
) {
  return requestJson(
    `/api/v1/ops/models/${encodeURIComponent(modelKey)}/retraining-jobs`,
    apiKey,
    {
      method: "POST",
      body: JSON.stringify(payload),
    },
  );
}

export async function updateModelRetrainingJobStatus(
  jobId: string,
  payload: unknown,
  apiKey: string,
) {
  return requestJson(
    `/api/v1/ops/model-retraining-jobs/${encodeURIComponent(jobId)}/status`,
    apiKey,
    {
      method: "POST",
      body: JSON.stringify(payload),
    },
  );
}

export async function claimNextModelRetrainingJob(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/ops/model-retraining-jobs/claim-next", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function completeModelRetrainingJob(
  jobId: string,
  payload: unknown,
  apiKey: string,
) {
  return requestJson(
    `/api/v1/ops/model-retraining-jobs/${encodeURIComponent(jobId)}/output`,
    apiKey,
    {
      method: "POST",
      body: JSON.stringify(payload),
    },
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

function modelLifecycleBody(modelKey: string, version: string) {
  return JSON.stringify({
    evidence_refs: [`model_versions:${modelKey}:${version}`],
  });
}

export async function activateModel(modelKey: string, version: string, apiKey: string) {
  return requestJson(`/api/v1/ops/models/${encodeURIComponent(modelKey)}/activate`, apiKey, {
    method: "POST",
    body: modelLifecycleBody(modelKey, version),
  });
}

export async function rollbackModel(modelKey: string, version: string, apiKey: string) {
  return requestJson(`/api/v1/ops/models/${encodeURIComponent(modelKey)}/rollback`, apiKey, {
    method: "POST",
    body: modelLifecycleBody(modelKey, version),
  });
}

export async function listDatasets(apiKey: string) {
  return requestJson("/api/v1/ops/datasets", apiKey);
}

export async function listFactorReadiness(apiKey: string) {
  return requestJson("/api/v1/ops/factors/readiness", apiKey);
}

export async function listModelEvaluations(apiKey: string) {
  return requestJson("/api/v1/ops/model-evaluations", apiKey);
}

export async function getDashboardSummary(apiKey: string) {
  return requestJson("/api/v1/ops/dashboard/summary", apiKey);
}

export async function getProviderRiskSummary(apiKey: string) {
  return requestJson("/api/v1/ops/providers/risk-summary", apiKey);
}

export async function getMemberProfileSummary(memberId: string, apiKey: string) {
  return requestJson(
    `/api/v1/members/${encodeURIComponent(memberId)}/profile-summary`,
    apiKey,
  );
}

export async function listMedicalReviewQueue(apiKey: string, limit = 100) {
  return requestJson(`/api/v1/ops/medical-review/queue?limit=${encodeURIComponent(limit)}`, apiKey);
}

export async function submitMedicalReviewResult(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/ops/medical-review/results", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listFwaSchemes(apiKey: string) {
  return requestJson("/api/v1/ops/fwa-schemes", apiKey);
}

export async function getClaimAuditHistory(claimId: string, apiKey: string) {
  return requestJson(`/api/v1/audit/claims/${encodeURIComponent(claimId)}`, apiKey);
}

export type AuditEventListFilters = {
  limit?: number;
  event_group?: string;
  event_type?: string;
  actor_id?: string;
  run_id?: string;
  claim_id?: string;
  rule_id?: string;
  rule_version?: number | string;
  model_key?: string;
  model_version?: string;
  routing_policy_id?: string;
  routing_policy_version?: number | string;
  review_mode?: string;
  feedback_id?: string;
  qa_case_id?: string;
};

export async function listAuditEvents(
  apiKey: string,
  filters: AuditEventListFilters | number = 50,
) {
  const query =
    typeof filters === "number"
      ? { limit: filters }
      : { limit: 50, ...filters };
  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (value !== undefined && String(value).trim().length > 0) {
      params.set(key, String(value));
    }
  });
  return requestJson(`/api/v1/ops/audit-events?${params.toString()}`, apiKey);
}

export async function listGovernanceChangeEvents(apiKey: string, limit = 100) {
  return listAuditEvents(apiKey, { limit, event_group: "governance" });
}

export async function listWebhookEvents(apiKey: string) {
  return requestJson("/api/v1/ops/webhook-events", apiKey);
}

export async function submitWebhookDeliveryAttempt(
  eventId: string,
  payload: unknown,
  apiKey: string,
) {
  return requestJson(
    `/api/v1/ops/webhook-events/${encodeURIComponent(eventId)}/delivery-attempts`,
    apiKey,
    {
      method: "POST",
      body: JSON.stringify(payload),
    },
  );
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

export async function updateCaseStatus(caseId: string, payload: unknown, apiKey: string) {
  return requestJson(`/api/v1/ops/cases/${encodeURIComponent(caseId)}/status`, apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listAuditSamples(apiKey: string) {
  return requestJson("/api/v1/ops/audit-samples", apiKey);
}

export async function listAgentRuns(apiKey: string) {
  return requestJson("/api/v1/ops/agent-runs", apiKey);
}

export async function submitAgentApproval(agentRunId: string, payload: unknown, apiKey: string) {
  return requestJson(`/api/v1/ops/agent-runs/${encodeURIComponent(agentRunId)}/approvals`, apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listOpsAlerts(apiKey: string) {
  return requestJson("/api/v1/ops/alerts", apiKey);
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

export async function publishKnowledgeCase(payload: unknown, apiKey: string) {
  return requestJson("/api/v1/ops/knowledge/cases", apiKey, {
    method: "POST",
    body: JSON.stringify(payload),
  });
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

export async function listQaFeedbackItems(
  apiKey: string,
  filters: { status?: string; feedbackTarget?: string } = {},
) {
  const params = new URLSearchParams();
  if (filters.status) {
    params.set("status", filters.status);
  }
  if (filters.feedbackTarget) {
    params.set("feedback_target", filters.feedbackTarget);
  }
  const query = params.toString();
  return requestJson(`/api/v1/ops/qa/feedback-items${query ? `?${query}` : ""}`, apiKey);
}

export async function updateQaFeedbackStatus(
  feedbackId: string,
  payload: unknown,
  apiKey: string,
) {
  return requestJson(
    `/api/v1/ops/qa/feedback-items/${encodeURIComponent(feedbackId)}/status`,
    apiKey,
    {
      method: "POST",
      body: JSON.stringify(payload),
    },
  );
}

export async function listQaQueue(apiKey: string) {
  return requestJson("/api/v1/ops/qa/queue", apiKey);
}

export async function listQaQueueSummary(apiKey: string) {
  return requestJson("/api/v1/ops/qa/queue-summary", apiKey);
}

export async function listOutcomeLabels(apiKey: string) {
  return requestJson("/api/v1/ops/labels", apiKey);
}
