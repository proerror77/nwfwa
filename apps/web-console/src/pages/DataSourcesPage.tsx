import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getDataset, listDatasets, listModelEvaluations } from "../api";

export type FieldMappingRecord = {
  mapping_id: string;
  dataset_id: string;
  external_field: string;
  canonical_target: string;
  feature_name?: string | null;
  transform_kind: string;
  status: string;
};

export type DatasetRecord = {
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
  mappings?: FieldMappingRecord[];
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

type ModelEvaluationLineageRecord = {
  evaluation_run_id: string;
  model_key: string;
  model_version: string;
  model_dataset_id: string;
  source_dataset_id: string | null;
  source_dataset_key: string | null;
  source_dataset_version: string | null;
  source_data_quality_score: number | null;
  source_data_quality_status: string | null;
};

export type DatasetModelLineageRow = {
  evaluationRunId: string;
  modelLabel: string;
  modelDatasetId: string;
  sourceDatasetLabel: string;
  dataQualityLabel: string;
  dataQualityStatus: string;
  metricLabel: string;
};

export function buildDatasetMappingSummary(mappings: FieldMappingRecord[] = []) {
  const activeMappings = mappings.filter((mapping) => mapping.status === "active");
  const featureMappings = mappings.filter((mapping) => mapping.feature_name?.trim());
  const transformKinds = new Set(mappings.map((mapping) => mapping.transform_kind));
  return {
    mappingCount: mappings.length,
    activeMappingCount: activeMappings.length,
    featureMappingCount: featureMappings.length,
    transformKindCount: transformKinds.size,
    activeCoverageLabel:
      mappings.length === 0
        ? "0.0%"
        : `${((activeMappings.length / mappings.length) * 100).toFixed(1)}%`,
  };
}

export function buildDatasetFieldGovernanceSummary(dataset?: DatasetRecord | null) {
  const fields = dataset?.fields ?? [];
  const roleCounts = fields.reduce<Record<string, number>>((counts, field) => {
    counts[field.semantic_role] = (counts[field.semantic_role] ?? 0) + 1;
    return counts;
  }, {});
  return {
    fieldCount: fields.length,
    keyCount: roleCounts.key ?? 0,
    featureCount: roleCounts.feature ?? 0,
    labelCount: roleCounts.label ?? 0,
    partitionCount: roleCounts.partition ?? 0,
    ignoredCount: roleCounts.ignored ?? 0,
    leakageCandidateCount: roleCounts.leakage_candidate ?? 0,
    roleCoverageLabel:
      fields.length === 0
        ? "0.0%"
        : `${(
            (fields.filter((field) => field.semantic_role.trim().length > 0).length /
              fields.length) *
            100
          ).toFixed(1)}%`,
  };
}

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

export function buildDatasetModelLineageRows(
  dataset?: DatasetRecord | null,
  lineage: ModelEvaluationLineageRecord[] = [],
  evaluations: ModelEvaluationRecord[] = [],
): DatasetModelLineageRow[] {
  if (!dataset) return [];
  const evaluationsByRun = new Map(
    evaluations.map((evaluation) => [evaluation.evaluation_run_id, evaluation]),
  );
  return lineage
    .filter((row) => row.source_dataset_id === dataset.dataset_id)
    .map((row) => {
      const evaluation = evaluationsByRun.get(row.evaluation_run_id);
      return {
        evaluationRunId: row.evaluation_run_id,
        modelLabel: `${row.model_key}:${row.model_version}`,
        modelDatasetId: row.model_dataset_id,
        sourceDatasetLabel: `${row.source_dataset_key ?? dataset.dataset_key}:${
          row.source_dataset_version ?? dataset.dataset_version
        }`,
        dataQualityLabel:
          row.source_data_quality_score == null
            ? "-"
            : `${(row.source_data_quality_score * 100).toFixed(1)}%`,
        dataQualityStatus: row.source_data_quality_status ?? "unknown",
        metricLabel: `AUC ${evaluation?.auc ?? "-"} · F1 ${evaluation?.f1 ?? "-"}`,
      };
    });
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
      listModelEvaluations(apiKey) as Promise<{
        evaluations: ModelEvaluationRecord[];
        lineage: ModelEvaluationLineageRecord[];
      }>,
  });
  const selectedDataset = useMemo(
    () =>
      datasetsQuery.data?.datasets.find((dataset) => dataset.dataset_id === selectedDatasetId) ??
      datasetsQuery.data?.datasets[0],
    [datasetsQuery.data?.datasets, selectedDatasetId],
  );
  const selectedDatasetDetailQuery = useQuery({
    queryKey: ["dataset-detail", selectedDataset?.dataset_id, apiKey],
    queryFn: () => getDataset(selectedDataset!.dataset_id, apiKey) as Promise<DatasetRecord>,
    enabled: Boolean(selectedDataset?.dataset_id),
  });
  const selectedDatasetDetail = selectedDatasetDetailQuery.data ?? selectedDataset;
  const selectedDatasetHealth = useMemo(
    () =>
      datasetsQuery.data?.health.find(
        (health) => health.dataset_id === selectedDataset?.dataset_id,
      ),
    [datasetsQuery.data?.health, selectedDataset?.dataset_id],
  );
  const healthSummary = buildDatasetHealthSummary(selectedDatasetHealth);
  const fieldGovernanceSummary = buildDatasetFieldGovernanceSummary(selectedDatasetDetail);
  const mappingSummary = buildDatasetMappingSummary(selectedDatasetDetail?.mappings);
  const modelLineageRows = buildDatasetModelLineageRows(
    selectedDatasetDetail,
    evaluationsQuery.data?.lineage,
    evaluationsQuery.data?.evaluations,
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
            {selectedDatasetDetailQuery.error ? (
              <pre className="error">{String(selectedDatasetDetailQuery.error.message)}</pre>
            ) : null}
            <dl className="result-grid">
              <div>
                <dt>Dataset</dt>
                <dd>{selectedDatasetDetail?.dataset_key ?? selectedDataset.dataset_key}</dd>
              </div>
              <div>
                <dt>Version</dt>
                <dd>{selectedDatasetDetail?.dataset_version ?? selectedDataset.dataset_version}</dd>
              </div>
              <div>
                <dt>Grain</dt>
                <dd>{selectedDatasetDetail?.sample_grain ?? selectedDataset.sample_grain}</dd>
              </div>
              <div>
                <dt>Label</dt>
                <dd>{selectedDatasetDetail?.label_column ?? selectedDataset.label_column}</dd>
              </div>
              <div>
                <dt>Rows</dt>
                <dd>{selectedDatasetDetail?.row_count ?? selectedDataset.row_count}</dd>
              </div>
              <div>
                <dt>Format</dt>
                <dd>{selectedDatasetDetail?.storage_format ?? selectedDataset.storage_format}</dd>
              </div>
            </dl>
            <h3>Field Governance</h3>
            <div className="summary-grid">
              <div>
                <span>Fields</span>
                <strong>{fieldGovernanceSummary.fieldCount}</strong>
              </div>
              <div>
                <span>Role Coverage</span>
                <strong>{fieldGovernanceSummary.roleCoverageLabel}</strong>
              </div>
              <div>
                <span>Features</span>
                <strong>{fieldGovernanceSummary.featureCount}</strong>
              </div>
              <div>
                <span>Labels</span>
                <strong>{fieldGovernanceSummary.labelCount}</strong>
              </div>
              <div>
                <span>Keys</span>
                <strong>{fieldGovernanceSummary.keyCount}</strong>
              </div>
              <div>
                <span>Leakage Candidates</span>
                <strong>{fieldGovernanceSummary.leakageCandidateCount}</strong>
              </div>
            </div>
            <dl className="result-grid">
              <div>
                <dt>Partitions</dt>
                <dd>{fieldGovernanceSummary.partitionCount}</dd>
              </div>
              <div>
                <dt>Ignored</dt>
                <dd>{fieldGovernanceSummary.ignoredCount}</dd>
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
              <div>
                <span>Mappings</span>
                <strong>{mappingSummary.mappingCount}</strong>
              </div>
              <div>
                <span>Active Mappings</span>
                <strong>{mappingSummary.activeMappingCount}</strong>
              </div>
              <div>
                <span>Feature Mappings</span>
                <strong>{mappingSummary.featureMappingCount}</strong>
              </div>
              <div>
                <span>Active Coverage</span>
                <strong>{mappingSummary.activeCoverageLabel}</strong>
              </div>
              <div>
                <span>Transform Kinds</span>
                <strong>{mappingSummary.transformKindCount}</strong>
              </div>
            </div>
            <div className="table-list">
              {(selectedDatasetDetail?.mappings ?? []).slice(0, 8).map((mapping) => (
                <div className="metric-row compact-metric-row" key={mapping.mapping_id}>
                  <span>{mapping.external_field}</span>
                  <strong>{mapping.status}</strong>
                  <small>{mapping.canonical_target}</small>
                  <small>
                    {mapping.feature_name ?? "no feature"} · {mapping.transform_kind}
                  </small>
                </div>
              ))}
            </div>
            {(selectedDatasetDetail?.mappings ?? []).length === 0 ? (
              <p className="empty">No field mappings registered</p>
            ) : null}
            <div className="summary-grid">
              {(selectedDatasetDetail?.splits ?? selectedDataset.splits).map((split) => (
                <div key={split.split_name}>
                  <span>{split.split_name}</span>
                  <strong>{split.row_count}</strong>
                </div>
              ))}
            </div>
            <ul className="result-list compact-list">
              {(selectedDatasetDetail?.fields ?? selectedDataset.fields).slice(0, 12).map((field) => (
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
        <h2>Model Lineage</h2>
        {evaluationsQuery.error ? (
          <pre className="error">{String(evaluationsQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Linked Evaluations</span>
            <strong>{modelLineageRows.length}</strong>
          </div>
          <div>
            <span>Linked Models</span>
            <strong>{new Set(modelLineageRows.map((row) => row.modelLabel)).size}</strong>
          </div>
        </div>
        <div className="table-list">
          {modelLineageRows.map((row) => (
            <div className="metric-row" key={row.evaluationRunId}>
              <span>{row.evaluationRunId}</span>
              <strong>{row.modelLabel}</strong>
              <small>{row.metricLabel}</small>
              <small>
                {row.sourceDatasetLabel} · DQ {row.dataQualityLabel} ·{" "}
                {row.dataQualityStatus}
              </small>
            </div>
          ))}
        </div>
        {modelLineageRows.length === 0 ? (
          <p className="empty">No model evaluations linked to this dataset</p>
        ) : null}
      </div>
    </section>
  );
}
