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

export type DatasetHealthRecord = {
  dataset_id: string;
  dataset_key: string;
  dataset_version: string;
  data_quality_score: number;
  data_quality_status: string;
  field_count: number;
  label_count: number;
  entity_key_count: number;
  high_missing_count: number;
  unstable_field_count: number;
  unowned_field_count: number;
  online_ready_count: number;
  issue_count: number;
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

export function buildDatasetHealthSummary(health?: DatasetHealthRecord | null) {
  if (!health) {
    return {
      dataQualityScoreLabel: "-",
      dataQualityStatus: "empty",
      issueCount: 0,
      highMissingCount: 0,
      unstableFieldCount: 0,
      unownedFieldCount: 0,
      onlineReadyRateLabel: "-",
    };
  }

  const onlineReadyRate =
    health.field_count === 0 ? 0 : health.online_ready_count / health.field_count;

  return {
    dataQualityScoreLabel: `${(health.data_quality_score * 100).toFixed(1)}%`,
    dataQualityStatus: health.data_quality_status,
    issueCount: health.issue_count,
    highMissingCount: health.high_missing_count,
    unstableFieldCount: health.unstable_field_count,
    unownedFieldCount: health.unowned_field_count,
    onlineReadyRateLabel: `${(onlineReadyRate * 100).toFixed(1)}%`,
  };
}

export function DataSourcesPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedDatasetId, setSelectedDatasetId] = useState<string | null>(null);
  const datasetsQuery = useQuery({
    queryKey: ["datasets", apiKey],
    queryFn: () =>
      listDatasets(apiKey) as Promise<{
        datasets: DatasetRecord[];
        health: DatasetHealthRecord[];
      }>,
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
  const selectedDatasetHealth = useMemo(
    () =>
      datasetsQuery.data?.health.find(
        (health) => health.dataset_id === selectedDataset?.dataset_id,
      ),
    [datasetsQuery.data?.health, selectedDataset?.dataset_id],
  );
  const healthSummary = buildDatasetHealthSummary(selectedDatasetHealth);

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
              <div>
                <span>DQ Score</span>
                <strong>{healthSummary.dataQualityScoreLabel}</strong>
              </div>
              <div>
                <span>DQ Status</span>
                <strong>{healthSummary.dataQualityStatus}</strong>
              </div>
              <div>
                <span>Issue Count</span>
                <strong>{healthSummary.issueCount}</strong>
              </div>
              <div>
                <span>Online Ready</span>
                <strong>{healthSummary.onlineReadyRateLabel}</strong>
              </div>
            </div>
            <dl className="result-grid">
              <div>
                <dt>High Missing</dt>
                <dd>{healthSummary.highMissingCount}</dd>
              </div>
              <div>
                <dt>Unstable Fields</dt>
                <dd>{healthSummary.unstableFieldCount}</dd>
              </div>
              <div>
                <dt>Unowned Fields</dt>
                <dd>{healthSummary.unownedFieldCount}</dd>
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
