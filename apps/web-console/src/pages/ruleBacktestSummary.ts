export type RuleBacktestResponse = {
  sample_count: number;
  matched_count: number;
  reviewed_count: number;
  confirmed_fwa_count: number;
  false_positive_count: number;
  match_rate: number;
  precision: number;
  recall: number;
  lift: number;
  false_positive_rate: number;
  estimated_saving: string;
  promotion_recommendation: string;
  blockers: string[];
  matched_claim_ids: string[];
  evidence_refs: string[];
};

export function buildRuleBacktestSummary(result: RuleBacktestResponse) {
  return {
    sampleCount: result.sample_count,
    matchedCount: result.matched_count,
    reviewedCount: result.reviewed_count,
    confirmedFwaCount: result.confirmed_fwa_count,
    falsePositiveCount: result.false_positive_count,
    matchRateLabel: formatPercent(result.match_rate),
    precisionLabel: formatPercent(result.precision),
    recallLabel: formatPercent(result.recall),
    liftLabel: `${result.lift.toFixed(2)}x`,
    falsePositiveRateLabel: formatPercent(result.false_positive_rate),
    estimatedSaving: result.estimated_saving,
    recommendation: result.promotion_recommendation,
    blockerLabel: result.blockers.length === 0 ? "none" : result.blockers.join(", "),
    evidenceCount: result.evidence_refs.length,
    matchedClaimIds: result.matched_claim_ids,
    evidenceRefs: result.evidence_refs,
  };
}

function formatPercent(value: number) {
  return `${(value * 100).toFixed(1)}%`;
}
