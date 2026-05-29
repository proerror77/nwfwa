import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getModelPromotionGates,
  getClaimAuditHistory,
  getRoutingPolicyPromotionGates,
  getRulePromotionGates,
  listAuditEvents,
  listAgentRuns,
  listFwaSchemes,
  listGovernanceChangeEvents,
  listModels,
  listOpsAlerts,
  listOutcomeLabels,
  listRoutingPolicies,
  listRules,
  listWebhookEvents,
  submitAgentApproval,
  submitWebhookDeliveryAttempt,
  type AuditEventListFilters,
} from "../api";
import { buildFwaSchemeLabelMap, formatFwaSchemeLabel } from "./fwaSchemeOptions";
import { formatReviewModeLabel } from "./reviewMode";

type AuditEvent = {
  audit_id: string;
  run_id: string;
  event_type: string;
  event_status: string;
  summary: string;
  payload?: Record<string, unknown>;
  evidence_refs: string[];
  created_at?: string | null;
};

type ClaimAuditHistoryResponse = {
  claim_id: string;
  events: AuditEvent[];
};

type AuditEventListResponse = {
  events: AuditEvent[];
};

type AgentRunLog = {
  agent_run_id: string;
  claim_id: string;
  status: string;
  decision_boundary: string;
  evidence_refs: string[];
  steps: Array<Record<string, unknown>>;
  context_snapshots: AgentContextSnapshot[];
  policy_checks: AgentPolicyCheck[];
  tool_calls: AgentToolCall[];
  tool_results: AgentToolResult[];
  approvals: AgentApproval[];
  created_at?: string | null;
  completed_at?: string | null;
};

type AgentContextSnapshot = {
  snapshot_id: string;
  redaction_status: string;
  context_json: Record<string, unknown>;
  source_refs: string[];
  checksum: string;
};

type AgentToolCall = {
  tool_call_id: string;
  tool_name: string;
  status: string;
  input_json: Record<string, unknown>;
  evidence_refs: string[];
};

type AgentPolicyCheck = {
  policy_check_id: string;
  agent_run_id: string;
  tool_call_id: string;
  tool_name: string;
  policy_name: string;
  decision: string;
  reason: string;
  evidence_refs: string[];
  created_at?: string | null;
};

type AgentToolResult = {
  tool_result_id: string;
  tool_call_id: string;
  tool_name: string;
  status: string;
  output_json: Record<string, unknown>;
  evidence_refs: string[];
};

type AgentApproval = {
  approval_id: string;
  agent_run_id: string;
  proposed_action: string;
  decision: string;
  approver: string;
  reason: string;
  evidence_refs: string[];
  created_at?: string | null;
};

type AgentRunLogListResponse = {
  runs: AgentRunLog[];
};

type OpsAlert = {
  alert_id: string;
  alert_type: string;
  severity: string;
  status: string;
  claim_id: string;
  lead_id?: string | null;
  case_id?: string | null;
  scheme_family: string;
  message: string;
  recommended_action: string;
  evidence_refs: string[];
};

type OpsAlertListResponse = {
  alerts: OpsAlert[];
};

type WebhookEvent = {
  event_id: string;
  event_type: string;
  source_event_type: string;
  source_audit_id: string;
  claim_id: string;
  run_id: string;
  delivery_status: string;
  retry_count: number;
  max_attempts: number;
  next_attempt_at?: string | null;
  last_attempt_at?: string | null;
  last_response_status_code?: number | null;
  last_error_message?: string | null;
  idempotency_key: string;
  signature_key_id: string;
  signature_algorithm: string;
  signature_base_string: string;
  evidence_refs: string[];
  occurred_at?: string | null;
};

type WebhookEventListResponse = {
  events: WebhookEvent[];
};

export type OutcomeLabel = {
  label_id: string;
  claim_id: string;
  label_name: string;
  label_value: string;
  source_type: string;
  source_id: string;
  governance_status: string;
  feedback_target: string;
  currency?: string | null;
  evidence_refs: string[];
};

export type OutcomeLabelFilters = {
  sourceType?: string;
  feedbackTarget?: string;
  governanceStatus?: string;
};

type OutcomeLabelListResponse = {
  labels: OutcomeLabel[];
};

type PromotionGate = {
  label: string;
  passed: boolean;
  blocker: string;
  evidence_source: string;
};

type PromotionGateResponse = {
  decision: string;
  passed_count: number;
  total_count: number;
  review_mode?: string;
  status?: string;
  blockers?: string[];
  gates?: PromotionGate[];
};

type PromotionGateGovernanceItem = {
  domain: "Rule" | "Model" | "Routing";
  target_id: string;
  status?: string;
  review_mode?: string;
  response: PromotionGateResponse;
};

export type PromotionGateGovernanceRow = {
  domain: string;
  targetId: string;
  status: string;
  reviewMode: string;
  decision: string;
  passedCount: number;
  totalCount: number;
  blockerCount: number;
  topBlocker: string;
  evidenceSources: string;
};

type RuleSummary = {
  rule_id: string;
  status: string;
  review_mode: string;
};

type ModelVersion = {
  model_key: string;
  version: string;
  status: string;
  review_mode: string;
};

type RoutingPolicyRecord = {
  policy_id: string;
  version: number;
  review_mode: string;
  status: string;
};

type PromotionGateGovernanceResponse = {
  rows: PromotionGateGovernanceRow[];
};

type FwaSchemeDefinition = {
  scheme_family: string;
  display_name: string;
  risk_domain: string;
  description: string;
  minimum_evidence: string[];
  default_review_route: string;
  primary_layers: string[];
};

type FwaSchemeListResponse = {
  schemes: FwaSchemeDefinition[];
  scheme_count: number;
};

export type FwaSchemeGovernanceRow = {
  schemeFamily: string;
  displayName: string;
  riskDomain: string;
  defaultReviewRoute: string;
  evidenceCount: number;
  minimumEvidence: string;
  primaryLayers: string;
};

export type GovernanceChangeTimelineRow = {
  auditId: string;
  domain: string;
  eventType: string;
  targetId: string;
  statusTransition: string;
  actor: string;
  decision: string;
  summary: string;
  createdAt: string;
  evidenceRefs: string[];
};

export type GlobalAuditEventFilterState = {
  eventType: string;
  actorId: string;
  runId: string;
  claimId: string;
  feedbackId: string;
  qaCaseId: string;
  sampleId: string;
  ruleId: string;
  ruleVersion: string;
  modelKey: string;
  modelVersion: string;
  routingPolicyId: string;
  routingPolicyVersion: string;
  reviewMode: string;
  limit: string;
};

