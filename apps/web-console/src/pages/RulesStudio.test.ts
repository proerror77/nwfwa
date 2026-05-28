import { describe, expect, it } from "vitest";
import { buildRuleAuditFilters, buildRuleLabelReadinessSummary } from "./RulesStudio";

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
