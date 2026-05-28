import { describe, expect, it } from "vitest";
import { buildMemberProfileInsight } from "./MemberProfilePage";

describe("buildMemberProfileInsight", () => {
  it("summarizes member profile exposure and high-risk history", () => {
    expect(
      buildMemberProfileInsight({
        member_id: "MBR-1",
        claim_count: 4,
        policy_count: 2,
        total_claim_amount: "12800.00",
        currency: "CNY",
        high_risk_claim_count: 1,
        latest_claim_id: "CLM-4",
        risk_level_summary: "has_high_risk_history",
        profile_summary: "Member has 4 historical claims.",
        evidence_refs: ["members:MBR-1", "claims:CLM-4"],
      }),
    ).toEqual({
      exposureLabel: "CNY 12800.00",
      highRiskRateLabel: "25.0%",
      latestClaimLabel: "CLM-4",
      riskLevelLabel: "High risk history",
      evidenceCount: 2,
    });
  });

  it("returns empty labels before a profile is loaded", () => {
    expect(buildMemberProfileInsight(null)).toEqual({
      exposureLabel: "-",
      highRiskRateLabel: "0.0%",
      latestClaimLabel: "none",
      riskLevelLabel: "no profile",
      evidenceCount: 0,
    });
  });
});
