import { describe, expect, it } from "vitest";
import { buildAgentRunLogSummary, buildAuditSummary } from "./GovernancePage";

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

describe("buildAgentRunLogSummary", () => {
  it("summarizes audited agent tool activity", () => {
    const summary = buildAgentRunLogSummary([
      {
        agent_run_id: "agent_1",
        claim_id: "CLM-1",
        status: "succeeded",
        decision_boundary: "assistive_only",
        evidence_refs: ["agent_run:agent_1"],
        steps: [{ step_name: "evidence_finding" }],
        tool_calls: [
          {
            tool_call_id: "tool_call_1",
            tool_name: "knowledge.search_similar",
            status: "succeeded",
            input_json: { diagnosis_code: "J10" },
            evidence_refs: ["knowledge_query:CLM-1"],
          },
        ],
        tool_results: [
          {
            tool_result_id: "tool_result_1",
            tool_call_id: "tool_call_1",
            tool_name: "knowledge.search_similar",
            status: "succeeded",
            output_json: { result_count: 2 },
            evidence_refs: ["knowledge_cases:KC-1001"],
          },
        ],
      },
    ]);

    expect(summary).toEqual({
      runCount: 1,
      toolCallCount: 1,
      toolResultCount: 1,
      failedToolCallCount: 0,
    });
  });
});
