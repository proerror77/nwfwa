export type AuditTimelineContext = {
  claimId: string;
  auditId: string;
  runId: string;
  source: "runtime_scoring";
};

export type AgentRunAuditContext = {
  claimId: string;
  agentRunId: string;
  source: "agent_investigation";
};

export type GovernanceAuditContext = AuditTimelineContext | AgentRunAuditContext;

export type ScoringAuditSource = {
  claim_id: string;
  audit_id: string;
  run_id: string;
};

export function buildAuditTimelineContext(source: ScoringAuditSource): AuditTimelineContext {
  return {
    claimId: source.claim_id,
    auditId: source.audit_id,
    runId: source.run_id,
    source: "runtime_scoring",
  };
}

export function buildGovernanceClaimIdFromContext(
  context: GovernanceAuditContext | undefined,
  fallbackClaimId: string,
) {
  return context?.claimId ?? fallbackClaimId;
}

export function buildGovernanceAuditFiltersFromContext(context?: GovernanceAuditContext) {
  return {
    eventType: context?.source === "runtime_scoring" ? "scoring.completed" : "",
    actorId: "",
    runId: context?.source === "runtime_scoring" ? context.runId : "",
    claimId: context?.source === "runtime_scoring" ? context.claimId : "",
    feedbackId: "",
    qaCaseId: "",
    sampleId: "",
    agentRunId: context?.source === "agent_investigation" ? context.agentRunId : "",
    ruleId: "",
    ruleVersion: "",
    modelKey: "",
    modelVersion: "",
    routingPolicyId: "",
    routingPolicyVersion: "",
    reviewMode: "",
    datasetId: "",
    featureSetId: "",
    modelDatasetId: "",
    evaluationRunId: "",
    hasCanonicalTrace: false,
    limit: "50",
  };
}
