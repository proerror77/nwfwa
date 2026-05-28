import { describe, expect, it } from "vitest";
import { buildFactorCards, buildFactorReadinessSummary } from "./FactorFactoryPage";

describe("buildFactorCards", () => {
  it("derives factor cards from profiled dataset fields", () => {
    const cards = buildFactorCards({
      dataset_id: "dataset_1",
      dataset_key: "demo_claims_fwa",
      sample_grain: "claim",
      entity_keys: ["claim_id", "provider_id"],
      fields: [
        {
          field_name: "claim_amount_to_limit_ratio",
          logical_type: "decimal",
          nullable: false,
          semantic_role: "feature",
          description: "Claim amount divided by policy limit.",
          profile_json: {
            missing_rate: 0.03,
            business_meaning: "理赔金额占保障额度比例",
            risk_direction: "higher_is_riskier",
            calculation_window: "claim",
            calculation_logic: "claim_amount / policy_limit",
            source_table: "claims",
            source_fields: ["claim_amount", "coverage_limit_amount"],
            owner: "feature-ops",
            version: 2,
            iv: 0.21,
            auc_gain: 0.03,
            lift: 2.4,
            psi: 0.04,
            model_contribution: 0.18,
            convertible_to_rule: true,
            online_available: true,
            top_values: [{ value: "0.8", count: 12 }],
          },
        },
        {
          field_name: "confirmed_fwa",
          logical_type: "boolean",
          nullable: false,
          semantic_role: "label",
          description: "Confirmed FWA label.",
          profile_json: { missing_rate: 0 },
        },
      ],
    });

    expect(cards[0]).toMatchObject({
      factor_name: "claim_amount_to_limit_ratio",
      semantic_role: "feature",
      display_label: "Claim Amount To Limit Ratio",
      missing_rate_label: "3.0%",
      online_status: "ready",
      business_meaning: "理赔金额占保障额度比例",
      risk_direction: "higher_is_riskier",
      calculation_window: "claim",
      calculation_logic: "claim_amount / policy_limit",
      source_table: "claims",
      source_fields: ["claim_amount", "coverage_limit_amount"],
      owner: "feature-ops",
      version: "v2",
      iv_label: "0.210",
      auc_gain_label: "0.030",
      lift_label: "2.40x",
      stability_label: "stable",
      model_contribution_label: "18.0%",
      convertible_to_rule: true,
      online_available: true,
      is_label: false,
      is_entity_key: false,
      top_values: ["0.8 (12)"],
    });
    expect(cards[1]).toMatchObject({
      factor_name: "confirmed_fwa",
      semantic_role: "label",
      online_status: "review",
      business_meaning: "Confirmed FWA label.",
      risk_direction: "label",
      stability_label: "unmeasured",
      convertible_to_rule: false,
      online_available: false,
      is_label: true,
    });
  });
});

describe("buildFactorReadinessSummary", () => {
  it("summarizes factor factory readiness metrics", () => {
    const summary = buildFactorReadinessSummary({
      dataset_count: 2,
      factor_count: 12,
      label_count: 2,
      entity_key_count: 3,
      data_quality_score: 0.75,
      data_quality_status: "watch",
      online_ready_count: 7,
      rule_convertible_count: 5,
      mapped_factor_count: 6,
      high_missing_count: 1,
      unstable_factor_count: 2,
      unowned_factor_count: 4,
    });

    expect(summary).toEqual({
      datasetCount: 2,
      factorCount: 12,
      onlineReadyCount: 7,
      dataQualityScoreLabel: "75.0%",
      dataQualityStatus: "watch",
      ruleConvertibleCount: 5,
      mappedFactorCount: 6,
      reviewQueueCount: 7,
      onlineReadyRateLabel: "58.3%",
    });
  });
});
