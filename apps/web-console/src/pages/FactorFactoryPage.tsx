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
    business_meaning?: string;
    risk_direction?: string;
    calculation_window?: string;
    calculation_logic?: string;
    source_table?: string;
    source_fields?: string[];
    owner?: string;
    version?: string | number;
    iv?: number;
    auc_gain?: number;
    lift?: number;
    psi?: number;
    model_contribution?: number;
    convertible_to_rule?: boolean;
    online_available?: boolean;
    top_values?: ProfileValue[];
  };
};

type DatasetForFactors = {
  dataset_id: string;
  dataset_key: string;
  sample_grain: string;
  entity_keys: string[];
  fields: SchemaField[];
};

export type FactorCard = {
  factor_name: string;
  display_label: string;
  semantic_role: string;
  logical_type: string;
  description: string;
  business_meaning: string;
  risk_direction: string;
  calculation_window: string;
  calculation_logic: string;
  source_table: string;
  source_fields: string[];
  owner: string;
  version: string;
  missing_rate_label: string;
  iv_label: string;
  auc_gain_label: string;
  lift_label: string;
  stability_label: string;
  model_contribution_label: string;
  online_status: "ready" | "review";
  online_available: boolean;
  convertible_to_rule: boolean;
  is_label: boolean;
  is_entity_key: boolean;
  top_values: string[];
};

export function buildFactorCards(dataset: DatasetForFactors): FactorCard[] {
  return dataset.fields.map((field) => {
    const missingRate = field.profile_json.missing_rate ?? null;
    const isLabel = field.semantic_role === "label";
    const isEntityKey = dataset.entity_keys.includes(field.field_name);
    const onlineAvailable = field.profile_json.online_available ?? (!isLabel && !field.nullable);
    const convertibleToRule =
      field.profile_json.convertible_to_rule ?? (!isLabel && isRuleConvertibleType(field.logical_type));
    return {
      factor_name: field.field_name,
      display_label: titleize(field.field_name),
      semantic_role: field.semantic_role,
      logical_type: field.logical_type,
      description: field.description,
      business_meaning: field.profile_json.business_meaning ?? field.description,
      risk_direction: field.profile_json.risk_direction ?? (isLabel ? "label" : "unknown"),
      calculation_window: field.profile_json.calculation_window ?? dataset.sample_grain,
      calculation_logic: field.profile_json.calculation_logic ?? "registered_dataset_field",
      source_table: field.profile_json.source_table ?? dataset.dataset_key,
      source_fields: field.profile_json.source_fields ?? [field.field_name],
      owner: field.profile_json.owner ?? "unassigned",
      version: formatVersion(field.profile_json.version),
      missing_rate_label: missingRate === null ? "-" : `${(missingRate * 100).toFixed(1)}%`,
      iv_label: formatMetric(field.profile_json.iv, 3),
      auc_gain_label: formatMetric(field.profile_json.auc_gain, 3),
      lift_label:
        field.profile_json.lift === undefined ? "-" : `${field.profile_json.lift.toFixed(2)}x`,
      stability_label: stabilityLabel(field.profile_json.psi),
      model_contribution_label: formatPercent(field.profile_json.model_contribution),
      online_status: onlineAvailable && !isLabel && (missingRate ?? 1) <= 0.05 ? "ready" : "review",
      online_available: onlineAvailable && !isLabel,
      convertible_to_rule: convertibleToRule && !isLabel,
      is_label: isLabel,
      is_entity_key: isEntityKey,
      top_values: (field.profile_json.top_values ?? []).map(
        (item) => `${String(item.value)} (${item.count})`,
      ),
    };
  });
}

function isRuleConvertibleType(logicalType: string) {
  return ["decimal", "float", "float64", "int", "int8", "int32", "int64", "boolean"].includes(
    logicalType,
  );
}

function formatMetric(value: number | undefined, digits: number) {
  return value === undefined ? "-" : value.toFixed(digits);
}

function formatPercent(value: number | undefined) {
  return value === undefined ? "-" : `${(value * 100).toFixed(1)}%`;
}

function formatVersion(value: string | number | undefined) {
  if (value === undefined) {
    return "v1";
  }
  return typeof value === "number" ? `v${value}` : value;
}

function stabilityLabel(psi: number | undefined) {
  if (psi === undefined) {
    return "unmeasured";
  }
  if (psi < 0.1) {
    return "stable";
  }
  if (psi < 0.25) {
    return "watch";
  }
  return "drift";
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
              <dl className="result-grid">
                <div>
                  <dt>Risk Direction</dt>
                  <dd>{factor.risk_direction}</dd>
                </div>
                <div>
                  <dt>Window</dt>
                  <dd>{factor.calculation_window}</dd>
                </div>
                <div>
                  <dt>Stability</dt>
                  <dd>{factor.stability_label}</dd>
                </div>
                <div>
                  <dt>Contribution</dt>
                  <dd>{factor.model_contribution_label}</dd>
                </div>
                <div>
                  <dt>Rule Ready</dt>
                  <dd>{factor.convertible_to_rule ? "yes" : "no"}</dd>
                </div>
                <div>
                  <dt>Owner</dt>
                  <dd>{factor.owner}</dd>
                </div>
              </dl>
              <p>{factor.business_meaning}</p>
              <small>
                {factor.source_table} / {factor.version} / {factor.is_label ? "label" : "factor"}
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
