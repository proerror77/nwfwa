import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listDatasets } from "../api";

type ProfileValue = {
  value: unknown;
  count: number;
};

type SchemaField = {
  field_name: string;
  logical_type: string;
  nullable: boolean;
  semantic_role: string;
  description: string;
  profile_json: {
    missing_rate?: number;
    top_values?: ProfileValue[];
  };
};

type DatasetForFactors = {
  dataset_id: string;
  dataset_key: string;
  entity_keys: string[];
  fields: SchemaField[];
};

export type FactorCard = {
  factor_name: string;
  display_label: string;
  semantic_role: string;
  logical_type: string;
  description: string;
  missing_rate_label: string;
  online_status: "ready" | "review";
  is_label: boolean;
  is_entity_key: boolean;
  top_values: string[];
};

export function buildFactorCards(dataset: DatasetForFactors): FactorCard[] {
  return dataset.fields.map((field) => {
    const missingRate = field.profile_json.missing_rate ?? null;
    const isLabel = field.semantic_role === "label";
    const isEntityKey = dataset.entity_keys.includes(field.field_name);
    return {
      factor_name: field.field_name,
      display_label: titleize(field.field_name),
      semantic_role: field.semantic_role,
      logical_type: field.logical_type,
      description: field.description,
      missing_rate_label: missingRate === null ? "-" : `${(missingRate * 100).toFixed(1)}%`,
      online_status: !isLabel && (missingRate ?? 1) <= 0.05 ? "ready" : "review",
      is_label: isLabel,
      is_entity_key: isEntityKey,
      top_values: (field.profile_json.top_values ?? []).map(
        (item) => `${String(item.value)} (${item.count})`,
      ),
    };
  });
}

function titleize(value: string) {
  return value
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function FactorFactoryPage() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [selectedDatasetId, setSelectedDatasetId] = useState<string | null>(null);
  const datasetsQuery = useQuery({
    queryKey: ["factor-datasets", apiKey],
    queryFn: () => listDatasets(apiKey) as Promise<{ datasets: DatasetForFactors[] }>,
  });
  const selectedDataset = useMemo(
    () =>
      datasetsQuery.data?.datasets.find((dataset) => dataset.dataset_id === selectedDatasetId) ??
      datasetsQuery.data?.datasets[0],
    [datasetsQuery.data?.datasets, selectedDatasetId],
  );
  const factorCards = selectedDataset ? buildFactorCards(selectedDataset) : [];

  return (
    <section className="ops-grid">
      <div className="panel">
        <h2>Factor Factory</h2>
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
              <strong>{dataset.fields.length}</strong>
              <small>{dataset.entity_keys.join(", ")}</small>
            </button>
          ))}
        </div>
        {datasetsQuery.data?.datasets.length === 0 ? (
          <p className="empty">No profiled datasets</p>
        ) : null}
      </div>

      <div className="panel">
        <h2>Factor Cards</h2>
        <div className="factor-card-grid">
          {factorCards.map((factor) => (
            <article className="factor-card" key={factor.factor_name}>
              <div>
                <strong>{factor.display_label}</strong>
                <span>{factor.factor_name}</span>
              </div>
              <dl className="result-grid">
                <div>
                  <dt>Role</dt>
                  <dd>{factor.semantic_role}</dd>
                </div>
                <div>
                  <dt>Missing</dt>
                  <dd>{factor.missing_rate_label}</dd>
                </div>
                <div>
                  <dt>Status</dt>
                  <dd>{factor.online_status}</dd>
                </div>
                <div>
                  <dt>Type</dt>
                  <dd>{factor.logical_type}</dd>
                </div>
              </dl>
              <p>{factor.description}</p>
              <small>
                {factor.is_label ? "label" : "factor"}
                {factor.is_entity_key ? " / entity key" : ""}
              </small>
              {factor.top_values.length > 0 ? (
                <ul className="result-list compact-list">
                  {factor.top_values.slice(0, 4).map((value) => (
                    <li key={value}>{value}</li>
                  ))}
                </ul>
              ) : null}
            </article>
          ))}
        </div>
        {!datasetsQuery.isLoading && factorCards.length === 0 ? (
          <p className="empty">No factor cards available</p>
        ) : null}
      </div>
    </section>
  );
}
