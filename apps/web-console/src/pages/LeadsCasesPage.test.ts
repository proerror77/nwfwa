import { describe, expect, it } from "vitest";
import {
  buildCaseEvidenceSufficiencyRows,
  buildLeadSummary,
  caseEvidenceRefsFromPackage,
  caseEvidenceSufficiencyFromPackage,
  caseRoutingReason,
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
            evidence_refs: ["lead:triaged"],
          },
          {
            lead_id: "lead_CLM-3",
            claim_id: "CLM-3",
            scheme_family: "medical_necessity",
            status: "pending_evidence",
            disposition: "request_evidence",
            risk_score: 68,
            rag: "Amber",
            evidence_refs: [],
          },
          {
            lead_id: "lead_CLM-4",
            claim_id: "CLM-4",
            scheme_family: "duplicate_billing",
            status: "closed",
            disposition: "rejected",
            risk_score: 31,
            rag: "Green",
            evidence_refs: ["lead:rejected"],
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
            evidence_package: {
              evidence_sufficiency: {
                scheme_family: "provider_peer_outlier",
                status: "needs_more_evidence",
                minimum_evidence: ["peer_group_definition", "statistical_deviation"],
                present_evidence: ["peer_group_definition"],
                missing_evidence: ["statistical_deviation"],
              },
            },
          },
        ],
      },
    );

    expect(summary).toEqual({
      totalLeads: 4,
      pendingTriage: 1,
      openCaseLeads: 1,
      evidenceBackedLeads: 3,
      requestEvidenceLeads: 1,
      closedLeads: 1,
      openCases: 1,
      casesMissingEvidence: 1,
      highPriorityCases: 1,
      topScheme: "early_high_value_claim",
    });
  });
});

describe("caseEvidenceRefsFromPackage", () => {
  it("extracts case evidence refs from the evidence package", () => {
    expect(
      caseEvidenceRefsFromPackage({
        evidence_refs: ["audit:scoring.completed", "rule_runs:EARLY_CLAIM"],
      }),
    ).toEqual(["audit:scoring.completed", "rule_runs:EARLY_CLAIM"]);
  });

  it("ignores malformed evidence refs", () => {
    expect(caseEvidenceRefsFromPackage({ evidence_refs: "audit:scoring.completed" })).toEqual([]);
  });
});

describe("caseRoutingReason", () => {
  it("prefers the case routing reason and falls back to evidence package reason", () => {
    const baseCase = {
      case_id: "case_CLM-2",
      lead_id: "lead_CLM-2",
      claim_id: "CLM-2",
      status: "triage",
      priority: "high",
      assignee: "siu-reviewer-1",
      reviewer: "medical-reviewer-1",
    };

    expect(
      caseRoutingReason({
        ...baseCase,
        routing_reason: "High risk provider pattern",
        evidence_package: { reason: "Fallback reason" },
      }),
    ).toBe("High risk provider pattern");
    expect(caseRoutingReason({ ...baseCase, evidence_package: { reason: "Fallback reason" } })).toBe(
      "Fallback reason",
    );
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
