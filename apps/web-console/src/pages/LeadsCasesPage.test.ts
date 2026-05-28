import { describe, expect, it } from "vitest";
import {
  buildCaseEvidenceSufficiencyRows,
  buildLeadSummary,
  caseEvidenceSufficiencyFromPackage,
} from "./LeadsCasesPage";

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

describe("caseEvidenceSufficiencyFromPackage", () => {
  it("extracts case evidence sufficiency from the evidence package", () => {
    const sufficiency = caseEvidenceSufficiencyFromPackage({
      evidence_sufficiency: {
        scheme_family: "provider_peer_outlier",
        status: "needs_more_evidence",
        minimum_evidence: ["peer_group_definition", "specialty", "statistical_deviation"],
        present_evidence: ["peer_group_definition"],
        missing_evidence: ["specialty", "statistical_deviation"],
      },
    });

    expect(sufficiency?.status).toBe("needs_more_evidence");
    expect(buildCaseEvidenceSufficiencyRows(sufficiency)).toEqual([
      { item: "peer_group_definition", status: "present" },
      { item: "specialty", status: "missing" },
      { item: "statistical_deviation", status: "missing" },
    ]);
  });

  it("ignores malformed evidence sufficiency payloads", () => {
    expect(
      caseEvidenceSufficiencyFromPackage({
        evidence_sufficiency: {
          scheme_family: "provider_peer_outlier",
          status: "needs_more_evidence",
          minimum_evidence: "peer_group_definition",
          present_evidence: [],
          missing_evidence: [],
        },
      }),
    ).toBeNull();
  });
});
