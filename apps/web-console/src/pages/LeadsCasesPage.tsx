import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listCases, listLeads, triageLead } from "../api";

type LeadRecord = {
  lead_id: string;
  run_id?: string;
  claim_id: string;
  member_id?: string;
  provider_id?: string;
  source_system?: string;
  scheme_family: string;
  lead_source?: string;
  status: string;
  disposition: string;
  risk_score: number;
  rag: string;
  reason?: string;
  evidence_refs: string[];
};

type CaseRecord = {
  case_id: string;
  lead_id: string;
  claim_id: string;
  member_id?: string;
  provider_id?: string;
  source_system?: string;
  scheme_family?: string;
  lead_source?: string;
  status: string;
  assignee: string;
  reviewer: string;
  priority: string;
  routing_reason?: string;
  evidence_package?: Record<string, unknown>;
};

type LeadListResponse = {
  leads: LeadRecord[];
};

type CaseListResponse = {
  cases: CaseRecord[];
};

export function buildLeadSummary(leadsData?: LeadListResponse, casesData?: CaseListResponse) {
  const leads = leadsData?.leads ?? [];
  const cases = casesData?.cases ?? [];
  const schemeCounts = leads.reduce<Record<string, number>>((counts, lead) => {
    counts[lead.scheme_family] = (counts[lead.scheme_family] ?? 0) + 1;
    return counts;
  }, {});
  const topScheme =
    Object.entries(schemeCounts).sort((left, right) => right[1] - left[1])[0]?.[0] ?? "none";
  return {
    totalLeads: leads.length,
    pendingTriage: leads.filter((lead) => lead.disposition === "pending_triage").length,
    openCases: cases.length,
    highPriorityCases: cases.filter((item) => item.priority === "high").length,
    topScheme,
  };
}

export function LeadsCasesPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedLeadId, setSelectedLeadId] = useState("");
  const [assignee, setAssignee] = useState("siu-reviewer-1");
  const [reviewer, setReviewer] = useState("medical-reviewer-1");
  const [priority, setPriority] = useState("high");
  const [notes, setNotes] = useState("Open investigation from high-risk FWA lead.");
  const queryClient = useQueryClient();

  const leadsQuery = useQuery({
    queryKey: ["leads", apiKey],
    queryFn: () => listLeads(apiKey) as Promise<LeadListResponse>,
  });
  const casesQuery = useQuery({
    queryKey: ["cases", apiKey],
    queryFn: () => listCases(apiKey) as Promise<CaseListResponse>,
  });
  const selectedLead = useMemo(
    () =>
      leadsQuery.data?.leads.find((lead) => lead.lead_id === selectedLeadId) ??
      leadsQuery.data?.leads[0],
    [leadsQuery.data?.leads, selectedLeadId],
  );
  const summary = buildLeadSummary(leadsQuery.data, casesQuery.data);
  const triageMutation = useMutation({
    mutationFn: () => {
      if (!selectedLead) throw new Error("No lead selected");
      return triageLead(
        selectedLead.lead_id,
        {
          decision: "open_case",
          assignee,
          reviewer,
          priority,
          notes,
        },
        apiKey,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["leads"] });
      queryClient.invalidateQueries({ queryKey: ["cases"] });
    },
  });

  return (
    <section className="ops-grid">
      <div className="panel wide-panel">
        <div className="dashboard-header">
          <div>
            <h2>Leads & Cases</h2>
            <p>Signal to lead to triage case lifecycle</p>
          </div>
          <label>
            API Key
            <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
          </label>
        </div>
        <div className="summary-grid">
          <div>
            <span>Total Leads</span>
            <strong>{summary.totalLeads}</strong>
          </div>
          <div>
            <span>Pending Triage</span>
            <strong>{summary.pendingTriage}</strong>
          </div>
          <div>
            <span>Open Cases</span>
            <strong>{summary.openCases}</strong>
          </div>
          <div>
            <span>High Priority</span>
            <strong>{summary.highPriorityCases}</strong>
          </div>
          <div>
            <span>Top Scheme</span>
            <strong>{summary.topScheme}</strong>
          </div>
        </div>
      </div>

      <div className="panel">
        <h2>Leads</h2>
        {leadsQuery.error ? <pre className="error">{String(leadsQuery.error.message)}</pre> : null}
        <div className="table-list">
          {leadsQuery.data?.leads.map((lead) => (
            <button
              className={lead.lead_id === selectedLead?.lead_id ? "row-button active" : "row-button"}
              key={lead.lead_id}
              onClick={() => setSelectedLeadId(lead.lead_id)}
            >
              <span>{lead.claim_id}</span>
              <strong>{lead.risk_score}</strong>
              <small>{lead.scheme_family}</small>
            </button>
          ))}
        </div>
      </div>

      <div className="panel">
        <h2>Lead Detail</h2>
        {selectedLead ? (
          <div className="result-stack">
            <dl className="result-grid">
              <div>
                <dt>Lead</dt>
                <dd>{selectedLead.lead_id}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{selectedLead.status}</dd>
              </div>
              <div>
                <dt>Disposition</dt>
                <dd>{selectedLead.disposition}</dd>
              </div>
              <div>
                <dt>RAG</dt>
                <dd>{selectedLead.rag}</dd>
              </div>
              <div>
                <dt>Member</dt>
                <dd>{selectedLead.member_id || "-"}</dd>
              </div>
              <div>
                <dt>Provider</dt>
                <dd>{selectedLead.provider_id || "-"}</dd>
              </div>
            </dl>
            <div className="form-grid">
              <label>
                Assignee
                <input value={assignee} onChange={(event) => setAssignee(event.target.value)} />
              </label>
              <label>
                Reviewer
                <input value={reviewer} onChange={(event) => setReviewer(event.target.value)} />
              </label>
              <label>
                Priority
                <select value={priority} onChange={(event) => setPriority(event.target.value)}>
                  <option value="high">high</option>
                  <option value="medium">medium</option>
                  <option value="low">low</option>
                </select>
              </label>
            </div>
            <label>
              Notes
              <input value={notes} onChange={(event) => setNotes(event.target.value)} />
            </label>
            <button onClick={() => triageMutation.mutate()} disabled={triageMutation.isPending}>
              Open Case
            </button>
            {triageMutation.error ? (
              <pre className="error">{String(triageMutation.error.message)}</pre>
            ) : null}
            {triageMutation.data ? <pre>{JSON.stringify(triageMutation.data, null, 2)}</pre> : null}
            <ul className="result-list">
              {selectedLead.evidence_refs.map((ref) => (
                <li key={ref}>{ref}</li>
              ))}
            </ul>
          </div>
        ) : (
          <p className="empty">No leads available</p>
        )}
      </div>

      <div className="panel wide-panel">
        <h2>Cases</h2>
        {casesQuery.error ? <pre className="error">{String(casesQuery.error.message)}</pre> : null}
        <div className="case-grid">
          {casesQuery.data?.cases.map((item) => (
            <div className="factor-card" key={item.case_id}>
              <div>
                <strong>{item.case_id}</strong>
                <small>{item.claim_id}</small>
              </div>
              <dl className="result-grid">
                <div>
                  <dt>Status</dt>
                  <dd>{item.status}</dd>
                </div>
                <div>
                  <dt>Priority</dt>
                  <dd>{item.priority}</dd>
                </div>
                <div>
                  <dt>Assignee</dt>
                  <dd>{item.assignee}</dd>
                </div>
                <div>
                  <dt>Reviewer</dt>
                  <dd>{item.reviewer}</dd>
                </div>
              </dl>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
