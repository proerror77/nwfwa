import { describe, expect, it } from "vitest";
import {
  auditEventFilterShortcuts,
  buildAgentRunLogSummary,
  buildAuditSummary,
  buildAgentApprovalPayload,
  buildGlobalAuditEventFilters,
  canSubmitAgentApproval,
  buildFwaSchemeGovernanceRows,
  buildFwaSchemeGovernanceSummary,
  buildGovernanceChangeTimelineRows,
  buildOpsAlertSummary,
  buildOutcomeLabelSummary,
  buildPromotionGateGovernanceRows,
  buildPromotionGateGovernanceSummary,
  buildWebhookDeliverySummary,
  canRecordWebhookDeliveryAttempt,
  filterOutcomeLabels,
  hasPendingAgentApproval,
} from "./GovernancePage";

describe("auditEventFilterShortcuts", () => {
  it("covers common governance audit filters", () => {
    expect(auditEventFilterShortcuts).toEqual([
      { label: "Scoring", eventType: "scoring.completed" },
      { label: "QA Results", eventType: "qa.result.received" },
      { label: "QA Feedback Status", eventType: "qa.feedback.status.updated" },
      { label: "Case Status", eventType: "case.status.updated" },
      { label: "Rule Candidates", eventType: "rule.candidate.saved" },
      { label: "Audit Samples", eventType: "audit_sample.created" },
    ]);
  });
});

describe("buildGlobalAuditEventFilters", () => {
  it("maps UI filter state to audit event API filters including sample id", () => {
    expect(
      buildGlobalAuditEventFilters(
        {
          eventType: "audit_sample.created",
          actorId: "qa-lead",
          runId: "audit_sample_sample_1",
          claimId: "CLM-1",
          feedbackId: "qa_feedback_QA-1",
          qaCaseId: "QA-1",
          sampleId: "sample_1",
          ruleId: "rule_early_claim",
          ruleVersion: "2",
          modelKey: "baseline_fwa",
          modelVersion: "0.2.0",
          routingPolicyId: "fwa_risk_fusion_routing",
          routingPolicyVersion: "3",
          reviewMode: "post_payment",
          limit: "25",
        },
        25,
      ),
    ).toEqual({
      limit: 25,
      event_type: "audit_sample.created",
      actor_id: "qa-lead",
      run_id: "audit_sample_sample_1",
      claim_id: "CLM-1",
      feedback_id: "qa_feedback_QA-1",
      qa_case_id: "QA-1",
      sample_id: "sample_1",
      rule_id: "rule_early_claim",
      rule_version: "2",
      model_key: "baseline_fwa",
      model_version: "0.2.0",
      routing_policy_id: "fwa_risk_fusion_routing",
      routing_policy_version: "3",
      review_mode: "post_payment",
    });
  });
});

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

