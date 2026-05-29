import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getDashboardSummary, getProviderRiskSummary, listFwaSchemes } from "../api";
import { buildDashboardLayerRows, type DashboardLayerScore } from "./dashboardLayerRows";
import {
  buildSavingAttributionRows,
  type SavingAttributionSummary,
} from "./savingAttributionRows";
import {
  buildFwaSchemeLabelMap,
  formatFwaSchemeLabel,
  type FwaSchemeDefinition,
} from "./fwaSchemeOptions";

type DashboardModelScore = {
  scored_runs: number;
  average_score: number;
  high_risk_count: number;
};

type DashboardLabelPool = {
  total_labels: number;
  approved_for_training: number;
  needs_review: number;
  rule_feedback: number;
  model_feedback: number;
  workflow_feedback: number;
  case_status_labels?: number;
  medical_review_labels?: number;
  false_positive_labels?: number;
  evidence_backed_labels?: number;
};

type DashboardQaQueue = {
  sampled_cases: number;
  open_cases: number;
  reviewed_cases: number;
  disagreement_cases: number;
  disagreement_rate: number;
  feedback_open_count?: number;
  feedback_in_progress_count?: number;
  feedback_resolved_count?: number;
  feedback_dismissed_count?: number;
  unresolved_feedback_count?: number;
};

type DashboardCaseSla = {
  total_cases: number;
  open_cases: number;
  closed_cases: number;
  breached_cases: number;
  sla_breach_rate: number;
  average_time_to_triage_hours: number;
  average_time_to_closure_hours: number;
};

type DashboardAgentGovernance = {
  total_runs: number;
  successful_runs: number;
  pending_approvals: number;
  approved_approvals: number;
  rejected_approvals: number;
};

type DashboardModelGovernance = {
  total_models: number;
  evaluated_models: number;
  drift_watch_count: number;
  drift_detected_count: number;
  average_precision: number | null;
  average_recall: number | null;
};

type DashboardRuleGovernance = {
  total_rules: number;
  active_rules: number;
  triggered_rules: number;
  total_trigger_count: number;
  reviewed_count: number;
  confirmed_fwa_count: number;
  false_positive_count: number;
  precision: number;
  false_positive_rate: number;
  saving_amount: string;
  roi: number;
};

type DashboardValueMeasurement = {
  prevented_payment: string;
  recovered_amount: string;
  avoided_future_exposure: string;
  estimated_impact: string;
  review_cost: string;
  false_positive_operational_cost: string;
  reviewer_capacity_hours: string;
  net_value: string;
  currency: string;
  evidence_caveat: string;
};

type DashboardSavingSegment = {
  segment_type: string;
  segment_id: string;
  saving_amount: string;
  currency: string;
  claim_count: number;
  attribution_count: number;
  roi: number;
};

type DashboardSummary = {
  suspected_claims: number;
  confirmed_fwa: number;
  risk_amount: string;
  saving_amount: string;
  rag_distribution: Record<string, number>;
  scheme_distribution: Record<string, number>;
  rule_hits: number;
  model_scores: Record<string, DashboardModelScore>;
  layer_scores: Record<string, DashboardLayerScore>;
  saving_attributions: SavingAttributionSummary[];
  saving_segments: DashboardSavingSegment[];
  value_measurement: DashboardValueMeasurement;
  label_pool: DashboardLabelPool;
  qa_queue: DashboardQaQueue;
  case_sla: DashboardCaseSla;
  agent_governance: DashboardAgentGovernance;
  model_governance: DashboardModelGovernance;
  rule_governance: DashboardRuleGovernance;
  investigation_results: number;
  qa_reviews: number;
};

type ProviderRiskSummaryItem = {
  provider_id: string;
  risk_score: number;
  risk_tier: string;
  review_required: boolean;
  review_route: string;
  claim_count: number;
  latest_claim_id?: string | null;
  outlier_flags: string[];
  evidence_refs: string[];
};

type ProviderRiskSummary = {
  provider_count: number;
  review_required_count: number;
  high_risk_count: number;
  providers: ProviderRiskSummaryItem[];
};

function formatScore(score: number) {
  return score.toFixed(1);
}

function formatPercent(value: number) {
  const percentage = Math.round((value * 100 + 1e-9) * 10) / 10;
  return `${percentage.toFixed(1)}%`;
}

