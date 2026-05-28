import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getClaimAuditHistory,
  listAgentRuns,
  listOpsAlerts,
  listOutcomeLabels,
  listWebhookEvents,
  submitAgentApproval,
  submitWebhookDeliveryAttempt,
} from "../api";

type AuditEvent = {
  audit_id: string;
  run_id: string;
  event_type: string;
  event_status: string;
  summary: string;
  payload?: Record<string, unknown>;
  evidence_refs: string[];
  created_at?: string | null;
};

type ClaimAuditHistoryResponse = {
  claim_id: string;
  events: AuditEvent[];
};

type AgentRunLog = {
  agent_run_id: string;
  claim_id: string;
  status: string;
  decision_boundary: string;
  evidence_refs: string[];
  steps: Array<Record<string, unknown>>;
  context_snapshots: AgentContextSnapshot[];
  policy_checks: AgentPolicyCheck[];
  tool_calls: AgentToolCall[];
  tool_results: AgentToolResult[];
  approvals: AgentApproval[];
  created_at?: string | null;
  completed_at?: string | null;
};

type AgentContextSnapshot = {
  snapshot_id: string;
  redaction_status: string;
  context_json: Record<string, unknown>;
  source_refs: string[];
  checksum: string;
};

type AgentToolCall = {
  tool_call_id: string;
  tool_name: string;
  status: string;
  input_json: Record<string, unknown>;
  evidence_refs: string[];
};

type AgentPolicyCheck = {
  policy_check_id: string;
  agent_run_id: string;
  tool_call_id: string;
  tool_name: string;
  policy_name: string;
  decision: string;
  reason: string;
  evidence_refs: string[];
  created_at?: string | null;
};

type AgentToolResult = {
  tool_result_id: string;
  tool_call_id: string;
  tool_name: string;
  status: string;
  output_json: Record<string, unknown>;
  evidence_refs: string[];
};

type AgentApproval = {
  approval_id: string;
  agent_run_id: string;
  proposed_action: string;
  decision: string;
  approver: string;
  reason: string;
  evidence_refs: string[];
  created_at?: string | null;
};

type AgentRunLogListResponse = {
  runs: AgentRunLog[];
};

type OpsAlert = {
  alert_id: string;
  alert_type: string;
  severity: string;
  status: string;
  claim_id: string;
  lead_id?: string | null;
  case_id?: string | null;
  scheme_family: string;
  message: string;
  recommended_action: string;
  evidence_refs: string[];
};

type OpsAlertListResponse = {
  alerts: OpsAlert[];
};

type WebhookEvent = {
  event_id: string;
  event_type: string;
  source_event_type: string;
  source_audit_id: string;
  claim_id: string;
  run_id: string;
  delivery_status: string;
  retry_count: number;
  max_attempts: number;
  next_attempt_at?: string | null;
  last_attempt_at?: string | null;
  last_response_status_code?: number | null;
  last_error_message?: string | null;
  idempotency_key: string;
  signature_key_id: string;
  signature_algorithm: string;
  signature_base_string: string;
  evidence_refs: string[];
  occurred_at?: string | null;
};

type WebhookEventListResponse = {
  events: WebhookEvent[];
};

type OutcomeLabel = {
  label_id: string;
  claim_id: string;
  label_name: string;
  label_value: string;
  source_type: string;
  source_id: string;
  governance_status: string;
  feedback_target: string;
  currency?: string | null;
  evidence_refs: string[];
};

type OutcomeLabelListResponse = {
  labels: OutcomeLabel[];
};

export function buildAuditSummary(data?: ClaimAuditHistoryResponse) {
  const events = data?.events ?? [];
  return {
    totalEvents: events.length,
    succeededEvents: events.filter((event) => event.event_status === "succeeded").length,
    failedEvents: events.filter((event) => event.event_status === "failed").length,
    latestEventType: events.at(-1)?.event_type ?? "none",
  };
}

