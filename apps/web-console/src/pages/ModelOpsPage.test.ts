import { describe, expect, it } from "vitest";
import {
  buildModelAuditFilters,
  buildModelLabelReadinessSummary,
  buildModelRetrainingJobSummary,
  buildModelRetrainingSummary,
  formatSourceDataQuality,
} from "./ModelOpsPage";

describe("buildModelAuditFilters", () => {
  it("targets the selected model version for lifecycle audit history", () => {
    expect(
      buildModelAuditFilters({
        model_key: "baseline_fwa",
        version: "0.2.0-candidate",
        model_type: "baseline_classifier",
        runtime_kind: "python_http",
        execution_provider: "cpu",
        status: "approved",
        review_mode: "pre_payment",
        artifact_uri: "s3://models/baseline_fwa/model.onnx",
        endpoint_url: null,
      }),
    ).toEqual({
      limit: 25,
      model_key: "baseline_fwa",
      model_version: "0.2.0-candidate",
    });
  });
});

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

describe("buildModelRetrainingJobSummary", () => {
  it("summarizes retraining job queue state", () => {
    const summary = buildModelRetrainingJobSummary([
      {
        job_id: "job_2",
        model_key: "baseline_fwa",
        model_version: "0.1.0",
        status: "running",
        requested_by: "model-ops",
        request_notes: "drift",
        status_note: "started",
        updated_by: "trainer-worker",
        readiness_recommendation: "prepare_retraining",
        trigger_summary: ["score drift status: drift"],
        blocker_summary: [],
        candidate_model_version: null,
        candidate_artifact_uri: null,
        candidate_endpoint_url: null,
        validation_report_uri: null,
        output_evaluation_id: null,
        created_at: null,
        updated_at: null,
      },
      {
        job_id: "job_1",
        model_key: "baseline_fwa",
        model_version: "0.1.0",
        status: "queued",
        requested_by: "model-ops",
        request_notes: "drift",
        status_note: "queued",
        updated_by: "model-ops",
        readiness_recommendation: "prepare_retraining",
        trigger_summary: ["approved model labels available"],
        blocker_summary: [],
        candidate_model_version: "0.2.0-candidate",
        candidate_artifact_uri: "s3://models/model.onnx",
        candidate_endpoint_url: null,
        validation_report_uri: "s3://models/validation.json",
        output_evaluation_id: "eval_candidate",
        created_at: null,
        updated_at: null,
      },
    ]);

    expect(summary).toEqual({
      jobCount: 2,
      queuedCount: 1,
      runningCount: 1,
      completedCount: 0,
      artifactReadyCount: 1,
      validationReportCount: 1,
      evaluationCount: 1,
      latestStatus: "running",
      latestArtifactStatus: "available",
    });
  });
});
