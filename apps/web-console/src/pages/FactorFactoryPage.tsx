import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listDatasets, listFactorReadiness } from "../api";

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

type FactorReadiness = {
  dataset_count: number;
  factor_count: number;
  label_count: number;
  entity_key_count: number;
  data_quality_score: number;
  data_quality_status: string;
  online_ready_count: number;
  rule_convertible_count: number;
  mapped_factor_count: number;
  high_missing_count: number;
  unstable_factor_count: number;
  unowned_factor_count: number;
  ready_factor_count?: number;
  review_factor_count?: number;
  readiness_issue_counts?: Record<string, number>;
  factor_cards?: ApiFactorCard[];
};

type ApiFactorCard = {
  dataset_id: string;
  dataset_key: string;
  dataset_version: string;
  factor_name: string;
  chinese_name: string;
  entity_type: string;
  semantic_role: string;
  logical_type: string;
  calculation_window: string;
  calculation_logic: string;
  source_table: string;
  source_fields: string[];
  business_meaning: string;
  risk_direction: string;
  missing_rate: number | null;
  iv: number | null;
  auc_gain: number | null;
  lift: number | null;
  psi: number | null;
  stability: string;
  model_contribution: number | null;
  rule_convertible: boolean;
  online_available: boolean;
  readiness_status: "ready" | "needs_review";
  readiness_issues: string[];
  version: string;
  owner: string;
  is_label: boolean;
  is_entity_key: boolean;
  evidence_refs: string[];
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
  source_lineage_label: string;
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
  readiness_issues: string[];
  is_label: boolean;
  is_entity_key: boolean;
  top_values: string[];
};

export function buildFactorReadinessSummary(readiness?: FactorReadiness) {
  const factorCount = readiness?.factor_count ?? 0;
  const onlineReadyCount = readiness?.online_ready_count ?? 0;
  const reviewQueueCount =
    (readiness?.high_missing_count ?? 0) +
    (readiness?.unstable_factor_count ?? 0) +
    (readiness?.unowned_factor_count ?? 0);
  return {
    datasetCount: readiness?.dataset_count ?? 0,
    factorCount,
    onlineReadyCount,
    dataQualityScoreLabel: `${((readiness?.data_quality_score ?? 0) * 100).toFixed(1)}%`,
    dataQualityStatus: readiness?.data_quality_status ?? "empty",
    ruleConvertibleCount: readiness?.rule_convertible_count ?? 0,
    mappedFactorCount: readiness?.mapped_factor_count ?? 0,
    readyFactorCount: readiness?.ready_factor_count ?? 0,
    reviewFactorCount: readiness?.review_factor_count ?? reviewQueueCount,
    topReadinessIssues: Object.entries(readiness?.readiness_issue_counts ?? {})
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .map(([issue, count]) => `${issue}: ${count}`),
    reviewQueueCount,
    onlineReadyRateLabel:
      factorCount === 0 ? "0.0%" : `${((onlineReadyCount / factorCount) * 100).toFixed(1)}%`,
  };
}

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
      source_lineage_label: `${field.profile_json.source_table ?? dataset.dataset_key}.${
        (field.profile_json.source_fields ?? [field.field_name]).join(",")
      }`,
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
      readiness_issues: [],
      is_label: isLabel,
      is_entity_key: isEntityKey,
      top_values: (field.profile_json.top_values ?? []).map(
        (item) => `${String(item.value)} (${item.count})`,
      ),
    };
  });
}

