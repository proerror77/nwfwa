import { formatFwaSchemeLabel } from "./fwaSchemeOptions";

export type ClinicalEvidenceAssessment = {
  review_required: boolean;
  review_route: string;
  evidence_status: string;
  minimum_evidence: string[];
  missing_evidence: string[];
  item_findings: Array<{
    item_code: string;
    issue_type: string;
    required_evidence: string[];
    missing_evidence: string[];
    reason: string;
    review_route: string;
    evidence_refs: string[];
  }>;
  evidence_refs: string[];
};

export type ProviderRelationshipGraphAssessment = {
  provider_id: string;
  risk_score: number;
  risk_tier: string;
  review_required: boolean;
  review_route: string;
  graph_reasons: string[];
  findings: Array<{
    signal: string;
    risk_score: number;
    reason: string;
    evidence_ref: string;
  }>;
  evidence_refs: string[];
};

export type SimilarCase = {
  case_id: string;
  title: string;
  scheme_family: string;
  similarity_score: number;
  matched_signals: string[];
  retrieval_method: string;
  provenance_refs: string[];
  summary: string;
  outcome: string;
  evidence_refs: string[];
};

export function buildClinicalEvidenceInspection(assessment?: ClinicalEvidenceAssessment) {
  if (!assessment) {
    return null;
  }

  const firstFinding = assessment.item_findings[0];

  return {
    reviewLabel: assessment.review_required ? "Medical review required" : "No clinical review",
    routeLabel: assessment.review_route,
    statusLabel: assessment.evidence_status,
    findingCount: assessment.item_findings.length,
    firstFindingLabel: firstFinding
      ? `${firstFinding.item_code} · ${firstFinding.issue_type}`
      : "No item findings",
    minimumEvidenceSummary: summarizeList(assessment.minimum_evidence, "None"),
    missingEvidenceSummary: summarizeList(assessment.missing_evidence, "None"),
    evidenceSummary: summarizeList(assessment.evidence_refs, "No evidence refs"),
  };
}

export function buildProviderGraphInspection(graph?: ProviderRelationshipGraphAssessment) {
  if (!graph) {
    return null;
  }

  const topFinding = graph.findings.slice().sort((left, right) => right.risk_score - left.risk_score)[0];

  return {
    providerId: graph.provider_id,
    riskLabel: `${graph.risk_tier} / ${graph.risk_score}`,
    reviewLabel: graph.review_required ? "Graph review required" : "No graph review",
    routeLabel: graph.review_route,
    topSignalLabel: topFinding ? `${topFinding.signal} / ${topFinding.risk_score}` : "No graph signal",
    reasonSummary: summarizeList(graph.graph_reasons, "No graph reasons"),
    evidenceSummary: summarizeList(graph.evidence_refs, "No evidence refs"),
  };
}

export function buildSimilarCaseInspection(
  cases: SimilarCase[] = [],
  schemeLabelMap: Record<string, string> = {},
) {
  const topCase = cases.slice().sort((left, right) => right.similarity_score - left.similarity_score)[0];

  return {
    caseCount: cases.length,
    topCaseLabel: topCase
      ? `${topCase.case_id} · ${(topCase.similarity_score * 100).toFixed(0)}%`
      : "No similar cases",
    schemeLabel: topCase ? formatFwaSchemeLabel(topCase.scheme_family, schemeLabelMap) : "None",
    matchedSignalsSummary: topCase ? summarizeList(topCase.matched_signals, "No matched signals") : "None",
    provenanceSummary: topCase ? summarizeList(topCase.provenance_refs, "No provenance refs") : "None",
  };
}

function summarizeList(values: string[], fallback: string) {
  return values.length > 0 ? values.join(", ") : fallback;
}
