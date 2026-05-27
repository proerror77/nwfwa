export type ProviderProfileAssessment = {
  provider_id: string;
  risk_score: number;
  risk_tier: string;
  review_required: boolean;
  review_route: string;
  specialty?: string | null;
  network_status?: string | null;
  outlier_flags: string[];
  window_findings: Array<{
    window_days: number;
    risk_score: number;
    outlier_flags?: string[];
    reason: string;
    evidence_ref?: string;
  }>;
  evidence_refs: string[];
};

export function buildProviderProfileInspection(profile?: ProviderProfileAssessment) {
  if (!profile) {
    return null;
  }

  const highestWindow = profile.window_findings
    .slice()
    .sort((left, right) => right.risk_score - left.risk_score)[0];

  return {
    providerId: profile.provider_id,
    routeLabel: profile.review_route,
    reviewLabel: profile.review_required ? "Review required" : "No provider review",
    maxWindowLabel: highestWindow
      ? `${highestWindow.window_days}d / ${highestWindow.risk_score}`
      : "No profile window",
    outlierSummary: profile.outlier_flags.length > 0 ? profile.outlier_flags.join(", ") : "None",
    evidenceSummary:
      profile.evidence_refs.length > 0 ? profile.evidence_refs.join(", ") : "No evidence refs",
  };
}
