import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { listFwaSchemes, scoreClaim } from "../api";
import {
  buildProviderProfileInspection,
  type ProviderProfileAssessment,
} from "./providerProfileInspection";
import {
  buildFwaSchemeLabelMap,
  formatFwaSchemeLabel,
  type FwaSchemeDefinition,
} from "./fwaSchemeOptions";
import { formatReviewModeLabel } from "./reviewMode";
import {
  buildClinicalEvidenceInspection,
  buildProviderGraphInspection,
  buildSimilarCaseInspection,
  type ClinicalEvidenceAssessment,
  type ProviderRelationshipGraphAssessment,
  type SimilarCase,
} from "./runtimeEvidence";
import { buildScoringLayerSummary, type ScoringLayer } from "./scoringLayers";

type ScoringResponse = {
  run_id: string;
  audit_id: string;
  claim_id: string;
  review_mode: string;
  risk_score: number;
  rag: string;
  risk_level: string;
  recommended_action: string;
  confidence_score: number;
  confidence: string;
  routing_reason: string;
  routing_policy: RoutingPolicy;
  scores: {
    peer_deviation_score: number;
    rule_score: number;
    anomaly_score: number;
    ml_score: number;
    medical_reasonableness_score: number;
    provider_network_score: number;
    similar_case_score: number;
    final_score: number;
  };
  alerts: Array<{
    alert_code: string;
    severity: string;
    reason: string;
    rule_id: string;
    rule_version: number;
  }>;
  layers: ScoringLayer[];
  top_reasons: string[];
  clinical_evidence?: ClinicalEvidenceAssessment;
  provider_profile?: ProviderProfileAssessment;
  provider_relationships?: ProviderRelationshipGraphAssessment;
  similar_cases: SimilarCase[];
  feature_values: FeatureTraceValue[];
  evidence_refs: unknown[];
};

type FeatureEvidenceRef = {
  entity_type: string;
  entity_id: string;
  field: string;
};

type FeatureTraceValue = {
  name: string;
  version: number;
  value: unknown;
  evidence_refs: FeatureEvidenceRef[];
};

type RoutingPolicy = {
  policy_id: string;
  version: number;
  review_mode: string;
  risk_thresholds: {
    low_max: number;
    medium_min: number;
    high_min: number;
    critical_min: number;
  };
  confidence_thresholds: {
    low_confidence_below: number;
    high_confidence_min: number;
  };
  provider_review_threshold: number;
};

export function buildRoutingPolicySummary(policy?: RoutingPolicy | null) {
  if (!policy) {
    return null;
  }

  return {
    policyLabel: `${policy.policy_id} v${policy.version}`,
    reviewModeLabel: formatReviewModeLabel(policy.review_mode),
    riskThresholdLabel: `Low <= ${policy.risk_thresholds.low_max}, Medium >= ${policy.risk_thresholds.medium_min}, High >= ${policy.risk_thresholds.high_min}, Critical >= ${policy.risk_thresholds.critical_min}`,
    confidenceThresholdLabel: `Low < ${policy.confidence_thresholds.low_confidence_below}, High >= ${policy.confidence_thresholds.high_confidence_min}`,
    providerThresholdLabel: `Provider review >= ${policy.provider_review_threshold}`,
  };
}

export function buildFeatureTraceRows(featureValues: FeatureTraceValue[] = []) {
  return featureValues.map((feature) => ({
    key: `${feature.name}:${feature.version}`,
    name: feature.name,
    versionLabel: `v${feature.version}`,
    valueLabel: formatFeatureValue(feature.value),
    evidenceLabel:
      feature.evidence_refs.length === 0
        ? "No evidence refs"
        : feature.evidence_refs
            .map((evidence) => `${evidence.entity_type}:${evidence.entity_id}.${evidence.field}`)
            .join(", "),
  }));
}

export function buildRuntimeEvidenceRefRows(evidenceRefs: unknown[] = []) {
  return evidenceRefs.map((reference, index) => {
    const label = formatRuntimeEvidenceRef(reference);
    return {
      key: `${index}:${label}`,
      label,
      kind: isFeatureEvidenceRef(reference) ? "feature" : "reference",
    };
  });
}

