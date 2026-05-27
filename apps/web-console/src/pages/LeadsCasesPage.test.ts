import { describe, expect, it } from "vitest";
import { buildLeadSummary } from "./LeadsCasesPage";

describe("buildLeadSummary", () => {
  it("summarizes lead lifecycle and case workload for operations", () => {
    const summary = buildLeadSummary(
      {
        leads: [
          {
            lead_id: "lead_CLM-1",
            claim_id: "CLM-1",
            scheme_family: "early_high_value_claim",
            status: "new",
            disposition: "pending_triage",
            risk_score: 82,
            rag: "Red",
            evidence_refs: ["audit:scoring.completed"],
          },
          {
            lead_id: "lead_CLM-2",
            claim_id: "CLM-2",
            scheme_family: "provider_peer_outlier",
            status: "triaged",
            disposition: "open_case",
            risk_score: 73,
            rag: "Amber",
            evidence_refs: [],
          },
        ],
      },
      {
        cases: [
          {
            case_id: "case_CLM-2",
            lead_id: "lead_CLM-2",
            claim_id: "CLM-2",
            status: "triage",
            priority: "high",
            assignee: "siu-reviewer-1",
            reviewer: "medical-reviewer-1",
          },
        ],
      },
    );

    expect(summary).toEqual({
      totalLeads: 2,
      pendingTriage: 1,
      openCases: 1,
      highPriorityCases: 1,
      topScheme: "early_high_value_claim",
    });
  });
});
