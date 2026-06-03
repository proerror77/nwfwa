# Public Data MVP Pack

This pack uses public healthcare data to exercise the `nwfwa` schema, Parquet
manifest, feature materialization, training/export pipeline, demo scoring, and
monitoring contracts. It does not replace customer claims, customer labels, QA
outcomes, or production shadow traffic.

## Boundary

Public data may validate:

- claim-like schema and Parquet split handling;
- provider utilization and payment baseline features;
- deterministic excluded-provider screening;
- CMS policy corpus ingestion for medical necessity grounding;
- logistic-regression training, Rust artifact export, evaluation reports, and
  scheduled monitoring plan generation.

Public data must not be used as:

- customer production model effectiveness evidence;
- confirmed fraud labels;
- automatic claim denial evidence;
- customer payer policy proof;
- proof that a promoted model is safe for pre-payment routing.

## Official Sources

| Source | Use In MVP | Limitation |
| --- | --- | --- |
| CMS Medicare Claims Synthetic Public Use Files | claim-like schema, synthetic demo rows, pipeline training fixture | synthetic data, limited inferential value |
| CMS Medicare Physician & Other Practitioners by Provider | provider utilization and payment peer baseline | provider-level summary, not individual claim truth |
| HHS-OIG List of Excluded Individuals/Entities | provider/entity exclusion screening feature | screening list only, not full FWA detection |
| CMS Medicare Coverage Database | policy corpus for medical necessity RAG | must be adapted to customer payer policy |

Reference URLs are written into generated `sources.json` and maintained in
`scripts/data/build_public_data_mvp.py`.

## Generate A Minimal Fixture

Use this when no public CSV files have been downloaded yet:

```bash
uv run --project apps/ml-service \
  python scripts/data/build_public_data_mvp.py \
  --synthetic-fixture \
  --output-dir data/public-mvp \
  --dataset-version 2026-06-public-mvp
```

This writes:

- `data/public-mvp/manifest.json`;
- `data/public-mvp/sources.json`;
- `data/public-mvp/split=train/part-00000.parquet`;
- `data/public-mvp/split=validation/part-00000.parquet`;
- `data/public-mvp/split=out_of_time/part-00000.parquet`.

The manifest uses `confirmed_fwa` only as a weak pipeline label under
`label_policy = weak_public_data_pipeline_label_not_production_evidence`.

## Generate From Downloaded Public CSVs

After downloading source extracts locally, run:

```bash
uv run --project apps/ml-service \
  python scripts/data/build_public_data_mvp.py \
  --synpuf-claims-csv /path/to/synpuf_claims.csv \
  --provider-summary-csv /path/to/cms_provider_summary.csv \
  --leie-csv /path/to/leie.csv \
  --policy-corpus-dir /path/to/policy_texts \
  --output-dir data/public-mvp \
  --dataset-version 2026-06-public-mvp
```

The script normalizes the inputs into the training columns expected by the
current pipeline:

- `claim_id`;
- `member_id`;
- `policy_id`;
- `provider_id`;
- `service_date`;
- `service_date_ord`;
- `claim_amount_to_limit_ratio`;
- `provider_profile_score`;
- `high_cost_item_ratio`;
- `provider_peer_payment_zscore`;
- `leie_excluded_provider`;
- `confirmed_fwa`.

`confirmed_fwa` remains a weak public-data label. It is derived only to keep
the model-training contract executable.

## Kaggle Provider Fraud MVP

When the Kaggle Healthcare Provider Fraud Detection archive is available
locally, it can be converted into a demo claim-grain dataset and inbox-compatible
TPA claim payloads:

```bash
uv run --project apps/ml-service \
  python scripts/data/build_kaggle_provider_fraud_mvp.py \
  --archive /Users/proerror/Downloads/archive.zip \
  --output-dir data/kaggle-provider-fraud \
  --dataset-version 2026-06-kaggle-provider-fraud-demo \
  --max-claims 5000 \
  --max-tpa-payloads 100
```

This writes:

