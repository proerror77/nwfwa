import { describe, expect, it } from "vitest";
import { buildAuditSamplingSummary } from "./AuditSamplingPage";

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
          outcome_distribution: {},
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
          outcome_distribution: {},
          created_at: "2026-05-27T11:00:00Z",
        },
      ],
    });

    expect(summary).toEqual({
      totalSamples: 2,
      selectedLeadCount: 2,
      requestedSampleSize: 3,
      topSampleMode: "risk_ranked",
      latestAssignmentQueue: "Calibration",
    });
  });
});