const governanceChangeEventTypes = new Set([
  "dataset.registered",
  "dataset.field_mapping.added",
  "feature_set.registered",
  "model_dataset.registered",
  "model_evaluation.registered",
  "rule.candidate.saved",
  "rule.status.changed",
  "rule.rollback.completed",
  "rule.promotion.reviewed",
  "model.promotion.reviewed",
  "model.activation.completed",
  "model.rollback.completed",
  "agent.approval.decided",
  "audit_sample.created",
  "qa.feedback.status.updated",
  "routing_policy.candidate.saved",
  "routing_policy.status.changed",
  "routing_policy.activation.completed",
  "routing_policy.rollback.completed",
]);

export const auditEventFilterShortcuts = [
  { label: "Scoring", eventType: "scoring.completed" },
  { label: "QA Results", eventType: "qa.result.received" },
  { label: "QA Feedback Status", eventType: "qa.feedback.status.updated" },
  { label: "Case Status", eventType: "case.status.updated" },
  { label: "Rule Candidates", eventType: "rule.candidate.saved" },
  { label: "Audit Samples", eventType: "audit_sample.created" },
];

export function buildGlobalAuditEventFilters(
  filters: GlobalAuditEventFilterState,
  limit: number,
): AuditEventListFilters {
  return {
    limit,
    event_type: filters.eventType,
    actor_id: filters.actorId,
    run_id: filters.runId,
    claim_id: filters.claimId,
    feedback_id: filters.feedbackId,
    qa_case_id: filters.qaCaseId,
    sample_id: filters.sampleId,
    rule_id: filters.ruleId,
    rule_version: filters.ruleVersion,
    model_key: filters.modelKey,
    model_version: filters.modelVersion,
    routing_policy_id: filters.routingPolicyId,
    routing_policy_version: filters.routingPolicyVersion,
    review_mode: filters.reviewMode,
  };
}

export function buildAuditSummary(data?: { events: AuditEvent[]; claim_id?: string }) {
  const events = data?.events ?? [];
  const latestDatedEvent = events.filter((event) => event.created_at).reduce<
    AuditEvent | undefined
  >((latest, event) => {
    if (!latest) return event;
    return String(event.created_at) > String(latest.created_at) ? event : latest;
  }, undefined);
  return {
    totalEvents: events.length,
    succeededEvents: events.filter((event) => event.event_status === "succeeded").length,
    failedEvents: events.filter((event) => event.event_status === "failed").length,
    latestEventType: latestDatedEvent?.event_type ?? events.at(-1)?.event_type ?? "none",
  };
}

function payloadString(payload: Record<string, unknown> | undefined, key: string) {
  const value = payload?.[key];
  if (value === undefined || value === null) return "";
  return String(value);
}

function governanceChangeDomain(eventType: string) {
  if (
    eventType.startsWith("dataset.") ||
    eventType.startsWith("feature_set.") ||
    eventType.startsWith("model_dataset.") ||
    eventType.startsWith("model_evaluation.")
  ) {
    return "Data";
  }
  if (eventType.startsWith("rule.")) return "Rule";
  if (eventType.startsWith("model.")) return "Model";
  if (eventType.startsWith("agent.")) return "Agent";
  if (eventType.startsWith("audit_sample.")) return "QA";
  if (eventType.startsWith("qa.")) return "QA";
  if (eventType.startsWith("routing_policy.")) return "Routing";
  return "Governance";
}

function governanceChangeTargetId(event: AuditEvent) {
  const payload = event.payload;
  if (event.event_type === "dataset.registered") {
    const datasetKey = payloadString(payload, "dataset_key");
    const version = payloadString(payload, "dataset_version");
    return version ? `${datasetKey}@${version}` : datasetKey;
  }
  if (event.event_type === "dataset.field_mapping.added") {
    return [
      payloadString(payload, "dataset_id"),
      payloadString(payload, "feature_name") || payloadString(payload, "external_field"),
    ]
      .filter(Boolean)
      .join(" / ");
  }
  if (event.event_type === "feature_set.registered") {
    const featureSetKey = payloadString(payload, "feature_set_key");
    const version = payloadString(payload, "version");
    return version ? `${featureSetKey}@${version}` : featureSetKey;
  }
  if (event.event_type === "model_dataset.registered") {
    return payloadString(payload, "model_dataset_id");
  }
  if (event.event_type === "model_evaluation.registered") {
    return [
      payloadString(payload, "model_key"),
      payloadString(payload, "model_version"),
      payloadString(payload, "evaluation_run_id"),
    ]
      .filter(Boolean)
      .join(" / ");
  }
  if (event.event_type.startsWith("rule.")) {
    const ruleId = payloadString(payload, "rule_id");
    const version = payloadString(payload, "rule_version");
    return version ? `${ruleId}@v${version}` : ruleId;
  }
  if (event.event_type.startsWith("model.")) {
    const modelKey = payloadString(payload, "model_key");
    const version = payloadString(payload, "model_version");
    return version ? `${modelKey}@${version}` : modelKey;
  }
  if (event.event_type === "qa.feedback.status.updated") {
    return (
      payloadString(payload, "feedback_id") ||
      payloadString(payload, "qa_case_id") ||
      payloadString(payload, "claim_id")
    );
  }
  if (event.event_type === "agent.approval.decided") {
    return [payloadString(payload, "agent_run_id"), payloadString(payload, "proposed_action")]
      .filter(Boolean)
      .join(" / ");
  }
  if (event.event_type === "audit_sample.created") {
    return payloadString(payload, "sample_id");
  }
  if (event.event_type.startsWith("routing_policy.")) {
    const policyId = payloadString(payload, "policy_id");
    const version = payloadString(payload, "version");
    const reviewMode = payloadString(payload, "review_mode");
    return [version ? `${policyId}@v${version}` : policyId, reviewMode]
      .filter(Boolean)
      .join(" / ");
  }
  return payloadString(payload, "id") || event.run_id;
}

