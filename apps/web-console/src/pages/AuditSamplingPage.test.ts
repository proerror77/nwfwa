import { describe, expect, it } from "vitest";
import {
  buildAuditSampleLeadDetailRows,
  buildAuditSampleRequest,
  buildAuditSamplingSummary,
} from "./AuditSamplingPage";

describe("buildAuditSamplingSummary", () => {
  it("summarizes sample coverage and latest assignment", () => {
    const summary = buildAuditSamplingSummary({
      samples: [
        {
          sample_id: "sample_1",
          sample_mode: "risk_ranked",
          population_definition: "RED claims",
          selection_method: "risk_score_desc",
          sample_size: 2,
          reviewer: "qa-reviewer-1",
          assignment_queue: "QA Review",
          selected_leads: [
            {
              lead_id: "lead_1",
              claim_id: "CLM-1",
              scheme_family: "early_high_value_claim",
              risk_score: 91,
              rag: "RED",
              evidence_refs: ["audit:scoring.completed"],
            },
            {
              lead_id: "lead_2",
              claim_id: "CLM-2",
              scheme_family: "provider_peer_outlier",
              risk_score: 77,
              rag: "RED",
              evidence_refs: [],
            },
          ],
          outcome_distribution: {
            selected_count: 2,
            reviewed_count: 1,
            open_count: 1,
            qa_conclusions: {
              issue_found_escalate: 1,
            },
          },
          created_at: "2026-05-27T10:00:00Z",
        },
        {
          sample_id: "sample_2",
          sample_mode: "random_control",
          population_definition: "Weekly control group",
          selection_method: "deterministic_hash",
          sample_size: 1,
          reviewer: "qa-reviewer-2",
          assignment_queue: "Calibration",
          selected_leads: [],
          outcome_distribution: {
            selected_count: 0,
            reviewed_count: 0,
            open_count: 0,
            qa_conclusions: {},
            baseline_measurement: {
              control_cohort: true,
              measurement_goal: "false_positive_and_missed_risk_baseline",
              missed_risk_review_targets: 3,
              false_positive_review_targets: 2,
            },
          },
          created_at: "2026-05-27T11:00:00Z",
        },
      ],
    });

    expect(summary).toEqual({
      totalSamples: 2,
      selectedLeadCount: 2,
      reviewedCaseCount: 1,
      openCaseCount: 1,
      requestedSampleSize: 3,
      topSampleMode: "risk_ranked",
      topQaConclusion: "issue_found_escalate",
      latestAssignmentQueue: "Calibration",
      controlCohortCount: 1,
      missedRiskReviewTargets: 3,
      falsePositiveReviewTargets: 2,
    });
  });
});

describe("buildAuditSampleRequest", () => {
  it("includes operational strata criteria when provided", () => {
    expect(
      buildAuditSampleRequest({
        sampleMode: "stratified",
        populationDefinition: "Clinic dental critical claims",
        minRiskScore: "70",
        reviewMode: "post_payment",
        providerType: "clinic",
        providerRegion: "BJ",
        policyType: "DENTAL",
        riskBand: "critical",
        deterministicSeed: "strata-week-1",
        sampleSize: "5",
        reviewer: "qa-reviewer-1",
        assignmentQueue: "QA Review",
      }),
    ).toEqual({
      sample_mode: "stratified",
      population_definition: "Clinic dental critical claims",
      inclusion_criteria: {
        min_risk_score: 70,
        review_mode: "post_payment",
        provider_type: "clinic",
        provider_region: "BJ",
        policy_type: "DENTAL",
        risk_band: "critical",
      },
      deterministic_seed: "strata-week-1",
      sample_size: 5,
      reviewer: "qa-reviewer-1",
      assignment_queue: "QA Review",
    });
  });
});

describe("buildAuditSampleLeadDetailRows", () => {
  it("shows operational strata fields for selected leads", () => {
    expect(
      buildAuditSampleLeadDetailRows({
        lead_id: "lead_1",
        claim_id: "CLM-1",
        scheme_family: "provider_peer_outlier",
        review_mode: "post_payment",
        provider_id: "PRV-1",
        provider_type: "clinic",
        provider_region: "BJ",
        policy_type: "DENTAL",
        risk_band: "critical",
        strata_key:
          "scheme=provider_peer_outlier|provider_type=clinic|region=BJ|policy_type=DENTAL|risk_band=critical",
        prior_reviewer_sample_count: 2,
        risk_score: 94,
        rag: "RED",
        evidence_refs: ["audit:scoring.completed"],
      }),
    ).toEqual([
      ["Review Mode", "post_payment"],
      ["Provider", "clinic / BJ"],
      ["Policy Type", "DENTAL"],
      ["Risk Band", "critical"],
      ["Prior Reviewer Samples", "2"],
      [
        "Strata",
        "scheme=provider_peer_outlier|provider_type=clinic|region=BJ|policy_type=DENTAL|risk_band=critical",
      ],
    ]);
  });
});