export function buildTpaEmbeddedPanelSummary(result?: ScoringResponse | null) {
  if (!result) {
    return null;
  }
  return {
    claimId: result.claim_id,
    riskScore: result.risk_score,
    rag: result.rag,
    recommendedAction: result.recommended_action,
    reviewModeLabel: formatReviewModeLabel(result.review_mode),
    confidenceLabel: `${result.confidence} (${result.confidence_score})`,
    alertCount: result.alerts.length,
    topReasonCount: result.top_reasons.length,
    evidenceCount: result.evidence_refs.length,
    auditId: result.audit_id,
  };
}

function formatFeatureValue(value: unknown) {
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (value === null || value === undefined) {
    return "-";
  }
  return JSON.stringify(value);
}

function formatRuntimeEvidenceRef(reference: unknown) {
  if (isFeatureEvidenceRef(reference)) {
    return `${reference.entity_type}:${reference.entity_id}.${reference.field}`;
  }
  if (typeof reference === "string" || typeof reference === "number" || typeof reference === "boolean") {
    return String(reference);
  }
  if (reference === null || reference === undefined) {
    return "-";
  }
  return JSON.stringify(reference);
}

function isFeatureEvidenceRef(reference: unknown): reference is FeatureEvidenceRef {
  if (!reference || typeof reference !== "object") {
    return false;
  }
  const candidate = reference as Partial<FeatureEvidenceRef>;
  return (
    typeof candidate.entity_type === "string" &&
    typeof candidate.entity_id === "string" &&
    typeof candidate.field === "string"
  );
}

const defaultPayload = JSON.stringify(
  {
    source_system: "tpa-demo",
    review_mode: "pre_payment",
    claim: {
      external_claim_id: "CLM-0287",
      claim_amount: "8000",
      currency: "CNY",
      service_date: "2026-01-06",
      diagnosis_code: "J10",
      items: [
        {
          item_code: "IMG-900",
          item_type: "procedure",
          description: "High cost imaging",
          quantity: 1,
          unit_amount: "8000",
          total_amount: "8000",
        },
      ],
      policy: {
        external_policy_id: "POL-0287",
        product_code: "MED",
        coverage_start_date: "2026-01-01",
        coverage_end_date: "2026-12-31",
        coverage_limit: "10000",
        currency: "CNY",
      },
      member: {
        external_member_id: "MBR-0287",
      },
      provider: {
        external_provider_id: "PRV-DEMO",
        name: "Demo Hospital",
        provider_type: "hospital",
        region: "Shanghai",
        risk_tier: "Medium",
      },
      provider_profile: {
        specialty: "imaging",
        network_status: "in_network",
        windows: [
          {
            window_days: 90,
            claim_count: 126,
            total_claim_amount: "420000",
            high_cost_item_ratio: 0.72,
            diagnosis_procedure_mismatch_rate: 0.38,
            peer_amount_percentile: 97,
            peer_frequency_percentile: 96,
            confirmed_fwa_count: 4,
            false_positive_count: 1,
          },
        ],
      },
      provider_relationships: {
        high_risk_neighbor_ratio: 0.34,
        provider_patient_overlap_score: 0.68,
        referral_concentration_score: 0.72,
        connected_confirmed_fwa_count: 2,
        network_component_risk_score: 82,
        evidence_refs: ["relationship_edges:PRV-DEMO"],
      },
    },
  },
  null,
  2,
);

export function buildClaimIdScorePayload(sourceSystem: string, claimId: string, reviewMode: string) {
  return {
    source_system: sourceSystem,
    claim_id: claimId.trim(),
    review_mode: reviewMode,
  };
}

