import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getModelPerformance,
  getModelPromotionGates,
  listModels,
  submitModelPromotionReview,
} from "../api";

type ModelVersion = {
  model_key: string;
  version: string;
  model_type: string;
  runtime_kind: string;
  execution_provider: string;
  status: string;
  endpoint_url: string | null;
};

type ModelPromotionGatesResponse = {
  decision: string;
  passed_count: number;
  total_count: number;
  latest_evaluation_id: string;
  data_status: string;
  scored_runs: number;
  blockers: string[];
  gates: Array<{
    label: string;
    passed: boolean;
    blocker: string;
  }>;
};

export function ModelOpsPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedModelKey, setSelectedModelKey] = useState("baseline_fwa");
  const [reviewer, setReviewer] = useState("model-governance");
  const [notes, setNotes] = useState("Approved for continued shadow evaluation only.");
  const queryClient = useQueryClient();
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
  const promotionQuery = useQuery({
    queryKey: ["model-promotion-gates", selectedModel?.model_key, apiKey],
    queryFn: () =>
      getModelPromotionGates(
        selectedModel!.model_key,
        apiKey,
      ) as Promise<ModelPromotionGatesResponse>,
    enabled: Boolean(selectedModel?.model_key),
  });
  const reviewMutation = useMutation({
    mutationFn: (decision: "approved" | "rejected") => {
      if (!selectedModel) throw new Error("No model selected");
      return submitModelPromotionReview(
        selectedModel.model_key,
        { decision, reviewer, notes },
        apiKey,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["model-promotion-gates"] });
    },
  });

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
        {promotionQuery.error ? (
          <pre className="error">{String(promotionQuery.error.message)}</pre>
        ) : null}
        {promotionQuery.data ? (
          <>
            <div className="summary-grid">
              <div>
                <span>Routing Decision</span>
                <strong>{promotionQuery.data.decision}</strong>
              </div>
              <div>
                <span>Gates Passed</span>
                <strong>
                  {promotionQuery.data.passed_count}/{promotionQuery.data.total_count}
                </strong>
              </div>
              <div>
                <span>Latest Evaluation</span>
                <strong>{promotionQuery.data.latest_evaluation_id}</strong>
              </div>
              <div>
                <span>Runtime Data</span>
                <strong>{promotionQuery.data.data_status}</strong>
              </div>
              <div>
                <span>Scored Runs</span>
                <strong>{promotionQuery.data.scored_runs}</strong>
              </div>
              <div>
                <span>Blockers</span>
                <strong>{promotionQuery.data.blockers.length}</strong>
              </div>
            </div>
            <div className="table-list">
              {promotionQuery.data.gates.map((gate) => (
                <div className="metric-row compact-metric-row" key={gate.label}>
                  <span>{gate.label}</span>
                  <strong>{gate.passed ? "passed" : gate.blocker}</strong>
                </div>
              ))}
            </div>
            <div className="result-stack">
              <label>
                Reviewer
                <input value={reviewer} onChange={(event) => setReviewer(event.target.value)} />
              </label>
              <label>
                Governance Note
                <textarea value={notes} onChange={(event) => setNotes(event.target.value)} />
              </label>
              <div className="button-row">
                <button
                  onClick={() => reviewMutation.mutate("approved")}
                  disabled={reviewMutation.isPending}
                >
                  Approve
                </button>
                <button
                  onClick={() => reviewMutation.mutate("rejected")}
                  disabled={reviewMutation.isPending}
                >
                  Reject
                </button>
              </div>
              {reviewMutation.error ? (
                <pre className="error">{String(reviewMutation.error.message)}</pre>
              ) : null}
            </div>
          </>
        ) : (
          <p className="empty">No promotion gate data loaded</p>
        )}
      </div>
    </section>
  );
}
