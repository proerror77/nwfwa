import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createAuditSample, listAuditSamples, listFwaSchemes } from "../api";
import {
  buildFwaSchemeLabelMap,
  formatFwaSchemeLabel,
  type FwaSchemeDefinition,
} from "./fwaSchemeOptions";

type AuditSampleLead = {
  lead_id: string;
  claim_id: string;
  scheme_family: string;
  review_mode?: string;
  provider_id?: string;
  provider_type?: string;
  provider_region?: string;
  policy_type?: string;
  risk_band?: string;
  strata_key?: string;
  prior_reviewer_sample_count?: number;
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

type AuditSampleRequestForm = {
  sampleMode: string;
  populationDefinition: string;
  minRiskScore: string;
  reviewMode: string;
  providerType: string;
  providerRegion: string;
  policyType: string;
  riskBand: string;
  deterministicSeed: string;
  sampleSize: string;
  reviewer: string;
  assignmentQueue: string;
};

function outcomeCount(sample: AuditSampleRecord, key: "reviewed_count" | "open_count") {
  const value = sample.outcome_distribution[key];
  return typeof value === "number" ? value : 0;
}

function topDistributionKey(distribution: unknown) {
  if (!distribution || typeof distribution !== "object" || Array.isArray(distribution)) {
    return "none";
  }
  return (
    Object.entries(distribution as Record<string, unknown>)
      .filter((entry): entry is [string, number] => typeof entry[1] === "number")
      .sort((left, right) => right[1] - left[1])[0]?.[0] ?? "none"
  );
}

function baselineMeasurement(sample: AuditSampleRecord) {
  const measurement = sample.outcome_distribution.baseline_measurement;
  if (!measurement || typeof measurement !== "object" || Array.isArray(measurement)) {
    return undefined;
  }
  return measurement as Record<string, unknown>;
}

function baselineMeasurementCount(
  sample: AuditSampleRecord,
  key: "missed_risk_review_targets" | "false_positive_review_targets",
) {
  const value = baselineMeasurement(sample)?.[key];
  return typeof value === "number" ? value : 0;
}

function addOptionalCriterion(
  criteria: Record<string, string | number>,
  key: string,
  value: string,
) {
  const trimmed = value.trim();
  if (trimmed) {
    criteria[key] = trimmed;
  }
}

export function buildAuditSampleRequest(form: AuditSampleRequestForm) {
  const inclusionCriteria: Record<string, string | number> = {
    min_risk_score: Number(form.minRiskScore),
  };
  addOptionalCriterion(inclusionCriteria, "review_mode", form.reviewMode);
  addOptionalCriterion(inclusionCriteria, "provider_type", form.providerType);
  addOptionalCriterion(inclusionCriteria, "provider_region", form.providerRegion);
  addOptionalCriterion(inclusionCriteria, "policy_type", form.policyType);
  addOptionalCriterion(inclusionCriteria, "risk_band", form.riskBand);

  return {
    sample_mode: form.sampleMode,
    population_definition: form.populationDefinition,
    inclusion_criteria: inclusionCriteria,
    deterministic_seed: form.deterministicSeed,
    sample_size: Number(form.sampleSize),
    reviewer: form.reviewer,
    assignment_queue: form.assignmentQueue,
  };
}

export function buildAuditSampleLeadDetailRows(lead: AuditSampleLead) {
  return [
    ["Review Mode", lead.review_mode ?? "unknown"],
    ["Provider", `${lead.provider_type ?? "unknown"} / ${lead.provider_region ?? "unknown"}`],
    ["Policy Type", lead.policy_type ?? "unknown"],
    ["Risk Band", lead.risk_band ?? "unknown"],
    ["Prior Reviewer Samples", String(lead.prior_reviewer_sample_count ?? 0)],
    ["Strata", lead.strata_key ?? "unknown"],
  ];
}

function formatInclusionCriteria(criteria?: Record<string, unknown>) {
  if (!criteria) {
    return "none";
  }
  const entries = Object.entries(criteria)
    .filter(([, value]) => value !== null && value !== undefined && String(value).trim() !== "")
    .sort(([left], [right]) => left.localeCompare(right));
  if (entries.length === 0) {
    return "none";
  }
  return entries.map(([key, value]) => `${key}=${String(value)}`).join(", ");
}

export function buildAuditSampleRunDetailRows(sample: AuditSampleRecord) {
  return [
    ["Population", sample.population_definition],
    ["Criteria", formatInclusionCriteria(sample.inclusion_criteria)],
    ["Selection", sample.selection_method],
    ["Seed", sample.deterministic_seed ?? "none"],
    ["Reviewer", sample.reviewer],
  ];
}

export function buildAuditSampleCreateSummary(sample?: AuditSampleRecord | null) {
  if (!sample) {
    return null;
  }
  return {
    sampleId: sample.sample_id,
    sampleMode: sample.sample_mode,
    populationDefinition: sample.population_definition,
    criteriaLabel: formatInclusionCriteria(sample.inclusion_criteria),
    selectionMethod: sample.selection_method,
    seed: sample.deterministic_seed ?? "none",
    reviewer: sample.reviewer,
    assignmentQueue: sample.assignment_queue,
    requestedSize: sample.sample_size,
    selectedLeadCount: sample.selected_leads.length,
    reviewedCount: outcomeCount(sample, "reviewed_count"),
    openCount: outcomeCount(sample, "open_count"),
    topQaConclusion: topDistributionKey(sample.outcome_distribution.qa_conclusions),
  };
}

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
    reviewedCaseCount: samples.reduce(
      (total, sample) => total + outcomeCount(sample, "reviewed_count"),
      0,
    ),
    openCaseCount: samples.reduce(
      (total, sample) => total + outcomeCount(sample, "open_count"),
      0,
    ),
    requestedSampleSize: samples.reduce((total, sample) => total + sample.sample_size, 0),
    controlCohortCount: samples.filter((sample) => baselineMeasurement(sample)?.control_cohort)
      .length,
    missedRiskReviewTargets: samples.reduce(
      (total, sample) => total + baselineMeasurementCount(sample, "missed_risk_review_targets"),
      0,
    ),
    falsePositiveReviewTargets: samples.reduce(
      (total, sample) =>
        total + baselineMeasurementCount(sample, "false_positive_review_targets"),
      0,
    ),
    topSampleMode,
    topQaConclusion: topDistributionKey(
      samples.reduce<Record<string, number>>((counts, sample) => {
        const conclusions = sample.outcome_distribution.qa_conclusions;
        if (!conclusions || typeof conclusions !== "object" || Array.isArray(conclusions)) {
          return counts;
        }
        for (const [key, value] of Object.entries(conclusions)) {
          if (typeof value === "number") {
            counts[key] = (counts[key] ?? 0) + value;
          }
        }
        return counts;
      }, {}),
    ),
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
  const [reviewMode, setReviewMode] = useState("");
  const [providerType, setProviderType] = useState("");
  const [providerRegion, setProviderRegion] = useState("");
  const [policyType, setPolicyType] = useState("");
  const [riskBand, setRiskBand] = useState("");
  const [deterministicSeed, setDeterministicSeed] = useState("pilot-week-1");
  const [sampleSize, setSampleSize] = useState("5");
  const [reviewer, setReviewer] = useState("qa-reviewer-1");
  const [assignmentQueue, setAssignmentQueue] = useState("QA Review");
  const queryClient = useQueryClient();

  const samplesQuery = useQuery({
    queryKey: ["audit-samples", apiKey],
    queryFn: () => listAuditSamples(apiKey) as Promise<AuditSampleListResponse>,
  });
  const schemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<{ schemes: FwaSchemeDefinition[] }>,
  });
  const schemeLabelMap = buildFwaSchemeLabelMap(schemesQuery.data?.schemes);
  const summary = buildAuditSamplingSummary(samplesQuery.data);
  const createMutation = useMutation({
    mutationFn: () =>
      createAuditSample(
        buildAuditSampleRequest({
          sampleMode,
          populationDefinition,
          minRiskScore,
          reviewMode,
          providerType,
          providerRegion,
          policyType,
          riskBand,
          deterministicSeed,
          sampleSize,
          reviewer,
          assignmentQueue,
        }),
        apiKey,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["audit-samples"] });
    },
  });
  const createdSampleSummary = buildAuditSampleCreateSummary(
    createMutation.data as AuditSampleRecord | undefined,
  );

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
            <span>Reviewed</span>
            <strong>{summary.reviewedCaseCount}</strong>
          </div>
          <div>
            <span>Open QA</span>
            <strong>{summary.openCaseCount}</strong>
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
            <span>Control Cohorts</span>
            <strong>{summary.controlCohortCount}</strong>
          </div>
          <div>
            <span>Missed Risk Targets</span>
            <strong>{summary.missedRiskReviewTargets}</strong>
          </div>
          <div>
            <span>False Positive Targets</span>
            <strong>{summary.falsePositiveReviewTargets}</strong>
          </div>
          <div>
            <span>Latest Queue</span>
            <strong>{summary.latestAssignmentQueue}</strong>
          </div>
          <div>
            <span>Top QA Conclusion</span>
            <strong>{summary.topQaConclusion}</strong>
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
              Review Mode
              <select value={reviewMode} onChange={(event) => setReviewMode(event.target.value)}>
                <option value="">any</option>
                <option value="pre_payment">pre_payment</option>
                <option value="post_payment">post_payment</option>
                <option value="both">both</option>
              </select>
            </label>
            <label>
              Risk Band
              <select value={riskBand} onChange={(event) => setRiskBand(event.target.value)}>
                <option value="">any</option>
                <option value="low">low</option>
                <option value="medium">medium</option>
                <option value="high">high</option>
                <option value="critical">critical</option>
              </select>
            </label>
          </div>
          <div className="form-grid">
            <label>
              Provider Type
              <input
                value={providerType}
                onChange={(event) => setProviderType(event.target.value)}
              />
            </label>
            <label>
              Provider Region
              <input
                value={providerRegion}
                onChange={(event) => setProviderRegion(event.target.value)}
              />
            </label>
            <label>
              Policy Type
              <input value={policyType} onChange={(event) => setPolicyType(event.target.value)} />
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
          {createdSampleSummary ? (
            <dl className="result-grid">
              <div>
                <dt>Sample</dt>
                <dd>{createdSampleSummary.sampleId}</dd>
              </div>
              <div>
                <dt>Mode</dt>
                <dd>{createdSampleSummary.sampleMode}</dd>
              </div>
              <div>
                <dt>Population</dt>
                <dd>{createdSampleSummary.populationDefinition}</dd>
              </div>
              <div>
                <dt>Criteria</dt>
                <dd>{createdSampleSummary.criteriaLabel}</dd>
              </div>
              <div>
                <dt>Selection</dt>
                <dd>{createdSampleSummary.selectionMethod}</dd>
              </div>
              <div>
                <dt>Seed</dt>
                <dd>{createdSampleSummary.seed}</dd>
              </div>
              <div>
                <dt>Reviewer</dt>
                <dd>{createdSampleSummary.reviewer}</dd>
              </div>
              <div>
                <dt>Queue</dt>
                <dd>{createdSampleSummary.assignmentQueue}</dd>
              </div>
              <div>
                <dt>Requested</dt>
                <dd>{createdSampleSummary.requestedSize}</dd>
              </div>
              <div>
                <dt>Selected Leads</dt>
                <dd>{createdSampleSummary.selectedLeadCount}</dd>
              </div>
              <div>
                <dt>Reviewed</dt>
                <dd>{createdSampleSummary.reviewedCount}</dd>
              </div>
              <div>
                <dt>Open QA</dt>
                <dd>{createdSampleSummary.openCount}</dd>
              </div>
              <div>
                <dt>Top QA Conclusion</dt>
                <dd>{createdSampleSummary.topQaConclusion}</dd>
              </div>
            </dl>
          ) : null}
        </div>
      </div>

      <div className="panel">
        <h2>Sample Runs</h2>
        {samplesQuery.error ? (
          <pre className="error">{String(samplesQuery.error.message)}</pre>
        ) : null}
        {schemesQuery.error ? (
          <pre className="error">{String(schemesQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {samplesQuery.data?.samples.map((sample) => {
            const topQaConclusion = topDistributionKey(sample.outcome_distribution.qa_conclusions);
            return (
              <div className="row-button" key={sample.sample_id}>
                <span>{sample.sample_id}</span>
                <strong>{sample.sample_size}</strong>
                <small>{sample.sample_mode}</small>
                <small>{sample.assignment_queue}</small>
                <small>
                  reviewed {outcomeCount(sample, "reviewed_count")} / open{" "}
                  {outcomeCount(sample, "open_count")}
                </small>
                <small>{topQaConclusion}</small>
                {buildAuditSampleRunDetailRows(sample).map(([label, value]) => (
                  <small key={label}>
                    {label}: {value}
                  </small>
                ))}
              </div>
            );
          })}
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
                    <dd>{formatFwaSchemeLabel(lead.scheme_family, schemeLabelMap)}</dd>
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
                  {buildAuditSampleLeadDetailRows(lead).map(([label, value]) => (
                    <div key={label}>
                      <dt>{label}</dt>
                      <dd>{value}</dd>
                    </div>
                  ))}
                </dl>
              </div>
            )),
          )}
        </div>
      </div>
    </section>
  );
}
