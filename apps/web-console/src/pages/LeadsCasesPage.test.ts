import { describe, expect, it } from "vitest";
import {
  buildCaseStatusUpdateSummary,
  buildCaseEvidenceSufficiencyRows,
  buildLeadTriagePayload,
  buildInvestigationResultPayload,
  buildInvestigationWritebackSummary,
  buildLeadTriageSummary,
  buildLeadSummary,
  caseEvidenceBucketsFromPackage,
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
            sla_target_hours: 48,
            sla_status: "breached",
            time_to_triage_hours: 1.25,
            time_to_closure_hours: null,
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
      breachedCases: 1,
      onTrackCases: 0,
      closedCases: 0,
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

  it("extracts PRD evidence buckets from the evidence package", () => {
    expect(
      caseEvidenceBucketsFromPackage({
        evidence_refs_by_type: {
          claim: ["claims:CLM-1"],
          rule: ["rule_runs:EARLY_CLAIM"],
          model: ["model_scores:fwa_baseline"],
          anomaly: ["scoring_runs:run_1:anomaly_score"],
          document: ["documents:DOC-1"],
          similar_case: ["knowledge_cases:KC-1001"],
        },
      }),
    ).toEqual({
      claimRefs: ["claims:CLM-1"],
      ruleRefs: ["rule_runs:EARLY_CLAIM"],
      modelRefs: ["model_scores:fwa_baseline"],
      anomalyRefs: ["scoring_runs:run_1:anomaly_score"],
      documentRefs: ["documents:DOC-1"],
      similarCaseRefs: ["knowledge_cases:KC-1001"],
    });
  });
});

describe("buildLeadTriageSummary", () => {
  it("builds lead triage payloads with evidence refs for the API contract", () => {
    expect(
      buildLeadTriagePayload(
        {
          lead_id: "lead_CLM-1",
          claim_id: "CLM-1",
          scheme_family: "early_high_value_claim",
          status: "new",
          disposition: "pending_triage",
          risk_score: 91,
          rag: "RED",
          evidence_refs: ["rule_runs:EARLY_CLAIM", "rule_runs:EARLY_CLAIM"],
        },
        "open_case",
        "",
        "siu-reviewer-1",
        "medical-reviewer-1",
        "high",
        "Open investigation.",
      ),
    ).toEqual({
      decision: "open_case",
      merge_target_lead_id: undefined,
      assignee: "siu-reviewer-1",
      reviewer: "medical-reviewer-1",
      priority: "high",
      notes: "Open investigation.",
      evidence_refs: [
        "triage_decisions:open_case",
        "leads:lead_CLM-1",
        "rule_runs:EARLY_CLAIM",
      ],
    });
  });

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
          sla_target_hours: 48,
          sla_status: "on_track",
          time_to_triage_hours: 1.5,
          time_to_closure_hours: null,
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
      slaStatus: "on_track",
      slaTargetHours: 48,
      timeToTriageHours: "1.5",
      timeToClosureHours: "open",
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
      case_id: "case_CLM-1",
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
