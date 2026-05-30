# External Data And Parquet Dataset Contract

Date: 2026-05-27

Status: proposed

## Context

The legacy 20211105 AutoML material is useful as a data workflow reference, not as a direct FWA claim schema. Its sample files describe a renewal-retention prediction task:

- grain: one policy/order sample row
- entity keys: `policy_no`, `order_no`
- label: `m_2_keep_status`
- task type: binary classification
- common signal groups: applicant demographics, channel/media, region/city tier, premium, payment/withholding events, education/contact events, and historical conversion rates

That data shape is different from the current MVP runtime shape:

```text
claim/member/policy/provider -> FWA features -> rules/model -> score/audit
```

The platform therefore needs an external data contract before it tries to match external data into FWA domain objects. The contract must answer:

1. What external data source did we receive?
2. What is the row grain and business label?
3. What physical files and schemas were imported?
4. Which fields map to canonical entities or features?
5. Which feature set and model dataset version used those fields?
6. Which runtime endpoint, if any, can consume the result?

## Decision

Use Parquet as the first-class dataset format for analytical, training, validation, and feature-matrix data. Do not build the platform around CSV import scripts.

CSV can exist only as a temporary legacy source outside the platform boundary. Before the platform imports a dataset, the dataset should be converted to a governed Parquet dataset with a manifest and schema profile.

## Why Parquet

Parquet is a better platform boundary than CSV for this project because it preserves typed columns, supports column pruning, compresses well, scales to partitioned datasets, and is compatible with Python, Rust, DuckDB, Spark, Polars, Arrow, and most warehouse tools.

CSV is still acceptable for ad hoc inspection or one-time legacy conversion, but it should not be the registered dataset format inside `nwfwa`.

## Storage Model

Separate row-level analytical data from operational metadata.

```text
object storage / local data lake
  landing/
    external source as received, normalized to Parquet
  canonical/
    canonical analytical samples, still business-domain-specific
  features/
    model-ready feature matrices
  predictions/
    offline model outputs and scoring exports

PostgreSQL
  dataset catalog, schemas, mappings, feature definitions, evaluation runs,
  runtime binding, audit metadata
```

PostgreSQL should not store every external training row as JSONB. It should store catalog and governance metadata plus pointers to Parquet files. The current `feature_values`, `model_scores`, and `scoring_runs` tables remain for online runtime scoring decisions, not bulk training data.

## Parquet Dataset Layout

Each registered dataset version is a directory, not a single file:

```text
data/
  external/
    renewal_automl_20211105/
      v1/
        manifest.json
        schema.json
        profile.json
        split=train/
          part-00000.parquet
        split=validation/
          part-00000.parquet
```

Required manifest fields:

- `dataset_key`: stable key, for example `renewal_automl_20211105`
- `dataset_version`: immutable version, for example `v1`
- `business_domain`: for example `renewal_retention`
- `sample_grain`: for example `policy_order`
- `label_column`: for example `m_2_keep_status`
- `entity_keys`: for example `["policy_no", "order_no"]`
- `splits`: train, validation, test, production, or backtest
- `row_count_by_split`
- `schema_hash`
- `source_files`
- `created_at`

Required schema profile fields:

- field name
- logical type
- nullable flag
- missing count and missing rate by split
- distinct count when cheap enough
- top values for low-cardinality categorical fields
- numeric min, max, mean, and quantiles when applicable
- label distribution by split

## Type Rules

Identifiers must be strings in Parquet, even when the source file looks numeric:

- `policy_no`: string
- `order_no`: string
- external member/provider/policy/claim ids: string

This avoids Excel/scientific-notation corruption such as `6.40111E+24`.

Recommended logical types:

- binary flags: boolean when semantically true/false, or int8 when the original values are model-coded `0/1`
- money and premium fields: decimal, with currency when available
- dates and event timestamps: date or timestamp, not strings
- categorical fields: utf8 string first; dictionary encoding is a physical optimization
- labels: typed and named explicitly, for example `m_2_keep_status` as int8 with allowed values `0` and `1`

## PostgreSQL Catalog

Add a dataset catalog layer rather than extending `claims` directly.

### `external_data_sources`

Registers a source system or historical dataset family.

- `source_key`
- `display_name`
- `business_domain`
- `owner`
- `description`
- `status`
- `created_at`
- `updated_at`

### `external_dataset_versions`

Registers one immutable dataset version.

- `dataset_id`
- `source_key`
- `dataset_key`
- `dataset_version`
- `sample_grain`
- `label_column`
- `manifest_uri`
- `schema_uri`
- `profile_uri`
- `storage_format`: `parquet`
- `schema_hash`
- `row_count`
- `status`
- `created_at`

### `external_dataset_splits`

Stores split-level counts and label distribution.

- `dataset_id`
- `split_name`
- `data_uri`
- `row_count`
- `positive_count`
- `negative_count`
- `label_distribution_json`

### `external_schema_fields`

Stores field-level data dictionary and profiling metadata.

- `dataset_id`
- `field_name`
- `logical_type`
- `nullable`
- `semantic_role`: key, feature, label, partition, ignored, leakage_candidate
- `description`
- `profile_json`

### `external_field_mappings`

Defines how external fields map into canonical entities or features.

- `mapping_id`
- `dataset_id`
- `external_field`
- `canonical_target`: for example `policy.premium_amount`, `member.age`, or `feature.channel_media`
- `feature_name`
- `transform_kind`: direct, cast, enum_map, derived, aggregate
- `transform_json`
- `status`

### `feature_definitions`

Defines reusable feature contracts.

