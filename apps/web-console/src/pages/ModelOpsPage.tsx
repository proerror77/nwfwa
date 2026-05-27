import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getModelPerformance, listModelEvaluations, listModels } from "../api";
import {
  buildModelPromotionGateSummary,
  type PromotionModelEvaluation,
} from "./modelPromotionGates";

type ModelVersion = {
  model_key: string;
  version: string;
  model_type: string;
  runtime_kind: string;
  execution_provider: string;
  status: string;
  endpoint_url: string | null;
};

type ModelEvaluationRecord = PromotionModelEvaluation & {
  ks: string | null;
  f1: string | null;
  accuracy: string | null;
  confusion_matrix_json?: Record<string, unknown>;
};

export function ModelOpsPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedModelKey, setSelectedModelKey] = useState("baseline_fwa");
  const modelsQuery = useQuery({
    queryKey: ["models", apiKey],
    queryFn: () => listModels(apiKey) as Promise<{ models: ModelVersion[] }>,
  });
  const selectedModel = useMemo(
    () =>
      modelsQuery.data?.models.find((model) => model.model_key === selectedModelKey) ??
      modelsQuery.data?.models[0],
    [modelsQuery.data?.models, selectedModelKey],
  );
  const performanceQuery = useQuery({
    queryKey: ["model-performance", selectedModel?.model_key, apiKey],
    queryFn: () => getModelPerformance(selectedModel!.model_key, apiKey),
    enabled: Boolean(selectedModel?.model_key),
  });
  const evaluationsQuery = useQuery({
    queryKey: ["model-evaluations", apiKey],
    queryFn: () =>
      listModelEvaluations(apiKey) as Promise<{ evaluations: ModelEvaluationRecord[] }>,
  });
  const promotionSummary = buildModelPromotionGateSummary(
    selectedModel,
    performanceQuery.data,
    evaluationsQuery.data?.evaluations ?? [],
  );

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Models</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {modelsQuery.error ? <pre className="error">{String(modelsQuery.error.message)}</pre> : null}
        <div className="table-list">
          {modelsQuery.data?.models.map((model) => (
            <button
              className={
                model.model_key === selectedModel?.model_key ? "row-button active" : "row-button"
              }
              key={`${model.model_key}-${model.version}`}
              onClick={() => setSelectedModelKey(model.model_key)}
            >
              <span>{model.model_key}</span>
              <strong>{model.status}</strong>
              <small>{model.runtime_kind}</small>
            </button>
          ))}
        </div>
      </div>
      <div className="panel">
        <h2>Model Detail</h2>
        {selectedModel ? (
          <dl className="result-grid">
            <div>
              <dt>Model</dt>
              <dd>{selectedModel.model_key}</dd>
            </div>
            <div>
              <dt>Version</dt>
              <dd>{selectedModel.version}</dd>
            </div>
            <div>
              <dt>Type</dt>
              <dd>{selectedModel.model_type}</dd>
            </div>
            <div>
              <dt>Runtime</dt>
              <dd>{selectedModel.runtime_kind}</dd>
            </div>
            <div>
              <dt>Provider</dt>
              <dd>{selectedModel.execution_provider}</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>{selectedModel.status}</dd>
            </div>
          </dl>
        ) : (
          <p className="empty">No models available</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Performance</h2>
        {performanceQuery.error ? (
          <pre className="error">{String(performanceQuery.error.message)}</pre>
        ) : null}
        {performanceQuery.data ? (
          <dl className="result-grid">
            <div>
              <dt>Data Status</dt>
              <dd>{performanceQuery.data.data_status}</dd>
            </div>
            <div>
              <dt>Scored Runs</dt>
              <dd>{performanceQuery.data.scored_runs}</dd>
            </div>
            <div>
              <dt>Average Score</dt>
              <dd>{performanceQuery.data.average_score}</dd>
            </div>
            <div>
              <dt>High Risk</dt>
              <dd>{performanceQuery.data.high_risk_count}</dd>
            </div>
          </dl>
        ) : (
          <p className="empty">No performance data loaded</p>
        )}
      </div>
      <div className="panel wide-panel">
        <h2>Promotion Gates</h2>
        {evaluationsQuery.error ? (
          <pre className="error">{String(evaluationsQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Routing Decision</span>
            <strong>{promotionSummary.decision}</strong>
          </div>
          <div>
            <span>Gates Passed</span>
            <strong>
              {promotionSummary.passedCount}/{promotionSummary.totalCount}
            </strong>
          </div>
          <div>
            <span>Latest Evaluation</span>
            <strong>{promotionSummary.latestEvaluationId}</strong>
          </div>
          <div>
            <span>Runtime Data</span>
            <strong>{promotionSummary.dataStatus}</strong>
          </div>
          <div>
            <span>Scored Runs</span>
            <strong>{promotionSummary.scoredRuns}</strong>
          </div>
          <div>
            <span>Blockers</span>
            <strong>{promotionSummary.blockers.length}</strong>
          </div>
        </div>
        <div className="table-list">
          {promotionSummary.gates.map((gate) => (
            <div className="metric-row compact-metric-row" key={gate.label}>
              <span>{gate.label}</span>
              <strong>{gate.passed ? "passed" : gate.blocker}</strong>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