export function RuntimeScoring() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [requestMode, setRequestMode] = useState("full_payload");
  const [sourceSystem, setSourceSystem] = useState("tpa-demo");
  const [claimId, setClaimId] = useState("CLM-0287");
  const [reviewMode, setReviewMode] = useState("pre_payment");
  const [payload, setPayload] = useState(defaultPayload);
  const schemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<{ schemes: FwaSchemeDefinition[] }>,
  });
  const schemeLabelMap = buildFwaSchemeLabelMap(schemesQuery.data?.schemes);
  const mutation = useMutation({
    mutationFn: () =>
      scoreClaim(
        requestMode === "claim_id"
          ? buildClaimIdScorePayload(sourceSystem, claimId, reviewMode)
          : JSON.parse(payload),
        apiKey,
      ) as Promise<ScoringResponse>,
  });
  const result = mutation.data;
  const providerInspection = result
    ? buildProviderProfileInspection(result.provider_profile)
    : null;
  const providerProfile = result?.provider_profile;
  const clinicalInspection = result
    ? buildClinicalEvidenceInspection(result.clinical_evidence)
    : null;
  const providerGraphInspection = result
    ? buildProviderGraphInspection(result.provider_relationships)
    : null;
  const providerGraph = result?.provider_relationships;
  const similarCaseInspection = result
    ? buildSimilarCaseInspection(result.similar_cases, schemeLabelMap)
    : null;
  const layerSummary = result ? buildScoringLayerSummary(result.layers) : null;
  const routingPolicySummary = result
    ? buildRoutingPolicySummary(result.routing_policy)
    : null;
  const featureTraceRows = result ? buildFeatureTraceRows(result.feature_values) : [];
  const evidenceRefRows = result ? buildRuntimeEvidenceRefRows(result.evidence_refs) : [];
  const tpaPanelSummary = buildTpaEmbeddedPanelSummary(result);

  return (
    <section className="runtime">
      <div className="panel">
        <h2>Runtime Scoring</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {schemesQuery.error ? (
          <pre className="error">{String(schemesQuery.error.message)}</pre>
        ) : null}
        <div className="form-grid">
          <label>
            Request Mode
            <select value={requestMode} onChange={(event) => setRequestMode(event.target.value)}>
              <option value="full_payload">Full Payload</option>
              <option value="claim_id">Claim ID</option>
            </select>
          </label>
          <label>
            Review Mode
            <select value={reviewMode} onChange={(event) => setReviewMode(event.target.value)}>
              <option value="pre_payment">Pre Payment</option>
              <option value="post_payment">Post Payment</option>
              <option value="both">Both</option>
            </select>
          </label>
          <label>
            Source System
            <input value={sourceSystem} onChange={(event) => setSourceSystem(event.target.value)} />
          </label>
        </div>
        {requestMode === "claim_id" ? (
          <label>
            Claim ID
            <input value={claimId} onChange={(event) => setClaimId(event.target.value)} />
          </label>
        ) : (
          <label>
            Claim Request JSON
            <textarea value={payload} onChange={(event) => setPayload(event.target.value)} />
          </label>
        )}
        <button onClick={() => mutation.mutate()} disabled={mutation.isPending}>
          Score Claim
        </button>
      </div>
      <div className="panel">
        <h2>Result</h2>
        {mutation.error ? <pre className="error">{String(mutation.error.message)}</pre> : null}
        {result ? (
          <div className="result-stack">
            <div className="score-hero">
              <div>
                <span>Risk Score</span>
                <strong>{result.risk_score}</strong>
              </div>
              <div>
                <span>RAG</span>
                <strong>{result.rag}</strong>
              </div>
              <div>
                <span>Risk Level</span>
                <strong>{result.risk_level}</strong>
              </div>
              <div>
                <span>Action</span>
                <strong>{result.recommended_action}</strong>
              </div>
            </div>
            <dl className="result-grid">
              <div>
                <dt>Claim</dt>
                <dd>{result.claim_id}</dd>
              </div>
              <div>
                <dt>Run</dt>
                <dd>{result.run_id}</dd>
              </div>
              <div>
                <dt>Audit</dt>
                <dd>{result.audit_id}</dd>
              </div>
              <div>
                <dt>Review Mode</dt>
                <dd>{formatReviewModeLabel(result.review_mode)}</dd>
              </div>
              <div>
                <dt>Confidence</dt>
                <dd>
                  {result.confidence} ({result.confidence_score})
                </dd>
              </div>
              <div>
                <dt>Routing</dt>
                <dd>{result.routing_reason}</dd>
              </div>
              <div>
                <dt>Peer Deviation</dt>
                <dd>{result.scores.peer_deviation_score}</dd>
              </div>
              <div>
                <dt>Rule Score</dt>
                <dd>{result.scores.rule_score}</dd>
              </div>
              <div>
                <dt>Anomaly Score</dt>
                <dd>{result.scores.anomaly_score}</dd>
              </div>
              <div>
                <dt>ML Score</dt>
                <dd>{result.scores.ml_score}</dd>
              </div>
              <div>
                <dt>Medical Score</dt>
                <dd>{result.scores.medical_reasonableness_score}</dd>
              </div>
              <div>
                <dt>Provider Score</dt>
                <dd>{result.scores.provider_network_score}</dd>
              </div>
              <div>
                <dt>Similar Case</dt>
                <dd>{result.scores.similar_case_score}</dd>
              </div>
              <div>
                <dt>Final Score</dt>
                <dd>{result.scores.final_score}</dd>
              </div>
            </dl>
            <section>
              <h3>TPA Embedded Panel</h3>
              {tpaPanelSummary ? (
                <dl className="result-grid">
                  <div>
                    <dt>Claim</dt>
                    <dd>{tpaPanelSummary.claimId}</dd>
                  </div>
                  <div>
                    <dt>Risk</dt>
                    <dd>{tpaPanelSummary.riskScore}</dd>
                  </div>
                  <div>
                    <dt>RAG</dt>
                    <dd>{tpaPanelSummary.rag}</dd>
                  </div>
                  <div>
                    <dt>Action</dt>
                    <dd>{tpaPanelSummary.recommendedAction}</dd>
                  </div>
                  <div>
                    <dt>Mode</dt>
                    <dd>{tpaPanelSummary.reviewModeLabel}</dd>
                  </div>
                  <div>
                    <dt>Confidence</dt>
                    <dd>{tpaPanelSummary.confidenceLabel}</dd>
                  </div>
                  <div>
                    <dt>Alerts</dt>
                    <dd>{tpaPanelSummary.alertCount}</dd>
                  </div>
                  <div>
                    <dt>Reasons</dt>
                    <dd>{tpaPanelSummary.topReasonCount}</dd>
                  </div>
                  <div>
                    <dt>Evidence</dt>
                    <dd>{tpaPanelSummary.evidenceCount}</dd>
                  </div>
                  <div>
                    <dt>Audit</dt>
                    <dd>{tpaPanelSummary.auditId}</dd>
                  </div>
                </dl>
              ) : null}
            </section>
            <section>
              <h3>Routing Policy</h3>
              {routingPolicySummary ? (
                <dl className="result-grid">
                  <div>
                    <dt>Policy</dt>
                    <dd>{routingPolicySummary.policyLabel}</dd>
                  </div>
                  <div>
                    <dt>Mode</dt>
                    <dd>{routingPolicySummary.reviewModeLabel}</dd>
                  </div>
                  <div>
                    <dt>Risk Thresholds</dt>
                    <dd>{routingPolicySummary.riskThresholdLabel}</dd>
                  </div>
                  <div>
                    <dt>Confidence Thresholds</dt>
                    <dd>{routingPolicySummary.confidenceThresholdLabel}</dd>
                  </div>
                  <div>
                    <dt>Provider Threshold</dt>
                    <dd>{routingPolicySummary.providerThresholdLabel}</dd>
                  </div>
                </dl>
              ) : (
                <p className="empty">No routing policy</p>
              )}
            </section>
            <section>
              <h3>Seven Layer Detection</h3>
              {layerSummary ? (
                <>
                  <div className="summary-grid">
                    <div>
                      <span>Layers</span>
                      <strong>{layerSummary.layerCount}</strong>
                    </div>
                    <div>
                      <span>Layer Coverage</span>
                      <strong>{layerSummary.coverageLabel}</strong>
                    </div>
                    <div>
                      <span>Expected Layers</span>
                      <strong>{layerSummary.expectedLayerCount}</strong>
                    </div>
                    <div>
                      <span>Active</span>
                      <strong>{layerSummary.activeCount}</strong>
                    </div>
                    <div>
                      <span>Baseline</span>
                      <strong>{layerSummary.baselineCount}</strong>
                    </div>
                    <div>
                      <span>Missing Layers</span>
                      <strong>{layerSummary.missingLayerLabel}</strong>
                    </div>
                    <div>
                      <span>Highest</span>
                      <strong>{layerSummary.highestLayerLabel}</strong>
                    </div>
                  </div>
                  <div className="table-list">
                    {result.layers.map((layer) => (
                      <div className="metric-row compact-metric-row" key={layer.layer_id}>
                        <span>{layer.layer_id}</span>
                        <strong>{layer.score}</strong>
                        <small>{layer.name}</small>
                        <small>
                          {layer.status} · {layer.reason}
                        </small>
                      </div>
                    ))}
                  </div>
                </>
              ) : (
                <p className="empty">No layer data</p>
              )}
            </section>
            <section>
              <h3>Alerts</h3>
              {result.alerts.length > 0 ? (
                <ul className="result-list">
                  {result.alerts.map((alert) => (
                    <li key={`${alert.rule_id}-${alert.rule_version}`}>
                      <strong>{alert.alert_code}</strong>
                      <span>{alert.reason}</span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="empty">No alerts</p>
              )}
            </section>
            <section>
              <h3>Top Reasons</h3>
              <ul className="result-list">
                {result.top_reasons.map((reason) => (
                  <li key={reason}>{reason}</li>
                ))}
              </ul>
            </section>
            <section>
              <h3>Provider Risk Profile</h3>
              {providerInspection && providerProfile ? (
                <div className="factor-card provider-profile-card">
                  <div>
                    <span>Provider</span>
                    <strong>{providerInspection.providerId}</strong>
                  </div>
                  <dl className="result-grid">
                    <div>
                      <dt>Provider Score</dt>
                      <dd>{providerProfile.risk_score}</dd>
                    </div>
                    <div>
                      <dt>Review Route</dt>
                      <dd>{providerInspection.routeLabel}</dd>
                    </div>
                    <div>
                      <dt>Review Status</dt>
                      <dd>{providerInspection.reviewLabel}</dd>
                    </div>
                    <div>
                      <dt>Max Window</dt>
                      <dd>{providerInspection.maxWindowLabel}</dd>
                    </div>
                    <div>
                      <dt>Specialty</dt>
                      <dd>{providerProfile.specialty ?? "Unspecified"}</dd>
                    </div>
                    <div>
                      <dt>Network</dt>
                      <dd>{providerProfile.network_status ?? "Unspecified"}</dd>
                    </div>
                  </dl>
                  <div>
                    <span>Outlier Flags</span>
                    <p>{providerInspection.outlierSummary}</p>
                  </div>
                  {providerProfile.window_findings.length > 0 ? (
                    <ul className="result-list compact-list">
                      {providerProfile.window_findings.map((finding) => (
                        <li key={`${finding.window_days}-${finding.risk_score}`}>
                          <strong>
                            {finding.window_days}d / {finding.risk_score}
                          </strong>
                          <span>{finding.reason}</span>
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p className="empty">No provider profile windows</p>
                  )}
                  <div>
                    <span>Evidence Refs</span>
                    <p>{providerInspection.evidenceSummary}</p>
                  </div>
                </div>
              ) : null}
            </section>
            <section>
              <h3>Clinical Evidence</h3>
              {clinicalInspection && result.clinical_evidence ? (
                <div className="factor-card">
                  <dl className="result-grid">
                    <div>
                      <dt>Status</dt>
                      <dd>{clinicalInspection.statusLabel}</dd>
                    </div>
                    <div>
                      <dt>Review</dt>
                      <dd>{clinicalInspection.reviewLabel}</dd>
                    </div>
                    <div>
                      <dt>Route</dt>
                      <dd>{clinicalInspection.routeLabel}</dd>
                    </div>
                    <div>
                      <dt>Findings</dt>
                      <dd>{clinicalInspection.findingCount}</dd>
                    </div>
                    <div>
                      <dt>First Finding</dt>
                      <dd>{clinicalInspection.firstFindingLabel}</dd>
                    </div>
                    <div>
                      <dt>Missing Evidence</dt>
                      <dd>{clinicalInspection.missingEvidenceSummary}</dd>
                    </div>
                  </dl>
                  <div>
                    <span>Minimum Evidence</span>
                    <p>{clinicalInspection.minimumEvidenceSummary}</p>
                  </div>
                  <div>
                    <span>Evidence Refs</span>
                    <p>{clinicalInspection.evidenceSummary}</p>
                  </div>
                </div>
              ) : (
                <p className="empty">No clinical evidence assessment</p>
              )}
            </section>
            <section>
              <h3>Provider Graph Risk</h3>
              {providerGraphInspection && providerGraph ? (
                <div className="factor-card">
                  <dl className="result-grid">
                    <div>
                      <dt>Provider</dt>
                      <dd>{providerGraphInspection.providerId}</dd>
                    </div>
                    <div>
                      <dt>Risk</dt>
                      <dd>{providerGraphInspection.riskLabel}</dd>
                    </div>
                    <div>
                      <dt>Review</dt>
                      <dd>{providerGraphInspection.reviewLabel}</dd>
                    </div>
                    <div>
                      <dt>Route</dt>
                      <dd>{providerGraphInspection.routeLabel}</dd>
                    </div>
                    <div>
                      <dt>Top Signal</dt>
                      <dd>{providerGraphInspection.topSignalLabel}</dd>
                    </div>
                    <div>
                      <dt>Evidence</dt>
                      <dd>{providerGraphInspection.evidenceSummary}</dd>
                    </div>
                  </dl>
                  <div>
                    <span>Graph Reasons</span>
                    <p>{providerGraphInspection.reasonSummary}</p>
                  </div>
                  {providerGraph.findings.length > 0 ? (
                    <ul className="result-list compact-list">
                      {providerGraph.findings.map((finding) => (
                        <li key={finding.evidence_ref}>
                          <strong>
                            {finding.signal} / {finding.risk_score}
                          </strong>
                          <span>{finding.reason}</span>
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p className="empty">No graph findings</p>
                  )}
                </div>
              ) : (
                <p className="empty">No provider graph assessment</p>
              )}
            </section>
            <section>
              <h3>Similar Cases</h3>
              {similarCaseInspection ? (
                <div className="factor-card">
                  <dl className="result-grid">
                    <div>
                      <dt>Cases</dt>
                      <dd>{similarCaseInspection.caseCount}</dd>
                    </div>
                    <div>
                      <dt>Top Case</dt>
                      <dd>{similarCaseInspection.topCaseLabel}</dd>
                    </div>
                    <div>
                      <dt>Scheme</dt>
                      <dd>{similarCaseInspection.schemeLabel}</dd>
                    </div>
                    <div>
                      <dt>Signals</dt>
                      <dd>{similarCaseInspection.matchedSignalsSummary}</dd>
                    </div>
                  </dl>
                  <div>
                    <span>Provenance</span>
                    <p>{similarCaseInspection.provenanceSummary}</p>
                  </div>
                  {result.similar_cases.length > 0 ? (
                    <ul className="result-list compact-list">
                      {result.similar_cases.map((similarCase) => (
                        <li key={similarCase.case_id}>
                          <strong>
                            {similarCase.case_id} ·{" "}
                            {(similarCase.similarity_score * 100).toFixed(0)}%
                          </strong>
                          <span>{similarCase.title}</span>
                          <small>{formatFwaSchemeLabel(similarCase.scheme_family, schemeLabelMap)}</small>
                          <small>{similarCase.provenance_refs.join(", ")}</small>
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p className="empty">No similar cases</p>
                  )}
                </div>
              ) : null}
            </section>
            <section>
              <h3>Feature Trace</h3>
              {featureTraceRows.length > 0 ? (
                <ul className="result-list compact-list">
                  {featureTraceRows.map((feature) => (
                    <li key={feature.key}>
                      <strong>
                        {feature.name} · {feature.versionLabel} · {feature.valueLabel}
                      </strong>
                      <small>{feature.evidenceLabel}</small>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="empty">No feature trace</p>
              )}
            </section>
            <section>
              <h3>Evidence Refs</h3>
              {evidenceRefRows.length > 0 ? (
                <ol className="audit-timeline">
                  {evidenceRefRows.map((reference) => (
                    <li key={reference.key}>
                      <strong>{reference.kind}</strong>
                      <span>{reference.label}</span>
                    </li>
                  ))}
                </ol>
              ) : (
                <p className="empty">No evidence refs</p>
              )}
            </section>
            <section>
              <h3>Raw JSON</h3>
              <pre>{JSON.stringify(result, null, 2)}</pre>
            </section>
          </div>
        ) : null}
      </div>
    </section>
  );
}