describe("buildGovernanceChangeTimelineRows", () => {
  it("extracts rule model and routing lifecycle changes from global audit events", () => {
    const rows = buildGovernanceChangeTimelineRows([
      {
        audit_id: "audit_rule",
        run_id: "run_rule",
        event_type: "rule.promotion.reviewed",
        event_status: "succeeded",
        summary: "Rule promotion review: approved",
        payload: {
          rule_id: "rule_early_claim",
          rule_version: 2,
          decision: "approved",
          reviewer: "rule-governance",
        },
        evidence_refs: ["rules:rule_early_claim:v2"],
        created_at: "2026-05-27T10:00:00Z",
      },
      {
        audit_id: "audit_model",
        run_id: "run_model",
        event_type: "model.activation.completed",
        event_status: "succeeded",
        summary: "Model activation completed",
        payload: {
          model_key: "baseline_fwa",
          model_version: "0.2.0",
          from_status: "approved",
          to_status: "active",
        },
        evidence_refs: ["model_versions:baseline_fwa:0.2.0"],
      },
      {
        audit_id: "audit_routing",
        run_id: "run_routing",
        event_type: "routing_policy.status.changed",
        event_status: "succeeded",
        summary: "Routing policy approved",
        payload: {
          policy_id: "fwa_risk_fusion_routing",
          version: 3,
          review_mode: "post_payment",
          from_status: "submitted",
          to_status: "approved",
          owner: "policy-ops",
        },
        evidence_refs: ["routing_policies:fwa_risk_fusion_routing:v3:post_payment"],
      },
      {
        audit_id: "audit_dataset",
        run_id: "run_dataset",
        event_type: "dataset.registered",
        event_status: "succeeded",
        summary: "Dataset registered",
        payload: {
          dataset_key: "claims_training",
          dataset_version: "v1",
          to_status: "registered",
          owner: "data-ops",
        },
        evidence_refs: ["datasets:claims_training:v1"],
      },
      {
        audit_id: "audit_qa_feedback_status",
        run_id: "run_qa_feedback_status",
        event_type: "qa.feedback.status.updated",
        event_status: "succeeded",
        summary: "QA feedback status updated",
        payload: {
          feedback_id: "qa_feedback_QA-1",
          qa_case_id: "QA-1",
          claim_id: "CLM-1",
          from_status: "open",
          to_status: "in_progress",
          actor_id: "qa-lead",
        },
        evidence_refs: ["qa_feedback:qa_feedback_QA-1"],
      },
      {
        audit_id: "audit_agent_approval",
        run_id: "agent_approval_agent_CLM-1",
        event_type: "agent.approval.decided",
        event_status: "succeeded",
        summary: "Agent approval decision: approved",
        payload: {
          agent_run_id: "agent_CLM-1",
          proposed_action: "manual_review_required",
          decision: "approved",
          approver: "qa-lead",
        },
        evidence_refs: ["agent_approval:manual_review_required"],
      },
      {
        audit_id: "audit_sample_created",
        run_id: "audit_sample_SAMPLE-1",
        event_type: "audit_sample.created",
        event_status: "succeeded",
        summary: "Audit sample created: stratified",
        payload: {
          sample_id: "SAMPLE-1",
          sample_mode: "stratified",
          selection_method: "stratified_round_robin",
          reviewer: "qa-governance-reviewer",
        },
        evidence_refs: ["audit_samples:SAMPLE-1"],
      },
      {
        audit_id: "audit_scoring",
        run_id: "run_scoring",
        event_type: "scoring.completed",
        event_status: "succeeded",
        summary: "Scoring completed",
        evidence_refs: ["scoring_runs:run_scoring"],
      },
    ]);

    expect(rows).toEqual([
      {
        auditId: "audit_rule",
        domain: "Rule",
        eventType: "rule.promotion.reviewed",
        targetId: "rule_early_claim@v2",
        statusTransition: "review -> approved",
        actor: "rule-governance",
        decision: "approved",
        summary: "Rule promotion review: approved",
        createdAt: "2026-05-27T10:00:00Z",
        evidenceRefs: ["rules:rule_early_claim:v2"],
      },
      {
        auditId: "audit_model",
        domain: "Model",
        eventType: "model.activation.completed",
        targetId: "baseline_fwa@0.2.0",
        statusTransition: "approved -> active",
        actor: "system",
        decision: "active",
        summary: "Model activation completed",
        createdAt: "run_model",
        evidenceRefs: ["model_versions:baseline_fwa:0.2.0"],
      },
      {
        auditId: "audit_routing",
        domain: "Routing",
        eventType: "routing_policy.status.changed",
        targetId: "fwa_risk_fusion_routing@v3 / post_payment",
        statusTransition: "submitted -> approved",
        actor: "policy-ops",
        decision: "approved",
        summary: "Routing policy approved",
        createdAt: "run_routing",
        evidenceRefs: ["routing_policies:fwa_risk_fusion_routing:v3:post_payment"],
      },
      {
        auditId: "audit_dataset",
        domain: "Data",
        eventType: "dataset.registered",
        targetId: "claims_training@v1",
        statusTransition: "- -> registered",
        actor: "data-ops",
        decision: "registered",
        summary: "Dataset registered",
        createdAt: "run_dataset",
        evidenceRefs: ["datasets:claims_training:v1"],
      },
      {
        auditId: "audit_qa_feedback_status",
        domain: "QA",
        eventType: "qa.feedback.status.updated",
        targetId: "qa_feedback_QA-1",
        statusTransition: "open -> in_progress",
        actor: "qa-lead",
        decision: "in_progress",
        summary: "QA feedback status updated",
        createdAt: "run_qa_feedback_status",
        evidenceRefs: ["qa_feedback:qa_feedback_QA-1"],
      },
      {
        auditId: "audit_agent_approval",
        domain: "Agent",
        eventType: "agent.approval.decided",
        targetId: "agent_CLM-1 / manual_review_required",
        statusTransition: "review -> approved",
        actor: "qa-lead",
        decision: "approved",
        summary: "Agent approval decision: approved",
        createdAt: "agent_approval_agent_CLM-1",
        evidenceRefs: ["agent_approval:manual_review_required"],
      },
      {
        auditId: "audit_sample_created",
        domain: "QA",
        eventType: "audit_sample.created",
        targetId: "SAMPLE-1",
        statusTransition: "created -> stratified",
        actor: "qa-governance-reviewer",
        decision: "stratified_round_robin",
        summary: "Audit sample created: stratified",
        createdAt: "audit_sample_SAMPLE-1",
        evidenceRefs: ["audit_samples:SAMPLE-1"],
      },
    ]);
  });
});

