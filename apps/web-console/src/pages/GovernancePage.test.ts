import { describe, expect, it } from "vitest";
import { buildAuditSummary } from "./GovernancePage";

describe("buildAuditSummary", () => {
  it("summarizes claim audit events for governance review", () => {
    const summary = buildAuditSummary({
      claim_id: "CLM-0287",
      events: [
        {
          audit_id: "audit_1",
          run_id: "run_1",
          event_type: "scoring.completed",
          event_status: "succeeded",
          summary: "Scoring completed",
          evidence_refs: ["rule_runs:EARLY_CLAIM"],
          created_at: "2026-05-27T10:00:00Z",
        },
        {
          audit_id: "audit_2",
          run_id: "run_2",
          event_type: "qa.result.received",
          event_status: "failed",
          summary: "QA result failed",
          evidence_refs: [],
          created_at: "2026-05-27T11:00:00Z",
        },
      ],
    });

    expect(summary).toEqual({
      totalEvents: 2,
      succeededEvents: 1,
      failedEvents: 1,
      latestEventType: "qa.result.received",
    });
  });
});
