import { describe, expect, it } from "vitest";
import { buildMedicalReviewQueueSummary } from "./MedicalReviewPage";

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
        },
      ]),
    ).toEqual({
      queueCount: 2,
      highScoreCount: 1,
      missingEvidenceCount: 1,
      evidenceBackedCount: 1,
      topClaimId: "CLM-1",
    });
  });
});
