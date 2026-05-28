import { describe, expect, it } from "vitest";
import {
  buildAgentRunLogSummary,
  buildAuditSummary,
  buildOutcomeLabelSummary,
} from "./GovernancePage";

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
        context_snapshots: [
          {
            snapshot_id: "snapshot_1",
            redaction_status: "pii_masked",
            context_json: { claim_id: "CLM-1" },
            source_refs: ["claims:CLM-1"],
            checksum: "snapshot:abc123",
          },
        ],
        tool_calls: [
          {
            tool_call_id: "tool_call_1",
            tool_name: "knowledge.search_similar",
            status: "succeeded",
            input_json: { diagnosis_code: "J10" },
            evidence_refs: ["knowledge_query:CLM-1"],
          },
        ],
        policy_checks: [
          {
            policy_check_id: "policy_check_1",
            agent_run_id: "agent_1",
            tool_call_id: "tool_call_1",
            tool_name: "knowledge.search_similar",
            policy_name: "agent_tool_allowlist",
            decision: "allowed",
            reason: "Tool is allowlisted for read-only evidence retrieval.",
            evidence_refs: ["policy:agent_tool_allowlist"],
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
        approvals: [
          {
            approval_id: "approval_1",
            agent_run_id: "agent_1",
            proposed_action: "manual_review_required",
            decision: "pending",
            approver: "unassigned",
            reason: "Agent output requires human approval before downstream action.",
            evidence_refs: ["agent_run:agent_1"],
          },
        ],
      },
    ]);

    expect(summary).toEqual({
      runCount: 1,
      contextSnapshotCount: 1,
      piiMaskedContextCount: 1,
      toolCallCount: 1,
      toolResultCount: 1,
      failedToolCallCount: 0,
      policyCheckCount: 1,
      deniedPolicyCheckCount: 0,
      approvalCount: 1,
      pendingApprovalCount: 1,
    });
  });
});

describe("buildOutcomeLabelSummary", () => {
  it("summarizes governed labels for training and review queues", () => {
    const summary = buildOutcomeLabelSummary([
      {
        label_id: "label_1",
        claim_id: "CLM-1",
        label_name: "confirmed_fwa",
        label_value: "true",
        source_type: "investigation_result",
        source_id: "INV-1",
        governance_status: "approved_for_training",
        feedback_target: "models",
        currency: null,
        evidence_refs: ["investigation_results:INV-1"],
      },
      {
        label_id: "label_2",
        claim_id: "CLM-1",
        label_name: "medical_necessity_issue",
        label_value: "true",
        source_type: "qa_review",
        source_id: "QA-1",
        governance_status: "needs_review",
        feedback_target: "models",
        currency: null,
        evidence_refs: ["qa_reviews:QA-1"],
      },
      {
        label_id: "label_3",
        claim_id: "CLM-2",
        label_name: "amount_prevented",
        label_value: "8200.00",
        source_type: "investigation_result",
        source_id: "INV-2",
        governance_status: "approved_for_training",
        feedback_target: "workflow",
        currency: "CNY",
        evidence_refs: ["saving_attributions:saving_1"],
      },
    ]);

    expect(summary).toEqual({
      labelCount: 3,
      approvedForTrainingCount: 2,
      needsReviewCount: 1,
      modelFeedbackCount: 2,
      ruleFeedbackCount: 0,
      amountPreventedTotal: 8200,
      amountPreventedCurrency: "CNY",
    });
  });
});
