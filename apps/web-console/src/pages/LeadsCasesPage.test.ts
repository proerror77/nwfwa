import { describe, expect, it } from "vitest";
import {
  buildCaseStatusUpdateSummary,
  buildCaseEvidenceSufficiencyRows,
  buildInvestigationResultPayload,
  buildInvestigationWritebackSummary,
  buildLeadTriageSummary,
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

describe("buildLeadTriageSummary", () => {
  it("summarizes lead triage workflow results", () => {
    expect(
      buildLeadTriageSummary({
        audit_id: "audit_lead_triaged_1",
        lead: {
          lead_id: "lead_CLM-1",
          claim_id: "CLM-1",
          scheme_family: "early_high_value_claim",
          status: "triaged",
          disposition: "open_case",
          risk_score: 91,
          rag: "RED",
          evidence_refs: ["audit:scoring.completed"],
        },
        case: {
          case_id: "case_CLM-1",
          lead_id: "lead_CLM-1",
          claim_id: "CLM-1",
          status: "triage",
          priority: "high",
          assignee: "siu-reviewer-1",
          reviewer: "medical-reviewer-1",
          evidence_package: {
            evidence_refs: ["audit:scoring.completed"],
          },
        },
      }),
    ).toEqual({
      auditId: "audit_lead_triaged_1",
      leadId: "lead_CLM-1",
      claimId: "CLM-1",
      disposition: "open_case",
      status: "triaged",
      riskScore: 91,
      rag: "RED",
      evidenceCount: 1,
      caseId: "case_CLM-1",
      caseStatus: "triage",
      casePriority: "high",
    });
    expect(buildLeadTriageSummary(null)).toBeNull();
  });
});

describe("buildCaseStatusUpdateSummary", () => {
  it("summarizes case status workflow updates", () => {
    expect(
      buildCaseStatusUpdateSummary({
        audit_id: "audit_case_status_1",
        case: {
          case_id: "case_CLM-1",
          lead_id: "lead_CLM-1",
          claim_id: "CLM-1",
          status: "investigating",
          priority: "high",
          assignee: "siu-reviewer-1",
          reviewer: "medical-reviewer-1",
          evidence_package: {
            evidence_refs: ["case_workflow:investigating", "audit:scoring.completed"],
          },
        },
      }),
    ).toEqual({
      auditId: "audit_case_status_1",
      caseId: "case_CLM-1",
      claimId: "CLM-1",
      status: "investigating",
      priority: "high",
      assignee: "siu-reviewer-1",
      reviewer: "medical-reviewer-1",
      slaStatus: "not_available",
      evidenceCount: 2,
    });
    expect(buildCaseStatusUpdateSummary(null)).toBeNull();
  });
});

describe("buildInvestigationResultPayload", () => {
  it("builds a TPA investigation result writeback with deduplicated evidence refs", () => {
    expect(
      buildInvestigationResultPayload(
        {
          case_id: "case_CLM-1",
          lead_id: "lead_CLM-1",
          claim_id: "CLM-1",
          status: "investigating",
          priority: "high",
          assignee: "siu-reviewer-1",
          reviewer: "medical-reviewer-1",
          evidence_package: {
            evidence_refs: ["audit:scoring.completed", "agent_run:agent_CLM-1"],
          },
        },
        {
          investigationId: " INV-CLM-1 ",
          outcome: "confirmed_fwa",
          confirmedFwa: true,
          financialImpactType: "prevented_payment",
          savingAmount: "8200.00",
          currency: "CNY",
          notes: "TPA investigation confirmed over-treatment signals.",
          evidenceRefsText: "agent_run:agent_CLM-1\nmedical_review:MR-1",
        },
      ),
    ).toEqual({
      claim_id: "CLM-1",
      investigation_id: "INV-CLM-1",
      outcome: "confirmed_fwa",
      confirmed_fwa: true,
      financial_impact_type: "prevented_payment",
      saving_amount: "8200.00",
      currency: "CNY",
      notes: "TPA investigation confirmed over-treatment signals.",
      evidence_refs: [
        "investigation_cases:case_CLM-1",
        "audit:scoring.completed",
        "agent_run:agent_CLM-1",
        "medical_review:MR-1",
      ],
    });
  });
});

describe("buildInvestigationWritebackSummary", () => {
  it("summarizes investigation writeback audit output", () => {
    expect(
      buildInvestigationWritebackSummary({
        claim_id: "CLM-1",
        event_type: "investigation.result.received",
        event_status: "succeeded",
        audit_id: "audit_investigation_1",
        run_id: "investigation_INV-CLM-1",
        evidence_refs: ["investigation_results:INV-CLM-1"],
      }),
    ).toEqual({
      claimId: "CLM-1",
      eventType: "investigation.result.received",
      eventStatus: "succeeded",
      auditId: "audit_investigation_1",
      runId: "investigation_INV-CLM-1",
      evidenceCount: 1,
      evidenceRefs: ["investigation_results:INV-CLM-1"],
    });
    expect(buildInvestigationWritebackSummary(null)).toBeNull();
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
