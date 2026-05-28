import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getDashboardSummary } from "../api";
import { buildDashboardLayerRows, type DashboardLayerScore } from "./dashboardLayerRows";
import {
  buildSavingAttributionRows,
  type SavingAttributionSummary,
} from "./savingAttributionRows";

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
};

type DashboardSummary = {
  suspected_claims: number;
  confirmed_fwa: number;
  risk_amount: string;
  saving_amount: string;
  rag_distribution: Record<string, number>;
  rule_hits: number;
  model_scores: Record<string, DashboardModelScore>;
  layer_scores: Record<string, DashboardLayerScore>;
  saving_attributions: SavingAttributionSummary[];
  label_pool: DashboardLabelPool;
  investigation_results: number;
  qa_reviews: number;
};

function formatScore(score: number) {
  return score.toFixed(1);
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
    trainingReadyRateLabel:
      totalLabels === 0 ? "0.0%" : `${((approvedForTraining / totalLabels) * 100).toFixed(1)}%`,
  };
}

export function DashboardPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const dashboardQuery = useQuery({
    queryKey: ["dashboard-summary", apiKey],
    queryFn: () => getDashboardSummary(apiKey) as Promise<DashboardSummary>,
  });
  const summary = dashboardQuery.data;
  const ragRows = Object.entries(summary?.rag_distribution ?? {});
  const modelRows = Object.entries(summary?.model_scores ?? {});
  const layerRows = buildDashboardLayerRows(summary?.layer_scores ?? {});
  const savingAttributionRows = buildSavingAttributionRows(summary?.saving_attributions ?? []);
  const labelPoolSummary = buildDashboardLabelPoolSummary(summary?.label_pool);

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
          </div>
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
              </div>
            ))}
          </div>
          {!dashboardQuery.isLoading && savingAttributionRows.length === 0 ? (
            <p className="empty">No saving attribution</p>
          ) : null}
        </div>
      </div>
    </section>
  );
}
