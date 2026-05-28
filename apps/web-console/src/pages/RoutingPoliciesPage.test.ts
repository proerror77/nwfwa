import { describe, expect, it } from "vitest";
import { buildRoutingPolicySummary, RoutingPolicyRecord } from "./RoutingPoliciesPage";

const basePolicy: RoutingPolicyRecord = {
  policy_id: "fwa_risk_fusion_routing",
  version: 1,
  review_mode: "pre_payment",
  status: "active",
  owner: "system",
  risk_thresholds: {
    low_max: 30,
    medium_min: 31,
    high_min: 70,
    critical_min: 90,
  },
  confidence_thresholds: {
    low_confidence_below: 60,
    high_confidence_min: 80,
  },
  provider_review_threshold: 75,
  activated_at: null,
  created_at: null,
};

describe("buildRoutingPolicySummary", () => {
  it("summarizes routing policy lifecycle status for L7 governance", () => {
    const summary = buildRoutingPolicySummary([
      basePolicy,
      {
        ...basePolicy,
        version: 2,
        status: "draft",
      },
      {
        ...basePolicy,
        version: 3,
        review_mode: "post_payment",
        status: "submitted",
      },
      {
        ...basePolicy,
        version: 4,
        review_mode: "both",
        status: "approved",
      },
    ]);

    expect(summary).toEqual({
      policyCount: 4,
      activeCount: 1,
      draftCount: 1,
      submittedCount: 1,
      approvedCount: 1,
      reviewModeCount: 3,
    });
  });
});
