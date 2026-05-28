import { describe, expect, it } from "vitest";
import { buildDatasetHealthSummary, buildDatasetModelLineageRows } from "./DataSourcesPage";

describe("buildDatasetHealthSummary", () => {
  it("formats dataset data quality health for the data sources page", () => {
    const summary = buildDatasetHealthSummary({
      dataset_id: "dataset_1",
      dataset_key: "demo_claims_fwa",
      dataset_version: "v1",
      data_quality_score: 0.75,
      data_quality_status: "watch",
      field_count: 8,
      label_count: 1,
      entity_key_count: 2,
      high_missing_count: 1,
      unstable_field_count: 2,
      unowned_field_count: 3,
      online_ready_count: 4,
      issue_count: 6,
    });

    expect(summary).toEqual({
      dataQualityScoreLabel: "75.0%",
      dataQualityStatus: "watch",
      issueCount: 6,
      highMissingCount: 1,
      unstableFieldCount: 2,
      unownedFieldCount: 3,
      onlineReadyRateLabel: "50.0%",
    });
  });

  it("uses empty labels when no dataset health is available", () => {
    expect(buildDatasetHealthSummary(null)).toEqual({
      dataQualityScoreLabel: "-",
      dataQualityStatus: "empty",
      issueCount: 0,
      highMissingCount: 0,
      unstableFieldCount: 0,
      unownedFieldCount: 0,
      onlineReadyRateLabel: "-",
    });
  });
});

describe("buildDatasetModelLineageRows", () => {
  it("filters model evaluation lineage to the selected source dataset", () => {
    const rows = buildDatasetModelLineageRows(
      {
        dataset_id: "dataset_1",
        source_key: "claims",
        business_domain: "claims_fwa",
        dataset_key: "demo_claims_fwa",
        dataset_version: "v1",
        sample_grain: "claim",
        label_column: "confirmed_fwa",
        storage_format: "parquet",
        row_count: 100,
        status: "registered",
        splits: [],
        fields: [],
      },
      [
        {
          evaluation_run_id: "eval_1",
          model_key: "baseline_fwa",
          model_version: "0.2.0",
          model_dataset_id: "model_dataset_1",
          source_dataset_id: "dataset_1",
          source_dataset_key: "demo_claims_fwa",
          source_dataset_version: "v1",
          source_data_quality_score: 0.875,
          source_data_quality_status: "ready",
        },
        {
          evaluation_run_id: "eval_2",
          model_key: "other_model",
          model_version: "0.1.0",
          model_dataset_id: "model_dataset_2",
          source_dataset_id: "dataset_2",
          source_dataset_key: "other_dataset",
          source_dataset_version: "v1",
          source_data_quality_score: 0.6,
          source_data_quality_status: "blocked",
        },
      ],
      [
        {
          evaluation_run_id: "eval_1",
          model_key: "baseline_fwa",
          model_version: "0.2.0",
          model_dataset_id: "model_dataset_1",
          auc: "0.84",
          ks: "0.45",
          precision: "0.76",
          recall: "0.70",
          f1: "0.73",
          accuracy: "0.78",
          threshold: "0.50",
        },
      ],
    );

    expect(rows).toEqual([
      {
        evaluationRunId: "eval_1",
        modelLabel: "baseline_fwa:0.2.0",
        modelDatasetId: "model_dataset_1",
        sourceDatasetLabel: "demo_claims_fwa:v1",
        dataQualityLabel: "87.5%",
        dataQualityStatus: "ready",
        metricLabel: "AUC 0.84 · F1 0.73",
      },
    ]);
  });
});