- `data/kaggle-provider-fraud/manifest.json`;
- `data/kaggle-provider-fraud/sources.json`;
- `data/kaggle-provider-fraud/split=train/part-00000.parquet`;
- `data/kaggle-provider-fraud/split=validation/part-00000.parquet`;
- `data/kaggle-provider-fraud/split=out_of_time/part-00000.parquet`;
- `data/kaggle-provider-fraud/tpa_claims.jsonl`;
- `data/kaggle-provider-fraud/tpa_claim_sample.json`;
- `data/kaggle-provider-fraud/tpa_claims_index.json`.

The generated TPA JSON uses masked demo identifiers and the existing inbox
payload shape: `systemCode`, `transNo`, `reportCase`, `policyList`,
`productList`, `claimLiabilityList`, `invoiceList`, `feeList`, and
`medicalRecordInfoList`.

Train/export a candidate from the Kaggle manifest:

```bash
uv run --project apps/ml-service \
  python -m app.train \
  --manifest data/kaggle-provider-fraud/manifest.json \
  --artifact-base-uri data/kaggle-provider-fraud-artifacts \
  --model-key baseline_fwa \
  --base-model-version kaggle-mvp \
  --job-id kaggle_provider_fraud_job_1 \
  --actor kaggle-data-builder
```

The Kaggle source label is provider-level `PotentialFraud`. The dataset builder
maps that label into `confirmed_fwa` only as a weak claim-level pipeline label
under
`label_policy = weak_provider_level_label_not_claim_level_production_evidence`.
It is useful for exercising ingestion, feature materialization, training,
artifact export, Rust serving-artifact compatibility, monitoring report
generation, and TPA inbox normalization.

It must not be interpreted as:

- customer production model effectiveness evidence;
- claim-level fraud truth;
- evidence for fraud accusation;
- automatic claim denial evidence;
- proof that a model is ready for customer pre-payment routing.

Because the split is time-sorted while the Kaggle label is provider-level, the
out-of-time split can have a skewed label distribution. Treat the result as a
demo/research signal for pipeline behavior, not calibrated production quality.

## Run The Existing Pipeline

Profile the generated manifest:

```bash
cargo run --locked -p worker -- profile-parquet \
  --manifest data/public-mvp/manifest.json \
  --output-dir data/public-mvp/profile
```

Train/export a candidate:

```bash
uv run --project apps/ml-service \
  python -m app.train \
  --manifest data/public-mvp/manifest.json \
  --artifact-base-uri data/public-mvp-artifacts \
  --model-key baseline_fwa \
  --base-model-version public-mvp \
  --job-id public_data_mvp_job_1 \
  --actor public-data-builder
```

Build an external training handoff:

```bash
cargo run --locked -p worker -- build-training-handoff \
  --manifest data/public-mvp/manifest.json \
  --artifact-base-uri s3://fwa-public-mvp/models \
  --model-key baseline_fwa \
  --base-model-version public-mvp \
  --job-id public_data_mvp_job_1 \
  --actor public-data-builder
```

Build a scheduled monitoring plan:

```bash
cargo run --locked -p worker -- build-mlops-monitoring-plan \
  --manifest-uri s3://fwa-public-mvp/datasets/public_data_mvp_claims/2026-06-public-mvp/manifest.json \
  --artifact-uri s3://fwa-public-mvp/models/baseline_fwa/public-mvp/rust_serving_artifact.json \
  --model-key baseline_fwa \
  --model-version public-mvp \
  --cron "0 2 * * *"
```

The monitoring plan remains a contract for an external scheduler. For public
data it proves the report URI and job-shape contract only; reviewer
disagreement and label delay need customer QA and outcome timestamps before
their metrics are meaningful.

## Production Interpretation

The public-data MVP pack closes the engineering loop. It proves the data shape,
batch transformation, training/export, artifact registration contract, and
monitoring plan are executable.

It does not close these production gaps:

- customer label provenance;
- delayed-label handling;
- reviewer-disagreement measurement;
- customer holdout validation;
- real shadow traffic against live routing and QA outcomes;
- calibrated probability evidence;
- customer-approved fairness and segment definitions;
- production object storage, retention, legal hold, and signing-key management.
