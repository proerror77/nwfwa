import { describe, expect, it } from "vitest";
import {
  buildDashboardAgentGovernanceSummary,
  buildDashboardQaFeedbackTargetRows,
  buildDashboardLabelPoolSummary,
  buildDashboardCaseSlaSummary,
  buildDashboardModelGovernanceSummary,
  buildDashboardQaQueueSummary,
  buildDashboardRuleGovernanceSummary,
  buildDashboardSavingSegmentRows,
  buildDashboardSchemeRows,
  buildDashboardValueMeasurementSummary,
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
      features_feedback: 1,
      provider_profile_feedback: 2,
      workflow_feedback: 2,
      case_status_labels: 1,
      medical_review_labels: 2,
      false_positive_labels: 1,
      evidence_backed_labels: 8,
    });

    expect(summary).toEqual({
      totalLabels: 9,
      approvedForTraining: 5,
      needsReview: 4,
      ruleFeedback: 3,
      modelFeedback: 4,
      featuresFeedback: 1,
      providerProfileFeedback: 2,
      workflowFeedback: 2,
      caseStatusLabels: 1,
      medicalReviewLabels: 2,
      falsePositiveLabels: 1,
      evidenceBackedLabels: 8,
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
      disagreement_cases: 2,
      disagreement_rate: 0.4,
      feedback_open_count: 3,
      feedback_in_progress_count: 1,
      feedback_resolved_count: 2,
      feedback_dismissed_count: 1,
      unresolved_feedback_count: 4,
      rules_unresolved_feedback_count: 2,
      models_unresolved_feedback_count: 1,
      features_unresolved_feedback_count: 0,
      provider_profile_unresolved_feedback_count: 1,
      workflow_unresolved_feedback_count: 0,
      tpa_unresolved_feedback_count: 0,
    });

    expect(summary).toEqual({
      sampledCases: 8,
      openCases: 3,
      reviewedCases: 5,
      disagreementCases: 2,
      feedbackOpenCount: 3,
      feedbackInProgressCount: 1,
      feedbackResolvedCount: 2,
      feedbackDismissedCount: 1,
      unresolvedFeedbackCount: 4,
      rulesUnresolvedFeedbackCount: 2,
      modelsUnresolvedFeedbackCount: 1,
      featuresUnresolvedFeedbackCount: 0,
      providerProfileUnresolvedFeedbackCount: 1,
      workflowUnresolvedFeedbackCount: 0,
      tpaUnresolvedFeedbackCount: 0,
      reviewedRateLabel: "62.5%",
      disagreementRateLabel: "40.0%",
    });
  });

  it("builds QA unresolved feedback target rows for dashboard display", () => {
    const summary = buildDashboardQaQueueSummary({
      sampled_cases: 8,
      open_cases: 3,
      reviewed_cases: 5,
      disagreement_cases: 2,
      disagreement_rate: 0.4,
      feedback_open_count: 3,
      feedback_in_progress_count: 1,
      feedback_resolved_count: 2,
      feedback_dismissed_count: 1,
      unresolved_feedback_count: 4,
      rules_unresolved_feedback_count: 2,
      models_unresolved_feedback_count: 1,
      features_unresolved_feedback_count: 0,
      provider_profile_unresolved_feedback_count: 1,
      workflow_unresolved_feedback_count: 0,
      tpa_unresolved_feedback_count: 0,
    });

    expect(buildDashboardQaFeedbackTargetRows(summary)).toEqual([
      { label: "Rules", count: 2 },
      { label: "Models", count: 1 },
      { label: "Features", count: 0 },
      { label: "Provider Profile", count: 1 },
      { label: "Workflow", count: 0 },
      { label: "TPA", count: 0 },
    ]);
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

describe("buildDashboardCaseSlaSummary", () => {
  it("summarizes case SLA timing and breach rate", () => {
    const summary = buildDashboardCaseSlaSummary({
      total_cases: 5,
      open_cases: 3,
      closed_cases: 2,
      breached_cases: 1,
      sla_breach_rate: 0.2,
      average_time_to_triage_hours: 1.25,
      average_time_to_closure_hours: 18.5,
    });

    expect(summary).toEqual({
      totalCases: 5,
      openCases: 3,
      closedCases: 2,
      breachedCases: 1,
      breachRateLabel: "20.0%",
      averageTimeToTriageLabel: "1.3h",
      averageTimeToClosureLabel: "18.5h",
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

describe("buildDashboardSchemeRows", () => {
  it("orders FWA scheme distribution by count then scheme name", () => {
    expect(
      buildDashboardSchemeRows({
        provider_peer_outlier: 1,
        early_high_value_claim: 3,
        diagnosis_procedure_mismatch: 3,
      }),
    ).toEqual([
      {
        schemeFamily: "diagnosis_procedure_mismatch",
        schemeLabel: "diagnosis_procedure_mismatch",
        count: 3,
      },
      {
        schemeFamily: "early_high_value_claim",
        schemeLabel: "early_high_value_claim",
        count: 3,
      },
      {
        schemeFamily: "provider_peer_outlier",
        schemeLabel: "provider_peer_outlier",
        count: 1,
      },
    ]);
  });

  it("uses FWA taxonomy labels when available", () => {
    expect(
      buildDashboardSchemeRows(
        { early_high_value_claim: 2 },
        { early_high_value_claim: "Early high-value claim (early_high_value_claim)" },
      ),
    ).toEqual([
      {
        schemeFamily: "early_high_value_claim",
        schemeLabel: "Early high-value claim (early_high_value_claim)",
        count: 2,
      },
    ]);
  });
});

describe("buildDashboardSavingSegmentRows", () => {
  it("orders provider and scheme ROI attribution rows", () => {
    expect(
      buildDashboardSavingSegmentRows([
        {
          segment_type: "scheme",
          segment_id: "provider_peer_outlier",
          saving_amount: "8200.00",
          currency: "CNY",
          claim_count: 1,
          attribution_count: 2,
          roi: 68.33,
        },
        {
          segment_type: "provider",
          segment_id: "PRV-0287",
          saving_amount: "8200.00",
          currency: "CNY",
          claim_count: 1,
          attribution_count: 2,
          roi: 68.33,
        },
      ]),
    ).toEqual([
      {
        key: "provider:PRV-0287:CNY",
        segmentLabel: "provider / PRV-0287",
        savingAmount: "8200.00",
        currency: "CNY",
        claimCount: 1,
        attributionCount: 2,
        roiLabel: "68.3x",
      },
      {
        key: "scheme:provider_peer_outlier:CNY",
        segmentLabel: "scheme / provider_peer_outlier",
        savingAmount: "8200.00",
        currency: "CNY",
        claimCount: 1,
        attributionCount: 2,
        roiLabel: "68.3x",
      },
    ]);
  });

  it("uses taxonomy labels for scheme ROI segment rows", () => {
    expect(
      buildDashboardSavingSegmentRows(
        [
          {
            segment_type: "scheme",
            segment_id: "provider_peer_outlier",
            saving_amount: "8200.00",
            currency: "CNY",
            claim_count: 1,
            attribution_count: 2,
            roi: 68.33,
          },
        ],
        { provider_peer_outlier: "Provider peer outlier (provider_peer_outlier)" },
      )[0].segmentLabel,
    ).toBe("scheme / Provider peer outlier (provider_peer_outlier)");
  });
});

describe("buildDashboardValueMeasurementSummary", () => {
  it("formats observed and estimated FWA value separately", () => {
    expect(
      buildDashboardValueMeasurementSummary({
        prevented_payment: "1000.00",
        recovered_amount: "250.00",
        avoided_future_exposure: "500.00",
        estimated_impact: "500.00",
        review_cost: "100.00",
        false_positive_operational_cost: "25.00",
        reviewer_capacity_hours: "0.25",
        net_value: "1650.00",
        currency: "CNY",
        evidence_caveat: "Estimated values require caveats.",
      }),
    ).toEqual({
      preventedPayment: "CNY 1000.00",
      recoveredAmount: "CNY 250.00",
      avoidedFutureExposure: "CNY 500.00",
      estimatedImpact: "CNY 500.00",
      reviewCost: "CNY 100.00",
      falsePositiveOperationalCost: "CNY 25.00",
      reviewerCapacityHours: "0.25h",
      netValue: "CNY 1650.00",
      evidenceCaveat: "Estimated values require caveats.",
    });
  });
});
