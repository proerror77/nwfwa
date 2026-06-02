import { describe, expect, it } from "vitest";
import {
  buildQaCanonicalTraceSummary,
  buildQaFeedbackLoopSummary,
  buildQaSubmitSummary,
  buildQaEvidenceRefs,
  canSubmitQaQueueItem,
  canUpdateQaFeedbackItem,
  QA_CONCLUSION_OPTIONS,
  QA_FEEDBACK_TARGET_OPTIONS,
  QA_ISSUE_TYPE_OPTIONS,
  QA_SUMMARY_FEEDBACK_ROWS,
  selectQaQueueItem,
} from "./QAReviewPage";

describe("QAReviewPage helpers", () => {
  it("selects the requested QA queue item or falls back to the first real queue item", () => {
    const queue = [
      {
        qa_case_id: "qa_sample_1_lead_1",
        sample_id: "sample_1",
        lead_id: "lead_1",
        claim_id: "CLM-1",
        scheme_family: "provider_peer_outlier",
        rag: "RED",
        risk_score: 82,
        reviewer: "qa-reviewer-1",
        assignment_queue: "QA Review",
        status: "open",
        evidence_refs: ["audit:scoring.completed"],
      },
      {
        qa_case_id: "qa_sample_1_lead_2",
        sample_id: "sample_1",
        lead_id: "lead_2",
        claim_id: "CLM-2",
        scheme_family: "medical_necessity",
        rag: "RED",
        risk_score: 91,
        reviewer: "qa-reviewer-1",
        assignment_queue: "QA Review",
        status: "open",
        evidence_refs: ["audit:scoring.completed"],
      },
    ];

    expect(selectQaQueueItem(queue, "qa_sample_1_lead_2")?.claim_id).toBe("CLM-2");
    expect(selectQaQueueItem(queue, "missing")?.claim_id).toBe("CLM-1");
    expect(selectQaQueueItem([], "missing")).toBeNull();
  });

  it("allows only open QA queue items to be submitted", () => {
    expect(
      canSubmitQaQueueItem({
        qa_case_id: "qa_sample_1_lead_1",
        sample_id: "sample_1",
        lead_id: "lead_1",
        claim_id: "CLM-1",
        scheme_family: "provider_peer_outlier",
        rag: "RED",
        risk_score: 82,
        reviewer: "qa-reviewer-1",
        assignment_queue: "QA Review",
        status: "open",
        evidence_refs: ["audit:scoring.completed"],
      }),
    ).toBe(true);
    expect(
      canSubmitQaQueueItem({
        qa_case_id: "qa_sample_1_lead_1",
        sample_id: "sample_1",
        lead_id: "lead_1",
        claim_id: "CLM-1",
        scheme_family: "provider_peer_outlier",
        rag: "RED",
        risk_score: 82,
        reviewer: "qa-reviewer-1",
        assignment_queue: "QA Review",
        status: "reviewed",
        qa_conclusion: "pass",
        evidence_refs: ["audit:scoring.completed"],
      }),
    ).toBe(false);
    expect(canSubmitQaQueueItem(null)).toBe(false);
  });

  it("allows only actionable QA feedback statuses to be updated", () => {
    const item = {
      feedback_id: "qa_feedback_QA-1",
      qa_case_id: "QA-1",
      claim_id: "CLM-1",
      feedback_target: "rules",
      issue_type: "alert_handling_incomplete",
      priority: "high",
      status: "open",
      summary: "Rule feedback",
      note_present: true,
      evidence_refs: ["rule_runs:EARLY_CLAIM"],
    };

    expect(canUpdateQaFeedbackItem(item)).toBe(true);
    expect(canUpdateQaFeedbackItem({ ...item, status: "in_progress" })).toBe(true);
    expect(canUpdateQaFeedbackItem({ ...item, status: "resolved" })).toBe(false);
    expect(canUpdateQaFeedbackItem({ ...item, status: "dismissed" })).toBe(false);
  });

  it("builds QA evidence refs from the selected sampled lead", () => {
    expect(
      buildQaEvidenceRefs({
        qa_case_id: "qa_sample_1_lead_1",
        sample_id: "sample_1",
        lead_id: "lead_1",
        claim_id: "CLM-1",
        scheme_family: "provider_peer_outlier",
        rag: "RED",
        risk_score: 82,
        reviewer: "qa-reviewer-1",
        assignment_queue: "QA Review",
        status: "open",
        evidence_refs: ["audit:scoring.completed", "lead:lead_1"],
        canonical_evidence_refs: ["invoice:INV-QA:fee_detail:LINE-1"],
        canonical_source_refs: ["medical_record:MR-QA-1"],
      }).split("\n"),
    ).toEqual([
      "qa_queue:qa_sample_1_lead_1",
      "audit_sample:sample_1",
      "lead:lead_1",
      "audit:scoring.completed",
      "invoice:INV-QA:fee_detail:LINE-1",
    ]);
    expect(buildQaEvidenceRefs(null)).toBe("");
  });

  it("summarizes canonical trace refs for QA review detail", () => {
    expect(
      buildQaCanonicalTraceSummary({
        qa_case_id: "qa_sample_1_lead_1",
        sample_id: "sample_1",
        lead_id: "lead_1",
        claim_id: "CLM-1",
        scheme_family: "provider_peer_outlier",
        rag: "RED",
        risk_score: 82,
        reviewer: "qa-reviewer-1",
        assignment_queue: "QA Review",
        status: "open",
        evidence_refs: ["audit:scoring.completed"],
        canonical_evidence_refs: ["invoice:INV-QA:fee_detail:LINE-1"],
        canonical_source_refs: ["medical_record:MR-QA-1"],
      }),
    ).toEqual({
      sourceRefCount: 1,
      evidenceRefCount: 1,
      hasCanonicalTrace: true,
    });
    expect(buildQaCanonicalTraceSummary(null)).toEqual({
      sourceRefCount: 0,
      evidenceRefCount: 0,
      hasCanonicalTrace: false,
    });
  });

  it("summarizes QA writeback audit metadata for the operator surface", () => {
    expect(
      buildQaSubmitSummary({
        claim_id: "CLM-1",
        event_type: "qa.result.received",
        event_status: "accepted",
        audit_id: "audit_qa_1",
        run_id: "pilot_qa_QA-1",
        evidence_refs: ["qa_queue:QA-1", "audit:scoring.completed"],
      }),
    ).toEqual({
      claimId: "CLM-1",
      eventType: "qa.result.received",
      eventStatus: "accepted",
      auditId: "audit_qa_1",
      runId: "pilot_qa_QA-1",
      evidenceCount: 2,
      evidenceRefs: ["qa_queue:QA-1", "audit:scoring.completed"],
    });
    expect(buildQaSubmitSummary(null)).toBeNull();
  });

  it("summarizes QA feedback loop readiness for rules models and TPA", () => {
    expect(
      buildQaFeedbackLoopSummary({
        open_count: 3,
        in_progress_count: 1,
        resolved_count: 2,
        dismissed_count: 1,
        unresolved_count: 4,
        rules_feedback_count: 2,
        models_feedback_count: 1,
        features_feedback_count: 1,
        provider_profile_feedback_count: 1,
        workflow_feedback_count: 1,
        tpa_feedback_count: 2,
        high_priority_count: 3,
        evidence_backed_count: 6,
        highest_priority: "high",
      }),
    ).toEqual({
      totalFeedbackCount: 8,
      unresolvedRateLabel: "50.0%",
      evidenceCoverageLabel: "75.0%",
      modelRuleFeedbackCount: 3,
      workflowFeedbackCount: 3,
      tpaWritebackFeedbackCount: 2,
      highestPriority: "high",
    });

    expect(buildQaFeedbackLoopSummary(null)).toEqual({
      totalFeedbackCount: 0,
      unresolvedRateLabel: "0.0%",
      evidenceCoverageLabel: "0.0%",
      modelRuleFeedbackCount: 0,
      workflowFeedbackCount: 0,
      tpaWritebackFeedbackCount: 0,
      highestPriority: "none",
    });
  });

  it("exposes QA review options that match the governed API labels", () => {
    expect(QA_CONCLUSION_OPTIONS.map((option) => option.value)).toEqual([
      "pass",
      "issue_found_return",
      "issue_found_escalate",
    ]);
    expect(QA_ISSUE_TYPE_OPTIONS.map((option) => option.value)).toEqual([
      "none",
      "confirmed_fwa",
      "false_positive",
      "improper_payment",
      "insufficient_evidence",
      "abuse_not_fraud",
      "documentation_issue",
      "medical_necessity_issue",
      "policy_exclusion",
      "qa_review_completed",
      "alert_handling_incomplete",
      "medical_reasonableness",
      "provider_pattern",
      "model_under_scored_confirmed_issue",
      "workflow_missing_evidence",
    ]);
    expect(QA_FEEDBACK_TARGET_OPTIONS.map((option) => option.value)).toEqual([
      "rules",
      "model",
      "features",
      "provider_profile",
      "workflow",
      "tpa",
    ]);
  });

  it("summarizes every governed QA feedback target in the queue panel", () => {
    expect(QA_SUMMARY_FEEDBACK_ROWS.map((row) => row.field)).toEqual([
      "rules_feedback_count",
      "models_feedback_count",
      "features_feedback_count",
      "provider_profile_feedback_count",
      "workflow_feedback_count",
      "tpa_feedback_count",
    ]);
  });
});
