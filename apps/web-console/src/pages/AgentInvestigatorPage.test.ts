import { describe, expect, it } from "vitest";
import {
  buildAgentApprovalSummary,
  buildAgentEvidencePackageSummary,
  buildAgentSimilarCaseRows,
  buildEvidenceSufficiencyRows,
  buildInvestigationApprovalPayload,
} from "./AgentInvestigatorPage";

describe("buildEvidenceSufficiencyRows", () => {
  it("marks minimum evidence items as present or missing", () => {
    expect(
      buildEvidenceSufficiencyRows({
        scheme_family: "provider_peer_outlier",
        status: "needs_more_evidence",
        minimum_evidence: ["peer_group_definition", "specialty", "statistical_deviation"],
        present_evidence: ["peer_group_definition"],
        missing_evidence: ["specialty", "statistical_deviation"],
      }),
    ).toEqual([
      { item: "peer_group_definition", status: "present" },
      { item: "specialty", status: "missing" },
      { item: "statistical_deviation", status: "missing" },
    ]);
  });
});

describe("buildInvestigationApprovalPayload", () => {
  it("builds human approval payloads with agent run evidence", () => {
    expect(
      buildInvestigationApprovalPayload(
        {
          agent_run_id: "agent_CLM-1",
          decision_boundary: "assistive_only",
          risk_summary: "High risk evidence package.",
          findings: [],
          investigation_checklist: [],
          similar_cases: [],
          qa_opinion_draft: "Review manually.",
          evidence_sufficiency: {
            scheme_family: "provider_peer_outlier",
            status: "sufficient",
            minimum_evidence: [],
            present_evidence: [],
            missing_evidence: [],
          },
          evidence_refs: ["knowledge_cases:KC-1", "agent_run:agent_CLM-1"],
        },
        "approved",
        " qa-lead ",
      ),
    ).toEqual({
      decision: "approved",
      approver: "qa-lead",
      reason: "Evidence package approved for manual review routing.",
      evidence_refs: ["knowledge_cases:KC-1", "agent_run:agent_CLM-1"],
    });
  });
});

describe("buildAgentEvidencePackageSummary", () => {
  it("summarizes evidence package completeness for governed review", () => {
    expect(
      buildAgentEvidencePackageSummary({
        agent_run_id: "agent_CLM-1",
        decision_boundary: "assistive_only",
        risk_summary: "High risk evidence package.",
        findings: [
          { finding: "Peer outlier", evidence_refs: ["features:peer_amount"] },
          { finding: "Similar case match", evidence_refs: ["knowledge_cases:KC-1"] },
        ],
        investigation_checklist: ["Check diagnosis support", "Verify provider peer group"],
        similar_cases: [
          {
            case_id: "KC-1",
            similarity_score: 0.91,
            matched_signals: ["diagnosis_code"],
          },
        ],
        qa_opinion_draft: "Review manually.",
        evidence_sufficiency: {
          scheme_family: "provider_peer_outlier",
          status: "needs_more_evidence",
          minimum_evidence: ["peer_group_definition", "specialty"],
          present_evidence: ["peer_group_definition"],
          missing_evidence: ["specialty"],
        },
        evidence_refs: ["features:peer_amount", "knowledge_cases:KC-1"],
      }),
    ).toEqual({
      agentRunId: "agent_CLM-1",
      decisionBoundary: "assistive_only",
      findingCount: 2,
      checklistCount: 2,
      similarCaseCount: 1,
      evidenceRefCount: 2,
      missingEvidenceCount: 1,
      evidenceStatus: "needs_more_evidence",
    });
  });
});

describe("buildAgentApprovalSummary", () => {
  it("summarizes pending and decided human approval gates", () => {
    expect(buildAgentApprovalSummary(null)).toEqual({
      proposedAction: "manual_review_required",
      decision: "pending",
      approver: "not_assigned",
      auditId: "-",
      evidenceCount: 0,
      reason: "Awaiting human approval.",
    });

    expect(
      buildAgentApprovalSummary({
        audit_id: "audit_agent_approval_1",
        approval: {
          decision: "approved",
          approver: "qa-lead",
          proposed_action: "manual_review_required",
          reason: "Evidence package approved for manual review routing.",
          evidence_refs: ["agent_run:agent_CLM-1", "knowledge_cases:KC-1"],
        },
      }),
    ).toEqual({
      proposedAction: "manual_review_required",
      decision: "approved",
      approver: "qa-lead",
      auditId: "audit_agent_approval_1",
      evidenceCount: 2,
      reason: "Evidence package approved for manual review routing.",
    });
  });
});

describe("buildAgentSimilarCaseRows", () => {
  it("formats similar cases for the investigation evidence package", () => {
    expect(
      buildAgentSimilarCaseRows([
        {
          case_id: "KC-1001",
          similarity_score: 0.914,
          matched_signals: ["diagnosis_code", "provider_region"],
        },
        {
          case_id: "KC-1002",
          similarity_score: 0.72,
          matched_signals: [],
        },
      ]),
    ).toEqual([
      {
        caseId: "KC-1001",
        similarityLabel: "91%",
        matchedSignalLabel: "diagnosis_code, provider_region",
      },
      {
        caseId: "KC-1002",
        similarityLabel: "72%",
        matchedSignalLabel: "none",
      },
    ]);
  });
});
