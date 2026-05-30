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
      memberIdLabel: "MBR-1",
      exposureLabel: "CNY 12800.00",
      highRiskClaimLabel: "1 / 4",
      highRiskRateLabel: "25.0%",
      latestClaimLabel: "CLM-4",
      riskLevelLabel: "High risk history",
      evidenceCount: 2,
      evidenceRefLabel: "2 refs",
      memberEvidenceStatus: "member_evidence_present",
      claimEvidenceStatus: "claim_evidence_partial",
      policyEvidenceStatus: "policy_evidence_missing",
      tpaEmbedReadiness: "profile_trace_ready",
    });
  });

  it("marks profile evidence complete when member claim and policy refs are present", () => {
    expect(
      buildMemberProfileInsight({
        member_id: "MBR-2",
        claim_count: 1,
        policy_count: 1,
        total_claim_amount: "8000.00",
        currency: "CNY",
        high_risk_claim_count: 0,
        latest_claim_id: "CLM-8",
        risk_level_summary: "no_high_risk_history",
        profile_summary: "Member has one claim.",
        evidence_refs: ["members:MBR-2", "claims:CLM-8", "policies:POL-8"],
      }),
    ).toMatchObject({
      memberEvidenceStatus: "member_evidence_present",
      claimEvidenceStatus: "claim_evidence_complete",
      policyEvidenceStatus: "policy_evidence_complete",
      tpaEmbedReadiness: "profile_trace_ready",
    });
  });

  it("marks TPA embed readiness incomplete without member or claim evidence", () => {
    expect(
      buildMemberProfileInsight({
        member_id: "MBR-3",
        claim_count: 2,
        policy_count: 0,
        total_claim_amount: "0.00",
        currency: "CNY",
        high_risk_claim_count: 0,
        latest_claim_id: null,
        risk_level_summary: "no_high_risk_history",
        profile_summary: "Member has no evidence refs.",
        evidence_refs: [],
      }),
    ).toMatchObject({
      memberEvidenceStatus: "member_evidence_missing",
      claimEvidenceStatus: "claim_evidence_missing",
      policyEvidenceStatus: "no_policy_history",
      tpaEmbedReadiness: "profile_trace_incomplete",
    });
  });

  it("returns empty labels before a profile is loaded", () => {
    expect(buildMemberProfileInsight(null)).toEqual({
      memberIdLabel: "none",
      exposureLabel: "-",
      highRiskClaimLabel: "0 / 0",
      highRiskRateLabel: "0.0%",
      latestClaimLabel: "none",
      riskLevelLabel: "no profile",
      evidenceCount: 0,
      evidenceRefLabel: "0 refs",
      memberEvidenceStatus: "not_available",
      claimEvidenceStatus: "not_available",
      policyEvidenceStatus: "not_available",
      tpaEmbedReadiness: "profile_not_loaded",
    });
  });
});
