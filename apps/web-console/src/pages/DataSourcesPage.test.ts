import { describe, expect, it } from "vitest";
import { buildDatasetHealthSummary } from "./DataSourcesPage";

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
