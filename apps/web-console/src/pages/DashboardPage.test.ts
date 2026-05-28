import { describe, expect, it } from "vitest";
import { buildDashboardLabelPoolSummary, buildProviderRiskSummary } from "./DashboardPage";

describe("buildDashboardLabelPoolSummary", () => {
  it("summarizes governed label pool readiness for dashboard display", () => {
    const summary = buildDashboardLabelPoolSummary({
      total_labels: 9,
      approved_for_training: 5,
      needs_review: 4,
      rule_feedback: 3,
      model_feedback: 4,
      workflow_feedback: 2,
    });

    expect(summary).toEqual({
      totalLabels: 9,
      approvedForTraining: 5,
      needsReview: 4,
      ruleFeedback: 3,
      modelFeedback: 4,
      workflowFeedback: 2,
      trainingReadyRateLabel: "55.6%",
    });
  });
});

describe("buildProviderRiskSummary", () => {
  it("summarizes provider risk review pressure", () => {
    const summary = buildProviderRiskSummary({
      provider_count: 4,
      review_required_count: 2,
      high_risk_count: 1,
      providers: [
        {
          provider_id: "PRV-1",
          risk_score: 91,
          risk_tier: "high",
          review_required: true,
          review_route: "provider_review",
          claim_count: 3,
          latest_claim_id: "CLM-1",
          outlier_flags: ["peer_amount_p97"],
          evidence_refs: ["provider_profile:PRV-1:90d"],
        },
      ],
    });

    expect(summary).toEqual({
      providerCount: 4,
      reviewRequiredCount: 2,
      highRiskCount: 1,
      reviewRateLabel: "50.0%",
      topProviderId: "PRV-1",
      topProviderScore: 91,
    });
  });
});