export function buildApiFactorCards(cards: ApiFactorCard[], datasetId?: string): FactorCard[] {
  return cards
    .filter((card) => !datasetId || card.dataset_id === datasetId)
    .map((card) => ({
      factor_name: card.factor_name,
      display_label: card.chinese_name,
      semantic_role: card.semantic_role,
      logical_type: card.logical_type,
      description: card.business_meaning,
      business_meaning: card.business_meaning,
      risk_direction: card.risk_direction,
      calculation_window: card.calculation_window,
      calculation_logic: card.calculation_logic,
      source_table: card.source_table,
      source_fields: card.source_fields,
      source_lineage_label: `${card.source_table}.${card.source_fields.join(",")}`,
      owner: card.owner || "unassigned",
      version: card.version,
      missing_rate_label:
        card.missing_rate === null ? "-" : `${(card.missing_rate * 100).toFixed(1)}%`,
      iv_label: formatMetric(card.iv ?? undefined, 3),
      auc_gain_label: formatMetric(card.auc_gain ?? undefined, 3),
      lift_label: card.lift === null ? "-" : `${card.lift.toFixed(2)}x`,
      stability_label: card.stability,
      model_contribution_label: formatPercent(card.model_contribution ?? undefined),
      online_status:
        card.readiness_status === "ready" && card.online_available && !card.is_label
          ? "ready"
          : "review",
      online_available: card.online_available,
      convertible_to_rule: card.rule_convertible,
      readiness_issues: card.readiness_issues,
      is_label: card.is_label,
      is_entity_key: card.is_entity_key,
      top_values: [],
    }));
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
  const readinessQuery = useQuery({
    queryKey: ["factor-readiness", apiKey],
    queryFn: () => listFactorReadiness(apiKey) as Promise<FactorReadiness>,
  });
  const selectedDataset = useMemo(
    () =>
      datasetsQuery.data?.datasets.find((dataset) => dataset.dataset_id === selectedDatasetId) ??
      datasetsQuery.data?.datasets[0],
    [datasetsQuery.data?.datasets, selectedDatasetId],
  );
  const factorCards = useMemo(() => {
    const apiCards = buildApiFactorCards(
      readinessQuery.data?.factor_cards ?? [],
      selectedDataset?.dataset_id,
    );
    return apiCards.length > 0
      ? apiCards
      : selectedDataset
        ? buildFactorCards(selectedDataset)
        : [];
  }, [readinessQuery.data?.factor_cards, selectedDataset]);
  const readinessSummary = buildFactorReadinessSummary(readinessQuery.data);

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
        {readinessQuery.error ? (
          <pre className="error">{String(readinessQuery.error.message)}</pre>
        ) : null}
        <div className="summary-grid">
          <div>
            <span>Datasets</span>
            <strong>{readinessSummary.datasetCount}</strong>
          </div>
          <div>
            <span>Factors</span>
            <strong>{readinessSummary.factorCount}</strong>
          </div>
          <div>
            <span>Online Ready</span>
            <strong>{readinessSummary.onlineReadyCount}</strong>
          </div>
          <div>
            <span>Ready Rate</span>
            <strong>{readinessSummary.onlineReadyRateLabel}</strong>
          </div>
          <div>
            <span>Data Quality</span>
            <strong>{readinessSummary.dataQualityScoreLabel}</strong>
          </div>
          <div>
            <span>Quality Status</span>
            <strong>{readinessSummary.dataQualityStatus}</strong>
          </div>
          <div>
            <span>Rule Ready</span>
            <strong>{readinessSummary.ruleConvertibleCount}</strong>
          </div>
          <div>
            <span>Review Queue</span>
            <strong>{readinessSummary.reviewQueueCount}</strong>
          </div>
          <div>
            <span>Ready Factors</span>
            <strong>{readinessSummary.readyFactorCount}</strong>
          </div>
          <div>
            <span>Review Factors</span>
            <strong>{readinessSummary.reviewFactorCount}</strong>
          </div>
        </div>
        {readinessSummary.topReadinessIssues.length > 0 ? (
          <ul className="result-list compact-list">
            {readinessSummary.topReadinessIssues.slice(0, 4).map((issue) => (
              <li key={issue}>{issue}</li>
            ))}
          </ul>
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
              <small>Source fields: {factor.source_lineage_label}</small>
              {factor.readiness_issues.length > 0 ? (
                <ul className="result-list compact-list">
                  {factor.readiness_issues.map((issue) => (
                    <li key={issue}>{issue}</li>
                  ))}
                </ul>
              ) : null}
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