export function buildGovernanceChangeTimelineRows(
  events: AuditEvent[] = [],
): GovernanceChangeTimelineRow[] {
  return events
    .filter((event) => governanceChangeEventTypes.has(event.event_type))
    .map((event) => {
      const fromStatus = payloadString(event.payload, "from_status") || "-";
      const toStatus = payloadString(event.payload, "to_status") || "-";
      const decision = payloadString(event.payload, "decision");
      const sampleMode = payloadString(event.payload, "sample_mode");
      const selectionMethod = payloadString(event.payload, "selection_method");
      return {
        auditId: event.audit_id,
        domain: governanceChangeDomain(event.event_type),
        eventType: event.event_type,
        targetId: governanceChangeTargetId(event),
        statusTransition:
          event.event_type === "audit_sample.created"
            ? `created -> ${sampleMode || "sample"}`
            : fromStatus === "-" && toStatus === "-" && decision
              ? `review -> ${decision}`
              : `${fromStatus} -> ${toStatus}`,
        actor:
          payloadString(event.payload, "reviewer") ||
          payloadString(event.payload, "owner") ||
          payloadString(event.payload, "actor_id") ||
          payloadString(event.payload, "approver") ||
          payloadString(event.payload, "requested_by") ||
          "system",
        decision: decision || selectionMethod || toStatus,
        summary: event.summary,
        createdAt: event.created_at ?? event.run_id,
        evidenceRefs: event.evidence_refs,
      };
    });
}

export function buildPromotionGateGovernanceRows(
  items: PromotionGateGovernanceItem[] = [],
): PromotionGateGovernanceRow[] {
  return items.map((item) => {
    const gates = item.response.gates ?? [];
    const blockers =
      item.response.blockers ?? gates.filter((gate) => !gate.passed).map((gate) => gate.blocker);
    const evidenceSources = Array.from(
      new Set(gates.map((gate) => gate.evidence_source || "missing")),
    );
    return {
      domain: item.domain,
      targetId: item.target_id,
      status: item.status ?? item.response.status ?? "unknown",
      reviewMode: item.review_mode ?? item.response.review_mode ?? "unknown",
      decision: item.response.decision,
      passedCount: item.response.passed_count,
      totalCount: item.response.total_count,
      blockerCount: blockers.length,
      topBlocker: blockers[0] ?? "none",
      evidenceSources: evidenceSources.length ? evidenceSources.join(", ") : "missing",
    };
  });
}

export function buildPromotionGateGovernanceSummary(
  rows: PromotionGateGovernanceRow[] = [],
) {
  return {
    targetCount: rows.length,
    allowedTargetCount: rows.filter((row) => row.blockerCount === 0).length,
    blockedTargetCount: rows.filter((row) => row.blockerCount > 0).length,
    passedGateCount: rows.reduce((total, row) => total + row.passedCount, 0),
    totalGateCount: rows.reduce((total, row) => total + row.totalCount, 0),
    blockerCount: rows.reduce((total, row) => total + row.blockerCount, 0),
  };
}

export function buildFwaSchemeGovernanceRows(
  schemes: FwaSchemeDefinition[] = [],
): FwaSchemeGovernanceRow[] {
  return [...schemes]
    .sort(
      (left, right) =>
        left.risk_domain.localeCompare(right.risk_domain) ||
        left.scheme_family.localeCompare(right.scheme_family),
    )
    .map((scheme) => ({
      schemeFamily: scheme.scheme_family,
      displayName: scheme.display_name,
      riskDomain: scheme.risk_domain,
      defaultReviewRoute: scheme.default_review_route,
      evidenceCount: scheme.minimum_evidence.length,
      minimumEvidence: scheme.minimum_evidence.join(", "),
      primaryLayers: scheme.primary_layers.join(", "),
    }));
}

export function buildFwaSchemeGovernanceSummary(rows: FwaSchemeGovernanceRow[] = []) {
  return {
    schemeCount: rows.length,
    domainCount: new Set(rows.map((row) => row.riskDomain)).size,
    evidenceRequirementCount: rows.reduce((total, row) => total + row.evidenceCount, 0),
    medicalReviewCount: rows.filter((row) => row.defaultReviewRoute === "medical_review").length,
    providerReviewCount: rows.filter((row) => row.defaultReviewRoute === "provider_review").length,
  };
}

async function loadPromotionGateGovernance(
  apiKey: string,
): Promise<PromotionGateGovernanceResponse> {
  const [ruleList, modelList, routingPolicyList] = await Promise.all([
    listRules(apiKey) as Promise<{ rules: RuleSummary[] }>,
    listModels(apiKey) as Promise<{ models: ModelVersion[] }>,
    listRoutingPolicies(apiKey) as Promise<{ policies: RoutingPolicyRecord[] }>,
  ]);
  const items = await Promise.all([
    ...ruleList.rules.map(async (rule) => ({
      domain: "Rule" as const,
      target_id: rule.rule_id,
      status: rule.status,
      review_mode: rule.review_mode,
      response: (await getRulePromotionGates(rule.rule_id, apiKey)) as PromotionGateResponse,
    })),
    ...modelList.models.map(async (model) => ({
      domain: "Model" as const,
      target_id: `${model.model_key}@${model.version}`,
      status: model.status,
      review_mode: model.review_mode,
      response: (await getModelPromotionGates(model.model_key, apiKey)) as PromotionGateResponse,
    })),
    ...routingPolicyList.policies.map(async (policy) => ({
      domain: "Routing" as const,
      target_id: `${policy.policy_id}@v${policy.version}`,
      status: policy.status,
      review_mode: policy.review_mode,
      response: (await getRoutingPolicyPromotionGates(policy, apiKey)) as PromotionGateResponse,
    })),
  ]);
  return { rows: buildPromotionGateGovernanceRows(items) };
}

export function buildAgentRunLogSummary(runs: AgentRunLog[] = []) {
  const contextSnapshots = runs.flatMap((run) => run.context_snapshots ?? []);
  const policyChecks = runs.flatMap((run) => run.policy_checks ?? []);
  const toolCalls = runs.flatMap((run) => run.tool_calls ?? []);
  const toolResults = runs.flatMap((run) => run.tool_results ?? []);
  const approvals = runs.flatMap((run) => run.approvals ?? []);
  return {
    runCount: runs.length,
    contextSnapshotCount: contextSnapshots.length,
    piiMaskedContextCount: contextSnapshots.filter(
      (snapshot) => snapshot.redaction_status === "pii_masked",
    ).length,
    toolCallCount: toolCalls.length,
    toolResultCount: toolResults.length,
    failedToolCallCount: toolCalls.filter((call) => call.status === "failed").length,
    policyCheckCount: policyChecks.length,
    deniedPolicyCheckCount: policyChecks.filter((check) => check.decision === "denied").length,
    approvalCount: approvals.length,
    pendingApprovalCount: approvals.filter((approval) => approval.decision === "pending").length,
  };
}

