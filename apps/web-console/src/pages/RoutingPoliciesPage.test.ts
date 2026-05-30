import { describe, expect, it } from "vitest";
import {
  buildRoutingPolicyCandidateSaveSummary,
  buildRoutingPolicyAuditFilters,
  buildRoutingPolicySummary,
  buildRoutingPolicyThresholdGovernance,
  RoutingPolicyRecord,
} from "./RoutingPoliciesPage";

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

describe("buildRoutingPolicyAuditFilters", () => {
  it("builds exact audit query filters for a routing policy version", () => {
    expect(buildRoutingPolicyAuditFilters(basePolicy)).toEqual({
      limit: 25,
      routing_policy_id: "fwa_risk_fusion_routing",
      routing_policy_version: 1,
      review_mode: "pre_payment",
    });
  });
});

describe("buildRoutingPolicyCandidateSaveSummary", () => {
  it("summarizes saved routing policy candidate thresholds for governance review", () => {
    expect(
      buildRoutingPolicyCandidateSaveSummary({
        ...basePolicy,
        version: 2,
        status: "draft",
        owner: "policy-ops",
        created_at: "2026-05-29T13:20:00Z",
        risk_thresholds: {
          low_max: 24,
          medium_min: 25,
          high_min: 65,
          critical_min: 88,
        },
        confidence_thresholds: {
          low_confidence_below: 55,
          high_confidence_min: 85,
        },
        provider_review_threshold: 72,
      }),
    ).toEqual({
      policyId: "fwa_risk_fusion_routing",
      versionLabel: "v2",
      reviewMode: "pre_payment",
      status: "draft",
      owner: "policy-ops",
      riskThresholdLabel: "Low <= 24, Medium >= 25, High >= 65, Critical >= 88",
      confidenceThresholdLabel: "Low confidence < 55, High confidence >= 85",
      providerThresholdLabel: "Provider review >= 72",
      createdAt: "2026-05-29T13:20:00Z",
    });
    expect(buildRoutingPolicyCandidateSaveSummary(null)).toBeNull();
  });
});

describe("buildRoutingPolicyThresholdGovernance", () => {
  it("summarizes L7 routing thresholds and aligned provider review boundary", () => {
    expect(buildRoutingPolicyThresholdGovernance(basePolicy)).toEqual({
      thresholdIntegrity: "thresholds_ordered",
      riskRouteBand: "low<=30 medium>=31 high>=70 critical>=90",
      confidenceRouteBand: "low_confidence<60 high_confidence>=80",
      providerRouteBand: "provider_review>=75",
      routeBoundaryStatus: "provider_route_high_risk_aligned",
    });
  });

  it("flags unordered routing thresholds for review", () => {
    expect(
      buildRoutingPolicyThresholdGovernance({
        ...basePolicy,
        risk_thresholds: {
          low_max: 30,
          medium_min: 30,
          high_min: 70,
          critical_min: 90,
        },
      }),
    ).toMatchObject({
      thresholdIntegrity: "threshold_review_required",
    });
  });

  it("marks medium-risk provider routing boundaries explicitly", () => {
    expect(
      buildRoutingPolicyThresholdGovernance({
        ...basePolicy,
        provider_review_threshold: 60,
      }),
    ).toMatchObject({
      thresholdIntegrity: "thresholds_ordered",
      providerRouteBand: "provider_review>=60",
      routeBoundaryStatus: "provider_route_medium_risk",
    });
  });

  it("uses not-available labels before a policy is selected", () => {
    expect(buildRoutingPolicyThresholdGovernance(null)).toEqual({
      thresholdIntegrity: "not_available",
      riskRouteBand: "not_available",
      confidenceRouteBand: "not_available",
      providerRouteBand: "not_available",
      routeBoundaryStatus: "not_available",
    });
  });
});