export function buildDashboardLabelPoolSummary(labelPool?: DashboardLabelPool) {
  const totalLabels = labelPool?.total_labels ?? 0;
  const approvedForTraining = labelPool?.approved_for_training ?? 0;
  return {
    totalLabels,
    approvedForTraining,
    needsReview: labelPool?.needs_review ?? 0,
    ruleFeedback: labelPool?.rule_feedback ?? 0,
    modelFeedback: labelPool?.model_feedback ?? 0,
    workflowFeedback: labelPool?.workflow_feedback ?? 0,
    caseStatusLabels: labelPool?.case_status_labels ?? 0,
    medicalReviewLabels: labelPool?.medical_review_labels ?? 0,
    falsePositiveLabels: labelPool?.false_positive_labels ?? 0,
    evidenceBackedLabels: labelPool?.evidence_backed_labels ?? 0,
    trainingReadyRateLabel:
      totalLabels === 0 ? "0.0%" : formatPercent(approvedForTraining / totalLabels),
  };
}

export function buildDashboardQaQueueSummary(queue?: DashboardQaQueue) {
  const sampledCases = queue?.sampled_cases ?? 0;
  const reviewedCases = queue?.reviewed_cases ?? 0;
  return {
    sampledCases,
    openCases: queue?.open_cases ?? 0,
    reviewedCases,
    disagreementCases: queue?.disagreement_cases ?? 0,
    feedbackOpenCount: queue?.feedback_open_count ?? 0,
    feedbackInProgressCount: queue?.feedback_in_progress_count ?? 0,
    feedbackResolvedCount: queue?.feedback_resolved_count ?? 0,
    feedbackDismissedCount: queue?.feedback_dismissed_count ?? 0,
    unresolvedFeedbackCount: queue?.unresolved_feedback_count ?? 0,
    reviewedRateLabel:
      sampledCases === 0 ? "0.0%" : formatPercent(reviewedCases / sampledCases),
    disagreementRateLabel: formatPercent(queue?.disagreement_rate ?? 0),
  };
}

export function buildDashboardCaseSlaSummary(sla?: DashboardCaseSla) {
  return {
    totalCases: sla?.total_cases ?? 0,
    openCases: sla?.open_cases ?? 0,
    closedCases: sla?.closed_cases ?? 0,
    breachedCases: sla?.breached_cases ?? 0,
    breachRateLabel: formatPercent(sla?.sla_breach_rate ?? 0),
    averageTimeToTriageLabel: `${(sla?.average_time_to_triage_hours ?? 0).toFixed(1)}h`,
    averageTimeToClosureLabel: `${(sla?.average_time_to_closure_hours ?? 0).toFixed(1)}h`,
  };
}

export function buildDashboardAgentGovernanceSummary(governance?: DashboardAgentGovernance) {
  const totalRuns = governance?.total_runs ?? 0;
  const successfulRuns = governance?.successful_runs ?? 0;
  const approvedApprovals = governance?.approved_approvals ?? 0;
  const rejectedApprovals = governance?.rejected_approvals ?? 0;
  const decidedApprovals = approvedApprovals + rejectedApprovals;
  return {
    totalRuns,
    successfulRuns,
    pendingApprovals: governance?.pending_approvals ?? 0,
    approvedApprovals,
    rejectedApprovals,
    successRateLabel:
      totalRuns === 0 ? "0.0%" : formatPercent(successfulRuns / totalRuns),
    approvalRateLabel:
      decidedApprovals === 0 ? "0.0%" : formatPercent(approvedApprovals / decidedApprovals),
  };
}

export function buildDashboardModelGovernanceSummary(governance?: DashboardModelGovernance) {
  const totalModels = governance?.total_models ?? 0;
  const evaluatedModels = governance?.evaluated_models ?? 0;
  const averagePrecision = governance?.average_precision ?? null;
  const averageRecall = governance?.average_recall ?? null;
  return {
    totalModels,
    evaluatedModels,
    driftWatchCount: governance?.drift_watch_count ?? 0,
    driftDetectedCount: governance?.drift_detected_count ?? 0,
    evaluationCoverageLabel:
      totalModels === 0 ? "0.0%" : formatPercent(evaluatedModels / totalModels),
    averagePrecisionLabel: averagePrecision === null ? "n/a" : formatPercent(averagePrecision),
    averageRecallLabel: averageRecall === null ? "n/a" : formatPercent(averageRecall),
  };
}

