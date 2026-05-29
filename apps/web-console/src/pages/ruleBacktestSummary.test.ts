import { describe, expect, it } from "vitest";
import { buildRuleBacktestSummary } from "./ruleBacktestSummary";

describe("buildRuleBacktestSummary", () => {
  it("formats labeled rule backtest governance metrics", () => {
    expect(
      buildRuleBacktestSummary({
        sample_count: 12,
        matched_count: 4,
        reviewed_count: 10,
        confirmed_fwa_count: 3,
        false_positive_count: 1,
        match_rate: 0.3333,
        precision: 0.75,
        recall: 0.6,
        lift: 1.5,
        false_positive_rate: 0.25,
        estimated_saving: "2400.00",
        promotion_recommendation: "eligible_for_review",
        blockers: [],
        matched_claim_ids: ["CLM-1"],
        evidence_refs: ["rules:candidate:v1"],
      }),
    ).toEqual({
      sampleCount: 12,
      matchedCount: 4,
      reviewedCount: 10,
      confirmedFwaCount: 3,
      falsePositiveCount: 1,
      matchRateLabel: "33.3%",
      precisionLabel: "75.0%",
      recallLabel: "60.0%",
      liftLabel: "1.50x",
      falsePositiveRateLabel: "25.0%",
      estimatedSaving: "2400.00",
      recommendation: "eligible_for_review",
      blockerLabel: "none",
      evidenceCount: 1,
      matchedClaimIds: ["CLM-1"],
      evidenceRefs: ["rules:candidate:v1"],
    });
  });
});
