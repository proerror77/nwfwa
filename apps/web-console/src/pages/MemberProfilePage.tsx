import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getMemberProfileSummary } from "../api";

type MemberProfileSummary = {
  member_id: string;
  claim_count: number;
  policy_count: number;
  total_claim_amount: string;
  currency: string;
  high_risk_claim_count: number;
  latest_claim_id?: string | null;
  risk_level_summary: string;
  profile_summary: string;
  evidence_refs: string[];
};

export function buildMemberProfileInsight(profile?: MemberProfileSummary | null) {
  if (!profile) {
    return {
      memberIdLabel: "none",
      exposureLabel: "-",
      highRiskClaimLabel: "0 / 0",
      highRiskRateLabel: "0.0%",
      latestClaimLabel: "none",
      riskLevelLabel: "no profile",
      evidenceCount: 0,
      evidenceRefLabel: "0 refs",
    };
  }
  return {
    memberIdLabel: profile.member_id,
    exposureLabel: `${profile.currency} ${profile.total_claim_amount}`,
    highRiskClaimLabel: `${profile.high_risk_claim_count} / ${profile.claim_count}`,
    highRiskRateLabel:
      profile.claim_count === 0
        ? "0.0%"
        : `${((profile.high_risk_claim_count / profile.claim_count) * 100).toFixed(1)}%`,
    latestClaimLabel: profile.latest_claim_id ?? "none",
    riskLevelLabel:
      profile.risk_level_summary === "has_high_risk_history"
        ? "High risk history"
        : "No high risk history",
    evidenceCount: profile.evidence_refs.length,
    evidenceRefLabel: `${profile.evidence_refs.length} refs`,
  };
}

export function MemberProfilePage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [memberIdInput, setMemberIdInput] = useState("MBR-PROFILE-1");
  const [memberId, setMemberId] = useState("MBR-PROFILE-1");
  const memberProfileQuery = useQuery({
    queryKey: ["member-profile-summary", apiKey, memberId],
    queryFn: () => getMemberProfileSummary(memberId, apiKey) as Promise<MemberProfileSummary>,
    enabled: memberId.trim().length > 0,
  });
  const profile = memberProfileQuery.data;
  const insight = buildMemberProfileInsight(profile);

  return (
    <section className="ops-grid">
      <div className="panel dashboard-header">
        <div>
          <h2>Member Profile</h2>
          <p>TPA-facing member claim history, high-risk exposure, and evidence summary.</p>
        </div>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
      </div>

      <div className="panel">
        <h2>Lookup</h2>
        <label>
          Member ID
          <input
            value={memberIdInput}
            onChange={(event) => setMemberIdInput(event.target.value)}
          />
        </label>
        <button
          disabled={memberIdInput.trim().length === 0 || memberProfileQuery.isFetching}
          onClick={() => setMemberId(memberIdInput.trim())}
          type="button"
        >
          Load Profile
        </button>
        {memberProfileQuery.error ? (
          <pre className="error">{String(memberProfileQuery.error.message)}</pre>
        ) : null}
      </div>

      <div className="panel">
        <h2>Risk Summary</h2>
        <div className="summary-grid">
          <div>
            <span>Member</span>
            <strong>{insight.memberIdLabel}</strong>
          </div>
          <div>
            <span>Claims</span>
            <strong>{profile?.claim_count ?? 0}</strong>
          </div>
          <div>
            <span>Policies</span>
            <strong>{profile?.policy_count ?? 0}</strong>
          </div>
          <div>
            <span>Exposure</span>
            <strong>{insight.exposureLabel}</strong>
          </div>
          <div>
            <span>High Risk Rate</span>
            <strong>{insight.highRiskRateLabel}</strong>
          </div>
          <div>
            <span>High Risk Claims</span>
            <strong>{insight.highRiskClaimLabel}</strong>
          </div>
          <div>
            <span>Risk History</span>
            <strong>{insight.riskLevelLabel}</strong>
          </div>
          <div>
            <span>Latest Claim</span>
            <strong>{insight.latestClaimLabel}</strong>
          </div>
          <div>
            <span>Evidence Refs</span>
            <strong>{insight.evidenceRefLabel}</strong>
          </div>
        </div>
      </div>

      <div className="panel wide-panel">
        <h2>Profile Narrative</h2>
        {profile ? (
          <div className="result-stack">
            <p>{profile.profile_summary}</p>
            <div className="table-list">
              {profile.evidence_refs.map((reference) => (
                <div className="metric-row compact-metric-row" key={reference}>
                  <span>{reference}</span>
                  <strong>evidence</strong>
                </div>
              ))}
            </div>
            {insight.evidenceCount === 0 ? <p className="empty">No evidence refs</p> : null}
          </div>
        ) : (
          <p className="empty">Load a member profile</p>
        )}
      </div>
    </section>
  );
}
