import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getProviderRiskSummary } from "../api";

type ProviderRiskSummaryItem = {
  provider_id: string;
  risk_score: number;
  risk_tier: string;
  review_required: boolean;
  review_route: string;
  claim_count: number;
  network_risk_score?: number | null;
  latest_claim_id?: string | null;
  review_failure_count: number;
  confirmed_fwa_count: number;
  false_positive_count: number;
  outlier_flags: string[];
  graph_reasons: string[];
  evidence_refs: string[];
};

type ProviderRiskSummary = {
  provider_count: number;
  review_required_count: number;
  high_risk_count: number;
  providers: ProviderRiskSummaryItem[];
};

export function buildProviderRiskOpsSummary(summary?: ProviderRiskSummary) {
  const providerCount = summary?.provider_count ?? 0;
  const reviewRequiredCount = summary?.review_required_count ?? 0;
  const highRiskCount = summary?.high_risk_count ?? 0;
  const providers = summary?.providers ?? [];
  const graphRiskCount =
    providers.filter(
      (provider) =>
        provider.review_route === "provider_graph_review" ||
        (provider.network_risk_score ?? 0) >= 70 ||
        provider.graph_reasons.length > 0,
    ).length;
  const evidenceBackedCount =
    providers.filter((provider) => provider.evidence_refs.length > 0).length;
  const networkScoreCount = providers.filter(
    (provider) => provider.network_risk_score != null,
  ).length;
  const graphReasonCount = providers.filter((provider) => provider.graph_reasons.length > 0).length;
  const reviewFailureHistoryCount = providers.reduce(
    (total, provider) => total + provider.review_failure_count,
    0,
  );
  const confirmedFwaHistoryCount = providers.reduce(
    (total, provider) => total + provider.confirmed_fwa_count,
    0,
  );
  const falsePositiveHistoryCount = providers.reduce(
    (total, provider) => total + provider.false_positive_count,
    0,
  );
  return {
    providerCount,
    reviewRequiredCount,
    highRiskCount,
    graphRiskCount,
    evidenceBackedCount,
    networkScoreCount,
    graphReasonCount,
    reviewFailureHistoryCount,
    confirmedFwaHistoryCount,
    falsePositiveHistoryCount,
    graphEvidenceGapCount: Math.max(graphRiskCount - evidenceBackedCount, 0),
    graphEvidenceStatus:
      graphRiskCount === 0
        ? "no_graph_risk"
        : graphRiskCount <= evidenceBackedCount
          ? "graph_evidence_complete"
          : "graph_evidence_gap",
    reviewRateLabel:
      providerCount === 0
        ? "0.0%"
        : `${((reviewRequiredCount / providerCount) * 100).toFixed(1)}%`,
  };
}

export function filterProviderRiskItems(
  providers: ProviderRiskSummaryItem[],
  filter: "all" | "review_required" | "high_risk",
) {
  if (filter === "review_required") {
    return providers.filter((provider) => provider.review_required);
  }
  if (filter === "high_risk") {
    return providers.filter((provider) => provider.risk_score >= 80);
  }
  return providers;
}

export function ProviderRiskPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [filter, setFilter] = useState<"all" | "review_required" | "high_risk">("all");
  const providerRiskQuery = useQuery({
    queryKey: ["provider-risk-summary", "ops-page", apiKey],
    queryFn: () => getProviderRiskSummary(apiKey) as Promise<ProviderRiskSummary>,
  });
  const summary = buildProviderRiskOpsSummary(providerRiskQuery.data);
  const providers = useMemo(
    () => filterProviderRiskItems(providerRiskQuery.data?.providers ?? [], filter),
    [filter, providerRiskQuery.data?.providers],
  );

  return (
    <section className="ops-grid">
      <div className="panel dashboard-header">
        <div>
          <h2>Provider Risk</h2>
          <p>Provider profile, peer outlier, review routing, and evidence queue.</p>
        </div>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
      </div>

      <div className="panel">
        <h2>Risk Pressure</h2>
        {providerRiskQuery.error ? (
          <pre className="error">{String(providerRiskQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Providers</span>
            <strong>{summary.providerCount}</strong>
          </div>
          <div>
            <span>Review Required</span>
            <strong>{summary.reviewRequiredCount}</strong>
          </div>
          <div>
            <span>High Risk</span>
            <strong>{summary.highRiskCount}</strong>
          </div>
          <div>
            <span>Graph Risk</span>
            <strong>{summary.graphRiskCount}</strong>
          </div>
          <div>
            <span>Network Scores</span>
            <strong>{summary.networkScoreCount}</strong>
          </div>
          <div>
            <span>Graph Reasons</span>
            <strong>{summary.graphReasonCount}</strong>
          </div>
          <div>
            <span>Review Failure History</span>
            <strong>{summary.reviewFailureHistoryCount}</strong>
          </div>
          <div>
            <span>Confirmed FWA History</span>
            <strong>{summary.confirmedFwaHistoryCount}</strong>
          </div>
          <div>
            <span>False Positive History</span>
            <strong>{summary.falsePositiveHistoryCount}</strong>
          </div>
          <div>
            <span>Evidence</span>
            <strong>
              {summary.evidenceBackedCount}/{summary.providerCount}
            </strong>
          </div>
          <div>
            <span>Graph Evidence Gaps</span>
            <strong>{summary.graphEvidenceGapCount}</strong>
          </div>
          <div>
            <span>Graph Evidence Status</span>
            <strong>{summary.graphEvidenceStatus}</strong>
          </div>
          <div>
            <span>Review Rate</span>
            <strong>{summary.reviewRateLabel}</strong>
          </div>
        </div>
        <label>
          Queue Filter
          <select
            value={filter}
            onChange={(event) =>
              setFilter(event.target.value as "all" | "review_required" | "high_risk")
            }
          >
            <option value="all">All Providers</option>
            <option value="review_required">Review Required</option>
            <option value="high_risk">High Risk</option>
          </select>
        </label>
      </div>

      <div className="panel wide-panel">
        <h2>Provider Queue</h2>
        <div className="table-list">
          {providers.map((provider) => (
            <div className="metric-row compact-metric-row" key={provider.provider_id}>
              <span>{provider.provider_id}</span>
              <strong>{provider.risk_score}</strong>
              <small>{provider.review_route}</small>
              <small>{provider.risk_tier}</small>
              <small>
                network {provider.network_risk_score == null ? "n/a" : provider.network_risk_score}
              </small>
              <small>review failures {provider.review_failure_count}</small>
              <small>confirmed FWA {provider.confirmed_fwa_count}</small>
              <small>false positive {provider.false_positive_count}</small>
              <small>{provider.claim_count} claims</small>
              <small>{provider.latest_claim_id ?? "no latest claim"}</small>
              <small>{provider.outlier_flags.join(", ") || "no outliers"}</small>
              <small>{provider.graph_reasons.join(", ") || "no graph signals"}</small>
              <small>{provider.evidence_refs.join(", ") || "no evidence refs"}</small>
            </div>
          ))}
        </div>
        {!providerRiskQuery.isLoading && providers.length === 0 ? (
          <p className="empty">No provider risk profiles</p>
        ) : null}
      </div>
    </section>
  );
}
