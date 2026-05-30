import { describe, expect, it } from "vitest";
import { buildProviderRiskOpsSummary, filterProviderRiskItems } from "./ProviderRiskPage";

const providers = [
  {
    provider_id: "PRV-1",
    risk_score: 91,
    risk_tier: "high",
    review_required: true,
    review_route: "provider_review",
    claim_count: 3,
    network_risk_score: null,
    latest_claim_id: "CLM-1",
    outlier_flags: ["peer_amount_p97"],
    graph_reasons: [],
    evidence_refs: ["provider_profile:PRV-1:90d"],
  },
  {
    provider_id: "PRV-2",
    risk_score: 42,
    risk_tier: "medium",
    review_required: false,
    review_route: "standard_review",
    claim_count: 1,
    network_risk_score: 82,
    latest_claim_id: "CLM-2",
    outlier_flags: [],
    graph_reasons: ["Provider 所在关系社区整体风险偏高"],
    evidence_refs: [],
  },
];

describe("buildProviderRiskOpsSummary", () => {
  it("summarizes provider risk pressure for operations", () => {
    expect(
      buildProviderRiskOpsSummary({
        provider_count: 4,
        review_required_count: 2,
        high_risk_count: 1,
        providers,
      }),
    ).toEqual({
      providerCount: 4,
      reviewRequiredCount: 2,
      highRiskCount: 1,
      graphRiskCount: 1,
      evidenceBackedCount: 1,
      networkScoreCount: 1,
      graphReasonCount: 1,
      graphEvidenceGapCount: 0,
      graphEvidenceStatus: "graph_evidence_complete",
      reviewRateLabel: "50.0%",
    });
  });

  it("flags graph risk evidence gaps for provider review", () => {
    expect(
      buildProviderRiskOpsSummary({
        provider_count: 1,
        review_required_count: 1,
        high_risk_count: 0,
        providers: [
          {
            provider_id: "PRV-GRAPH-GAP",
            risk_score: 70,
            risk_tier: "medium",
            review_required: true,
            review_route: "provider_graph_review",
            claim_count: 4,
            network_risk_score: 88,
            latest_claim_id: "CLM-GRAPH-GAP",
            outlier_flags: [],
            graph_reasons: ["Shared high-risk community"],
            evidence_refs: [],
          },
        ],
      }),
    ).toMatchObject({
      graphRiskCount: 1,
      evidenceBackedCount: 0,
      networkScoreCount: 1,
      graphReasonCount: 1,
      graphEvidenceGapCount: 1,
      graphEvidenceStatus: "graph_evidence_gap",
    });
  });

  it("uses no graph risk status when no provider has L6 graph signals", () => {
    expect(
      buildProviderRiskOpsSummary({
        provider_count: 1,
        review_required_count: 0,
        high_risk_count: 0,
        providers: [
          {
            provider_id: "PRV-NORMAL",
            risk_score: 21,
            risk_tier: "low",
            review_required: false,
            review_route: "standard_review",
            claim_count: 1,
            network_risk_score: null,
            latest_claim_id: "CLM-NORMAL",
            outlier_flags: [],
            graph_reasons: [],
            evidence_refs: [],
          },
        ],
      }),
    ).toMatchObject({
      graphRiskCount: 0,
      graphEvidenceGapCount: 0,
      graphEvidenceStatus: "no_graph_risk",
    });
  });
});

describe("filterProviderRiskItems", () => {
  it("filters provider risk queues by review and high risk state", () => {
    expect(filterProviderRiskItems(providers, "all")).toEqual(providers);
    expect(filterProviderRiskItems(providers, "review_required")).toEqual([providers[0]]);
    expect(filterProviderRiskItems(providers, "high_risk")).toEqual([providers[0]]);
  });
});
