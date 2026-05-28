import { describe, expect, it } from "vitest";
import { buildDashboardLabelPoolSummary } from "./DashboardPage";

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
