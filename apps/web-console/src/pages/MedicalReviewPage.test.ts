import { describe, expect, it } from "vitest";
import {
  buildMedicalReviewDecisionSummary,
  buildMedicalReviewEvidenceRefs,
  buildMedicalReviewQueueSummary,
  buildMedicalReviewSubmitSummary,
} from "./MedicalReviewPage";

describe("buildMedicalReviewQueueSummary", () => {
  it("summarizes medical review queue pressure", () => {
    expect(
      buildMedicalReviewQueueSummary([
        {
          claim_id: "CLM-1",
          run_id: "run_1",
          audit_id: "audit_1",
          medical_reasonableness_score: 100,
          review_route: "medical_review",
          evidence_status: "missing_required_evidence",
          missing_evidence: ["medical_record"],
          item_finding_count: 1,
          first_item_code: "IMG-900",
          first_issue_type: "medical_necessity_review_required",
          evidence_refs: ["claim_items:IMG-900"],
          created_at: null,
          review_status: "open",
          review_audit_id: null,
          review_decision: null,
          reviewer: null,
          reviewed_at: null,
        },
        {
          claim_id: "CLM-2",
          run_id: "run_2",
          audit_id: "audit_2",
          medical_reasonableness_score: 65,
          review_route: "medical_review",
          evidence_status: "missing_required_evidence",
          missing_evidence: [],
          item_finding_count: 1,
          first_item_code: null,
          first_issue_type: null,
          evidence_refs: [],
          created_at: null,
          review_status: "pending_evidence",
          review_audit_id: "audit_review_1",
          review_decision: "request_more_evidence",
          reviewer: "medical-reviewer-1",
          reviewed_at: null,
        },
      ]),
    ).toEqual({
      queueCount: 2,
      highScoreCount: 1,
      missingEvidenceCount: 1,
      evidenceBackedCount: 1,
      pendingEvidenceCount: 1,
      completedCount: 0,
      topClaimId: "CLM-1",
    });
  });

  it("builds medical review evidence refs from scoring audit and clinical refs", () => {
    expect(
      buildMedicalReviewEvidenceRefs({
        claim_id: "CLM-1",
        run_id: "run_1",
        audit_id: "audit_1",
        medical_reasonableness_score: 100,
        review_route: "medical_review",
        evidence_status: "missing_required_evidence",
        missing_evidence: ["medical_record"],
        item_finding_count: 1,
        first_item_code: "IMG-900",
        first_issue_type: "medical_necessity_review_required",
        evidence_refs: ["claim_items:IMG-900", "claim_items:IMG-900"],
        created_at: null,
        review_status: "open",
        review_audit_id: null,
        review_decision: null,
        reviewer: null,
        reviewed_at: null,
      }),
    ).toBe("audit:audit_1\nclaim_items:IMG-900");
  });
});

describe("buildMedicalReviewDecisionSummary", () => {
  it("summarizes structured medical review decision fields", () => {
    expect(
      buildMedicalReviewDecisionSummary({
        claim_id: "CLM-2",
        run_id: "run_2",
        audit_id: "audit_2",
        medical_reasonableness_score: 65,
        review_route: "medical_review",
        evidence_status: "missing_required_evidence",
        missing_evidence: [],
        item_finding_count: 1,
        first_item_code: null,
        first_issue_type: null,
        evidence_refs: [],
        created_at: null,
        review_status: "completed_issue_found",
        review_audit_id: "audit_review_1",
        review_decision: "medical_necessity_issue",
        reviewer: "medical-reviewer-1",
        reviewed_at: "2026-05-29T12:00:00Z",
      }),
    ).toEqual({
      decision: "medical_necessity_issue",
      reviewer: "medical-reviewer-1",
      reviewedAt: "2026-05-29T12:00:00Z",
    });
  });

  it("uses pending labels before medical review is recorded", () => {
    expect(buildMedicalReviewDecisionSummary(null)).toEqual({
      decision: "pending",
      reviewer: "unassigned",
      reviewedAt: "not reviewed",
    });
  });
});

describe("buildMedicalReviewSubmitSummary", () => {
  it("summarizes medical review writeback audit metadata", () => {
    expect(
      buildMedicalReviewSubmitSummary({
        claim_id: "CLM-1",
        event_type: "medical.review.recorded",
        event_status: "succeeded",
        audit_id: "audit_medical_review_1",
        run_id: "run_medical_review_1",
        review_status: "pending_evidence",
        evidence_refs: ["audit:scoring_1", "claim_items:IMG-900"],
      }),
    ).toEqual({
      claimId: "CLM-1",
      eventType: "medical.review.recorded",
      eventStatus: "succeeded",
      auditId: "audit_medical_review_1",
      runId: "run_medical_review_1",
      reviewStatus: "pending_evidence",
      evidenceCount: 2,
      evidenceRefs: ["audit:scoring_1", "claim_items:IMG-900"],
    });
    expect(buildMedicalReviewSubmitSummary(null)).toBeNull();
  });
});
