import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listDatasets, listModelEvaluations } from "../api";

type DatasetRecord = {
  dataset_id: string;
  source_key: string;
  business_domain: string;
  dataset_key: string;
  dataset_version: string;
  sample_grain: string;
  label_column: string;
  storage_format: string;
  row_count: number;
  status: string;
  splits: Array<{ split_name: string; row_count: number }>;
  fields: Array<{ field_name: string; semantic_role: string }>;
};

type ModelEvaluationRecord = {
  evaluation_run_id: string;
  model_key: string;
  model_version: string;
  model_dataset_id: string;
  auc: string | null;
  ks: string | null;
  precision: string | null;
  recall: string | null;
  f1: string | null;
  accuracy: string | null;
  threshold: string | null;
};

export function DataSourcesPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedDatasetId, setSelectedDatasetId] = useState<string | null>(null);
  const datasetsQuery = useQuery({
    queryKey: ["datasets", apiKey],
    queryFn: () => listDatasets(apiKey) as Promise<{ datasets: DatasetRecord[] }>,
  });
  const evaluationsQuery = useQuery({
    queryKey: ["model-evaluations", apiKey],
    queryFn: () =>
      listModelEvaluations(apiKey) as Promise<{ evaluations: ModelEvaluationRecord[] }>,
  });
  const selectedDataset = useMemo(
    () =>
      datasetsQuery.data?.datasets.find((dataset) => dataset.dataset_id === selectedDatasetId) ??
      datasetsQuery.data?.datasets[0],
    [datasetsQuery.data?.datasets, selectedDatasetId],
  );

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Datasets</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        {datasetsQuery.error ? (
          <pre className="error">{String(datasetsQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {datasetsQuery.data?.datasets.map((dataset) => (
            <button
              className={
                dataset.dataset_id === selectedDataset?.dataset_id ? "row-button active" : "row-button"
              }
              key={dataset.dataset_id}
              onClick={() => setSelectedDatasetId(dataset.dataset_id)}
            >
              <span>{dataset.dataset_key}</span>
              <strong>{dataset.status}</strong>
              <small>{dataset.business_domain}</small>
            </button>
          ))}
        </div>
        {datasetsQuery.data?.datasets.length === 0 ? (
          <p className="empty">No datasets registered</p>
        ) : null}
      </div>

      <div className="panel">
        <h2>Dataset Detail</h2>
        {selectedDataset ? (
          <div className="result-stack">
            <dl className="result-grid">
              <div>
                <dt>Dataset</dt>
                <dd>{selectedDataset.dataset_key}</dd>
              </div>
              <div>
                <dt>Version</dt>
                <dd>{selectedDataset.dataset_version}</dd>
              </div>
              <div>
                <dt>Grain</dt>
                <dd>{selectedDataset.sample_grain}</dd>
              </div>
              <div>
                <dt>Label</dt>
                <dd>{selectedDataset.label_column}</dd>
              </div>
              <div>
                <dt>Rows</dt>
                <dd>{selectedDataset.row_count}</dd>
              </div>
              <div>
                <dt>Format</dt>
                <dd>{selectedDataset.storage_format}</dd>
              </div>
            </dl>
            <div className="summary-grid">
              {selectedDataset.splits.map((split) => (
                <div key={split.split_name}>
                  <span>{split.split_name}</span>
                  <strong>{split.row_count}</strong>
                </div>
              ))}
            </div>
            <ul className="result-list compact-list">
              {selectedDataset.fields.slice(0, 12).map((field) => (
                <li key={field.field_name}>
                  <strong>{field.field_name}</strong>
                  <span>{field.semantic_role}</span>
                </li>
              ))}
            </ul>
          </div>
        ) : (
          <p className="empty">No dataset selected</p>
        )}
      </div>

      <div className="panel wide-panel">
        <h2>Model Evaluations</h2>
        {evaluationsQuery.error ? (
          <pre className="error">{String(evaluationsQuery.error.message)}</pre>
        ) : null}
        <div className="table-list">
          {evaluationsQuery.data?.evaluations.map((evaluation) => (
            <div className="metric-row" key={evaluation.evaluation_run_id}>
              <span>{evaluation.evaluation_run_id}</span>
              <strong>{evaluation.model_key}</strong>
              <small>AUC {evaluation.auc ?? "-"} · KS {evaluation.ks ?? "-"}</small>
              <small>
                F1 {evaluation.f1 ?? "-"} · Threshold {evaluation.threshold ?? "-"}
              </small>
            </div>
          ))}
        </div>
        {evaluationsQuery.data?.evaluations.length === 0 ? (
          <p className="empty">No model evaluations registered</p>
        ) : null}
      </div>
    </section>
  );
}
