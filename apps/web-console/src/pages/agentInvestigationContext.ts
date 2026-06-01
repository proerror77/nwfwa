import type { SimilarCase } from "./runtimeEvidence";

export type AgentInvestigationContext = {
  source: "runtime_scoring";
  sourceRunId: string;
  claimId: string;
  riskScore: number;
  rag: string;
  schemeFamily?: string;
  topReasons: string[];
  diagnosisCode?: string;
  providerRegion?: string;
  tags: string[];
};

type RuntimeScoringAgentContextInput = {
  run_id: string;
  claim_id: string;
  risk_score: number;
  rag: string;
  top_reasons: string[];
  alerts: Array<{ alert_code: string }>;
  similar_cases: SimilarCase[];
};

type ScoringPayload = {
  claim?: {
    diagnosis_code?: unknown;
    provider?: {
      region?: unknown;
    };
  };
};

export function buildAgentInvestigationContextFromScoring(
  result: RuntimeScoringAgentContextInput,
  payloadText?: string,
): AgentInvestigationContext {
  const topSimilarCase = result.similar_cases
    .slice()
    .sort((left, right) => right.similarity_score - left.similarity_score)[0];
  const payloadHints = parseScoringPayloadHints(payloadText);
  return {
    source: "runtime_scoring",
    sourceRunId: result.run_id,
    claimId: result.claim_id,
    riskScore: result.risk_score,
    rag: result.rag,
    schemeFamily: topSimilarCase?.scheme_family,
    topReasons: result.top_reasons,
    diagnosisCode: payloadHints.diagnosisCode,
    providerRegion: payloadHints.providerRegion,
    tags: uniqueStrings([
      ...(topSimilarCase?.matched_signals ?? []),
      ...result.alerts.map((alert) => alert.alert_code.toLowerCase()),
    ]),
  };
}

function parseScoringPayloadHints(payloadText?: string) {
  if (!payloadText) {
    return {};
  }
  try {
    const payload = JSON.parse(payloadText) as ScoringPayload;
    return {
      diagnosisCode:
        typeof payload.claim?.diagnosis_code === "string"
          ? payload.claim.diagnosis_code
          : undefined,
      providerRegion:
        typeof payload.claim?.provider?.region === "string"
          ? payload.claim.provider.region
          : undefined,
    };
  } catch {
    return {};
  }
}

function uniqueStrings(values: string[]) {
  return values.filter((value, index, list) => value.length > 0 && list.indexOf(value) === index);
}
