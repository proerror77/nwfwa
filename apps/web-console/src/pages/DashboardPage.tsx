import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getDashboardSummary } from "../api";

type DashboardModelScore = {
  scored_runs: number;
  average_score: number;
  high_risk_count: number;
};

type DashboardSummary = {
  suspected_claims: number;
  confirmed_fwa: number;
  risk_amount: string;
  saving_amount: string;
  rag_distribution: Record<string, number>;
  rule_hits: number;
  model_scores: Record<string, DashboardModelScore>;
  investigation_results: number;
  qa_reviews: number;
};

function formatScore(score: number) {
  return score.toFixed(1);
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
      </div>
    </section>
  );
}
