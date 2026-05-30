import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  listCases,
  listFwaSchemes,
  listLeads,
  submitInvestigationResult,
  triageLead,
  updateCaseStatus,
} from "../api";
import {
  buildFwaSchemeLabelMap,
  formatFwaSchemeLabel,
  type FwaSchemeDefinition,
} from "./fwaSchemeOptions";

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
  sla_target_hours?: number;
  sla_status?: string;
  time_to_triage_hours?: number;
  time_to_closure_hours?: number | null;
};

type EvidenceSufficiency = {
  scheme_family: string;
  status: string;
  minimum_evidence: string[];
  present_evidence: string[];
  missing_evidence: string[];
};

type LeadListResponse = {
  leads: LeadRecord[];
};

type CaseListResponse = {
  cases: CaseRecord[];
};

type TriageLeadResponse = {
  lead: LeadRecord;
  case?: CaseRecord | null;
  audit_id: string;
};

type UpdateCaseStatusResponse = {
  case: CaseRecord;
  audit_id: string;
};

type InvestigationResultDraft = {
  investigationId: string;
  outcome: string;
  confirmedFwa: boolean;
  financialImpactType: string;
  savingAmount: string;
  currency: string;
  notes: string;
  evidenceRefsText: string;
};

type PilotWritebackResponse = {
  claim_id: string;
  event_type: string;
  event_status: string;
  audit_id: string;
  run_id: string;
  evidence_refs: string[];
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
  const casesMissingEvidence = cases.filter(
    (item) => caseEvidenceSufficiencyFromPackage(item.evidence_package)?.missing_evidence.length,
  ).length;
  const breachedCases = cases.filter((item) =>
    ["breached", "closed_breached"].includes(item.sla_status ?? ""),
  ).length;
  const onTrackCases = cases.filter((item) => item.sla_status === "on_track").length;
  const closedCases = cases.filter((item) =>
    ["confirmed", "rejected", "closed"].includes(item.status),
  ).length;
  return {
    totalLeads: leads.length,
    pendingTriage: leads.filter((lead) => lead.disposition === "pending_triage").length,
    openCaseLeads: leads.filter((lead) => lead.disposition === "open_case").length,
    evidenceBackedLeads: leads.filter((lead) => lead.evidence_refs.length > 0).length,
    requestEvidenceLeads: leads.filter((lead) => lead.disposition === "request_evidence").length,
    closedLeads: leads.filter((lead) =>
      ["rejected", "merged", "closed"].includes(lead.disposition),
    ).length,
    openCases: cases.length,
    casesMissingEvidence,
    breachedCases,
    onTrackCases,
    closedCases,
    highPriorityCases: cases.filter((item) => item.priority === "high").length,
    topScheme,
  };
}

export function caseEvidenceSufficiencyFromPackage(
  evidencePackage?: Record<string, unknown>,
): EvidenceSufficiency | null {
  const sufficiency = evidencePackage?.evidence_sufficiency;
  if (!isRecord(sufficiency)) return null;
  if (
    typeof sufficiency.scheme_family !== "string" ||
    typeof sufficiency.status !== "string" ||
    !isStringArray(sufficiency.minimum_evidence) ||
    !isStringArray(sufficiency.present_evidence) ||
    !isStringArray(sufficiency.missing_evidence)
  ) {
    return null;
  }
  return {
    scheme_family: sufficiency.scheme_family,
    status: sufficiency.status,
    minimum_evidence: sufficiency.minimum_evidence,
    present_evidence: sufficiency.present_evidence,
    missing_evidence: sufficiency.missing_evidence,
  };
}

export function buildCaseEvidenceSufficiencyRows(
  sufficiency?: EvidenceSufficiency | null,
) {
  const present = new Set(sufficiency?.present_evidence ?? []);
  return (sufficiency?.minimum_evidence ?? []).map((item) => ({
    item,
    status: present.has(item) ? "present" : "missing",
  }));
}

export function caseEvidenceRefsFromPackage(
  evidencePackage?: Record<string, unknown>,
) {
  return isStringArray(evidencePackage?.evidence_refs) ? evidencePackage.evidence_refs : [];
}