export function buildDashboardRuleGovernanceSummary(governance?: DashboardRuleGovernance) {
  return {
    totalRules: governance?.total_rules ?? 0,
    activeRules: governance?.active_rules ?? 0,
    triggeredRules: governance?.triggered_rules ?? 0,
    totalTriggerCount: governance?.total_trigger_count ?? 0,
    reviewedCount: governance?.reviewed_count ?? 0,
    confirmedFwaCount: governance?.confirmed_fwa_count ?? 0,
    falsePositiveCount: governance?.false_positive_count ?? 0,
    precisionLabel: governance ? formatPercent(governance.precision) : "0.0%",
    falsePositiveRateLabel: governance
      ? formatPercent(governance.false_positive_rate)
      : "0.0%",
    savingAmount: governance?.saving_amount ?? "0.00",
    roiLabel: `${(governance?.roi ?? 0).toFixed(1)}x`,
  };
}

export function buildDashboardSchemeRows(
  distribution: Record<string, number> = {},
  schemeLabelMap: Record<string, string> = {},
) {
  return Object.entries(distribution)
    .map(([schemeFamily, count]) => ({
      schemeFamily,
      schemeLabel: formatFwaSchemeLabel(schemeFamily, schemeLabelMap),
      count,
    }))
    .sort(
      (left, right) =>
        right.count - left.count || left.schemeFamily.localeCompare(right.schemeFamily),
    );
}

export function buildDashboardSavingSegmentRows(
  segments: DashboardSavingSegment[] = [],
  schemeLabelMap: Record<string, string> = {},
) {
  return [...segments]
    .sort((left, right) => {
      const leftTypeRank = left.segment_type === "provider" ? 0 : 1;
      const rightTypeRank = right.segment_type === "provider" ? 0 : 1;
      return (
        leftTypeRank - rightTypeRank ||
        Number(right.saving_amount) - Number(left.saving_amount) ||
        left.segment_id.localeCompare(right.segment_id)
      );
    })
    .map((segment) => ({
      key: `${segment.segment_type}:${segment.segment_id}:${segment.currency}`,
      segmentLabel:
        segment.segment_type === "scheme"
          ? `${segment.segment_type} / ${formatFwaSchemeLabel(segment.segment_id, schemeLabelMap)}`
          : `${segment.segment_type} / ${segment.segment_id}`,
      savingAmount: segment.saving_amount,
      currency: segment.currency,
      claimCount: segment.claim_count,
      attributionCount: segment.attribution_count,
      roiLabel: `${segment.roi.toFixed(1)}x`,
    }));
}

export function buildDashboardValueMeasurementSummary(value?: DashboardValueMeasurement) {
  const currency = value?.currency ?? "CNY";
  return {
    preventedPayment: `${currency} ${value?.prevented_payment ?? "0.00"}`,
    recoveredAmount: `${currency} ${value?.recovered_amount ?? "0.00"}`,
    avoidedFutureExposure: `${currency} ${value?.avoided_future_exposure ?? "0.00"}`,
    estimatedImpact: `${currency} ${value?.estimated_impact ?? "0.00"}`,
    reviewCost: `${currency} ${value?.review_cost ?? "0.00"}`,
    falsePositiveOperationalCost: `${currency} ${value?.false_positive_operational_cost ?? "0.00"}`,
    reviewerCapacityHours: `${value?.reviewer_capacity_hours ?? "0.00"}h`,
    netValue: `${currency} ${value?.net_value ?? "0.00"}`,
    evidenceCaveat: value?.evidence_caveat ?? "No value measurement caveat available.",
  };
}

export function buildProviderRiskSummary(summary?: ProviderRiskSummary) {
  const providerCount = summary?.provider_count ?? 0;
  const reviewRequiredCount = summary?.review_required_count ?? 0;
  const topProvider = summary?.providers[0];
  return {
    providerCount,
    reviewRequiredCount,
    highRiskCount: summary?.high_risk_count ?? 0,
    reviewRateLabel:
      providerCount === 0 ? "0.0%" : formatPercent(reviewRequiredCount / providerCount),
    topProviderId: topProvider?.provider_id ?? "none",
    topProviderScore: topProvider?.risk_score ?? 0,
  };
}

