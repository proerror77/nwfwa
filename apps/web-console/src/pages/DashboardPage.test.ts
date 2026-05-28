import { describe, expect, it } from "vitest";
import {
  buildDashboardAgentGovernanceSummary,
  buildDashboardLabelPoolSummary,
  buildDashboardModelGovernanceSummary,
  buildDashboardQaQueueSummary,
  buildDashboardRuleGovernanceSummary,
  buildProviderRiskSummary,
} from "./DashboardPage";

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

describe("buildDashboardQaQueueSummary", () => {
  it("summarizes QA sampled queue completion", () => {
    const summary = buildDashboardQaQueueSummary({
      sampled_cases: 8,
      open_cases: 3,
      reviewed_cases: 5,
    });

    expect(summary).toEqual({
      sampledCases: 8,
      openCases: 3,
      reviewedCases: 5,
      reviewedRateLabel: "62.5%",
    });
  });
});

describe("buildDashboardAgentGovernanceSummary", () => {
  it("summarizes agent run success and approval adoption", () => {
    const summary = buildDashboardAgentGovernanceSummary({
      total_runs: 5,
      successful_runs: 4,
      pending_approvals: 1,
      approved_approvals: 3,
      rejected_approvals: 1,
    });

    expect(summary).toEqual({
      totalRuns: 5,
      successfulRuns: 4,
      pendingApprovals: 1,
      approvedApprovals: 3,
      rejectedApprovals: 1,
      successRateLabel: "80.0%",
      approvalRateLabel: "75.0%",
    });
  });
});

describe("buildDashboardModelGovernanceSummary", () => {
  it("summarizes model evaluation and drift coverage", () => {
    const summary = buildDashboardModelGovernanceSummary({
      total_models: 4,
      evaluated_models: 3,
      drift_watch_count: 1,
      drift_detected_count: 1,
      average_precision: 0.7123,
      average_recall: 0.6345,
    });

    expect(summary).toEqual({
      totalModels: 4,
      evaluatedModels: 3,
      driftWatchCount: 1,
      driftDetectedCount: 1,
      evaluationCoverageLabel: "75.0%",
      averagePrecisionLabel: "71.2%",
      averageRecallLabel: "63.5%",
    });
  });
});

describe("buildDashboardRuleGovernanceSummary", () => {
  it("summarizes rule performance and ROI governance", () => {
    const summary = buildDashboardRuleGovernanceSummary({
      total_rules: 10,
      active_rules: 8,
      triggered_rules: 3,
      total_trigger_count: 12,
      reviewed_count: 4,
      confirmed_fwa_count: 3,
      false_positive_count: 1,
      precision: 0.75,
      false_positive_rate: 0.25,
      saving_amount: "8200.00",
      roi: 6.83,
    });

    expect(summary).toEqual({
      totalRules: 10,
      activeRules: 8,
      triggeredRules: 3,
      totalTriggerCount: 12,
      reviewedCount: 4,
      confirmedFwaCount: 3,
      falsePositiveCount: 1,
      precisionLabel: "75.0%",
      falsePositiveRateLabel: "25.0%",
      savingAmount: "8200.00",
      roiLabel: "6.8x",
    });
  });
});