- `feature_name`
- `business_domain`
- `version`
- `value_type`
- `source_fields_json`
- `calculation_kind`
- `description`
- `owner`
- `status`

### `feature_set_versions`

Defines a model-ready feature matrix stored in Parquet.

- `feature_set_id`
- `business_domain`
- `feature_set_key`
- `version`
- `dataset_id`
- `features_uri`
- `feature_list_json`
- `row_count`
- `label_column`
- `status`

### `model_dataset_versions`

Defines the train/validation/test dataset used by a model experiment.

- `model_dataset_id`
- `business_domain`
- `task_type`
- `label_name`
- `feature_set_id`
- `train_uri`
- `validation_uri`
- `test_uri`
- `row_counts_json`
- `label_distribution_json`
- `status`

### `model_evaluation_runs`

Persists offline evaluation metrics.

- `evaluation_run_id`
- `model_key`
- `model_version`
- `model_dataset_id`
- `auc`
- `ks`
- `precision`
- `recall`
- `f1`
- `accuracy`
- `threshold`
- `confusion_matrix_json`
- `feature_importance_uri`
- `metrics_json`
- `created_at`

## Mapping Into Current Runtime

External datasets do not automatically become FWA runtime objects.

Use this matching sequence:

```text
external Parquet dataset
-> dataset catalog
-> schema profile
-> field mappings
-> canonical analytical sample
-> feature set
-> model dataset
-> evaluation run
-> optional runtime binding
```

Only after a business domain is approved for online serving should it receive a runtime endpoint.

Examples:

- FWA claim risk data can bind to `POST /api/v1/claims/score` if it maps to `ClaimContext`.
- Renewal-retention data should not bind to `POST /api/v1/claims/score`.
- Renewal-retention can later receive a separate endpoint such as `POST /api/v1/policies/renewal-score`.

## Legacy Renewal Dataset Registration

The 20211105 sample should be registered as:

- `source_key`: `renewal_automl_20211105`
- `business_domain`: `renewal_retention`
- `sample_grain`: `policy_order`
- `entity_keys`: `policy_no`, `order_no`
- `label_column`: `m_2_keep_status`
- `splits`: train and validation
- `storage_format`: `parquet`

Initial high-value field mappings:

- `policy_no` -> canonical key `policy.external_policy_id`
- `order_no` -> canonical key `order.external_order_id`
- `age` -> feature `member_age`
- `applicant_age` -> feature `applicant_age`
- `channel_media` -> feature `channel_media`
- `channel_media_delivery_rate` -> feature `channel_media_delivery_rate`
- `channel_source_delivery_rate` -> feature `channel_source_delivery_rate`
- `sum_premium` -> feature `sum_premium`
- `issue_rate` -> feature `issue_rate`
- `first_day_sign_status_no_out` -> feature `first_day_sign_status_no_out`
- `is_out_withhold_30` -> feature `is_out_withhold_30`
- `is_success_policy_1` -> feature `is_success_policy_1`
- `m_2_keep_status` -> label `renewal_m2_keep_status`

## Implementation Plan

### Phase A: Contract And Catalog

1. Add the PostgreSQL catalog tables listed above.
2. Add repository methods for registering dataset versions, splits, fields, mappings, feature sets, model datasets, and evaluation runs.
3. Add API routes for dataset catalog read/write:
   - `POST /api/v1/ops/datasets`
   - `GET /api/v1/ops/datasets`
   - `GET /api/v1/ops/datasets/{dataset_id}`
   - `POST /api/v1/ops/datasets/{dataset_id}/mappings`

Verification:

- migration applies cleanly
- repository tests cover dataset registration and mapping creation
- OpenAPI includes dataset catalog paths

### Phase B: Parquet Profile Boundary

1. Add a Parquet profiler command or worker job.
2. Input is a Parquet dataset directory and manifest, not a CSV path.
3. Output is `schema.json`, `profile.json`, and catalog rows.
4. Show Factor Factory cards with source lineage, readiness, and predictive metrics such as IV, AUC gain, lift, stability, and model contribution.

Verification:

- profiler rejects non-Parquet dataset manifests
- ids remain strings
- label distribution and missing rates match the registered profile
- Factor Factory exposes factor card evaluation metrics without inventing missing values

### Phase C: Feature Matrix

1. Compile mapped fields into feature-set Parquet.
2. Register the feature set in `feature_set_versions`.
3. Register model dataset versions that point to train and validation Parquet URIs.

Verification:

- feature list is immutable by feature-set version
- train/validation row counts and label distributions are recorded
- feature matrix does not overwrite original landing data

### Phase D: Model Evaluation

1. Store evaluation metrics in `model_evaluation_runs`.
2. Link metrics back to `model_dataset_versions`.
3. Show dataset and evaluation summaries in Operations Studio.

Verification:

- model metrics cannot be reported without a registered model dataset
- feature importance is stored as an artifact URI, not a large JSON blob when it is large

## Non-Goals

- No general CSV ingestion endpoint.
- No automatic mapping from renewal-retention samples into FWA claim scoring.
- No online serving endpoint for renewal-retention until its runtime contract is separately designed.
- No row-level bulk external samples in PostgreSQL.
- No AutoML training loop in the first pass; the first pass is data contract, catalog, profile, mapping, and evaluation traceability.

## Acceptance Criteria

- The project can describe an external dataset before importing it into business runtime tables.
- Parquet is the required registered format for analytical datasets.
- PostgreSQL stores dataset governance metadata and artifact URIs, not bulk row-level training data.
- The old renewal sample has a clear registration shape and does not pollute FWA claim scoring.
- Model evaluation metrics are traceable to an immutable feature set and dataset version.