export function caseRoutingReason(item: CaseRecord) {
  const packageReason = item.evidence_package?.reason;
  return item.routing_reason || (typeof packageReason === "string" ? packageReason : "");
}

export function buildLeadTriageSummary(response?: TriageLeadResponse | null) {
  if (!response) {
    return null;
  }
  return {
    auditId: response.audit_id,
    leadId: response.lead.lead_id,
    claimId: response.lead.claim_id,
    disposition: response.lead.disposition,
    status: response.lead.status,
    riskScore: response.lead.risk_score,
    rag: response.lead.rag,
    evidenceCount: response.lead.evidence_refs.length,
    caseId: response.case?.case_id ?? "none",
    caseStatus: response.case?.status ?? "not_opened",
    casePriority: response.case?.priority ?? "none",
  };
}

export function buildCaseStatusUpdateSummary(response?: UpdateCaseStatusResponse | null) {
  if (!response) {
    return null;
  }
  const evidenceRefs = caseEvidenceRefsFromPackage(response.case.evidence_package);
  return {
    auditId: response.audit_id,
    caseId: response.case.case_id,
    claimId: response.case.claim_id,
    status: response.case.status,
    priority: response.case.priority,
    assignee: response.case.assignee,
    reviewer: response.case.reviewer,
    slaStatus: response.case.sla_status ?? "not_available",
    slaTargetHours: response.case.sla_target_hours ?? "not_available",
    timeToTriageHours:
      response.case.time_to_triage_hours == null
        ? "not_available"
        : response.case.time_to_triage_hours.toFixed(1),
    timeToClosureHours:
      response.case.time_to_closure_hours == null
        ? "open"
        : response.case.time_to_closure_hours.toFixed(1),
    evidenceCount: evidenceRefs.length,
  };
}

export function buildInvestigationResultPayload(
  item: CaseRecord,
  draft: InvestigationResultDraft,
) {
  const caseEvidenceRefs = caseEvidenceRefsFromPackage(item.evidence_package);
  const evidenceRefs = [
    `investigation_cases:${item.case_id}`,
    ...caseEvidenceRefs,
    ...draft.evidenceRefsText
      .split(/\n|,/)
      .map((value) => value.trim())
      .filter(Boolean),
  ].filter((value, index, refs) => refs.indexOf(value) === index);
  return {
    claim_id: item.claim_id,
    investigation_id: draft.investigationId.trim(),
    outcome: draft.outcome,
    confirmed_fwa: draft.confirmedFwa,
    financial_impact_type: draft.financialImpactType || undefined,
    saving_amount: draft.savingAmount || undefined,
    currency: draft.currency || undefined,
    notes: draft.notes,
    evidence_refs: evidenceRefs,
  };
}