describe("FWA scheme governance helpers", () => {
  it("summarizes scheme taxonomy evidence requirements and review routes", () => {
    const rows = buildFwaSchemeGovernanceRows([
      {
        scheme_family: "provider_peer_outlier",
        display_name: "Provider peer outlier",
        risk_domain: "Provider",
        description: "Provider deviates from peer group.",
        minimum_evidence: ["peer_group_definition", "time_window", "statistical_deviation"],
        default_review_route: "provider_review",
        primary_layers: ["L1_PEER_BENCHMARK", "L6_PROVIDER_GRAPH_RISK"],
      },
      {
        scheme_family: "medically_unnecessary_service",
        display_name: "Medically unnecessary service",
        risk_domain: "Clinical",
        description: "Medical necessity needs review.",
        minimum_evidence: ["diagnosis", "order"],
        default_review_route: "medical_review",
        primary_layers: ["L5_MEDICAL_REASONABLENESS"],
      },
    ]);

    expect(rows).toEqual([
      {
        schemeFamily: "medically_unnecessary_service",
        displayName: "Medically unnecessary service",
        riskDomain: "Clinical",
        defaultReviewRoute: "medical_review",
        evidenceCount: 2,
        minimumEvidence: "diagnosis, order",
        primaryLayers: "L5_MEDICAL_REASONABLENESS",
      },
      {
        schemeFamily: "provider_peer_outlier",
        displayName: "Provider peer outlier",
        riskDomain: "Provider",
        defaultReviewRoute: "provider_review",
        evidenceCount: 3,
        minimumEvidence: "peer_group_definition, time_window, statistical_deviation",
        primaryLayers: "L1_PEER_BENCHMARK, L6_PROVIDER_GRAPH_RISK",
      },
    ]);
    expect(buildFwaSchemeGovernanceSummary(rows)).toEqual({
      schemeCount: 2,
      domainCount: 2,
      evidenceRequirementCount: 5,
      medicalReviewCount: 1,
      providerReviewCount: 1,
    });
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
    expect(canSubmitAgentApproval(run, " qa-lead ")).toBe(true);
    expect(canSubmitAgentApproval(run, " ")).toBe(false);
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
      {
        alert_id: "alert_3",
        alert_type: "medical_review_required",
        severity: "high",
        status: "open",
        claim_id: "CLM-3",
        lead_id: null,
        case_id: null,
        scheme_family: "medically_unnecessary_service",
        message: "Clinical evidence gap pending medical review.",
        recommended_action: "Assign a medical reviewer.",
        evidence_refs: ["audit:audit_1"],
      },
      {
        alert_id: "alert_4",
        alert_type: "agent_approval_pending",
        severity: "high",
        status: "open",
        claim_id: "CLM-4",
        lead_id: null,
        case_id: null,
        scheme_family: "provider_peer_outlier",
        message: "Agent output pending approval.",
        recommended_action: "Review the evidence package.",
        evidence_refs: ["agent_run:agent_CLM-4"],
      },
    ]);

    expect(summary).toEqual({
      alertCount: 4,
      openAlertCount: 3,
      criticalAlertCount: 1,
      routingAlertCount: 1,
      slaBreachCount: 1,
      medicalReviewAlertCount: 1,
      agentApprovalAlertCount: 1,
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
      {
        label_id: "label_4",
        claim_id: "CLM-3",
        label_name: "false_positive",
        label_value: "true",
        source_type: "case_status",
        source_id: "case_CLM-3",
        governance_status: "needs_review",
        feedback_target: "rules",
        currency: null,
        evidence_refs: ["investigation_cases:case_CLM-3"],
      },
      {
        label_id: "label_5",
        claim_id: "CLM-4",
        label_name: "clinical_evidence_sufficient",
        label_value: "true",
        source_type: "medical_review",
        source_id: "audit_medical_review_1",
        governance_status: "approved_for_training",
        feedback_target: "workflow",
        currency: null,
        evidence_refs: ["medical_review:MR-1"],
      },
    ]);

    expect(summary).toEqual({
      labelCount: 5,
      approvedForTrainingCount: 3,
      needsReviewCount: 2,
      modelFeedbackCount: 2,
      ruleFeedbackCount: 1,
      falsePositiveCount: 1,
      caseStatusLabelCount: 1,
      medicalReviewLabelCount: 1,
      evidenceBackedCount: 5,
      sourceTypeRows: [
        "investigation_result: 2",
        "case_status: 1",
        "medical_review: 1",
        "qa_review: 1",
      ],
      amountPreventedTotal: 8200,
      amountPreventedCurrency: "CNY",
    });
  });
});

