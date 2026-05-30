import { describe, expect, it } from "vitest";
import {
  buildDataLineageRegistrationSummary,
  buildDatasetFieldGovernanceSummary,
  buildDatasetHealthSummary,
  buildDatasetMappingSummary,
  buildDatasetModelLineageRows,
} from "./DataSourcesPage";

describe("buildDataLineageRegistrationSummary", () => {
  it("summarizes direct dataset and feature-set registration responses", () => {
    expect(
      buildDataLineageRegistrationSummary("dataset", {
        dataset_id: "dataset_1",
        status: "draft",
      }),
    ).toEqual({
      kind: "dataset",
      id: "dataset_1",
      status: "draft",
      evidenceTarget: "dataset:dataset_1",
    });

    expect(
      buildDataLineageRegistrationSummary("feature_set", {
        feature_set_id: "feature_set_1",
        status: "registered",
      }),
    ).toEqual({
      kind: "feature_set",
      id: "feature_set_1",
      status: "registered",
      evidenceTarget: "feature_set:feature_set_1",
    });
  });

  it("unwraps nested field mapping and model evaluation registration responses", () => {
    expect(
      buildDataLineageRegistrationSummary("field_mapping", {
        mapping: {
          mapping_id: "mapping_1",
          status: "active",
        },
      }),
    ).toEqual({
      kind: "field_mapping",
      id: "mapping_1",
      status: "active",
      evidenceTarget: "field_mapping:mapping_1",
    });

    expect(
      buildDataLineageRegistrationSummary("model_evaluation", {
        evaluation: {
          evaluation_run_id: "eval_1",
        },
      }),
    ).toEqual({
      kind: "model_evaluation",
      id: "eval_1",
      status: "not_available",
      evidenceTarget: "model_evaluation:eval_1",
    });
  });
});

describe("buildDatasetMappingSummary", () => {
  it("summarizes field mapping coverage for the selected dataset", () => {
    expect(
      buildDatasetMappingSummary([
        {
          mapping_id: "mapping_1",
          dataset_id: "dataset_1",
          external_field: "policy_no",
          canonical_target: "feature.policy_no",
          feature_name: "policy_no",
          transform_kind: "direct",
          status: "active",
        },
        {
          mapping_id: "mapping_2",
          dataset_id: "dataset_1",
          external_field: "raw_note",
          canonical_target: "document.raw_note",
          feature_name: null,
          transform_kind: "derived",
          status: "draft",
        },
      ]),
    ).toEqual({
      mappingCount: 2,
      activeMappingCount: 1,
      featureMappingCount: 1,
      transformKindCount: 2,
      activeCoverageLabel: "50.0%",
    });
  });

  it("uses empty mapping defaults when no mappings are registered", () => {
    expect(buildDatasetMappingSummary()).toEqual({
      mappingCount: 0,
      activeMappingCount: 0,
      featureMappingCount: 0,
      transformKindCount: 0,
      activeCoverageLabel: "0.0%",
    });
  });
});

describe("buildDatasetFieldGovernanceSummary", () => {
  it("summarizes semantic roles for the dataset field dictionary", () => {
    expect(
      buildDatasetFieldGovernanceSummary({
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
        fields: [
          { field_name: "claim_id", semantic_role: "key" },
          { field_name: "member_id", semantic_role: "key" },
          { field_name: "claim_amount", semantic_role: "feature" },
          { field_name: "provider_region", semantic_role: "feature" },
          { field_name: "confirmed_fwa", semantic_role: "label" },
          { field_name: "snapshot_date", semantic_role: "partition" },
          { field_name: "raw_note", semantic_role: "ignored" },
          { field_name: "post_decision_code", semantic_role: "leakage_candidate" },
        ],
      }),
    ).toEqual({
      fieldCount: 8,
      keyCount: 2,
      featureCount: 2,
      labelCount: 1,
      partitionCount: 1,
      ignoredCount: 1,
      leakageCandidateCount: 1,
      roleCoverageLabel: "100.0%",
    });
  });

  it("uses empty field governance defaults when no dataset is selected", () => {
    expect(buildDatasetFieldGovernanceSummary(null)).toEqual({
      fieldCount: 0,
      keyCount: 0,
      featureCount: 0,
      labelCount: 0,
      partitionCount: 0,
      ignoredCount: 0,
      leakageCandidateCount: 0,
      roleCoverageLabel: "0.0%",
    });
  });
});

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
