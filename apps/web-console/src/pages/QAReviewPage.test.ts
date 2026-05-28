import { describe, expect, it } from "vitest";
import {
  buildQaEvidenceRefs,
  canSubmitQaQueueItem,
  QA_CONCLUSION_OPTIONS,
  QA_FEEDBACK_TARGET_OPTIONS,
  QA_ISSUE_TYPE_OPTIONS,
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
      }).split("\n"),
    ).toEqual([
      "qa_queue:qa_sample_1_lead_1",
      "audit_sample:sample_1",
      "lead:lead_1",
      "audit:scoring.completed",
    ]);
    expect(buildQaEvidenceRefs(null)).toBe("");
  });

  it("exposes QA review options that match the governed API labels", () => {
    expect(QA_CONCLUSION_OPTIONS.map((option) => option.value)).toEqual([
      "pass",
      "issue_found_return",
      "issue_found_escalate",
    ]);
    expect(QA_ISSUE_TYPE_OPTIONS.map((option) => option.value)).toEqual([
      "none",
      "qa_review_completed",
      "alert_handling_incomplete",
      "medical_reasonableness",
      "medical_necessity_issue",
      "provider_pattern",
      "model_under_scored_confirmed_issue",
      "workflow_missing_evidence",
    ]);
    expect(QA_FEEDBACK_TARGET_OPTIONS.map((option) => option.value)).toEqual([
      "rules",
      "models",
      "features",
      "provider_profile",
      "workflow",
      "tpa",
    ]);
  });
});