describe("filterOutcomeLabels", () => {
  it("filters governed labels by source target and status", () => {
    const labels = [
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
        claim_id: "CLM-2",
        label_name: "false_positive",
        label_value: "true",
        source_type: "case_status",
        source_id: "case_CLM-2",
        governance_status: "needs_review",
        feedback_target: "rules",
        currency: null,
        evidence_refs: ["investigation_cases:case_CLM-2"],
      },
      {
        label_id: "label_3",
        claim_id: "CLM-3",
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
        label_id: "label_4",
        claim_id: "CLM-4",
        label_name: "clinical_evidence_sufficient",
        label_value: "true",
        source_type: "medical_review",
        source_id: "audit_medical_review_1",
        governance_status: "approved_for_training",
        feedback_target: "workflow",
        currency: null,
        evidence_refs: ["medical_review:MR-1"],
      },
    ];

    expect(
      filterOutcomeLabels(labels, {
        sourceType: "case_status",
        feedbackTarget: "rules",
        governanceStatus: "needs_review",
      }).map((label) => label.label_id),
    ).toEqual(["label_2"]);
    expect(filterOutcomeLabels(labels, { feedbackTarget: "models" })).toHaveLength(2);
    expect(
      filterOutcomeLabels(labels, { sourceType: "medical_review" }).map(
        (label) => label.label_id,
      ),
    ).toEqual(["label_4"]);
    expect(filterOutcomeLabels(labels, {})).toHaveLength(4);
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