export function hasPendingAgentApproval(run: AgentRunLog) {
  return run.approvals.some((approval) => approval.decision === "pending");
}

export function canSubmitAgentApproval(run: AgentRunLog, approver: string) {
  return hasPendingAgentApproval(run) && approver.trim().length > 0;
}

export function buildAgentApprovalPayload(
  run: AgentRunLog,
  decision: "approved" | "rejected",
  approver: string,
) {
  const pendingApproval = run.approvals.find((approval) => approval.decision === "pending");
  const evidenceRefs = [
    ...(pendingApproval?.evidence_refs ?? []),
    ...run.evidence_refs,
    `agent_run:${run.agent_run_id}`,
  ].filter((value, index, refs) => refs.indexOf(value) === index);
  return {
    decision,
    approver: approver.trim(),
    reason:
      decision === "approved"
        ? "Evidence package approved for manual review routing."
        : "Evidence package rejected pending stronger support.",
    evidence_refs: evidenceRefs,
  };
}

export function buildOpsAlertSummary(alerts: OpsAlert[] = []) {
  return {
    alertCount: alerts.length,
    openAlertCount: alerts.filter((alert) => alert.status === "open").length,
    criticalAlertCount: alerts.filter((alert) => alert.severity === "critical").length,
    routingAlertCount: alerts.filter((alert) => alert.alert_type === "high_risk_routing").length,
    slaBreachCount: alerts.filter((alert) => alert.alert_type === "sla_breach").length,
    medicalReviewAlertCount: alerts.filter(
      (alert) => alert.alert_type === "medical_review_required",
    ).length,
    agentApprovalAlertCount: alerts.filter(
      (alert) => alert.alert_type === "agent_approval_pending",
    ).length,
  };
}

export function buildWebhookDeliverySummary(events: WebhookEvent[] = []) {
  return {
    eventCount: events.length,
    pendingCount: events.filter((event) => event.delivery_status === "pending").length,
    retryWaitCount: events.filter((event) => event.delivery_status === "retry_wait").length,
    deliveredCount: events.filter((event) => event.delivery_status === "delivered").length,
    failedCount: events.filter((event) => event.delivery_status === "failed").length,
    signedCount: events.filter(
      (event) => event.signature_algorithm === "hmac-sha256" && event.signature_key_id.length > 0,
    ).length,
  };
}

export function canRecordWebhookDeliveryAttempt(event: WebhookEvent) {
  return event.delivery_status === "pending" || event.delivery_status === "retry_wait";
}

export function buildOutcomeLabelSummary(labels: OutcomeLabel[] = []) {
  const amountPreventedLabels = labels.filter((label) => label.label_name === "amount_prevented");
  const amountPreventedTotal = amountPreventedLabels.reduce((total, label) => {
    const value = Number(label.label_value);
    return Number.isFinite(value) ? total + value : total;
  }, 0);
  const sourceTypeCounts = labels.reduce<Record<string, number>>((counts, label) => {
    counts[label.source_type] = (counts[label.source_type] ?? 0) + 1;
    return counts;
  }, {});
  return {
    labelCount: labels.length,
    approvedForTrainingCount: labels.filter(
      (label) => label.governance_status === "approved_for_training",
    ).length,
    needsReviewCount: labels.filter((label) => label.governance_status === "needs_review").length,
    modelFeedbackCount: labels.filter((label) => label.feedback_target === "models").length,
    ruleFeedbackCount: labels.filter((label) => label.feedback_target === "rules").length,
    falsePositiveCount: labels.filter((label) => label.label_name === "false_positive").length,
    caseStatusLabelCount: labels.filter((label) => label.source_type === "case_status").length,
    medicalReviewLabelCount: labels.filter((label) => label.source_type === "medical_review")
      .length,
    evidenceBackedCount: labels.filter((label) => label.evidence_refs.length > 0).length,
    sourceTypeRows: Object.entries(sourceTypeCounts)
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .map(([sourceType, count]) => `${sourceType}: ${count}`),
    amountPreventedTotal,
    amountPreventedCurrency: amountPreventedLabels[0]?.currency ?? "N/A",
  };
}

export function filterOutcomeLabels(
  labels: OutcomeLabel[] = [],
  filters: OutcomeLabelFilters = {},
) {
  const sourceType = filters.sourceType?.trim();
  const feedbackTarget = filters.feedbackTarget?.trim();
  const governanceStatus = filters.governanceStatus?.trim();
  return labels.filter(
    (label) =>
      (!sourceType || label.source_type === sourceType) &&
      (!feedbackTarget || label.feedback_target === feedbackTarget) &&
      (!governanceStatus || label.governance_status === governanceStatus),
  );
}

function sortedUniqueLabels(labels: OutcomeLabel[], field: keyof OutcomeLabel) {
  return Array.from(new Set(labels.map((label) => String(label[field])).filter(Boolean))).sort();
}

