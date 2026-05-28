import { describe, expect, it } from "vitest";
import {
  buildAgentRunLogSummary,
  buildAuditSummary,
  buildAgentApprovalPayload,
  buildOpsAlertSummary,
  buildOutcomeLabelSummary,
  buildPromotionGateGovernanceRows,
  buildPromotionGateGovernanceSummary,
  buildWebhookDeliverySummary,
  canRecordWebhookDeliveryAttempt,
  hasPendingAgentApproval,
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

  it("uses timestamps for latest event even when global audit events are newest-first", () => {
    const summary = buildAuditSummary({
      events: [
        {
          audit_id: "audit_new",
          run_id: "run_new",
          event_type: "routing_policy.activation.completed",
          event_status: "succeeded",
          summary: "Routing policy activated",
          evidence_refs: ["routing_policies:fwa:v2:pre_payment"],
          created_at: "2026-05-27T12:00:00Z",
        },
        {
          audit_id: "audit_old",
          run_id: "run_old",
          event_type: "routing_policy.candidate.saved",
          event_status: "succeeded",
          summary: "Routing policy saved",
          evidence_refs: ["routing_policies:fwa:v2:pre_payment"],
          created_at: "2026-05-27T11:00:00Z",
        },
      ],
    });

    expect(summary.latestEventType).toBe("routing_policy.activation.completed");
  });
});

