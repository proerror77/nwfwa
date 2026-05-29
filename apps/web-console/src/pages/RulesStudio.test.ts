import { describe, expect, it } from "vitest";
import {
  buildRuleAuditFilters,
  buildRuleCandidateSaveSummary,
  buildRuleDetailSummary,
  buildRuleDiscoverySummary,
  buildRuleLabelReadinessSummary,
} from "./RulesStudio";

describe("buildRuleLabelReadinessSummary", () => {
  it("summarizes rule-governance labels from human outcomes", () => {
    const summary = buildRuleLabelReadinessSummary([
      {
        label_id: "label_rule_issue",
        claim_id: "CLM-1",
        label_name: "documentation_issue",
        label_value: "true",
        source_type: "qa_review",
        source_id: "QA-1",
        governance_status: "needs_review",
        feedback_target: "rules",
        currency: null,
        evidence_refs: ["qa_reviews:QA-1", "rule_runs:EARLY_CLAIM"],
      },
      {
        label_id: "label_rule_confirmed",
        claim_id: "CLM-2",
        label_name: "confirmed_fwa",
        label_value: "true",
        source_type: "investigation_result",
        source_id: "INV-1",
        governance_status: "approved_for_training",
        feedback_target: "rules",
        currency: null,
        evidence_refs: ["investigation_results:INV-1"],
      },
      {
        label_id: "label_model_issue",
        claim_id: "CLM-3",
        label_name: "false_positive",
        label_value: "true",
        source_type: "investigation_result",
        source_id: "INV-2",
        governance_status: "needs_review",
        feedback_target: "models",
        currency: null,
        evidence_refs: ["investigation_results:INV-2"],
      },
    ]);

    expect(summary).toEqual({
      ruleLabelCount: 2,
      approvedForTrainingCount: 1,
      needsReviewCount: 1,
      evidenceBackedCount: 2,
      confirmedFwaCount: 1,
    });
  });
});

describe("buildRuleDiscoverySummary", () => {
  it("summarizes candidate rule discovery metrics for governance review", () => {
    const summary = buildRuleDiscoverySummary({
      sample_count: 12,
      positive_count: 3,
      candidates: [
        {
          rule: {
            rule_id: "candidate_early_high_amount",
            name: "Early high amount candidate",
          },
          support: 4,
          precision: 0.75,
          recall: 1,
          lift: 3,
          estimated_saving: "8200.00",
          false_positive_rate: 0.25,
          matched_claim_ids: ["CLM-1", "CLM-2"],
          explanation: "保单生效早期且理赔金额接近保障额度",
        },
      ],
    });

    expect(summary).toEqual({
      sampleCount: 12,
      positiveCount: 3,
      candidateCount: 1,
      topRuleId: "candidate_early_high_amount",
      topSupport: 4,
      topPrecisionLabel: "75.0%",
      topRecallLabel: "100.0%",
      topLiftLabel: "3.00x",
      topFalsePositiveRateLabel: "25.0%",
      topSaving: "8200.00",
    });
  });
});

describe("buildRuleDetailSummary", () => {
  it("summarizes rule detail versions and audit evidence", () => {
    expect(
      buildRuleDetailSummary({
        summary: {
          rule_id: "rule_early_claim",
          name: "Early claim",
          status: "active",
          owner: "rule-governance",
          active_version: 1,
          latest_version: 2,
          review_mode: "both",
          scheme_family: "early_high_value_claim",
          score: 25,
          alert_code: "EARLY_CLAIM",
          recommended_action: "ManualReview",
        },
        versions: [
          {
            version: 1,
            status: "active",
            dsl: { conditions: [{ field: "days_since_policy_start" }] },
            review_mode: "both",
            scheme_family: "early_high_value_claim",
            score: 25,
            alert_code: "EARLY_CLAIM",
            recommended_action: "ManualReview",
            reason: "保单生效后短期理赔",
          },
          {
            version: 2,
            status: "submitted",
            dsl: {
              conditions: [
                { field: "days_since_policy_start" },
                { field: "claim_amount_percentile_peer" },
              ],
            },
            review_mode: "pre_payment",
            scheme_family: "early_high_value_claim",
            score: 40,
            alert_code: "EARLY_HIGH_CLAIM",
            recommended_action: "ManualReview",
            reason: "保单生效后短期高额理赔",
          },
        ],
        audit_events: [
          {
            audit_id: "audit_rule_submitted",
            run_id: "rule_lifecycle",
            event_type: "rule.submitted",
            event_status: "succeeded",
            summary: "Rule submitted",
            evidence_refs: ["rules:rule_early_claim:v2"],
          },
        ],
      }),
    ).toEqual({
      ruleId: "rule_early_claim",
      name: "Early claim",
      status: "active",
      owner: "rule-governance",
      activeVersionLabel: "v1",
      latestVersionLabel: "v2",
      versionCount: 2,
      auditEventCount: 1,
      latestStatus: "submitted",
      latestReviewMode: "pre_payment",
      latestSchemeFamily: "early_high_value_claim",
      latestScore: 40,
      latestAlertCode: "EARLY_HIGH_CLAIM",
      latestAction: "ManualReview",
      latestReason: "保单生效后短期高额理赔",
      latestConditionCount: 2,
    });
    expect(buildRuleDetailSummary(null)).toBeNull();
  });
});

describe("buildRuleAuditFilters", () => {
  it("targets the selected rule version for lifecycle audit history", () => {
    expect(
      buildRuleAuditFilters({
        rule_id: "candidate_early_claim",
        name: "Candidate early claim",
        status: "submitted",
        owner: "rule-discovery",
        active_version: null,
        latest_version: 2,
        review_mode: "pre_payment",
        scheme_family: "rule",
        score: 25,
        alert_code: "EARLY_CLAIM",
        recommended_action: "ManualReview",
      }),
    ).toEqual({
      limit: 25,
      rule_id: "candidate_early_claim",
      rule_version: 2,
    });
  });
});

describe("buildRuleCandidateSaveSummary", () => {
  it("summarizes saved rule candidate metadata for governance review", () => {
    expect(
      buildRuleCandidateSaveSummary({
        summary: {
          rule_id: "candidate_early_claim",
          name: "Candidate early claim",
          status: "draft",
          owner: "rule-discovery",
          active_version: null,
          latest_version: 1,
          review_mode: "pre_payment",
          scheme_family: "early_high_value_claim",
          score: 25,
          alert_code: "EARLY_CLAIM",
          recommended_action: "ManualReview",
        },
        versions: [
          {
            version: 1,
            status: "draft",
            review_mode: "pre_payment",
            scheme_family: "early_high_value_claim",
          },
        ],
        audit_events: [
          {
            audit_id: "audit_rule_candidate_saved",
            run_id: "rule_candidate_saved",
            event_type: "rule.candidate.saved",
            event_status: "succeeded",
            summary: "Rule candidate saved",
            evidence_refs: ["rules:candidate_early_claim:v1"],
          },
        ],
      }),
    ).toEqual({
      ruleId: "candidate_early_claim",
      name: "Candidate early claim",
      status: "draft",
      owner: "rule-discovery",
      versionLabel: "v1",
      reviewMode: "pre_payment",
      schemeFamily: "early_high_value_claim",
      score: 25,
      alertCode: "EARLY_CLAIM",
      recommendedAction: "ManualReview",
      versionCount: 1,
      auditEventCount: 1,
    });
    expect(buildRuleCandidateSaveSummary(null)).toBeNull();
  });
});