export function buildAgentRunLogSummary(runs: AgentRunLog[] = []) {
  const contextSnapshots = runs.flatMap((run) => run.context_snapshots ?? []);
  const policyChecks = runs.flatMap((run) => run.policy_checks ?? []);
  const toolCalls = runs.flatMap((run) => run.tool_calls ?? []);
  const toolResults = runs.flatMap((run) => run.tool_results ?? []);
  const approvals = runs.flatMap((run) => run.approvals ?? []);
  return {
    runCount: runs.length,
    contextSnapshotCount: contextSnapshots.length,
    piiMaskedContextCount: contextSnapshots.filter(
      (snapshot) => snapshot.redaction_status === "pii_masked",
    ).length,
    toolCallCount: toolCalls.length,
    toolResultCount: toolResults.length,
    failedToolCallCount: toolCalls.filter((call) => call.status === "failed").length,
    policyCheckCount: policyChecks.length,
    deniedPolicyCheckCount: policyChecks.filter((check) => check.decision === "denied").length,
    approvalCount: approvals.length,
    pendingApprovalCount: approvals.filter((approval) => approval.decision === "pending").length,
  };
}

export function hasPendingAgentApproval(run: AgentRunLog) {
  return run.approvals.some((approval) => approval.decision === "pending");
}

export function buildAgentApprovalPayload(
  run: AgentRunLog,
  decision: "approved" | "rejected",
  approver: string,
) {
  const pendingApproval = run.approvals.find((approval) => approval.decision === "pending");
  const evidenceRefs = [
    ...(pendingApproval?.evidence_refs ?? []),
    ...run.evidence_refs,
    `agent_run:${run.agent_run_id}`,
  ].filter((value, index, refs) => refs.indexOf(value) === index);
  return {
    decision,
    approver: approver.trim(),
    reason:
      decision === "approved"
        ? "Evidence package approved for manual review routing."
        : "Evidence package rejected pending stronger support.",
    evidence_refs: evidenceRefs,
  };
}

export function buildOpsAlertSummary(alerts: OpsAlert[] = []) {
  return {
    alertCount: alerts.length,
    openAlertCount: alerts.filter((alert) => alert.status === "open").length,
    criticalAlertCount: alerts.filter((alert) => alert.severity === "critical").length,
    routingAlertCount: alerts.filter((alert) => alert.alert_type === "high_risk_routing").length,
    slaBreachCount: alerts.filter((alert) => alert.alert_type === "sla_breach").length,
  };
}

export function buildWebhookDeliverySummary(events: WebhookEvent[] = []) {
  return {
    eventCount: events.length,
    pendingCount: events.filter((event) => event.delivery_status === "pending").length,
    retryWaitCount: events.filter((event) => event.delivery_status === "retry_wait").length,
    deliveredCount: events.filter((event) => event.delivery_status === "delivered").length,
    failedCount: events.filter((event) => event.delivery_status === "failed").length,
    signedCount: events.filter(
      (event) => event.signature_algorithm === "hmac-sha256" && event.signature_key_id.length > 0,
    ).length,
  };
}

export function canRecordWebhookDeliveryAttempt(event: WebhookEvent) {
  return event.delivery_status === "pending" || event.delivery_status === "retry_wait";
}

export function buildOutcomeLabelSummary(labels: OutcomeLabel[] = []) {
  const amountPreventedLabels = labels.filter((label) => label.label_name === "amount_prevented");
  const amountPreventedTotal = amountPreventedLabels.reduce((total, label) => {
    const value = Number(label.label_value);
    return Number.isFinite(value) ? total + value : total;
  }, 0);
  return {
    labelCount: labels.length,
    approvedForTrainingCount: labels.filter(
      (label) => label.governance_status === "approved_for_training",
    ).length,
    needsReviewCount: labels.filter((label) => label.governance_status === "needs_review").length,
    modelFeedbackCount: labels.filter((label) => label.feedback_target === "models").length,
    ruleFeedbackCount: labels.filter((label) => label.feedback_target === "rules").length,
    amountPreventedTotal,
    amountPreventedCurrency: amountPreventedLabels[0]?.currency ?? "N/A",
  };
}

