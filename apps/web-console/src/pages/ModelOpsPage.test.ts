import { describe, expect, it } from "vitest";
import {
  buildModelLabelReadinessSummary,
  buildModelRetrainingSummary,
  formatSourceDataQuality,
} from "./ModelOpsPage";

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

describe("formatSourceDataQuality", () => {
  it("formats nullable model source data quality scores", () => {
    expect(formatSourceDataQuality(0.875)).toBe("87.5%");
    expect(formatSourceDataQuality(null)).toBe("-");
  });
});

describe("buildModelRetrainingSummary", () => {
  it("summarizes retraining readiness for model operators", () => {
    const summary = buildModelRetrainingSummary({
      recommendation: "prepare_retraining",
      latest_evaluation_id: "eval_1",
      drift_status: "drift",
      source_dataset_id: "dataset_1",
      source_data_quality_score: 0.91,
      source_data_quality_status: "ready",
      open_model_feedback_count: 2,
      approved_label_count: 5,
      needs_review_label_count: 0,
      retraining_triggers: ["score drift status: drift", "approved model labels available"],
      blockers: [],
    });

    expect(summary).toEqual({
      recommendation: "prepare_retraining",
      triggerCount: 2,
      blockerCount: 0,
      openFeedbackCount: 2,
      approvedLabelCount: 5,
      sourceDataQualityLabel: "91.0%",
      sourceDataQualityStatus: "ready",
    });
  });
});
