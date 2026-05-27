import { describe, expect, it } from "vitest";
import { buildProviderProfileInspection } from "./providerProfileInspection";

describe("buildProviderProfileInspection", () => {
  it("summarizes provider review route, outliers, and evidence", () => {
    const inspection = buildProviderProfileInspection({
      provider_id: "PRV-PROVIDER-1",
      risk_score: 86,
      risk_tier: "high",
      review_required: true,
      review_route: "provider_review",
      outlier_flags: ["peer_amount_p97", "high_cost_item_ratio_90d"],
      evidence_refs: ["provider_profile:PRV-PROVIDER-1:90d"],
      window_findings: [
        {
          window_days: 30,
          risk_score: 58,
          reason: "Provider 30d behavior is elevated.",
        },
        {
          window_days: 90,
          risk_score: 86,
          reason: "Provider 90d peer amount percentile is high.",
        },
      ],
    });

    expect(inspection).toEqual({
      providerId: "PRV-PROVIDER-1",
      routeLabel: "provider_review",
      reviewLabel: "Review required",
      maxWindowLabel: "90d / 86",
      outlierSummary: "peer_amount_p97, high_cost_item_ratio_90d",
      evidenceSummary: "provider_profile:PRV-PROVIDER-1:90d",
    });
  });

  it("does not build an inspection when provider profile is missing", () => {
    expect(buildProviderProfileInspection(undefined)).toBeNull();
  });
});
