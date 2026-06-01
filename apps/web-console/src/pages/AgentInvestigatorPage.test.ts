import { describe, expect, it } from "vitest";
import {
  buildAgentApprovalSummary,
  buildAgentEvidenceBucketRows,
  buildAgentEvidencePackageSummary,
  buildAgentSimilarCaseRows,
  buildEvidenceSufficiencyRows,
  buildAgentInvestigatorDefaults,
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
        evidence_refs_by_type: {
          claim: ["claim:CLM-1:top_reason:1"],
          rule: ["rule_runs:EARLY_CLAIM"],
          model: ["model_scores:fwa_baseline"],
          anomaly: [],
          document: [],
          similar_case: ["knowledge_cases:KC-1"],
        },
      }),
    ).toEqual({
      agentRunId: "agent_CLM-1",
      decisionBoundary: "assistive_only",
      findingCount: 2,
      checklistCount: 2,
      similarCaseCount: 1,
      evidenceRefCount: 2,
      bucketedEvidenceCount: 4,
      missingEvidenceCount: 1,
      evidenceStatus: "needs_more_evidence",
    });
  });

  it("builds bucket rows for PRD evidence references", () => {
    expect(
      buildAgentEvidenceBucketRows({
        claim: ["claim:CLM-1:top_reason:1"],
        rule: ["rule_runs:EARLY_CLAIM"],
        model: ["model_scores:fwa_baseline"],
        anomaly: ["scoring_runs:run_1:anomaly_score"],
        document: [],
        similar_case: ["knowledge_cases:KC-1"],
      }),
    ).toEqual([
      { label: "Claim", count: 1, refs: ["claim:CLM-1:top_reason:1"] },
      { label: "Rule", count: 1, refs: ["rule_runs:EARLY_CLAIM"] },
      { label: "Model", count: 1, refs: ["model_scores:fwa_baseline"] },
      { label: "Anomaly", count: 1, refs: ["scoring_runs:run_1:anomaly_score"] },
      { label: "Document", count: 0, refs: [] },
      { label: "Similar Case", count: 1, refs: ["knowledge_cases:KC-1"] },
    ]);
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

describe("buildAgentInvestigatorDefaults", () => {
  it("uses runtime scoring context to prefill the investigation form", () => {
    expect(
      buildAgentInvestigatorDefaults({
        source: "runtime_scoring",
        sourceRunId: "run_CLM-0287",
        claimId: "CLM-0287",
        riskScore: 87,
        rag: "RED",
        schemeFamily: "early_high_value_claim",
        topReasons: ["金额高于同病种 P99", "诊断-项目匹配度偏低"],
        diagnosisCode: "J10",
        providerRegion: "Shanghai",
        tags: ["provider_region", "early_high_claim"],
      }),
    ).toEqual({
      claimId: "CLM-0287",
      riskScore: 87,
      rag: "RED",
      schemeFamily: "early_high_value_claim",
      topReasons: "金额高于同病种 P99\n诊断-项目匹配度偏低",
      diagnosisCode: "J10",
      providerRegion: "Shanghai",
      tags: "provider_region, early_high_claim",
    });
  });
});
