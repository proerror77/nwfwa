import { describe, expect, it } from "vitest";
import { buildModelPromotionGateSummary } from "./modelPromotionGates";

describe("buildModelPromotionGateSummary", () => {
  it("allows routing only when evaluation, threshold, explanation, shadow, leakage, and approval gates pass", () => {
    const summary = buildModelPromotionGateSummary(
      {
        model_key: "baseline_fwa",
        version: "0.1.0",
        status: "active",
      },
      {
        data_status: "live",
        scored_runs: 80,
      },
      [
        {
          evaluation_run_id: "eval-baseline-fwa",
          model_key: "baseline_fwa",
          model_version: "0.1.0",
          model_dataset_id: "dataset-v1",
          auc: "0.812",
          precision: "0.72",
          recall: "0.66",
          threshold: "0.70",
          feature_importance_uri: "s3://fwa-demo/feature_importance.json",
          metrics_json: {
            leakage_check_status: "passed",
            shadow_comparison_status: "passed",
            approval_status: "approved",
            review_capacity_threshold_status: "passed",
            out_of_time_auc: 0.79,
          },
        },
      ],
    );

    expect(summary.decision).toBe("routing_allowed");
    expect(summary.passedCount).toBe(9);
    expect(summary.blockers).toEqual([]);
  });

  it("blocks routing when shadow or approval evidence is missing", () => {
    const summary = buildModelPromotionGateSummary(
      {
        model_key: "baseline_fwa",
        version: "0.1.0",
        status: "candidate",
      },
      {
        data_status: "not_scored",
        scored_runs: 0,
      },
      [
        {
          evaluation_run_id: "eval-baseline-fwa",
          model_key: "baseline_fwa",
          model_version: "0.1.0",
          model_dataset_id: "dataset-v1",
          auc: "0.812",
          precision: "0.72",
          recall: "0.66",
          threshold: "0.70",
          feature_importance_uri: "s3://fwa-demo/feature_importance.json",
          metrics_json: {},
        },
      ],
    );

    expect(summary.decision).toBe("routing_blocked");
    expect(summary.blockers).toContain("shadow comparison missing");
    expect(summary.blockers).toContain("approval missing");
    expect(summary.blockers).toContain("model is not active");
  });
});
