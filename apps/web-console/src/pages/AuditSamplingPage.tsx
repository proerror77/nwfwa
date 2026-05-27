import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createAuditSample, listAuditSamples } from "../api";

type AuditSampleLead = {
  lead_id: string;
  claim_id: string;
  scheme_family: string;
  risk_score: number;
  rag: string;
  evidence_refs: string[];
};

type AuditSampleRecord = {
  sample_id: string;
  sample_mode: string;
  population_definition: string;
  inclusion_criteria?: Record<string, unknown>;
  deterministic_seed?: string | null;
  selection_method: string;
  sample_size: number;
  reviewer: string;
  assignment_queue: string;
  selected_leads: AuditSampleLead[];
  outcome_distribution: Record<string, unknown>;
  created_at?: string | null;
};

type AuditSampleListResponse = {
  samples: AuditSampleRecord[];
};

export function buildAuditSamplingSummary(data?: AuditSampleListResponse) {
  const samples = data?.samples ?? [];
  const modeCounts = samples.reduce<Record<string, number>>((counts, sample) => {
    counts[sample.sample_mode] = (counts[sample.sample_mode] ?? 0) + 1;
    return counts;
  }, {});
  const topSampleMode =
    Object.entries(modeCounts).sort((left, right) => right[1] - left[1])[0]?.[0] ?? "none";
  const latestSample = [...samples].sort((left, right) =>
    String(right.created_at ?? "").localeCompare(String(left.created_at ?? "")),
  )[0];

  return {
    totalSamples: samples.length,
    selectedLeadCount: samples.reduce(
      (total, sample) => total + sample.selected_leads.length,
      0,
    ),
    requestedSampleSize: samples.reduce((total, sample) => total + sample.sample_size, 0),
    topSampleMode,
    latestAssignmentQueue: latestSample?.assignment_queue ?? "none",
  };
}

export function AuditSamplingPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [sampleMode, setSampleMode] = useState("risk_ranked");
  const [populationDefinition, setPopulationDefinition] = useState(
    "RED and high risk leads for weekly QA",
  );
  const [minRiskScore, setMinRiskScore] = useState("70");
  const [deterministicSeed, setDeterministicSeed] = useState("pilot-week-1");
  const [sampleSize, setSampleSize] = useState("5");
  const [reviewer, setReviewer] = useState("qa-reviewer-1");
  const [assignmentQueue, setAssignmentQueue] = useState("QA Review");
  const queryClient = useQueryClient();

  const samplesQuery = useQuery({
    queryKey: ["audit-samples", apiKey],
    queryFn: () => listAuditSamples(apiKey) as Promise<AuditSampleListResponse>,
  });
  const summary = buildAuditSamplingSummary(samplesQuery.data);
  const createMutation = useMutation({
    mutationFn: () =>
      createAuditSample(
        {
          sample_mode: sampleMode,
          population_definition: populationDefinition,
          inclusion_criteria: {
            min_risk_score: Number(minRiskScore),
          },
          deterministic_seed: deterministicSeed,
          sample_size: Number(sampleSize),
          reviewer,
          assignment_queue: assignmentQueue,
        },
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["audit-samples"] });
    },
  });

  return (
    <section className="ops-grid">
      <div className="panel wide-panel">
        <div className="dashboard-header">
          <div>
            <h2>Audit Sampling</h2>
            <p>Risk-ranked, control, stratified, post-payment, and QA calibration samples</p>
          </div>
          <label>
            API Key
            <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
          </label>
        </div>
        <div className="summary-grid">
          <div>
            <span>Total Samples</span>
            <strong>{summary.totalSamples}</strong>
          </div>
          <div>
            <span>Selected Leads</span>
            <strong>{summary.selectedLeadCount}</strong>
          </div>
          <div>
            <span>Requested Size</span>
            <strong>{summary.requestedSampleSize}</strong>
          </div>
          <div>
            <span>Top Mode</span>
            <strong>{summary.topSampleMode}</strong>
          </div>
          <div>
            <span>Latest Queue</span>
            <strong>{summary.latestAssignmentQueue}</strong>
          </div>
        </div>
      </div>

      <div className="panel">
        <h2>Create Sample</h2>
        <div className="result-stack">
          <label>
            Sample Mode
            <select value={sampleMode} onChange={(event) => setSampleMode(event.target.value)}>
              <option value="risk_ranked">risk_ranked</option>
              <option value="random_control">random_control</option>
              <option value="stratified">stratified</option>
              <option value="post_payment_audit">post_payment_audit</option>
              <option value="qa_calibration">qa_calibration</option>
            </select>
          </label>
          <label>
            Population Definition
            <input
              value={populationDefinition}
              onChange={(event) => setPopulationDefinition(event.target.value)}
            />
          </label>
          <div className="form-grid">
            <label>
              Min Risk Score
              <input
                type="number"
                value={minRiskScore}
                onChange={(event) => setMinRiskScore(event.target.value)}
              />
            </label>
            <label>
              Sample Size
              <input
                type="number"
                min="1"
                value={sampleSize}
                onChange={(event) => setSampleSize(event.target.value)}
              />
            </label>
            <label>
              Deterministic Seed
              <input
                value={deterministicSeed}
                onChange={(event) => setDeterministicSeed(event.target.value)}
              />
            </label>
          </div>
          <div className="form-grid">
            <label>
              Reviewer
              <input value={reviewer} onChange={(event) => setReviewer(event.target.value)} />
            </label>
            <label>
              Assignment Queue
              <input
                value={assignmentQueue}
                onChange={(event) => setAssignmentQueue(event.target.value)}
              />
            </label>
          </div>
          <button onClick={() => createMutation.mutate()} disabled={createMutation.isPending}>
            Create Audit Sample
          </button>
          {createMutation.error ? (
            <pre className="error">{String(createMutation.error.message)}</pre>
          ) : null}
          {createMutation.data ? <pre>{JSON.stringify(createMutation.data, null, 2)}</pre> : null}
        </div>
      </div>

      <div className="panel">
        <h2>Sample Runs</h2>
        {samplesQuery.error ? (
          <pre className="error">{String(samplesQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {samplesQuery.data?.samples.map((sample) => (
            <div className="row-button" key={sample.sample_id}>
              <span>{sample.sample_id}</span>
              <strong>{sample.sample_size}</strong>
              <small>{sample.sample_mode}</small>
              <small>{sample.assignment_queue}</small>
            </div>
          ))}
        </div>
      </div>

      <div className="panel wide-panel">
        <h2>Selected Leads</h2>
        <div className="case-grid">
          {samplesQuery.data?.samples.flatMap((sample) =>
            sample.selected_leads.map((lead) => (
              <div className="factor-card" key={`${sample.sample_id}-${lead.lead_id}`}>
                <div>
                  <strong>{lead.claim_id}</strong>
                  <small>{sample.sample_id}</small>
                </div>
                <dl className="result-grid">
                  <div>
                    <dt>Scheme</dt>
                    <dd>{lead.scheme_family}</dd>
                  </div>
                  <div>
                    <dt>Risk</dt>
                    <dd>{lead.risk_score}</dd>
                  </div>
                  <div>
                    <dt>RAG</dt>
                    <dd>{lead.rag}</dd>
                  </div>
                  <div>
                    <dt>Evidence</dt>
                    <dd>{lead.evidence_refs.length}</dd>
                  </div>
                </dl>
              </div>
            )),
          )}
        </div>
      </div>
    </section>
  );
}
