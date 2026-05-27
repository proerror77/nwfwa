import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getClaimAuditHistory } from "../api";

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

export function buildAuditSummary(data?: ClaimAuditHistoryResponse) {
  const events = data?.events ?? [];
  return {
    totalEvents: events.length,
    succeededEvents: events.filter((event) => event.event_status === "succeeded").length,
    failedEvents: events.filter((event) => event.event_status === "failed").length,
    latestEventType: events.at(-1)?.event_type ?? "none",
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
  const summary = buildAuditSummary(auditQuery.data);

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
        </div>
        {auditQuery.error ? <pre className="error">{String(auditQuery.error.message)}</pre> : null}
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
    </section>
  );
}
