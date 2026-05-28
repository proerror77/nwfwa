import { describe, expect, it } from "vitest";
import { buildModelLabelReadinessSummary } from "./ModelOpsPage";

describe("buildModelLabelReadinessSummary", () => {
  it("summarizes model-governance labels from human outcomes", () => {
    const summary = buildModelLabelReadinessSummary([
      {
        label_id: "label_confirmed",
        claim_id: "CLM-1",
        label_name: "confirmed_fwa",
        label_value: "true",
        source_type: "investigation_result",
        source_id: "INV-1",
        governance_status: "approved_for_training",
        feedback_target: "models",
        currency: null,
        evidence_refs: ["investigation_results:INV-1"],
      },
      {
        label_id: "label_issue",
        claim_id: "CLM-2",
        label_name: "medical_necessity_issue",
        label_value: "true",
        source_type: "qa_review",
        source_id: "QA-1",
        governance_status: "needs_review",
        feedback_target: "models",
        currency: null,
        evidence_refs: ["qa_reviews:QA-1"],
      },
      {
        label_id: "label_rule",
        claim_id: "CLM-3",
        label_name: "false_positive",
        label_value: "true",
        source_type: "investigation_result",
        source_id: "INV-2",
        governance_status: "needs_review",
        feedback_target: "rules",
        currency: null,
        evidence_refs: ["investigation_results:INV-2"],
      },
    ]);

    expect(summary).toEqual({
      modelLabelCount: 2,
      approvedForTrainingCount: 1,
      needsReviewCount: 1,
      evidenceBackedCount: 2,
      confirmedFwaCount: 1,
    });
  });
});
