export type AuditTimelineContext = {
  claimId: string;
  auditId: string;
  runId: string;
  source: "runtime_scoring";
};

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
  context: AuditTimelineContext | undefined,
  fallbackClaimId: string,
) {
  return context?.claimId ?? fallbackClaimId;
}

export function buildGovernanceAuditFiltersFromContext(context?: AuditTimelineContext) {
  return {
    eventType: context ? "scoring.completed" : "",
    actorId: "",
    runId: context?.runId ?? "",
    claimId: context?.claimId ?? "",
    feedbackId: "",
    qaCaseId: "",
    sampleId: "",
    agentRunId: "",
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
    limit: "50",
  };
}