export function DashboardPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const dashboardQuery = useQuery({
    queryKey: ["dashboard-summary", apiKey],
    queryFn: () => getDashboardSummary(apiKey) as Promise<DashboardSummary>,
  });
  const providerRiskQuery = useQuery({
    queryKey: ["provider-risk-summary", apiKey],
    queryFn: () => getProviderRiskSummary(apiKey) as Promise<ProviderRiskSummary>,
  });
  const schemesQuery = useQuery({
    queryKey: ["fwa-schemes", apiKey],
    queryFn: () => listFwaSchemes(apiKey) as Promise<{ schemes: FwaSchemeDefinition[] }>,
  });
  const summary = dashboardQuery.data;
  const providerRisk = providerRiskQuery.data;
  const schemeLabelMap = buildFwaSchemeLabelMap(schemesQuery.data?.schemes);
  const ragRows = Object.entries(summary?.rag_distribution ?? {});
  const schemeRows = buildDashboardSchemeRows(
    summary?.scheme_distribution ?? {},
    schemeLabelMap,
  );
  const modelRows = Object.entries(summary?.model_scores ?? {});
  const layerRows = buildDashboardLayerRows(summary?.layer_scores ?? {});
  const savingAttributionRows = buildSavingAttributionRows(summary?.saving_attributions ?? []);
  const savingSegmentRows = buildDashboardSavingSegmentRows(
    summary?.saving_segments ?? [],
    schemeLabelMap,
  );
  const labelPoolSummary = buildDashboardLabelPoolSummary(summary?.label_pool);
  const qaQueueSummary = buildDashboardQaQueueSummary(summary?.qa_queue);
  const caseSlaSummary = buildDashboardCaseSlaSummary(summary?.case_sla);
  const agentGovernanceSummary = buildDashboardAgentGovernanceSummary(summary?.agent_governance);
  const modelGovernanceSummary = buildDashboardModelGovernanceSummary(summary?.model_governance);
  const ruleGovernanceSummary = buildDashboardRuleGovernanceSummary(summary?.rule_governance);
  const valueMeasurementSummary = buildDashboardValueMeasurementSummary(summary?.value_measurement);
  const providerRiskSummary = buildProviderRiskSummary(providerRisk);

  return (
    <section className="dashboard">
      <div className="panel dashboard-header">
        <div>
          <h2>Management Dashboard</h2>
          <p>FWA risk, review, model, and ROI summary.</p>
        </div>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
      </div>

      {dashboardQuery.error ? (
        <pre className="error">{String(dashboardQuery.error.message)}</pre>
      ) : null}
      {providerRiskQuery.error ? (
        <pre className="error">{String(providerRiskQuery.error.message)}</pre>
      ) : null}
      {schemesQuery.error ? <pre className="error">{String(schemesQuery.error.message)}</pre> : null}

      <div className="summary-grid dashboard-kpis">
        <div>
          <span>Suspected Claims</span>
          <strong>{summary?.suspected_claims ?? "-"}</strong>
        </div>
        <div>
          <span>Risk Amount</span>
          <strong>{summary ? `CNY ${summary.risk_amount}` : "-"}</strong>
        </div>
        <div>
          <span>Confirmed FWA</span>
          <strong>{summary?.confirmed_fwa ?? "-"}</strong>
        </div>
        <div>
          <span>Saving Amount</span>
          <strong>{summary ? `CNY ${summary.saving_amount}` : "-"}</strong>
        </div>
        <div>
          <span>Rule Hits</span>
          <strong>{summary?.rule_hits ?? "-"}</strong>
        </div>
        <div>
          <span>QA / Investigation</span>
          <strong>
            {summary ? `${summary.qa_reviews} / ${summary.investigation_results}` : "-"}
          </strong>
        </div>
      </div>

      <div className="ops-grid">
        <div className="panel">
          <h2>RAG Distribution</h2>
          <div className="table-list">
            {ragRows.map(([rag, count]) => (
              <div className="metric-row compact-metric-row" key={rag}>
                <span>{rag}</span>
                <strong>{count}</strong>
              </div>
            ))}
          </div>
          {!dashboardQuery.isLoading && ragRows.length === 0 ? (
            <p className="empty">No scored claims</p>
          ) : null}
        </div>

        <div className="panel">
          <h2>FWA Scheme Distribution</h2>
          <div className="table-list">
            {schemeRows.map((scheme) => (
              <div className="metric-row compact-metric-row" key={scheme.schemeFamily}>
                <span>{scheme.schemeLabel}</span>
                <strong>{scheme.count}</strong>
              </div>
            ))}
          </div>
          {!dashboardQuery.isLoading && schemeRows.length === 0 ? (
            <p className="empty">No scheme distribution</p>
          ) : null}
        </div>

        <div className="panel">
          <h2>Value Measurement</h2>
          <div className="summary-grid">
            <div>
              <span>Prevented</span>
              <strong>{valueMeasurementSummary.preventedPayment}</strong>
            </div>
            <div>
              <span>Recovered</span>
              <strong>{valueMeasurementSummary.recoveredAmount}</strong>
            </div>
            <div>
              <span>Estimated</span>
              <strong>{valueMeasurementSummary.estimatedImpact}</strong>
            </div>
            <div>
              <span>Net Value</span>
              <strong>{valueMeasurementSummary.netValue}</strong>
            </div>
          </div>
          <div className="table-list">
            <div className="metric-row compact-metric-row">
              <span>Avoided Exposure</span>
              <strong>{valueMeasurementSummary.avoidedFutureExposure}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Review Cost</span>
              <strong>{valueMeasurementSummary.reviewCost}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>False Positive Cost</span>
              <strong>{valueMeasurementSummary.falsePositiveOperationalCost}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Reviewer Capacity</span>
              <strong>{valueMeasurementSummary.reviewerCapacityHours}</strong>
            </div>
          </div>
          <p className="empty">{valueMeasurementSummary.evidenceCaveat}</p>
        </div>

        <div className="panel">
          <h2>Model Distribution</h2>
          <div className="table-list">
            {modelRows.map(([modelKey, model]) => (
              <div className="metric-row" key={modelKey}>
                <span>{modelKey}</span>
                <strong>{model.scored_runs} runs</strong>
                <small>Avg {formatScore(model.average_score)}</small>
                <small>High risk {model.high_risk_count}</small>
              </div>
            ))}
          </div>
          {!dashboardQuery.isLoading && modelRows.length === 0 ? (
            <p className="empty">No model scores</p>
          ) : null}
        </div>

        <div className="panel">
          <h2>Model Governance</h2>
          <div className="summary-grid">
            <div>
              <span>Models</span>
              <strong>{modelGovernanceSummary.totalModels}</strong>
            </div>
            <div>
              <span>Evaluated</span>
              <strong>{modelGovernanceSummary.evaluationCoverageLabel}</strong>
            </div>
            <div>
              <span>Precision</span>
              <strong>{modelGovernanceSummary.averagePrecisionLabel}</strong>
            </div>
            <div>
              <span>Recall</span>
              <strong>{modelGovernanceSummary.averageRecallLabel}</strong>
            </div>
          </div>
          <div className="table-list">
            <div className="metric-row compact-metric-row">
              <span>Drift Watch</span>
              <strong>{modelGovernanceSummary.driftWatchCount}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Drift Detected</span>
              <strong>{modelGovernanceSummary.driftDetectedCount}</strong>
            </div>
          </div>
        </div>

        <div className="panel">
          <h2>Rule Governance</h2>
          <div className="summary-grid">
            <div>
              <span>Rules</span>
              <strong>{ruleGovernanceSummary.totalRules}</strong>
            </div>
            <div>
              <span>Active</span>
              <strong>{ruleGovernanceSummary.activeRules}</strong>
            </div>
            <div>
              <span>Triggered</span>
              <strong>{ruleGovernanceSummary.triggeredRules}</strong>
            </div>
            <div>
              <span>Precision</span>
              <strong>{ruleGovernanceSummary.precisionLabel}</strong>
            </div>
          </div>
          <div className="table-list">
            <div className="metric-row compact-metric-row">
              <span>Trigger Count</span>
              <strong>{ruleGovernanceSummary.totalTriggerCount}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>False Positive</span>
              <strong>{ruleGovernanceSummary.falsePositiveRateLabel}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Saving</span>
              <strong>CNY {ruleGovernanceSummary.savingAmount}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>ROI</span>
              <strong>{ruleGovernanceSummary.roiLabel}</strong>
            </div>
          </div>
        </div>

        <div className="panel">
          <h2>QA Queue</h2>
          <div className="summary-grid">
            <div>
              <span>Sampled</span>
              <strong>{qaQueueSummary.sampledCases}</strong>
            </div>
            <div>
              <span>Open</span>
              <strong>{qaQueueSummary.openCases}</strong>
            </div>
            <div>
              <span>Reviewed</span>
              <strong>{qaQueueSummary.reviewedCases}</strong>
            </div>
            <div>
              <span>Review Rate</span>
              <strong>{qaQueueSummary.reviewedRateLabel}</strong>
            </div>
            <div>
              <span>Disagreement</span>
              <strong>{qaQueueSummary.disagreementRateLabel}</strong>
            </div>
          </div>
          <div className="table-list">
            <div className="metric-row compact-metric-row">
              <span>Disagreement Cases</span>
              <strong>{qaQueueSummary.disagreementCases}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Unresolved Feedback</span>
              <strong>{qaQueueSummary.unresolvedFeedbackCount}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Feedback Open</span>
              <strong>{qaQueueSummary.feedbackOpenCount}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Feedback In Progress</span>
              <strong>{qaQueueSummary.feedbackInProgressCount}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Feedback Resolved</span>
              <strong>{qaQueueSummary.feedbackResolvedCount}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Feedback Dismissed</span>
              <strong>{qaQueueSummary.feedbackDismissedCount}</strong>
            </div>
          </div>
        </div>

        <div className="panel">
          <h2>Case SLA</h2>
          <div className="summary-grid">
            <div>
              <span>Cases</span>
              <strong>{caseSlaSummary.totalCases}</strong>
            </div>
            <div>
              <span>Open</span>
              <strong>{caseSlaSummary.openCases}</strong>
            </div>
            <div>
              <span>Closed</span>
              <strong>{caseSlaSummary.closedCases}</strong>
            </div>
            <div>
              <span>Breach Rate</span>
              <strong>{caseSlaSummary.breachRateLabel}</strong>
            </div>
          </div>
          <div className="table-list">
            <div className="metric-row compact-metric-row">
              <span>Breached Cases</span>
              <strong>{caseSlaSummary.breachedCases}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Avg Triage</span>
              <strong>{caseSlaSummary.averageTimeToTriageLabel}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Avg Closure</span>
              <strong>{caseSlaSummary.averageTimeToClosureLabel}</strong>
            </div>
          </div>
        </div>

        <div className="panel">
          <h2>Agent Governance</h2>
          <div className="summary-grid">
            <div>
              <span>Agent Runs</span>
              <strong>{agentGovernanceSummary.totalRuns}</strong>
            </div>
            <div>
              <span>Success Rate</span>
              <strong>{agentGovernanceSummary.successRateLabel}</strong>
            </div>
            <div>
              <span>Pending</span>
              <strong>{agentGovernanceSummary.pendingApprovals}</strong>
            </div>
            <div>
              <span>Approval Rate</span>
              <strong>{agentGovernanceSummary.approvalRateLabel}</strong>
            </div>
          </div>
          <div className="table-list">
            <div className="metric-row compact-metric-row">
              <span>Approved</span>
              <strong>{agentGovernanceSummary.approvedApprovals}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Rejected</span>
              <strong>{agentGovernanceSummary.rejectedApprovals}</strong>
            </div>
          </div>
        </div>

        <div className="panel">
          <h2>Label Governance</h2>
          <div className="summary-grid">
            <div>
              <span>Total Labels</span>
              <strong>{labelPoolSummary.totalLabels}</strong>
            </div>
            <div>
              <span>Training Ready</span>
              <strong>{labelPoolSummary.approvedForTraining}</strong>
            </div>
            <div>
              <span>Needs Review</span>
              <strong>{labelPoolSummary.needsReview}</strong>
            </div>
            <div>
              <span>Ready Rate</span>
              <strong>{labelPoolSummary.trainingReadyRateLabel}</strong>
            </div>
          </div>
          <div className="table-list">
            <div className="metric-row compact-metric-row">
              <span>Rules</span>
              <strong>{labelPoolSummary.ruleFeedback}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Models</span>
              <strong>{labelPoolSummary.modelFeedback}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Workflow</span>
              <strong>{labelPoolSummary.workflowFeedback}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Case Status</span>
              <strong>{labelPoolSummary.caseStatusLabels}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Medical Review</span>
              <strong>{labelPoolSummary.medicalReviewLabels}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>False Positive</span>
              <strong>{labelPoolSummary.falsePositiveLabels}</strong>
            </div>
            <div className="metric-row compact-metric-row">
              <span>Evidence Backed</span>
              <strong>
                {labelPoolSummary.evidenceBackedLabels}/{labelPoolSummary.totalLabels}
              </strong>
            </div>
          </div>
        </div>

        <div className="panel">
          <h2>Provider Risk</h2>
          <div className="summary-grid">
            <div>
              <span>Providers</span>
              <strong>{providerRiskSummary.providerCount}</strong>
            </div>
            <div>
              <span>Review Required</span>
              <strong>{providerRiskSummary.reviewRequiredCount}</strong>
            </div>
            <div>
              <span>High Risk</span>
              <strong>{providerRiskSummary.highRiskCount}</strong>
            </div>
            <div>
              <span>Review Rate</span>
              <strong>{providerRiskSummary.reviewRateLabel}</strong>
            </div>
          </div>
          <div className="table-list">
            {(providerRisk?.providers ?? []).slice(0, 4).map((provider) => (
              <div className="metric-row" key={provider.provider_id}>
                <span>{provider.provider_id}</span>
                <strong>{provider.risk_score}</strong>
                <small>{provider.review_route}</small>
                <small>{provider.outlier_flags.slice(0, 2).join(", ") || "no outliers"}</small>
              </div>
            ))}
          </div>
          {!providerRiskQuery.isLoading && (providerRisk?.providers.length ?? 0) === 0 ? (
            <p className="empty">No provider risk profiles</p>
          ) : null}
        </div>

        <div className="panel wide-panel">
          <h2>Seven Layer Detection</h2>
          <div className="table-list">
            {layerRows.map((layer) => (
              <div className="metric-row" key={layer.layerId}>
                <span>{layer.layerId}</span>
                <strong>{layer.name}</strong>
                <small>{layer.scoredRuns} runs</small>
                <small>Avg {formatScore(layer.averageScore)}</small>
                <small>High risk {layer.highRiskCount}</small>
              </div>
            ))}
          </div>
          {!dashboardQuery.isLoading && layerRows.length === 0 ? (
            <p className="empty">No layer scores</p>
          ) : null}
        </div>

        <div className="panel wide-panel">
          <h2>Saving Attribution</h2>
          <div className="table-list">
            {savingAttributionRows.map((attribution) => (
              <div className="metric-row" key={attribution.key}>
                <span>{attribution.sourceLabel}</span>
                <strong>
                  {attribution.currency} {attribution.savingAmount}
                </strong>
                <small>{attribution.action}</small>
                <small>{attribution.claimCount} claims</small>
                <small>
                  Avg {attribution.currency} {attribution.averageSavingPerClaim} / claim
                </small>
              </div>
            ))}
          </div>
          {!dashboardQuery.isLoading && savingAttributionRows.length === 0 ? (
            <p className="empty">No saving attribution</p>
          ) : null}
        </div>

        <div className="panel wide-panel">
          <h2>Segment ROI</h2>
          <div className="table-list">
            {savingSegmentRows.map((segment) => (
              <div className="metric-row" key={segment.key}>
                <span>{segment.segmentLabel}</span>
                <strong>
                  {segment.currency} {segment.savingAmount}
                </strong>
                <small>{segment.claimCount} claims</small>
                <small>{segment.attributionCount} attributions</small>
                <small>{segment.roiLabel}</small>
              </div>
            ))}
          </div>
          {!dashboardQuery.isLoading && savingSegmentRows.length === 0 ? (
            <p className="empty">No segment ROI attribution</p>
          ) : null}
        </div>
      </div>
    </section>
  );
}