export function GovernancePage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [claimId, setClaimId] = useState("CLM-0287");
  const [agentApprover, setAgentApprover] = useState("qa-lead");
  const queryClient = useQueryClient();
  const auditQuery = useQuery({
    queryKey: ["claim-audit-history", apiKey, claimId],
    queryFn: () => getClaimAuditHistory(claimId, apiKey) as Promise<ClaimAuditHistoryResponse>,
    enabled: claimId.trim().length > 0,
  });
  const agentRunsQuery = useQuery({
    queryKey: ["agent-run-logs", apiKey],
    queryFn: () => listAgentRuns(apiKey) as Promise<AgentRunLogListResponse>,
  });
  const alertsQuery = useQuery({
    queryKey: ["ops-alerts", apiKey],
    queryFn: () => listOpsAlerts(apiKey) as Promise<OpsAlertListResponse>,
  });
  const labelsQuery = useQuery({
    queryKey: ["outcome-labels", apiKey],
    queryFn: () => listOutcomeLabels(apiKey) as Promise<OutcomeLabelListResponse>,
  });
  const webhookQuery = useQuery({
    queryKey: ["webhook-events", apiKey],
    queryFn: () => listWebhookEvents(apiKey) as Promise<WebhookEventListResponse>,
  });
  const summary = buildAuditSummary(auditQuery.data);
  const agentSummary = buildAgentRunLogSummary(agentRunsQuery.data?.runs);
  const alertSummary = buildOpsAlertSummary(alertsQuery.data?.alerts);
  const labelSummary = buildOutcomeLabelSummary(labelsQuery.data?.labels);
  const webhookSummary = buildWebhookDeliverySummary(webhookQuery.data?.events);
  const agentApprovalMutation = useMutation({
    mutationFn: ({
      run,
      decision,
    }: {
      run: AgentRunLog;
      decision: "approved" | "rejected";
    }) =>
      submitAgentApproval(
        run.agent_run_id,
        buildAgentApprovalPayload(run, decision, agentApprover),
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["agent-run-logs"] });
      queryClient.invalidateQueries({ queryKey: ["claim-audit-history"] });
    },
  });
  const deliveryAttemptMutation = useMutation({
    mutationFn: ({
      eventId,
      deliveryStatus,
    }: {
      eventId: string;
      deliveryStatus: "delivered" | "failed";
    }) =>
      submitWebhookDeliveryAttempt(
        eventId,
        {
          delivery_status: deliveryStatus,
          response_status_code: deliveryStatus === "delivered" ? 200 : 503,
          error_message:
            deliveryStatus === "failed"
              ? "Manual delivery check failed from Governance."
              : null,
        },
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["webhook-events"] });
    },
  });

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Governance</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        <label>
          Claim ID
          <input value={claimId} onChange={(event) => setClaimId(event.target.value)} />
        </label>
        <label>
          Agent Approver
          <input
            value={agentApprover}
            onChange={(event) => setAgentApprover(event.target.value)}
          />
        </label>
        <div className="summary-grid">
          <div>
            <span>Audit Events</span>
            <strong>{summary.totalEvents}</strong>
          </div>
          <div>
            <span>Succeeded</span>
            <strong>{summary.succeededEvents}</strong>
          </div>
          <div>
            <span>Failed</span>
            <strong>{summary.failedEvents}</strong>
          </div>
          <div>
            <span>Latest Event</span>
            <strong>{summary.latestEventType}</strong>
          </div>
          <div>
            <span>Agent Runs</span>
            <strong>{agentSummary.runCount}</strong>
          </div>
          <div>
            <span>Tool Calls</span>
            <strong>{agentSummary.toolCallCount}</strong>
          </div>
          <div>
            <span>Policy Checks</span>
            <strong>{agentSummary.policyCheckCount}</strong>
          </div>
          <div>
            <span>Contexts</span>
            <strong>{agentSummary.contextSnapshotCount}</strong>
          </div>
          <div>
            <span>Approvals</span>
            <strong>{agentSummary.pendingApprovalCount}</strong>
          </div>
          <div>
            <span>Alerts</span>
            <strong>{alertSummary.openAlertCount}</strong>
          </div>
          <div>
            <span>Critical Alerts</span>
            <strong>{alertSummary.criticalAlertCount}</strong>
          </div>
          <div>
            <span>Labels</span>
            <strong>{labelSummary.labelCount}</strong>
          </div>
          <div>
            <span>Webhooks</span>
            <strong>{webhookSummary.eventCount}</strong>
          </div>
          <div>
            <span>Training Ready</span>
            <strong>{labelSummary.approvedForTrainingCount}</strong>
          </div>
        </div>
        {auditQuery.error ? <pre className="error">{String(auditQuery.error.message)}</pre> : null}
        {agentRunsQuery.error ? (
          <pre className="error">{String(agentRunsQuery.error.message)}</pre>
        ) : null}
        {agentApprovalMutation.error ? (
          <pre className="error">{String(agentApprovalMutation.error.message)}</pre>
        ) : null}
        {alertsQuery.error ? (
          <pre className="error">{String(alertsQuery.error.message)}</pre>
        ) : null}
        {labelsQuery.error ? <pre className="error">{String(labelsQuery.error.message)}</pre> : null}
        {webhookQuery.error ? (
          <pre className="error">{String(webhookQuery.error.message)}</pre>
        ) : null}
        {deliveryAttemptMutation.error ? (
          <pre className="error">{String(deliveryAttemptMutation.error.message)}</pre>
        ) : null}
      </div>

      <div className="panel">
        <h2>Operations Alerts</h2>
        <div className="summary-grid">
          <div>
            <span>Total</span>
            <strong>{alertSummary.alertCount}</strong>
          </div>
          <div>
            <span>Routing</span>
            <strong>{alertSummary.routingAlertCount}</strong>
          </div>
          <div>
            <span>SLA Breach</span>
            <strong>{alertSummary.slaBreachCount}</strong>
          </div>
        </div>
        {alertsQuery.data?.alerts.length ? (
          <ol className="audit-timeline">
            {alertsQuery.data.alerts.map((alert) => (
              <li key={alert.alert_id}>
                <div>
                  <strong>{alert.alert_type}</strong>
                  <span>{alert.severity}</span>
                </div>
                <small>
                  {alert.claim_id} / {alert.scheme_family}
                </small>
                <p>{alert.message}</p>
                <p>{alert.recommended_action}</p>
                <ul className="result-list">
                  {alert.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No operations alerts loaded</p>
        )}
      </div>

      <div className="panel">
        <h2>Webhook Delivery</h2>
        <div className="summary-grid">
          <div>
            <span>Pending</span>
            <strong>{webhookSummary.pendingCount}</strong>
          </div>
          <div>
            <span>Retry Wait</span>
            <strong>{webhookSummary.retryWaitCount}</strong>
          </div>
          <div>
            <span>Delivered</span>
            <strong>{webhookSummary.deliveredCount}</strong>
          </div>
          <div>
            <span>Failed</span>
            <strong>{webhookSummary.failedCount}</strong>
          </div>
          <div>
            <span>Signed</span>
            <strong>{webhookSummary.signedCount}</strong>
          </div>
        </div>
        {webhookQuery.data?.events.length ? (
          <ol className="audit-timeline">
            {webhookQuery.data.events.map((event) => (
              <li key={event.event_id}>
                <div>
                  <strong>{event.event_type}</strong>
                  <span>{event.delivery_status}</span>
                </div>
                <small>
                  {event.claim_id} / retry {event.retry_count}/{event.max_attempts}
                </small>
                <p>{event.idempotency_key}</p>
                <p>
                  {event.signature_algorithm} / {event.signature_key_id}
                </p>
                {event.last_error_message ? <p>{event.last_error_message}</p> : null}
                {canRecordWebhookDeliveryAttempt(event) ? (
                  <div className="button-row">
                    <button
                      disabled={deliveryAttemptMutation.isPending}
                      onClick={() =>
                        deliveryAttemptMutation.mutate({
                          eventId: event.event_id,
                          deliveryStatus: "delivered",
                        })
                      }
                      type="button"
                    >
                      Mark Delivered
                    </button>
                    <button
                      disabled={deliveryAttemptMutation.isPending}
                      onClick={() =>
                        deliveryAttemptMutation.mutate({
                          eventId: event.event_id,
                          deliveryStatus: "failed",
                        })
                      }
                      type="button"
                    >
                      Mark Failed
                    </button>
                  </div>
                ) : null}
                <ul className="result-list">
                  {event.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No webhook events loaded</p>
        )}
      </div>

      <div className="panel">
        <h2>Audit Timeline</h2>
        {auditQuery.data?.events.length ? (
          <ol className="audit-timeline">
            {auditQuery.data.events.map((event) => (
              <li key={event.audit_id}>
                <div>
                  <strong>{event.event_type}</strong>
                  <span>{event.event_status}</span>
                </div>
                <small>{event.created_at || event.run_id}</small>
                <p>{event.summary}</p>
                <ul className="result-list">
                  {event.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No audit events loaded</p>
        )}
      </div>
      <div className="panel">
        <h2>Outcome Labels</h2>
        <div className="summary-grid">
          <div>
            <span>Needs Review</span>
            <strong>{labelSummary.needsReviewCount}</strong>
          </div>
          <div>
            <span>Model Feedback</span>
            <strong>{labelSummary.modelFeedbackCount}</strong>
          </div>
          <div>
            <span>Rule Feedback</span>
            <strong>{labelSummary.ruleFeedbackCount}</strong>
          </div>
          <div>
            <span>Prevented</span>
            <strong>
              {labelSummary.amountPreventedCurrency} {labelSummary.amountPreventedTotal}
            </strong>
          </div>
        </div>
        {labelsQuery.data?.labels.length ? (
          <ol className="audit-timeline">
            {labelsQuery.data.labels.map((label) => (
              <li key={label.label_id}>
                <div>
                  <strong>{label.label_name}</strong>
                  <span>{label.governance_status}</span>
                </div>
                <small>
                  {label.claim_id} / {label.source_type}:{label.source_id}
                </small>
                <p>
                  {label.label_value}
                  {label.currency ? ` ${label.currency}` : ""}{" "}
                  {"->"} {label.feedback_target}
                </p>
                <ul className="result-list">
                  {label.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No governed labels loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Agent Run Logs</h2>
        {agentRunsQuery.data?.runs.length ? (
          <ol className="audit-timeline">
            {agentRunsQuery.data.runs.map((run) => (
              <li key={run.agent_run_id}>
                <div>
                  <strong>{run.agent_run_id}</strong>
                  <span>{run.status}</span>
                </div>
                <small>{run.completed_at || run.created_at || run.claim_id}</small>
                <p>{run.decision_boundary}</p>
                <p>{run.steps.length} evidence-backed steps</p>
                <p>
                  {run.tool_calls.length} tool calls / {run.tool_results.length} tool results
                </p>
                <p>
                  {run.policy_checks.length} policy checks /{" "}
                  {run.policy_checks.filter((check) => check.decision === "denied").length} denied
                </p>
                <p>
                  {run.context_snapshots.length} context snapshots /{" "}
                  {
                    run.context_snapshots.filter(
                      (snapshot) => snapshot.redaction_status === "pii_masked",
                    ).length
                  }{" "}
                  masked
                </p>
                <p>
                  {run.approvals.length} approvals /{" "}
                  {run.approvals.filter((approval) => approval.decision === "pending").length}{" "}
                  pending
                </p>
                {hasPendingAgentApproval(run) ? (
                  <div className="button-row">
                    <button
                      disabled={agentApprovalMutation.isPending}
                      onClick={() =>
                        agentApprovalMutation.mutate({
                          run,
                          decision: "approved",
                        })
                      }
                      type="button"
                    >
                      Approve Agent Output
                    </button>
                    <button
                      disabled={agentApprovalMutation.isPending}
                      onClick={() =>
                        agentApprovalMutation.mutate({
                          run,
                          decision: "rejected",
                        })
                      }
                      type="button"
                    >
                      Reject Agent Output
                    </button>
                  </div>
                ) : null}
                <ul className="result-list">
                  {run.context_snapshots.map((snapshot) => (
                    <li key={snapshot.snapshot_id}>
                      {snapshot.redaction_status}: {snapshot.checksum}
                    </li>
                  ))}
                </ul>
                <ul className="result-list">
                  {run.approvals.map((approval) => (
                    <li key={approval.approval_id}>
                      {approval.proposed_action}: {approval.decision}
                    </li>
                  ))}
                </ul>
                <ul className="result-list">
                  {run.policy_checks.map((check) => (
                    <li key={check.policy_check_id}>
                      {check.policy_name}: {check.decision}
                    </li>
                  ))}
                </ul>
                <ul className="result-list">
                  {run.tool_calls.map((call) => (
                    <li key={call.tool_call_id}>
                      {call.tool_name}: {call.status}
                    </li>
                  ))}
                </ul>
                <ul className="result-list">
                  {run.evidence_refs.map((reference) => (
                    <li key={reference}>{reference}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty">No agent runs loaded</p>
        )}
      </div>
    </section>
  );
}