describe("promotion gate governance helpers", () => {
  it("normalizes rule model and routing promotion gates into one governance view", () => {
    const rows = buildPromotionGateGovernanceRows([
      {
        domain: "Rule",
        target_id: "rule_early_claim",
        status: "submitted",
        review_mode: "pre_payment",
        response: {
          decision: "routing_blocked",
          passed_count: 2,
          total_count: 3,
          blockers: ["approval missing"],
          gates: [
            {
              label: "Named owner",
              passed: true,
              blocker: "owner missing",
              evidence_source: "metadata",
            },
            {
              label: "Approval before routing",
              passed: false,
              blocker: "approval missing",
              evidence_source: "missing",
            },
          ],
        },
      },
      {
        domain: "Model",
        target_id: "baseline_fwa@0.1.0",
        status: "active",
        review_mode: "both",
        response: {
          decision: "routing_allowed",
          passed_count: 3,
          total_count: 3,
          blockers: [],
          gates: [
            {
              label: "Holdout metrics",
              passed: true,
              blocker: "holdout metrics missing",
              evidence_source: "evaluation",
            },
          ],
        },
      },
      {
        domain: "Routing",
        target_id: "fwa_risk_fusion_routing@v2",
        status: "approved",
        review_mode: "post_payment",
        response: {
          decision: "activation_allowed",
          passed_count: 4,
          total_count: 4,
          blockers: [],
          gates: [
            {
              label: "Governance approval",
              passed: true,
              blocker: "approval missing",
              evidence_source: "metadata",
            },
          ],
        },
      },
    ]);

    expect(rows).toEqual([
      {
        domain: "Rule",
        targetId: "rule_early_claim",
        status: "submitted",
        reviewMode: "pre_payment",
        decision: "routing_blocked",
        passedCount: 2,
        totalCount: 3,
        blockerCount: 1,
        topBlocker: "approval missing",
        evidenceSources: "metadata, missing",
      },
      {
        domain: "Model",
        targetId: "baseline_fwa@0.1.0",
        status: "active",
        reviewMode: "both",
        decision: "routing_allowed",
        passedCount: 3,
        totalCount: 3,
        blockerCount: 0,
        topBlocker: "none",
        evidenceSources: "evaluation",
      },
      {
        domain: "Routing",
        targetId: "fwa_risk_fusion_routing@v2",
        status: "approved",
        reviewMode: "post_payment",
        decision: "activation_allowed",
        passedCount: 4,
        totalCount: 4,
        blockerCount: 0,
        topBlocker: "none",
        evidenceSources: "metadata",
      },
    ]);
    expect(buildPromotionGateGovernanceSummary(rows)).toEqual({
      targetCount: 3,
      allowedTargetCount: 2,
      blockedTargetCount: 1,
      passedGateCount: 9,
      totalGateCount: 10,
      blockerCount: 1,
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

describe("agent approval helpers", () => {
  it("builds human approval payloads from pending agent runs", () => {
    const run = {
      agent_run_id: "agent_CLM-1",
      claim_id: "CLM-1",
      status: "succeeded",
      decision_boundary: "assistive_only",
      evidence_refs: ["agent_run:agent_CLM-1", "knowledge_cases:KC-1001"],
      steps: [],
      context_snapshots: [],
      tool_calls: [],
      policy_checks: [],
      tool_results: [],
      approvals: [
        {
          approval_id: "approval_agent_CLM-1",
          agent_run_id: "agent_CLM-1",
          proposed_action: "manual_review_required",
          decision: "pending",
          approver: "unassigned",
          reason: "Agent output requires human approval before downstream action.",
          evidence_refs: ["agent_run:agent_CLM-1"],
        },
      ],
    };

    expect(hasPendingAgentApproval(run)).toBe(true);
    expect(buildAgentApprovalPayload(run, "approved", " qa-lead ")).toEqual({
      decision: "approved",
      approver: "qa-lead",
      reason: "Evidence package approved for manual review routing.",
      evidence_refs: ["agent_run:agent_CLM-1", "knowledge_cases:KC-1001"],
    });
  });

  it("does not expose approval actions after the decision is recorded", () => {
    expect(
      hasPendingAgentApproval({
        agent_run_id: "agent_CLM-1",
        claim_id: "CLM-1",
        status: "succeeded",
        decision_boundary: "assistive_only",
        evidence_refs: [],
        steps: [],
        context_snapshots: [],
        tool_calls: [],
        policy_checks: [],
        tool_results: [],
        approvals: [
          {
            approval_id: "approval_agent_CLM-1",
            agent_run_id: "agent_CLM-1",
            proposed_action: "manual_review_required",
            decision: "approved",
            approver: "qa-lead",
            reason: "Approved.",
            evidence_refs: ["agent_run:agent_CLM-1"],
          },
        ],
      }),
    ).toBe(false);
  });
});

describe("buildOpsAlertSummary", () => {
  it("summarizes high-risk routing and SLA alerts", () => {
    const summary = buildOpsAlertSummary([
      {
        alert_id: "alert_1",
        alert_type: "high_risk_routing",
        severity: "critical",
        status: "open",
        claim_id: "CLM-1",
        lead_id: "lead_CLM-1",
        case_id: null,
        scheme_family: "provider_peer_outlier",
        message: "High-risk lead pending triage.",
        recommended_action: "Open an investigation case.",
        evidence_refs: ["rule_runs:PROVIDER_PROFILE_HIGH"],
      },
      {
        alert_id: "alert_2",
        alert_type: "sla_breach",
        severity: "high",
        status: "closed",
        claim_id: "CLM-2",
        lead_id: "lead_CLM-2",
        case_id: "case_CLM-2",
        scheme_family: "diagnosis_procedure_mismatch",
        message: "Case breached SLA.",
        recommended_action: "Escalate overdue case.",
        evidence_refs: ["case_workflow:overdue"],
      },
    ]);

    expect(summary).toEqual({
      alertCount: 2,
      openAlertCount: 1,
      criticalAlertCount: 1,
      routingAlertCount: 1,
      slaBreachCount: 1,
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

describe("buildWebhookDeliverySummary", () => {
  it("summarizes webhook delivery and signature readiness", () => {
    const summary = buildWebhookDeliverySummary([
      {
        event_id: "webhook_1",
        event_type: "fwa.score.completed",
        source_event_type: "scoring.completed",
        source_audit_id: "audit_1",
        claim_id: "CLM-1",
        run_id: "run_1",
        delivery_status: "pending",
        retry_count: 0,
        max_attempts: 3,
        idempotency_key: "fwa-webhook:fwa.score.completed:audit_1",
        signature_key_id: "tpa-webhook-v1",
        signature_algorithm: "hmac-sha256",
        signature_base_string: "fwa.score.completed.audit_1.run_1.CLM-1",
        evidence_refs: ["audit:scoring.completed"],
      },
      {
        event_id: "webhook_2",
        event_type: "fwa.qa.reviewed",
        source_event_type: "qa.result.received",
        source_audit_id: "audit_2",
        claim_id: "CLM-2",
        run_id: "run_2",
        delivery_status: "retry_wait",
        retry_count: 1,
        max_attempts: 3,
        last_error_message: "TPA unavailable",
        idempotency_key: "fwa-webhook:fwa.qa.reviewed:audit_2",
        signature_key_id: "tpa-webhook-v1",
        signature_algorithm: "hmac-sha256",
        signature_base_string: "fwa.qa.reviewed.audit_2.run_2.CLM-2",
        evidence_refs: ["audit:qa.result.received"],
      },
    ]);

    expect(summary).toEqual({
      eventCount: 2,
      pendingCount: 1,
      retryWaitCount: 1,
      deliveredCount: 0,
      failedCount: 0,
      signedCount: 2,
    });
  });

  it("allows delivery attempts only while webhook events are still actionable", () => {
    const baseEvent = {
      event_id: "webhook_1",
      event_type: "fwa.score.completed",
      source_event_type: "scoring.completed",
      source_audit_id: "audit_1",
      claim_id: "CLM-1",
      run_id: "run_1",
      retry_count: 0,
      max_attempts: 3,
      idempotency_key: "fwa-webhook:fwa.score.completed:audit_1",
      signature_key_id: "tpa-webhook-v1",
      signature_algorithm: "hmac-sha256",
      signature_base_string: "fwa.score.completed.audit_1.run_1.CLM-1",
      evidence_refs: ["audit:scoring.completed"],
    };

    expect(canRecordWebhookDeliveryAttempt({ ...baseEvent, delivery_status: "pending" })).toBe(
      true,
    );
    expect(canRecordWebhookDeliveryAttempt({ ...baseEvent, delivery_status: "retry_wait" })).toBe(
      true,
    );
    expect(canRecordWebhookDeliveryAttempt({ ...baseEvent, delivery_status: "delivered" })).toBe(
      false,
    );
    expect(canRecordWebhookDeliveryAttempt({ ...baseEvent, delivery_status: "failed" })).toBe(
      false,
    );
  });
});
