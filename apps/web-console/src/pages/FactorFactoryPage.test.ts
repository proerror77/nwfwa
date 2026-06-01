import { describe, expect, it } from "vitest";
import {
  buildApiFactorCards,
  buildFactorCards,
  buildFactorOwnerOptions,
  buildFactorReadinessSummary,
  buildFactorRuleCandidate,
  buildSavedFactorCandidateSummary,
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
      entity_type: "claim",
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
      evidence_refs: ["dataset_fields:demo_claims_fwa:claim_amount_to_limit_ratio"],
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
      evidence_refs: ["dataset_fields:demo_claims_fwa:confirmed_fwa"],
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
          scheme_family: "early_high_value_claim",
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
      scheme_family: "early_high_value_claim",
      display_label: "理赔金额占保额比例",
      entity_type: "claim",
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
      evidence_refs: ["dataset_fields:demo_claims_fwa:v1:claim_amount_to_limit_ratio"],
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
          scheme_family: "provider_peer_outlier",
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
      evidence_refs: ["dataset_fields:demo_claims_fwa:v1:provider_high_cost_ratio_30d"],
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
      scheme_readiness: [
        {
          scheme_family: "provider_peer_outlier",
          factor_count: 4,
          ready_factor_count: 1,
          review_factor_count: 3,
          online_ready_count: 2,
          rule_convertible_count: 2,
          readiness_issue_counts: {
            missing_owner: 3,
            unstable_distribution: 1,
          },
        },
        {
          scheme_family: "early_high_value_claim",
          factor_count: 8,
          ready_factor_count: 4,
          review_factor_count: 4,
          online_ready_count: 5,
          rule_convertible_count: 3,
          readiness_issue_counts: {
            high_missing_rate: 1,
          },
        },
      ],
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
      schemeReadiness: [
        {
          schemeFamily: "early_high_value_claim",
          factorCount: 8,
          readyFactorCount: 4,
          reviewFactorCount: 4,
          onlineReadyCount: 5,
          ruleConvertibleCount: 3,
          onlineReadyRateLabel: "62.5%",
          topReadinessIssues: ["high_missing_rate: 1"],
        },
        {
          schemeFamily: "provider_peer_outlier",
          factorCount: 4,
          readyFactorCount: 1,
          reviewFactorCount: 3,
          onlineReadyCount: 2,
          ruleConvertibleCount: 2,
          onlineReadyRateLabel: "50.0%",
          topReadinessIssues: ["missing_owner: 3", "unstable_distribution: 1"],
        },
      ],
    });
  });
});

