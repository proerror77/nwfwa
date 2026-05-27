import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getClaimAuditHistory, listAgentRuns } from "../api";

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
    approvalCount: approvals.length,
    pendingApprovalCount: approvals.filter((approval) => approval.decision === "pending").length,
  };
}

export function GovernancePage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [claimId, setClaimId] = useState("CLM-0287");
  const auditQuery = useQuery({
    queryKey: ["claim-audit-history", apiKey, claimId],
    queryFn: () => getClaimAuditHistory(claimId, apiKey) as Promise<ClaimAuditHistoryResponse>,
    enabled: claimId.trim().length > 0,
  });
  const agentRunsQuery = useQuery({
    queryKey: ["agent-run-logs", apiKey],
    queryFn: () => listAgentRuns(apiKey) as Promise<AgentRunLogListResponse>,
  });
  const summary = buildAuditSummary(auditQuery.data);
  const agentSummary = buildAgentRunLogSummary(agentRunsQuery.data?.runs);

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
            <span>Contexts</span>
            <strong>{agentSummary.contextSnapshotCount}</strong>
          </div>
          <div>
            <span>Approvals</span>
            <strong>{agentSummary.pendingApprovalCount}</strong>
          </div>
        </div>
        {auditQuery.error ? <pre className="error">{String(auditQuery.error.message)}</pre> : null}
        {agentRunsQuery.error ? (
          <pre className="error">{String(agentRunsQuery.error.message)}</pre>
        ) : null}
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