export function buildInvestigationWritebackSummary(response?: PilotWritebackResponse | null) {
  if (!response) {
    return null;
  }
  return {
    claimId: response.claim_id,
    eventType: response.event_type,
    eventStatus: response.event_status,
    auditId: response.audit_id,
    runId: response.run_id,
    evidenceCount: response.evidence_refs.length,
    evidenceRefs: response.evidence_refs,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

export function LeadsCasesPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedLeadId, setSelectedLeadId] = useState("");
  const [triageDecision, setTriageDecision] = useState("open_case");
  const [mergeTargetLeadId, setMergeTargetLeadId] = useState("");
  const [assignee, setAssignee] = useState("siu-reviewer-1");
  const [reviewer, setReviewer] = useState("medical-reviewer-1");
  const [priority, setPriority] = useState("high");
  const [notes, setNotes] = useState("Open investigation from high-risk FWA lead.");
  const [selectedCaseId, setSelectedCaseId] = useState("");
  const [caseStatus, setCaseStatus] = useState("investigating");
  const [caseNotes, setCaseNotes] = useState("Advance case workflow after reviewer action.");
  const [investigationId, setInvestigationId] = useState("INV-CLM-0287");
  const [investigationOutcome, setInvestigationOutcome] = useState("confirmed_fwa");
  const [confirmedFwa, setConfirmedFwa] = useState(true);
  const [financialImpactType, setFinancialImpactType] = useState("prevented_payment");
  const [savingAmount, setSavingAmount] = useState("8200.00");
  const [investigationCurrency, setInvestigationCurrency] = useState("CNY");
  const [investigationNotes, setInvestigationNotes] = useState(
    "TPA investigation confirmed over-treatment signals.",
  );
  const [investigationEvidenceRefs, setInvestigationEvidenceRefs] = useState(
    "agent_run:agent_CLM-0287",
  );
  const queryClient = useQueryClient();

  const leadsQuery = useQuery({
    queryKey: ["leads", apiKey],
    queryFn: () => listLeads(apiKey) as Promise<LeadListResponse>,
  });
  const casesQuery = useQuery({
    queryKey: ["cases", apiKey],
    queryFn: () => listCases(apiKey) as Promise<CaseListResponse>,
  });
  const schemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<{ schemes: FwaSchemeDefinition[] }>,
  });
  const schemeLabelMap = buildFwaSchemeLabelMap(schemesQuery.data?.schemes);
  const selectedLead = useMemo(
    () =>
      leadsQuery.data?.leads.find((lead) => lead.lead_id === selectedLeadId) ??
      leadsQuery.data?.leads[0],
    [leadsQuery.data?.leads, selectedLeadId],
  );
  const selectedCase = useMemo(
    () =>
      casesQuery.data?.cases.find((item) => item.case_id === selectedCaseId) ??
      casesQuery.data?.cases[0],
    [casesQuery.data?.cases, selectedCaseId],
  );
  const summary = buildLeadSummary(leadsQuery.data, casesQuery.data);
  const triageMutation = useMutation({
    mutationFn: () => {
      if (!selectedLead) throw new Error("No lead selected");
      return triageLead(
        selectedLead.lead_id,
        {
          decision: triageDecision,
          merge_target_lead_id: mergeTargetLeadId || undefined,
          assignee,
          reviewer,
          priority,
          notes,
        },
        apiKey,
      ) as Promise<TriageLeadResponse>;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["leads"] });
      queryClient.invalidateQueries({ queryKey: ["cases"] });
    },
  });
  const caseStatusMutation = useMutation({
    mutationFn: () => {
      if (!selectedCase) throw new Error("No case selected");
      return updateCaseStatus(
        selectedCase.case_id,
        {
          status: caseStatus,
          actor_id: assignee,
          notes: caseNotes,
          evidence_refs: [`case_workflow:${caseStatus}`],
        },
        apiKey,
      ) as Promise<UpdateCaseStatusResponse>;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cases"] });
      queryClient.invalidateQueries({ queryKey: ["outcome-labels"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard-summary"] });
    },
  });
  const investigationMutation = useMutation({
    mutationFn: () => {
      if (!selectedCase) throw new Error("No case selected");
      return submitInvestigationResult(
        buildInvestigationResultPayload(selectedCase, {
          investigationId,
          outcome: investigationOutcome,
          confirmedFwa,
          financialImpactType,
          savingAmount,
          currency: investigationCurrency,
          notes: investigationNotes,
          evidenceRefsText: investigationEvidenceRefs,
        }),
        apiKey,
      ) as Promise<PilotWritebackResponse>;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cases"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard-summary"] });
      queryClient.invalidateQueries({ queryKey: ["outcome-labels"] });
      queryClient.invalidateQueries({ queryKey: ["rule-audit-events"] });
    },
  });
  const triageSummary = buildLeadTriageSummary(triageMutation.data);
  const caseStatusSummary = buildCaseStatusUpdateSummary(caseStatusMutation.data);
  const investigationSummary = buildInvestigationWritebackSummary(investigationMutation.data);

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
            <span>Open Case Leads</span>
            <strong>{summary.openCaseLeads}</strong>
          </div>
          <div>
            <span>Evidence Backed</span>
            <strong>{summary.evidenceBackedLeads}</strong>
          </div>
          <div>
            <span>Request Evidence</span>
            <strong>{summary.requestEvidenceLeads}</strong>
          </div>
          <div>
            <span>Closed Leads</span>
            <strong>{summary.closedLeads}</strong>
          </div>
          <div>
            <span>Open Cases</span>
            <strong>{summary.openCases}</strong>
          </div>
          <div>
            <span>SLA Breached</span>
            <strong>{summary.breachedCases}</strong>
          </div>
          <div>
            <span>SLA On Track</span>
            <strong>{summary.onTrackCases}</strong>
          </div>
          <div>
            <span>Closed Cases</span>
            <strong>{summary.closedCases}</strong>
          </div>
          <div>
            <span>Missing Evidence Cases</span>
            <strong>{summary.casesMissingEvidence}</strong>
          </div>
          <div>
            <span>High Priority</span>
            <strong>{summary.highPriorityCases}</strong>
          </div>
          <div>
            <span>Top Scheme</span>
            <strong>{formatFwaSchemeLabel(summary.topScheme, schemeLabelMap)}</strong>
          </div>
        </div>
      </div>

      <div className="panel">
        <h2>Leads</h2>
        {leadsQuery.error ? <pre className="error">{String(leadsQuery.error.message)}</pre> : null}
        {schemesQuery.error ? (
          <pre className="error">{String(schemesQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {leadsQuery.data?.leads.map((lead) => (
            <button
              className={lead.lead_id === selectedLead?.lead_id ? "row-button active" : "row-button"}
              key={lead.lead_id}
              onClick={() => setSelectedLeadId(lead.lead_id)}
            >
              <span>{lead.claim_id}</span>
              <strong>{lead.risk_score}</strong>
              <small>{formatFwaSchemeLabel(lead.scheme_family, schemeLabelMap)}</small>
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
                <dt>Source</dt>
                <dd>{selectedLead.lead_source || selectedLead.source_system || "-"}</dd>
              </div>
              <div>
                <dt>Evidence</dt>
                <dd>{selectedLead.evidence_refs.length}</dd>
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
            <p>{selectedLead.reason || "No lead reason provided"}</p>
            <div className="form-grid">
              <label>
                Decision
                <select
                  value={triageDecision}
                  onChange={(event) => setTriageDecision(event.target.value)}
                >
                  <option value="open_case">open_case</option>
                  <option value="request_evidence">request_evidence</option>
                  <option value="reject_lead">reject_lead</option>
                  <option value="merge_lead">merge_lead</option>
                </select>
              </label>
              <label>
                Merge Target Lead
                <input
                  value={mergeTargetLeadId}
                  onChange={(event) => setMergeTargetLeadId(event.target.value)}
                />
              </label>
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
              Submit Triage
            </button>
            {triageMutation.error ? (
              <pre className="error">{String(triageMutation.error.message)}</pre>
            ) : null}
            {triageSummary ? (
              <dl className="result-grid">
                <div>
                  <dt>Triage Audit</dt>
                  <dd>{triageSummary.auditId}</dd>
                </div>
                <div>
                  <dt>Lead</dt>
                  <dd>{triageSummary.leadId}</dd>
                </div>
                <div>
                  <dt>Claim</dt>
                  <dd>{triageSummary.claimId}</dd>
                </div>
                <div>
                  <dt>Disposition</dt>
                  <dd>{triageSummary.disposition}</dd>
                </div>
                <div>
                  <dt>Status</dt>
                  <dd>{triageSummary.status}</dd>
                </div>
                <div>
                  <dt>Risk</dt>
                  <dd>{triageSummary.riskScore}</dd>
                </div>
                <div>
                  <dt>RAG</dt>
                  <dd>{triageSummary.rag}</dd>
                </div>
                <div>
                  <dt>Evidence</dt>
                  <dd>{triageSummary.evidenceCount}</dd>
                </div>
                <div>
                  <dt>Case</dt>
                  <dd>{triageSummary.caseId}</dd>
                </div>
                <div>
                  <dt>Case Status</dt>
                  <dd>{triageSummary.caseStatus}</dd>
                </div>
                <div>
                  <dt>Case Priority</dt>
                  <dd>{triageSummary.casePriority}</dd>
                </div>
              </dl>
            ) : null}
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
        <div className="form-grid">
          <label>
            Case
            <select
              value={selectedCase?.case_id ?? ""}
              onChange={(event) => setSelectedCaseId(event.target.value)}
            >
              {casesQuery.data?.cases.map((item) => (
                <option key={item.case_id} value={item.case_id}>
                  {item.case_id}
                </option>
              ))}
            </select>
          </label>
          <label>
            Status
            <select value={caseStatus} onChange={(event) => setCaseStatus(event.target.value)}>
              <option value="investigating">investigating</option>
              <option value="pending_evidence">pending_evidence</option>
              <option value="confirmed">confirmed</option>
              <option value="rejected">rejected</option>
              <option value="closed">closed</option>
            </select>
          </label>
          <label>
            Status Notes
            <input value={caseNotes} onChange={(event) => setCaseNotes(event.target.value)} />
          </label>
        </div>
        <button onClick={() => caseStatusMutation.mutate()} disabled={caseStatusMutation.isPending}>
          Update Case Status
        </button>
        {caseStatusMutation.error ? (
          <pre className="error">{String(caseStatusMutation.error.message)}</pre>
        ) : null}
        {caseStatusSummary ? (
          <dl className="result-grid">
            <div>
              <dt>Status Audit</dt>
              <dd>{caseStatusSummary.auditId}</dd>
            </div>
            <div>
              <dt>Case</dt>
              <dd>{caseStatusSummary.caseId}</dd>
            </div>
            <div>
              <dt>Claim</dt>
              <dd>{caseStatusSummary.claimId}</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>{caseStatusSummary.status}</dd>
            </div>
            <div>
              <dt>Priority</dt>
              <dd>{caseStatusSummary.priority}</dd>
            </div>
            <div>
              <dt>Assignee</dt>
              <dd>{caseStatusSummary.assignee}</dd>
            </div>
            <div>
              <dt>Reviewer</dt>
              <dd>{caseStatusSummary.reviewer}</dd>
            </div>
            <div>
              <dt>SLA</dt>
              <dd>{caseStatusSummary.slaStatus}</dd>
            </div>
            <div>
              <dt>SLA Target</dt>
              <dd>{caseStatusSummary.slaTargetHours}</dd>
            </div>
            <div>
              <dt>Triage Hours</dt>
              <dd>{caseStatusSummary.timeToTriageHours}</dd>
            </div>
            <div>
              <dt>Closure Hours</dt>
              <dd>{caseStatusSummary.timeToClosureHours}</dd>
            </div>
            <div>
              <dt>Evidence</dt>
              <dd>{caseStatusSummary.evidenceCount}</dd>
            </div>
          </dl>
        ) : null}
        <div className="result-stack">
          <h3>Investigation Result Writeback</h3>
          <div className="form-grid">
            <label>
              Investigation
              <input
                value={investigationId}
                onChange={(event) => setInvestigationId(event.target.value)}
              />
            </label>
            <label>
              Outcome
              <select
                value={investigationOutcome}
                onChange={(event) => setInvestigationOutcome(event.target.value)}
              >
                <option value="confirmed_fwa">confirmed_fwa</option>
                <option value="not_fwa">not_fwa</option>
                <option value="inconclusive">inconclusive</option>
              </select>
            </label>
            <label>
              Confirmed FWA
              <select
                value={confirmedFwa ? "true" : "false"}
                onChange={(event) => setConfirmedFwa(event.target.value === "true")}
              >
                <option value="true">true</option>
                <option value="false">false</option>
              </select>
            </label>
            <label>
              Impact Type
              <select
                value={financialImpactType}
                onChange={(event) => setFinancialImpactType(event.target.value)}
              >
                <option value="prevented_payment">prevented_payment</option>
                <option value="recovered_amount">recovered_amount</option>
                <option value="avoided_future_exposure">avoided_future_exposure</option>
                <option value="deterrence_estimate">deterrence_estimate</option>
                <option value="estimated_impact">estimated_impact</option>
              </select>
            </label>
            <label>
              Saving
              <input value={savingAmount} onChange={(event) => setSavingAmount(event.target.value)} />
            </label>
            <label>
              Currency
              <input
                value={investigationCurrency}
                onChange={(event) => setInvestigationCurrency(event.target.value)}
              />
            </label>
          </div>
          <label>
            Investigation Notes
            <input
              value={investigationNotes}
              onChange={(event) => setInvestigationNotes(event.target.value)}
            />
          </label>
          <label>
            Evidence Refs
            <textarea
              value={investigationEvidenceRefs}
              onChange={(event) => setInvestigationEvidenceRefs(event.target.value)}
            />
          </label>
          <button
            onClick={() => investigationMutation.mutate()}
            disabled={investigationMutation.isPending || !selectedCase}
          >
            Write Back Investigation
          </button>
          {investigationMutation.error ? (
            <pre className="error">{String(investigationMutation.error.message)}</pre>
          ) : null}
          {investigationSummary ? (
            <dl className="result-grid">
              <div>
                <dt>Investigation Audit</dt>
                <dd>{investigationSummary.auditId}</dd>
              </div>
              <div>
                <dt>Claim</dt>
                <dd>{investigationSummary.claimId}</dd>
              </div>
              <div>
                <dt>Event</dt>
                <dd>{investigationSummary.eventType}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{investigationSummary.eventStatus}</dd>
              </div>
              <div>
                <dt>Run</dt>
                <dd>{investigationSummary.runId}</dd>
              </div>
              <div>
                <dt>Evidence</dt>
                <dd>{investigationSummary.evidenceCount}</dd>
              </div>
            </dl>
          ) : null}
        </div>
        <div className="case-grid">
          {casesQuery.data?.cases.map((item) => {
            const evidenceSufficiency = caseEvidenceSufficiencyFromPackage(item.evidence_package);
            const evidenceRows = buildCaseEvidenceSufficiencyRows(evidenceSufficiency);
            const evidenceRefs = caseEvidenceRefsFromPackage(item.evidence_package);
            const routingReason = caseRoutingReason(item);
            return (
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
                    <dt>Scheme</dt>
                    <dd>{formatFwaSchemeLabel(item.scheme_family ?? "-", schemeLabelMap)}</dd>
                  </div>
                  <div>
                    <dt>Evidence Status</dt>
                    <dd>{evidenceSufficiency?.status ?? "not_available"}</dd>
                  </div>
                  <div>
                    <dt>SLA Status</dt>
                    <dd>{item.sla_status ?? "not_available"}</dd>
                  </div>
                  <div>
                    <dt>SLA Target</dt>
                    <dd>{item.sla_target_hours ?? "not_available"}</dd>
                  </div>
                  <div>
                    <dt>Triage Hours</dt>
                    <dd>
                      {item.time_to_triage_hours == null
                        ? "not_available"
                        : item.time_to_triage_hours.toFixed(1)}
                    </dd>
                  </div>
                  <div>
                    <dt>Closure Hours</dt>
                    <dd>
                      {item.time_to_closure_hours == null
                        ? "open"
                        : item.time_to_closure_hours.toFixed(1)}
                    </dd>
                  </div>
                  <div>
                    <dt>Missing Evidence</dt>
                    <dd>{evidenceSufficiency?.missing_evidence.length ?? 0}</dd>
                  </div>
                  <div>
                    <dt>Assignee</dt>
                    <dd>{item.assignee}</dd>
                  </div>
                  <div>
                    <dt>Reviewer</dt>
                    <dd>{item.reviewer}</dd>
                  </div>
                  <div>
                    <dt>Evidence Refs</dt>
                    <dd>{evidenceRefs.length}</dd>
                  </div>
                </dl>
                {routingReason ? <p>{routingReason}</p> : null}
                {evidenceRows.length ? (
                  <ul className="result-list compact-list">
                    {evidenceRows.map((row) => (
                      <li key={row.item}>
                        <strong>{row.item}</strong>
                        <span>{row.status}</span>
                      </li>
                    ))}
                  </ul>
                ) : null}
                {evidenceRefs.length ? (
                  <ul className="result-list compact-list">
                    {evidenceRefs.slice(0, 4).map((reference) => (
                      <li key={reference}>{reference}</li>
                    ))}
                  </ul>
                ) : null}
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
