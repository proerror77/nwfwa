import { describe, expect, it } from "vitest";
import {
  buildApiFactorCards,
  buildFactorCards,
  buildFactorOwnerOptions,
  buildFactorReadinessSummary,
  filterFactorCards,
} from "./FactorFactoryPage";
import type { FactorCard } from "./FactorFactoryPage";

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
      source_lineage_label: "claims.claim_amount,coverage_limit_amount",
      owner: "feature-ops",
      version: "v2",
      iv_label: "0.210",
      auc_gain_label: "0.030",
      lift_label: "2.40x",
      stability_label: "stable",
      model_contribution_label: "18.0%",
      convertible_to_rule: true,
      online_available: true,
      readiness_issues: [],
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
      readiness_issues: [],
      is_label: true,
    });
  });
});

describe("buildApiFactorCards", () => {
  it("uses backend factor card governance fields when readiness provides them", () => {
    const cards = buildApiFactorCards(
      [
        {
          dataset_id: "dataset_1",
          dataset_key: "demo_claims_fwa",
          dataset_version: "v1",
          factor_name: "claim_amount_to_limit_ratio",
          chinese_name: "理赔金额占保额比例",
          entity_type: "claim",
          semantic_role: "feature",
          logical_type: "decimal",
          calculation_window: "claim",
          calculation_logic: "claim_amount / policy_limit",
          source_table: "claims",
          source_fields: ["claim_amount", "coverage_limit_amount"],
          business_meaning: "理赔金额占保障额度比例",
          risk_direction: "higher_is_riskier",
          missing_rate: 0.02,
          iv: 0.21,
          auc_gain: 0.03,
          lift: 2.4,
          psi: 0.04,
          stability: "stable",
          model_contribution: 0.18,
          rule_convertible: true,
          online_available: true,
          readiness_status: "ready",
          readiness_issues: [],
          version: "v2",
          owner: "feature-ops",
          is_label: false,
          is_entity_key: false,
          evidence_refs: ["dataset_fields:demo_claims_fwa:v1:claim_amount_to_limit_ratio"],
        },
      ],
      "dataset_1",
    );

    expect(cards[0]).toMatchObject({
      factor_name: "claim_amount_to_limit_ratio",
      display_label: "理赔金额占保额比例",
      online_status: "ready",
      source_lineage_label: "claims.claim_amount,coverage_limit_amount",
      owner: "feature-ops",
      version: "v2",
      missing_rate_label: "2.0%",
      iv_label: "0.210",
      auc_gain_label: "0.030",
      lift_label: "2.40x",
      stability_label: "stable",
      model_contribution_label: "18.0%",
      convertible_to_rule: true,
      online_available: true,
      readiness_issues: [],
    });
  });

  it("keeps API factor readiness issues visible for operations review", () => {
    const cards = buildApiFactorCards(
      [
        {
          dataset_id: "dataset_1",
          dataset_key: "demo_claims_fwa",
          dataset_version: "v1",
          factor_name: "provider_high_cost_ratio_30d",
          chinese_name: "Provider 30 日高价项目比例",
          entity_type: "provider",
          semantic_role: "feature",
          logical_type: "decimal",
          calculation_window: "30d",
          calculation_logic: "high_cost_items / total_items",
          source_table: "provider_daily_profiles",
          source_fields: ["high_cost_items", "total_items"],
          business_meaning: "Provider high-cost billing concentration.",
          risk_direction: "higher_is_riskier",
          missing_rate: 0.12,
          iv: null,
          auc_gain: null,
          lift: null,
          psi: 0.31,
          stability: "drift",
          model_contribution: null,
          rule_convertible: true,
          online_available: true,
          readiness_status: "needs_review",
          readiness_issues: ["online_missing_rate_above_threshold", "unstable_distribution"],
          version: "v1",
          owner: "feature-ops",
          is_label: false,
          is_entity_key: false,
          evidence_refs: ["dataset_fields:demo_claims_fwa:v1:provider_high_cost_ratio_30d"],
        },
      ],
      "dataset_1",
    );

    expect(cards[0]).toMatchObject({
      online_status: "review",
      readiness_issues: ["online_missing_rate_above_threshold", "unstable_distribution"],
      stability_label: "drift",
      missing_rate_label: "12.0%",
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
      ready_factor_count: 5,
      review_factor_count: 7,
      readiness_issue_counts: {
        missing_owner: 4,
        unstable_distribution: 2,
        high_missing_rate: 1,
      },
    });

    expect(summary).toEqual({
      datasetCount: 2,
      factorCount: 12,
      onlineReadyCount: 7,
      dataQualityScoreLabel: "75.0%",
      dataQualityStatus: "watch",
      ruleConvertibleCount: 5,
      mappedFactorCount: 6,
      readyFactorCount: 5,
      reviewFactorCount: 7,
      topReadinessIssues: [
        "missing_owner: 4",
        "unstable_distribution: 2",
        "high_missing_rate: 1",
      ],
      reviewQueueCount: 7,
      onlineReadyRateLabel: "58.3%",
    });
  });
});

describe("filterFactorCards", () => {
  function factorCard(overrides: Partial<FactorCard>): FactorCard {
    return {
      factor_name: "claim_amount_to_limit_ratio",
      display_label: "Claim amount ratio",
      semantic_role: "feature",
      logical_type: "decimal",
      description: "Claim amount divided by policy limit.",
      business_meaning: "Claim amount pressure.",
      risk_direction: "higher_is_riskier",
      calculation_window: "claim",
      calculation_logic: "claim_amount / policy_limit",
      source_table: "claims",
      source_fields: ["claim_amount", "policy_limit"],
      source_lineage_label: "claims.claim_amount,policy_limit",
      owner: "feature-ops",
      version: "v1",
      missing_rate_label: "2.0%",
      iv_label: "0.210",
      auc_gain_label: "0.030",
      lift_label: "2.40x",
      stability_label: "stable",
      model_contribution_label: "18.0%",
      online_status: "ready",
      online_available: true,
      convertible_to_rule: true,
      readiness_issues: [],
      is_label: false,
      is_entity_key: false,
      top_values: [],
      ...overrides,
    };
  }

  it("builds sorted owner options from factor cards", () => {
    const cards = [
      factorCard({ owner: "model-ops" }),
      factorCard({ owner: "feature-ops" }),
      factorCard({ owner: "model-ops" }),
    ];

    expect(buildFactorOwnerOptions(cards)).toEqual(["feature-ops", "model-ops"]);
  });

  it("filters factor cards by readiness and owner", () => {
    const cards = [
      factorCard({ factor_name: "ready_feature", owner: "feature-ops", online_status: "ready" }),
      factorCard({
        factor_name: "review_feature",
        owner: "feature-ops",
        online_status: "review",
      }),
      factorCard({ factor_name: "model_feature", owner: "model-ops", online_status: "ready" }),
    ];

    expect(filterFactorCards(cards, "ready", "feature-ops").map((card) => card.factor_name)).toEqual([
      "ready_feature",
    ]);
    expect(filterFactorCards(cards, "all", "model-ops").map((card) => card.factor_name)).toEqual([
      "model_feature",
    ]);
    expect(filterFactorCards(cards, "review", "all").map((card) => card.factor_name)).toEqual([
      "review_feature",
    ]);
  });
});