export function GovernancePage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [claimId, setClaimId] = useState("CLM-0287");
  const [agentApprover, setAgentApprover] = useState("qa-lead");
  const [outcomeLabelFilters, setOutcomeLabelFilters] = useState<OutcomeLabelFilters>({
    sourceType: "",
    feedbackTarget: "",
    governanceStatus: "",
  });
  const [auditEventFilters, setAuditEventFilters] = useState<GlobalAuditEventFilterState>({
    eventType: "",
    actorId: "",
    runId: "",
    claimId: "",
    feedbackId: "",
    qaCaseId: "",
    sampleId: "",
    ruleId: "",
    ruleVersion: "",
    modelKey: "",
    modelVersion: "",
    routingPolicyId: "",
    routingPolicyVersion: "",
    reviewMode: "",
    limit: "50",
  });
  const parsedAuditEventLimit = Number.parseInt(auditEventFilters.limit, 10);
  const auditEventLimit = Number.isFinite(parsedAuditEventLimit)
    ? Math.min(Math.max(parsedAuditEventLimit, 1), 200)
    : 50;
  const queryClient = useQueryClient();
  const auditQuery = useQuery({
    queryKey: ["claim-audit-history", apiKey, claimId],
    queryFn: () => getClaimAuditHistory(claimId, apiKey) as Promise<ClaimAuditHistoryResponse>,
    enabled: claimId.trim().length > 0,
  });
  const globalAuditQuery = useQuery({
    queryKey: [
      "global-audit-events",
      apiKey,
      auditEventLimit,
      auditEventFilters.eventType,
      auditEventFilters.actorId,
      auditEventFilters.runId,
      auditEventFilters.claimId,
      auditEventFilters.feedbackId,
      auditEventFilters.qaCaseId,
      auditEventFilters.sampleId,
      auditEventFilters.ruleId,
      auditEventFilters.ruleVersion,
      auditEventFilters.modelKey,
      auditEventFilters.modelVersion,
      auditEventFilters.routingPolicyId,
      auditEventFilters.routingPolicyVersion,
      auditEventFilters.reviewMode,
    ],
    queryFn: () =>
      listAuditEvents(
        apiKey,
        buildGlobalAuditEventFilters(auditEventFilters, auditEventLimit),
      ) as Promise<AuditEventListResponse>,
  });
  const agentRunsQuery = useQuery({
    queryKey: ["agent-run-logs", apiKey],
    queryFn: () => listAgentRuns(apiKey) as Promise<AgentRunLogListResponse>,
  });
  const alertsQuery = useQuery({
    queryKey: ["ops-alerts", apiKey],
    queryFn: () => listOpsAlerts(apiKey) as Promise<OpsAlertListResponse>,
  });
  const labelsQuery = useQuery({
    queryKey: ["outcome-labels", apiKey],
    queryFn: () => listOutcomeLabels(apiKey) as Promise<OutcomeLabelListResponse>,
  });
  const webhookQuery = useQuery({
    queryKey: ["webhook-events", apiKey],
    queryFn: () => listWebhookEvents(apiKey) as Promise<WebhookEventListResponse>,
  });
  const governanceChangeEventsQuery = useQuery({
    queryKey: ["governance-change-events", apiKey],
    queryFn: () => listGovernanceChangeEvents(apiKey, 100) as Promise<AuditEventListResponse>,
  });
  const promotionGateGovernanceQuery = useQuery({
    queryKey: ["promotion-gate-governance", apiKey],
    queryFn: () => loadPromotionGateGovernance(apiKey),
  });
  const fwaSchemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<FwaSchemeListResponse>,
  });
  const summary = buildAuditSummary(auditQuery.data);
  const globalAuditSummary = buildAuditSummary(globalAuditQuery.data);
  const agentSummary = buildAgentRunLogSummary(agentRunsQuery.data?.runs);
  const alertSummary = buildOpsAlertSummary(alertsQuery.data?.alerts);
  const outcomeLabels = labelsQuery.data?.labels ?? [];
  const filteredOutcomeLabels = filterOutcomeLabels(outcomeLabels, outcomeLabelFilters);
  const labelSummary = buildOutcomeLabelSummary(outcomeLabels);
  const filteredLabelSummary = buildOutcomeLabelSummary(filteredOutcomeLabels);
  const outcomeLabelSourceTypes = sortedUniqueLabels(outcomeLabels, "source_type");
  const outcomeLabelFeedbackTargets = sortedUniqueLabels(outcomeLabels, "feedback_target");
  const outcomeLabelGovernanceStatuses = sortedUniqueLabels(outcomeLabels, "governance_status");
  const webhookSummary = buildWebhookDeliverySummary(webhookQuery.data?.events);
  const governanceChangeTimelineRows = buildGovernanceChangeTimelineRows(
    governanceChangeEventsQuery.data?.events,
  );
  const promotionGateSummary = buildPromotionGateGovernanceSummary(
    promotionGateGovernanceQuery.data?.rows,
  );
  const fwaSchemeRows = buildFwaSchemeGovernanceRows(fwaSchemesQuery.data?.schemes);
  const schemeLabelMap = buildFwaSchemeLabelMap(fwaSchemesQuery.data?.schemes);
  const fwaSchemeSummary = buildFwaSchemeGovernanceSummary(fwaSchemeRows);
  const agentApprovalMutation = useMutation({
    mutationFn: ({
      run,
      decision,
    }: {
      run: AgentRunLog;
      decision: "approved" | "rejected";
    }) =>
      submitAgentApproval(
        run.agent_run_id,
        buildAgentApprovalPayload(run, decision, agentApprover),
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["agent-run-logs"] });
      queryClient.invalidateQueries({ queryKey: ["claim-audit-history"] });
      queryClient.invalidateQueries({ queryKey: ["global-audit-events"] });
    },
  });
  const deliveryAttemptMutation = useMutation({
    mutationFn: ({
      eventId,
      deliveryStatus,
    }: {
      eventId: string;
      deliveryStatus: "delivered" | "failed";
    }) =>
      submitWebhookDeliveryAttempt(
        eventId,
        {
          delivery_status: deliveryStatus,
          response_status_code: deliveryStatus === "delivered" ? 200 : 503,
          error_message:
            deliveryStatus === "failed"
              ? "Manual delivery check failed from Governance."
              : null,
        },
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["webhook-events"] });
    },
  });

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Governance</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        <label>
          Claim ID
          <input value={claimId} onChange={(event) => setClaimId(event.target.value)} />
        </label>
        <label>
          Agent Approver
          <input
            value={agentApprover}
            onChange={(event) => setAgentApprover(event.target.value)}
          />
        </label>
        <div className="summary-grid">
          <div>
            <span>Audit Events</span>
            <strong>{summary.totalEvents}</strong>
          </div>
          <div>
            <span>Global Events</span>
            <strong>{globalAuditSummary.totalEvents}</strong>
          </div>
          <div>
            <span>Succeeded</span>
            <strong>{summary.succeededEvents}</strong>
          </div>
          <div>
            <span>Failed</span>
            <strong>{summary.failedEvents}</strong>
          </div>
          <div>
            <span>Latest Event</span>
            <strong>{summary.latestEventType}</strong>
          </div>
          <div>
            <span>Agent Runs</span>
            <strong>{agentSummary.runCount}</strong>
          </div>
          <div>
            <span>Tool Calls</span>
            <strong>{agentSummary.toolCallCount}</strong>
          </div>
          <div>
            <span>Policy Checks</span>
            <strong>{agentSummary.policyCheckCount}</strong>
          </div>
          <div>
            <span>Contexts</span>
            <strong>{agentSummary.contextSnapshotCount}</strong>
          </div>
          <div>
            <span>Approvals</span>
            <strong>{agentSummary.pendingApprovalCount}</strong>
          </div>
          <div>
            <span>Alerts</span>
            <strong>{alertSummary.openAlertCount}</strong>
          </div>
          <div>
            <span>Critical Alerts</span>
            <strong>{alertSummary.criticalAlertCount}</strong>
          </div>
          <div>
            <span>Labels</span>
            <strong>{labelSummary.labelCount}</strong>
          </div>
          <div>
            <span>Webhooks</span>
            <strong>{webhookSummary.eventCount}</strong>
          </div>
          <div>
            <span>Training Ready</span>
            <strong>{labelSummary.approvedForTrainingCount}</strong>
          </div>
        </div>
        {auditQuery.error ? <pre className="error">{String(auditQuery.error.message)}</pre> : null}
        {globalAuditQuery.error ? (
          <pre className="error">{String(globalAuditQuery.error.message)}</pre>
        ) : null}
        {agentRunsQuery.error ? (
          <pre className="error">{String(agentRunsQuery.error.message)}</pre>
        ) : null}
        {agentApprovalMutation.error ? (
          <pre className="error">{String(agentApprovalMutation.error.message)}</pre>
        ) : null}
        {alertsQuery.error ? (
          <pre className="error">{String(alertsQuery.error.message)}</pre>
        ) : null}
        {labelsQuery.error ? <pre className="error">{String(labelsQuery.error.message)}</pre> : null}
        {webhookQuery.error ? (
          <pre className="error">{String(webhookQuery.error.message)}</pre>
        ) : null}
        {deliveryAttemptMutation.error ? (
          <pre className="error">{String(deliveryAttemptMutation.error.message)}</pre>
        ) : null}
        {governanceChangeEventsQuery.error ? (
          <pre className="error">{String(governanceChangeEventsQuery.error.message)}</pre>
        ) : null}
        {promotionGateGovernanceQuery.error ? (
          <pre className="error">{String(promotionGateGovernanceQuery.error.message)}</pre>
        ) : null}
        {fwaSchemesQuery.error ? (
          <pre className="error">{String(fwaSchemesQuery.error.message)}</pre>
        ) : null}
      </div>

      <div className="panel">
        <h2>FWA Scheme Taxonomy</h2>
        <div className="summary-grid">
          <div>
            <span>Schemes</span>
            <strong>{fwaSchemeSummary.schemeCount}</strong>
          </div>
          <div>
            <span>Domains</span>
            <strong>{fwaSchemeSummary.domainCount}</strong>
          </div>
          <div>
            <span>Evidence Items</span>
            <strong>{fwaSchemeSummary.evidenceRequirementCount}</strong>
          </div>
          <div>
            <span>Medical Review</span>
            <strong>{fwaSchemeSummary.medicalReviewCount}</strong>
          </div>
          <div>
            <span>Provider Review</span>
            <strong>{fwaSchemeSummary.providerReviewCount}</strong>
          </div>
        </div>
        {fwaSchemeRows.length ? (
          <div className="table-list">
            {fwaSchemeRows.map((row) => (
              <div className="metric-row compact-metric-row" key={row.schemeFamily}>
                <span>
                  {row.riskDomain} / {row.displayName}
                </span>
                <strong>{row.defaultReviewRoute}</strong>
                <small>{row.primaryLayers}</small>
                <small>{row.minimumEvidence}</small>
              </div>
            ))}
          </div>
        ) : (
          <p className="empty">No FWA scheme taxonomy loaded</p>
        )}
      </div>

      <div className="panel">
        <h2>Promotion Gate Governance</h2>
        <div className="summary-grid">
          <div>
            <span>Targets</span>
            <strong>{promotionGateSummary.targetCount}</strong>
          </div>
          <div>
            <span>Allowed</span>
            <strong>{promotionGateSummary.allowedTargetCount}</strong>
          </div>
          <div>
            <span>Blocked</span>
            <strong>{promotionGateSummary.blockedTargetCount}</strong>
          </div>
          <div>
            <span>Gates</span>
            <strong>
              {promotionGateSummary.passedGateCount}/{promotionGateSummary.totalGateCount}
            </strong>
          </div>
          <div>
            <span>Blockers</span>
            <strong>{promotionGateSummary.blockerCount}</strong>
          </div>
        </div>
        {promotionGateGovernanceQuery.data?.rows.length ? (
          <div className="table-list">
            {promotionGateGovernanceQuery.data.rows.map((row) => (
              <div
                className="metric-row compact-metric-row"
                key={`${row.domain}:${row.targetId}:${row.reviewMode}`}
              >
                <span>
                  {row.domain} / {row.targetId}
                </span>
                <strong>{row.decision}</strong>
                <small>
                  {formatReviewModeLabel(row.reviewMode)} · {row.status} · {row.passedCount}/
                  {row.totalCount} · {row.topBlocker}
                </small>
                <small>{row.evidenceSources}</small>
              </div>
            ))}
          </div>
        ) : (
          <p className="empty">No promotion gate data loaded</p>
        )}
      </div>

      <div className="panel">
        <h2>Governance Change Timeline</h2>
        {governanceChangeTimelineRows.length ? (
          <ol className="audit-timeline">
            {governanceChangeTimelineRows.map((row) => (
              <li key={row.auditId}>
                <div>
                  <strong>
                    {row.domain} / {row.targetId}
                  </strong>
                  <span>{row.decision}</span>
                </div>
                <small>
                  {row.createdAt} · {row.eventType}
                </small>
                <p>
                  {row.statusTransition} · {row.actor}
                </p>
                <p>{row.summary}</p>
                <ul className="result-list">
                  {row.evidenceRefs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No governance change events loaded</p>
        )}
      </div>

      <div className="panel">
        <h2>Operations Alerts</h2>
        <div className="summary-grid">
          <div>
            <span>Total</span>
            <strong>{alertSummary.alertCount}</strong>
          </div>
          <div>
            <span>Routing</span>
            <strong>{alertSummary.routingAlertCount}</strong>
          </div>
          <div>
            <span>SLA Breach</span>
            <strong>{alertSummary.slaBreachCount}</strong>
          </div>
          <div>
            <span>Medical Review</span>
            <strong>{alertSummary.medicalReviewAlertCount}</strong>
          </div>
          <div>
            <span>Agent Approval</span>
            <strong>{alertSummary.agentApprovalAlertCount}</strong>
          </div>
        </div>
        {alertsQuery.data?.alerts.length ? (
          <ol className="audit-timeline">
            {alertsQuery.data.alerts.map((alert) => (
              <li key={alert.alert_id}>
                <div>
                  <strong>{alert.alert_type}</strong>
                  <span>{alert.severity}</span>
                </div>
                <small>
                  {alert.claim_id} / {formatFwaSchemeLabel(alert.scheme_family, schemeLabelMap)}
                </small>
                <p>{alert.message}</p>
                <p>{alert.recommended_action}</p>
                <ul className="result-list">
                  {alert.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No operations alerts loaded</p>
        )}
      </div>

      <div className="panel">
        <h2>Webhook Delivery</h2>
        <div className="summary-grid">
          <div>
            <span>Pending</span>
            <strong>{webhookSummary.pendingCount}</strong>
          </div>
          <div>
            <span>Retry Wait</span>
            <strong>{webhookSummary.retryWaitCount}</strong>
          </div>
          <div>
            <span>Delivered</span>
            <strong>{webhookSummary.deliveredCount}</strong>
          </div>
          <div>
            <span>Failed</span>
            <strong>{webhookSummary.failedCount}</strong>
          </div>
          <div>
            <span>Signed</span>
            <strong>{webhookSummary.signedCount}</strong>
          </div>
        </div>
        {webhookQuery.data?.events.length ? (
          <ol className="audit-timeline">
            {webhookQuery.data.events.map((event) => (
              <li key={event.event_id}>
                <div>
                  <strong>{event.event_type}</strong>
                  <span>{event.delivery_status}</span>
                </div>
                <small>
                  {event.claim_id} / retry {event.retry_count}/{event.max_attempts}
                </small>
                <p>{event.idempotency_key}</p>
                <p>
                  {event.signature_algorithm} / {event.signature_key_id}
                </p>
                {event.last_error_message ? <p>{event.last_error_message}</p> : null}
                {canRecordWebhookDeliveryAttempt(event) ? (
                  <div className="button-row">
                    <button
                      disabled={deliveryAttemptMutation.isPending}
                      onClick={() =>
                        deliveryAttemptMutation.mutate({
                          eventId: event.event_id,
                          deliveryStatus: "delivered",
                        })
                      }
                      type="button"
                    >
                      Mark Delivered
                    </button>
                    <button
                      disabled={deliveryAttemptMutation.isPending}
                      onClick={() =>
                        deliveryAttemptMutation.mutate({
                          eventId: event.event_id,
                          deliveryStatus: "failed",
                        })
                      }
                      type="button"
                    >
                      Mark Failed
                    </button>
                  </div>
                ) : null}
                <ul className="result-list">
                  {event.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No webhook events loaded</p>
        )}
      </div>

      <div className="panel">
        <h2>Global Audit Events</h2>
        <div className="button-row">
          {auditEventFilterShortcuts.map((shortcut) => (
            <button
              key={shortcut.eventType}
              onClick={() =>
                setAuditEventFilters((filters) => ({
                  ...filters,
                  eventType: shortcut.eventType,
                }))
              }
              type="button"
            >
              {shortcut.label}
            </button>
          ))}
          <button
            onClick={() =>
              setAuditEventFilters((filters) => ({
                ...filters,
                eventType: "",
              }))
            }
            type="button"
          >
            All Events
          </button>
        </div>
        <label>
          Event Type
          <input
            value={auditEventFilters.eventType}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                eventType: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Actor ID
          <input
            value={auditEventFilters.actorId}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                actorId: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Run ID
          <input
            value={auditEventFilters.runId}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                runId: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Audit Claim ID
          <input
            value={auditEventFilters.claimId}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                claimId: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Feedback ID
          <input
            value={auditEventFilters.feedbackId}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                feedbackId: event.target.value,
              }))
            }
          />
        </label>
        <label>
          QA Case ID
          <input
            value={auditEventFilters.qaCaseId}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                qaCaseId: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Sample ID
          <input
            value={auditEventFilters.sampleId}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                sampleId: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Rule ID
          <input
            value={auditEventFilters.ruleId}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                ruleId: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Rule Version
          <input
            value={auditEventFilters.ruleVersion}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                ruleVersion: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Model Key
          <input
            value={auditEventFilters.modelKey}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                modelKey: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Model Version
          <input
            value={auditEventFilters.modelVersion}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                modelVersion: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Routing Policy ID
          <input
            value={auditEventFilters.routingPolicyId}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                routingPolicyId: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Routing Policy Version
          <input
            value={auditEventFilters.routingPolicyVersion}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                routingPolicyVersion: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Review Mode
          <input
            value={auditEventFilters.reviewMode}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                reviewMode: event.target.value,
              }))
            }
          />
        </label>
        <label>
          Limit
          <input
            inputMode="numeric"
            value={auditEventFilters.limit}
            onChange={(event) =>
              setAuditEventFilters((filters) => ({
                ...filters,
                limit: event.target.value,
              }))
            }
          />
        </label>
        <div className="summary-grid">
          <div>
            <span>Total</span>
            <strong>{globalAuditSummary.totalEvents}</strong>
          </div>
          <div>
            <span>Succeeded</span>
            <strong>{globalAuditSummary.succeededEvents}</strong>
          </div>
          <div>
            <span>Failed</span>
            <strong>{globalAuditSummary.failedEvents}</strong>
          </div>
          <div>
            <span>Latest</span>
            <strong>{globalAuditSummary.latestEventType}</strong>
          </div>
        </div>
        {globalAuditQuery.data?.events.length ? (
          <ol className="audit-timeline">
            {globalAuditQuery.data.events.map((event) => (
              <li key={event.audit_id}>
                <div>
                  <strong>{event.event_type}</strong>
                  <span>{event.event_status}</span>
                </div>
                <small>{event.created_at || event.run_id}</small>
                <p>{event.summary}</p>
                <ul className="result-list">
                  {event.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No global audit events loaded</p>
        )}
      </div>

      <div className="panel">
        <h2>Audit Timeline</h2>
        {auditQuery.data?.events.length ? (
          <ol className="audit-timeline">
            {auditQuery.data.events.map((event) => (
              <li key={event.audit_id}>
                <div>
                  <strong>{event.event_type}</strong>
                  <span>{event.event_status}</span>
                </div>
                <small>{event.created_at || event.run_id}</small>
                <p>{event.summary}</p>
                <ul className="result-list">
                  {event.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No audit events loaded</p>
        )}
      </div>
      <div className="panel">
        <h2>Outcome Labels</h2>
        <label>
          Source Type
          <select
            value={outcomeLabelFilters.sourceType}
            onChange={(event) =>
              setOutcomeLabelFilters((filters) => ({
                ...filters,
                sourceType: event.target.value,
              }))
            }
          >
            <option value="">All sources</option>
            {outcomeLabelSourceTypes.map((sourceType) => (
              <option key={sourceType} value={sourceType}>
                {sourceType}
              </option>
            ))}
          </select>
        </label>
        <label>
          Feedback Target
          <select
            value={outcomeLabelFilters.feedbackTarget}
            onChange={(event) =>
              setOutcomeLabelFilters((filters) => ({
                ...filters,
                feedbackTarget: event.target.value,
              }))
            }
          >
            <option value="">All targets</option>
            {outcomeLabelFeedbackTargets.map((feedbackTarget) => (
              <option key={feedbackTarget} value={feedbackTarget}>
                {feedbackTarget}
              </option>
            ))}
          </select>
        </label>
        <label>
          Governance Status
          <select
            value={outcomeLabelFilters.governanceStatus}
            onChange={(event) =>
              setOutcomeLabelFilters((filters) => ({
                ...filters,
                governanceStatus: event.target.value,
              }))
            }
          >
            <option value="">All statuses</option>
            {outcomeLabelGovernanceStatuses.map((governanceStatus) => (
              <option key={governanceStatus} value={governanceStatus}>
                {governanceStatus}
              </option>
            ))}
          </select>
        </label>
        <div className="summary-grid">
          <div>
            <span>Matching</span>
            <strong>
              {filteredLabelSummary.labelCount}/{labelSummary.labelCount}
            </strong>
          </div>
          <div>
            <span>Needs Review</span>
            <strong>{filteredLabelSummary.needsReviewCount}</strong>
          </div>
          <div>
            <span>Model Feedback</span>
            <strong>{filteredLabelSummary.modelFeedbackCount}</strong>
          </div>
          <div>
            <span>Rule Feedback</span>
            <strong>{filteredLabelSummary.ruleFeedbackCount}</strong>
          </div>
          <div>
            <span>Case Status Labels</span>
            <strong>{filteredLabelSummary.caseStatusLabelCount}</strong>
          </div>
          <div>
            <span>Medical Review Labels</span>
            <strong>{filteredLabelSummary.medicalReviewLabelCount}</strong>
          </div>
          <div>
            <span>False Positives</span>
            <strong>{filteredLabelSummary.falsePositiveCount}</strong>
          </div>
          <div>
            <span>Evidence Backed</span>
            <strong>
              {filteredLabelSummary.evidenceBackedCount}/{filteredLabelSummary.labelCount}
            </strong>
          </div>
          <div>
            <span>Prevented</span>
            <strong>
              {filteredLabelSummary.amountPreventedCurrency}{" "}
              {filteredLabelSummary.amountPreventedTotal}
            </strong>
          </div>
        </div>
        {filteredLabelSummary.sourceTypeRows.length ? (
          <ul className="result-list compact-list">
            {filteredLabelSummary.sourceTypeRows.map((row) => (
              <li key={row}>{row}</li>
            ))}
          </ul>
        ) : null}
        {filteredOutcomeLabels.length ? (
          <ol className="audit-timeline">
            {filteredOutcomeLabels.map((label) => (
              <li key={label.label_id}>
                <div>
                  <strong>{label.label_name}</strong>
                  <span>{label.governance_status}</span>
                </div>
                <small>
                  {label.claim_id} / {label.source_type}:{label.source_id}
                </small>
                <p>
                  {label.label_value}
                  {label.currency ? ` ${label.currency}` : ""}{" "}
                  {"->"} {label.feedback_target}
                </p>
                <ul className="result-list">
                  {label.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No governed labels loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Agent Run Logs</h2>
        {agentRunsQuery.data?.runs.length ? (
          <ol className="audit-timeline">
            {agentRunsQuery.data.runs.map((run) => (
              <li key={run.agent_run_id}>
                <div>
                  <strong>{run.agent_run_id}</strong>
                  <span>{run.status}</span>
                </div>
                <small>{run.completed_at || run.created_at || run.claim_id}</small>
                <p>{run.decision_boundary}</p>
                <p>{run.steps.length} evidence-backed steps</p>
                <p>
                  {run.tool_calls.length} tool calls / {run.tool_results.length} tool results
                </p>
                <p>
                  {run.policy_checks.length} policy checks /{" "}
                  {run.policy_checks.filter((check) => check.decision === "denied").length} denied
                </p>
                <p>
                  {run.context_snapshots.length} context snapshots /{" "}
                  {
                    run.context_snapshots.filter(
                      (snapshot) => snapshot.redaction_status === "pii_masked",
                    ).length
                  }{" "}
                  masked
                </p>
                <p>
                  {run.approvals.length} approvals /{" "}
                  {run.approvals.filter((approval) => approval.decision === "pending").length}{" "}
                  pending
                </p>
                {hasPendingAgentApproval(run) ? (
                  <div className="button-row">
                    <button
                      disabled={
                        agentApprovalMutation.isPending ||
                        !canSubmitAgentApproval(run, agentApprover)
                      }
                      onClick={() =>
                        agentApprovalMutation.mutate({
                          run,
                          decision: "approved",
                        })
                      }
                      type="button"
                    >
                      Approve Agent Output
                    </button>
                    <button
                      disabled={
                        agentApprovalMutation.isPending ||
                        !canSubmitAgentApproval(run, agentApprover)
                      }
                      onClick={() =>
                        agentApprovalMutation.mutate({
                          run,
                          decision: "rejected",
                        })
                      }
                      type="button"
                    >
                      Reject Agent Output
                    </button>
                  </div>
                ) : null}
                <ul className="result-list">
                  {run.context_snapshots.map((snapshot) => (
                    <li key={snapshot.snapshot_id}>
                      {snapshot.redaction_status}: {snapshot.checksum}
                    </li>
                  ))}
                </ul>
                <ul className="result-list">
                  {run.approvals.map((approval) => (
                    <li key={approval.approval_id}>
                      {approval.proposed_action}: {approval.decision}
                    </li>
                  ))}
                </ul>
                <ul className="result-list">
                  {run.policy_checks.map((check) => (
                    <li key={check.policy_check_id}>
                      {check.policy_name}: {check.decision}
                    </li>
                  ))}
                </ul>
                <ul className="result-list">
                  {run.tool_calls.map((call) => (
                    <li key={call.tool_call_id}>
                      {call.tool_name}: {call.status}
                    </li>
                  ))}
                </ul>
                <ul className="result-list">
                  {run.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No agent runs loaded</p>
        )}
      </div>
    </section>
  );
}