describe("filterFactorCards", () => {
  function factorCard(overrides: Partial<FactorCard>): FactorCard {
    return {
      factor_name: "claim_amount_to_limit_ratio",
      scheme_family: "early_high_value_claim",
      display_label: "Claim amount ratio",
      entity_type: "claim",
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
      evidence_refs: ["dataset_fields:demo_claims_fwa:v1:claim_amount_to_limit_ratio"],
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

  it("filters factor cards by readiness owner and scheme family", () => {
    const cards = [
      factorCard({
        factor_name: "ready_feature",
        scheme_family: "early_high_value_claim",
        owner: "feature-ops",
        online_status: "ready",
      }),
      factorCard({
        factor_name: "review_feature",
        scheme_family: "provider_peer_outlier",
        owner: "feature-ops",
        online_status: "review",
      }),
      factorCard({
        factor_name: "model_feature",
        scheme_family: "provider_peer_outlier",
        owner: "model-ops",
        online_status: "ready",
      }),
    ];

    expect(
      filterFactorCards(cards, "ready", "feature-ops", "early_high_value_claim").map(
        (card) => card.factor_name,
      ),
    ).toEqual(["ready_feature"]);
    expect(filterFactorCards(cards, "all", "model-ops", "all").map((card) => card.factor_name)).toEqual([
      "model_feature",
    ]);
    expect(
      filterFactorCards(cards, "all", "all", "provider_peer_outlier").map((card) => card.factor_name),
    ).toEqual(["review_feature", "model_feature"]);
    expect(
      filterFactorCards(cards, "review", "all", "provider_peer_outlier").map(
        (card) => card.factor_name,
      ),
    ).toEqual(["review_feature"]);
  });
});

describe("buildFactorRuleCandidate", () => {
  function factorCard(overrides: Partial<FactorCard>): FactorCard {
    return {
      factor_name: "claim_amount_percentile_peer",
      scheme_family: "provider_peer_outlier",
      display_label: "Claim Amount Percentile Peer",
      entity_type: "claim",
      semantic_role: "feature",
      logical_type: "decimal",
      description: "Claim amount percentile within peers.",
      business_meaning: "Peer amount outlier.",
      risk_direction: "higher_is_riskier",
      calculation_window: "claim",
      calculation_logic: "peer percentile",
      source_table: "claim_features",
      source_fields: ["claim_amount_percentile_peer"],
      source_lineage_label: "claim_features.claim_amount_percentile_peer",
      owner: "feature-ops",
      version: "v1",
      missing_rate_label: "0.0%",
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
      evidence_refs: ["dataset_fields:demo_claims_fwa:v1:claim_amount_percentile_peer"],
      top_values: [],
      ...overrides,
    };
  }

  it("builds a saveable candidate rule from a rule-convertible factor", () => {
    const candidate = buildFactorRuleCandidate(factorCard({}));

    expect(candidate).toMatchObject({
      owner: "feature-ops",
      rule: {
        rule_id: "candidate_factor_claim_amount_percentile_peer",
        version: 1,
        name: "Claim Amount Percentile Peer candidate",
        review_mode: "both",
        scheme_family: "provider_peer_outlier",
        conditions: [
          {
            field: "claim_amount_percentile_peer",
            operator: ">=",
            value: 98,
          },
        ],
        action: {
          score: 20,
          alert_code: "FACTOR_CLAIM_AMOUNT_PERCENTILE_PEER",
          recommended_action: "ManualReview",
        },
      },
    });
  });

  it("does not create rule candidates from labels or non-convertible factors", () => {
    expect(buildFactorRuleCandidate(factorCard({ is_label: true }))).toBeNull();
    expect(buildFactorRuleCandidate(factorCard({ convertible_to_rule: false }))).toBeNull();
  });

  it("uses lower thresholds for lower-is-riskier score factors", () => {
    const candidate = buildFactorRuleCandidate(
      factorCard({
        factor_name: "diagnosis_procedure_match_score",
        display_label: "Diagnosis Procedure Match Score",
        risk_direction: "lower_is_riskier",
        owner: "unassigned",
      }),
    );

    expect(candidate?.owner).toBe("factor-factory");
    expect(candidate?.rule.conditions[0]).toEqual({
      field: "diagnosis_procedure_match_score",
      operator: "<=",
      value: 0.2,
    });
  });
});

describe("buildSavedFactorCandidateSummary", () => {
  it("summarizes a saved factor-derived rule candidate", () => {
    expect(
      buildSavedFactorCandidateSummary({
        summary: {
          rule_id: "candidate_factor_claim_amount_percentile_peer",
          name: "Claim Amount Percentile Peer candidate",
          status: "draft",
          owner: "feature-ops",
          active_version: null,
          latest_version: 1,
          review_mode: "both",
          scheme_family: "provider_peer_outlier",
          score: 20,
          alert_code: "FACTOR_CLAIM_AMOUNT_PERCENTILE_PEER",
          recommended_action: "ManualReview",
        },
        versions: [
          {
            version: 1,
            status: "draft",
            dsl: {
              conditions: [
                {
                  field: "claim_amount_percentile_peer",
                  operator: ">=",
                  value: 98,
                },
              ],
            },
          },
        ],
        audit_events: [{ audit_id: "audit_factor_candidate_saved" }],
      }),
    ).toEqual({
      ruleId: "candidate_factor_claim_amount_percentile_peer",
      name: "Claim Amount Percentile Peer candidate",
      status: "draft",
      owner: "feature-ops",
      versionLabel: "v1",
      reviewMode: "both",
      schemeFamily: "provider_peer_outlier",
      score: 20,
      alertCode: "FACTOR_CLAIM_AMOUNT_PERCENTILE_PEER",
      recommendedAction: "ManualReview",
      conditionLabel: "claim_amount_percentile_peer >= 98",
      versionCount: 1,
      auditEventCount: 1,
    });
    expect(buildSavedFactorCandidateSummary(null)).toBeNull();
  });
});
