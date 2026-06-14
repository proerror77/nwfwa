use anyhow::Context;
use arrow_array::{Float64Array, Int32Array, Int8Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use std::{fs, path::Path, sync::Arc};

use super::{
    required_non_empty, write_json, write_parquet, DemoMlDatasetPack, DemoMlDatasetSummary,
};

#[derive(Debug, Clone)]
struct DemoLabeledClaim {
    claim_id: &'static str,
    member_id: &'static str,
    policy_id: &'static str,
    provider_id: &'static str,
    service_date: &'static str,
    claim_amount: f64,
    amount_to_limit_ratio: f64,
    peer_percentile: f64,
    item_count: i32,
    high_cost_item_ratio: f64,
    provider_risk_tier: i32,
    diagnosis_procedure_mismatch: i8,
    confirmed_fwa: i8,
}

#[derive(Debug, Clone)]
struct DemoUnlabeledClaim {
    claim_id: &'static str,
    member_id: &'static str,
    policy_id: &'static str,
    provider_id: &'static str,
    service_date: &'static str,
    claim_amount: f64,
    amount_to_limit_ratio: f64,
    peer_percentile: f64,
    item_count: i32,
    high_cost_item_ratio: f64,
    provider_risk_tier: i32,
    diagnosis_procedure_mismatch: i8,
}

#[derive(Debug, Clone)]
struct DemoProviderPeerRow {
    provider_id: &'static str,
    cohort_key: &'static str,
    service_month: &'static str,
    claim_count: i32,
    avg_claim_amount: f64,
    high_cost_rate: f64,
    peer_z_score: f64,
    graph_degree: i32,
    community_id: i32,
}

pub fn build_demo_ml_datasets(
    output_dir: impl AsRef<Path>,
    dataset_version: &str,
) -> anyhow::Result<DemoMlDatasetPack> {
    let dataset_version = required_non_empty("dataset_version", dataset_version)?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)
        .with_context(|| format!("create demo ML dataset dir {}", output_dir.display()))?;

    let labeled_rows = demo_labeled_claims();
    let train_rows = labeled_rows[0..8].to_vec();
    let validation_rows = labeled_rows[8..12].to_vec();
    let out_of_time_rows = labeled_rows[12..16].to_vec();
    let labeled_dir = output_dir.join("labeled_claim_risk");
    write_labeled_split(&labeled_dir, "train", &train_rows)?;
    write_labeled_split(&labeled_dir, "validation", &validation_rows)?;
    write_labeled_split(&labeled_dir, "out_of_time", &out_of_time_rows)?;
    let labeled_manifest = serde_json::json!({
        "dataset_key": "rust_demo_claim_risk_labeled",
        "dataset_version": dataset_version,
        "business_domain": "health_fwa",
        "sample_grain": "claim",
        "display_name": "Rust Demo Claim Risk Labeled",
        "owner": "ml-ops",
        "description": "Rust-generated demo dataset for supervised model training, backtesting, and promotion-gate exercises.",
        "status": "demo_only",
        "label_column": "confirmed_fwa",
        "label_policy": "weak_rust_demo_label_not_production_evidence",
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date",
        "group_split_fields": ["member_id", "policy_id", "provider_id"],
        "splits": [
            {"split_name": "train", "data_uri": "split=train/"},
            {"split_name": "validation", "data_uri": "split=validation/"},
            {"split_name": "out_of_time", "data_uri": "split=out_of_time/"}
        ],
        "governance": {
            "allowed_uses": ["pipeline_validation", "backtest_contract_validation", "human_review_workflow_demo"],
            "blocked_uses": ["production_auto_deny", "customer_validation_claim", "production_roi_claim"]
        }
    });
    write_json(labeled_dir.join("manifest.json"), &labeled_manifest)?;

    let scoring_rows = demo_unlabeled_claims();
    let scoring_dir = output_dir.join("unlabeled_shadow_scoring");
    write_unlabeled_claim_split(&scoring_dir, "scoring", &scoring_rows)?;
    let scoring_manifest = serde_json::json!({
        "dataset_key": "rust_demo_claim_shadow_unlabeled",
        "dataset_version": dataset_version,
        "business_domain": "health_fwa",
        "sample_grain": "claim",
        "display_name": "Rust Demo Claim Shadow Scoring Unlabeled",
        "owner": "ml-ops",
        "description": "Rust-generated unlabeled claims for shadow scoring and drift exercises.",
        "status": "demo_only",
        "label_policy": "unlabeled_shadow_scoring_only",
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date",
        "splits": [
            {"split_name": "scoring", "data_uri": "split=scoring/"}
        ],
        "governance": {
            "allowed_uses": ["shadow_scoring", "claim_entity_clustering", "drift_monitoring_demo", "score_distribution_demo"],
            "blocked_uses": ["supervised_training", "production_promotion_evidence", "confirmed_fwa_labeling"]
        }
    });
    write_json(scoring_dir.join("manifest.json"), &scoring_manifest)?;

    let provider_rows = demo_provider_peer_rows();
    let provider_dir = output_dir.join("unlabeled_provider_peer_clustering");
    write_provider_peer_split(&provider_dir, "analysis", &provider_rows)?;
    let provider_manifest = serde_json::json!({
        "dataset_key": "rust_demo_provider_peer_unlabeled",
        "dataset_version": dataset_version,
        "business_domain": "health_fwa",
        "sample_grain": "provider_month",
        "display_name": "Rust Demo Provider Peer Clustering Unlabeled",
        "owner": "ml-ops",
        "description": "Rust-generated provider peer features for clustering and anomaly discovery exercises.",
        "status": "demo_only",
        "label_policy": "unlabeled_clustering_discovery_only",
        "entity_keys": ["provider_id"],
        "time_split_field": "service_month",
        "splits": [
            {"split_name": "analysis", "data_uri": "split=analysis/"}
        ],
        "governance": {
            "allowed_uses": ["provider_peer_clustering", "anomaly_candidate_discovery", "manual_review_prioritization_demo"],
            "blocked_uses": ["supervised_training", "confirmed_fwa_labeling", "automatic_claim_disposition"]
        }
    });
    write_json(provider_dir.join("manifest.json"), &provider_manifest)?;

    let pack = DemoMlDatasetPack {
        pack_kind: "rust_automl_demo_datasets".into(),
        dataset_version: dataset_version.into(),
        output_dir: output_dir.to_string_lossy().into_owned(),
        labeled_manifest_uri: labeled_dir.join("manifest.json").to_string_lossy().into_owned(),
        unlabeled_manifest_uris: vec![
            scoring_dir.join("manifest.json").to_string_lossy().into_owned(),
            provider_dir.join("manifest.json").to_string_lossy().into_owned(),
        ],
        dataset_manifests: vec![
            DemoMlDatasetSummary {
                dataset_key: "rust_demo_claim_risk_labeled".into(),
                sample_grain: "claim".into(),
                label_policy: "weak_rust_demo_label_not_production_evidence".into(),
                label_column: Some("confirmed_fwa".into()),
                manifest_uri: labeled_dir.join("manifest.json").to_string_lossy().into_owned(),
                split_count: 3,
                row_count: labeled_rows.len(),
            },
            DemoMlDatasetSummary {
                dataset_key: "rust_demo_claim_shadow_unlabeled".into(),
                sample_grain: "claim".into(),
                label_policy: "unlabeled_shadow_scoring_only".into(),
                label_column: None,
                manifest_uri: scoring_dir.join("manifest.json").to_string_lossy().into_owned(),
                split_count: 1,
                row_count: scoring_rows.len(),
            },
            DemoMlDatasetSummary {
                dataset_key: "rust_demo_provider_peer_unlabeled".into(),
                sample_grain: "provider_month".into(),
                label_policy: "unlabeled_clustering_discovery_only".into(),
                label_column: None,
                manifest_uri: provider_dir.join("manifest.json").to_string_lossy().into_owned(),
                split_count: 1,
                row_count: provider_rows.len(),
            },
        ],
        governance_boundary: "demo data only; unlabeled datasets cannot train supervised models; labeled data is weak demo evidence only".into(),
        next_worker_commands: vec![
            format!(
                "cargo run --locked -p worker -- profile-parquet --manifest {} --output-dir {}/profile",
                labeled_dir.join("manifest.json").display(),
                labeled_dir.display()
            ),
            format!(
                "cargo run --locked -p worker -- build-feature-set --manifest {} --output-dir {}/feature-set",
                labeled_dir.join("manifest.json").display(),
                labeled_dir.display()
            ),
            format!(
                "cargo run --locked -p worker -- build-training-handoff --manifest {} --artifact-base-uri s3://fwa-models --model-key baseline_fwa --base-model-version 0.1.0 --job-id model_retraining_job_1 --actor trainer-worker",
                labeled_dir.join("manifest.json").display()
            ),
            format!(
                "cargo run --locked -p worker -- build-training-handoff --manifest {} --artifact-base-uri s3://fwa-models --model-key baseline_fwa --base-model-version 0.1.0 --job-id model_retraining_job_1 --actor trainer-worker --algorithm xgboost",
                labeled_dir.join("manifest.json").display()
            ),
            format!(
                "cargo run --locked -p worker -- cluster-provider-peers --manifest {} --output-dir {}/clusters",
                provider_dir.join("manifest.json").display(),
                provider_dir.display()
            ),
            format!(
                "cargo run --locked -p worker -- cluster-provider-graph --manifest {} --output-dir {}/graph-communities",
                provider_dir.join("manifest.json").display(),
                provider_dir.display()
            ),
            format!(
                "cargo run --locked -p worker -- cluster-claim-entities --manifest {} --output-dir {}/entity-clusters",
                scoring_dir.join("manifest.json").display(),
                scoring_dir.display()
            ),
        ],
    };
    write_json(output_dir.join("index.json"), &pack)?;
    Ok(pack)
}

fn demo_labeled_claims() -> Vec<DemoLabeledClaim> {
    vec![
        DemoLabeledClaim {
            claim_id: "CLM-0001",
            member_id: "MBR-001",
            policy_id: "POL-001",
            provider_id: "PRV-101",
            service_date: "2026-01-03",
            claim_amount: 420.0,
            amount_to_limit_ratio: 0.18,
            peer_percentile: 0.22,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0002",
            member_id: "MBR-002",
            policy_id: "POL-002",
            provider_id: "PRV-102",
            service_date: "2026-01-06",
            claim_amount: 1280.0,
            amount_to_limit_ratio: 0.64,
            peer_percentile: 0.79,
            item_count: 5,
            high_cost_item_ratio: 0.40,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0003",
            member_id: "MBR-003",
            policy_id: "POL-003",
            provider_id: "PRV-103",
            service_date: "2026-01-09",
            claim_amount: 310.0,
            amount_to_limit_ratio: 0.12,
            peer_percentile: 0.18,
            item_count: 1,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0004",
            member_id: "MBR-004",
            policy_id: "POL-004",
            provider_id: "PRV-104",
            service_date: "2026-01-12",
            claim_amount: 2420.0,
            amount_to_limit_ratio: 0.91,
            peer_percentile: 0.96,
            item_count: 8,
            high_cost_item_ratio: 0.63,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0005",
            member_id: "MBR-005",
            policy_id: "POL-005",
            provider_id: "PRV-105",
            service_date: "2026-01-15",
            claim_amount: 560.0,
            amount_to_limit_ratio: 0.21,
            peer_percentile: 0.34,
            item_count: 3,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0006",
            member_id: "MBR-006",
            policy_id: "POL-006",
            provider_id: "PRV-106",
            service_date: "2026-01-18",
            claim_amount: 1760.0,
            amount_to_limit_ratio: 0.83,
            peer_percentile: 0.88,
            item_count: 6,
            high_cost_item_ratio: 0.50,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0007",
            member_id: "MBR-007",
            policy_id: "POL-007",
            provider_id: "PRV-107",
            service_date: "2026-01-21",
            claim_amount: 690.0,
            amount_to_limit_ratio: 0.27,
            peer_percentile: 0.39,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0008",
            member_id: "MBR-008",
            policy_id: "POL-008",
            provider_id: "PRV-108",
            service_date: "2026-01-24",
            claim_amount: 1580.0,
            amount_to_limit_ratio: 0.74,
            peer_percentile: 0.86,
            item_count: 7,
            high_cost_item_ratio: 0.57,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0009",
            member_id: "MBR-009",
            policy_id: "POL-009",
            provider_id: "PRV-109",
            service_date: "2026-02-04",
            claim_amount: 490.0,
            amount_to_limit_ratio: 0.16,
            peer_percentile: 0.24,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0010",
            member_id: "MBR-010",
            policy_id: "POL-010",
            provider_id: "PRV-110",
            service_date: "2026-02-09",
            claim_amount: 2240.0,
            amount_to_limit_ratio: 0.94,
            peer_percentile: 0.97,
            item_count: 9,
            high_cost_item_ratio: 0.67,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0011",
            member_id: "MBR-011",
            policy_id: "POL-011",
            provider_id: "PRV-111",
            service_date: "2026-02-13",
            claim_amount: 360.0,
            amount_to_limit_ratio: 0.10,
            peer_percentile: 0.17,
            item_count: 1,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0012",
            member_id: "MBR-012",
            policy_id: "POL-012",
            provider_id: "PRV-112",
            service_date: "2026-02-16",
            claim_amount: 1430.0,
            amount_to_limit_ratio: 0.68,
            peer_percentile: 0.81,
            item_count: 5,
            high_cost_item_ratio: 0.40,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0013",
            member_id: "MBR-013",
            policy_id: "POL-013",
            provider_id: "PRV-113",
            service_date: "2026-03-02",
            claim_amount: 520.0,
            amount_to_limit_ratio: 0.19,
            peer_percentile: 0.29,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0014",
            member_id: "MBR-014",
            policy_id: "POL-014",
            provider_id: "PRV-114",
            service_date: "2026-03-07",
            claim_amount: 2610.0,
            amount_to_limit_ratio: 0.98,
            peer_percentile: 0.99,
            item_count: 10,
            high_cost_item_ratio: 0.70,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0015",
            member_id: "MBR-015",
            policy_id: "POL-015",
            provider_id: "PRV-115",
            service_date: "2026-03-12",
            claim_amount: 450.0,
            amount_to_limit_ratio: 0.14,
            peer_percentile: 0.21,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
            confirmed_fwa: 0,
        },
        DemoLabeledClaim {
            claim_id: "CLM-0016",
            member_id: "MBR-016",
            policy_id: "POL-016",
            provider_id: "PRV-116",
            service_date: "2026-03-17",
            claim_amount: 1690.0,
            amount_to_limit_ratio: 0.78,
            peer_percentile: 0.84,
            item_count: 6,
            high_cost_item_ratio: 0.50,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
            confirmed_fwa: 1,
        },
    ]
}

fn demo_unlabeled_claims() -> Vec<DemoUnlabeledClaim> {
    vec![
        DemoUnlabeledClaim {
            claim_id: "CLM-S001",
            member_id: "MBR-S001",
            policy_id: "POL-S001",
            provider_id: "PRV-201",
            service_date: "2026-04-02",
            claim_amount: 380.0,
            amount_to_limit_ratio: 0.15,
            peer_percentile: 0.25,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S002",
            member_id: "MBR-S002",
            policy_id: "POL-S002",
            provider_id: "PRV-202",
            service_date: "2026-04-03",
            claim_amount: 1980.0,
            amount_to_limit_ratio: 0.89,
            peer_percentile: 0.93,
            item_count: 8,
            high_cost_item_ratio: 0.63,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S003",
            member_id: "MBR-S003",
            policy_id: "POL-S003",
            provider_id: "PRV-203",
            service_date: "2026-04-04",
            claim_amount: 740.0,
            amount_to_limit_ratio: 0.31,
            peer_percentile: 0.44,
            item_count: 3,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S004",
            member_id: "MBR-S004",
            policy_id: "POL-S004",
            provider_id: "PRV-204",
            service_date: "2026-04-05",
            claim_amount: 2260.0,
            amount_to_limit_ratio: 0.96,
            peer_percentile: 0.98,
            item_count: 9,
            high_cost_item_ratio: 0.67,
            provider_risk_tier: 3,
            diagnosis_procedure_mismatch: 1,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S005",
            member_id: "MBR-S005",
            policy_id: "POL-S005",
            provider_id: "PRV-205",
            service_date: "2026-04-06",
            claim_amount: 610.0,
            amount_to_limit_ratio: 0.22,
            peer_percentile: 0.36,
            item_count: 2,
            high_cost_item_ratio: 0.00,
            provider_risk_tier: 1,
            diagnosis_procedure_mismatch: 0,
        },
        DemoUnlabeledClaim {
            claim_id: "CLM-S006",
            member_id: "MBR-S006",
            policy_id: "POL-S006",
            provider_id: "PRV-206",
            service_date: "2026-04-07",
            claim_amount: 1510.0,
            amount_to_limit_ratio: 0.71,
            peer_percentile: 0.82,
            item_count: 6,
            high_cost_item_ratio: 0.50,
            provider_risk_tier: 2,
            diagnosis_procedure_mismatch: 1,
        },
    ]
}

fn demo_provider_peer_rows() -> Vec<DemoProviderPeerRow> {
    vec![
        DemoProviderPeerRow {
            provider_id: "PRV-201",
            cohort_key: "orthopedic_urban",
            service_month: "2026-04",
            claim_count: 42,
            avg_claim_amount: 640.0,
            high_cost_rate: 0.08,
            peer_z_score: -0.4,
            graph_degree: 3,
            community_id: 1,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-202",
            cohort_key: "orthopedic_urban",
            service_month: "2026-04",
            claim_count: 136,
            avg_claim_amount: 1830.0,
            high_cost_rate: 0.41,
            peer_z_score: 2.7,
            graph_degree: 12,
            community_id: 3,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-203",
            cohort_key: "primary_care_suburban",
            service_month: "2026-04",
            claim_count: 61,
            avg_claim_amount: 390.0,
            high_cost_rate: 0.03,
            peer_z_score: -0.1,
            graph_degree: 4,
            community_id: 1,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-204",
            cohort_key: "primary_care_suburban",
            service_month: "2026-04",
            claim_count: 118,
            avg_claim_amount: 1420.0,
            high_cost_rate: 0.34,
            peer_z_score: 2.2,
            graph_degree: 9,
            community_id: 2,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-205",
            cohort_key: "imaging_urban",
            service_month: "2026-04",
            claim_count: 54,
            avg_claim_amount: 760.0,
            high_cost_rate: 0.10,
            peer_z_score: 0.2,
            graph_degree: 5,
            community_id: 1,
        },
        DemoProviderPeerRow {
            provider_id: "PRV-206",
            cohort_key: "imaging_urban",
            service_month: "2026-04",
            claim_count: 153,
            avg_claim_amount: 2140.0,
            high_cost_rate: 0.47,
            peer_z_score: 3.1,
            graph_degree: 15,
            community_id: 3,
        },
    ]
}

fn write_labeled_split(
    dataset_dir: &Path,
    split_name: &str,
    rows: &[DemoLabeledClaim],
) -> anyhow::Result<()> {
    let split_dir = dataset_dir.join(format!("split={split_name}"));
    fs::create_dir_all(&split_dir)
        .with_context(|| format!("create labeled split dir {}", split_dir.display()))?;
    let schema = claim_schema(true);
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.claim_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.member_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.policy_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.provider_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.service_date).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.claim_amount).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.amount_to_limit_ratio)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.peer_percentile)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.item_count).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.high_cost_item_ratio)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter()
                    .map(|row| row.provider_risk_tier)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int8Array::from(
                rows.iter()
                    .map(|row| row.diagnosis_procedure_mismatch)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int8Array::from(
                rows.iter().map(|row| row.confirmed_fwa).collect::<Vec<_>>(),
            )),
        ],
    )?;
    write_parquet(split_dir.join("part-00000.parquet"), schema, &batch)
}

fn write_unlabeled_claim_split(
    dataset_dir: &Path,
    split_name: &str,
    rows: &[DemoUnlabeledClaim],
) -> anyhow::Result<()> {
    let split_dir = dataset_dir.join(format!("split={split_name}"));
    fs::create_dir_all(&split_dir)
        .with_context(|| format!("create unlabeled claim split dir {}", split_dir.display()))?;
    let schema = claim_schema(false);
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.claim_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.member_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.policy_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.provider_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.service_date).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.claim_amount).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.amount_to_limit_ratio)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.peer_percentile)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.item_count).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.high_cost_item_ratio)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter()
                    .map(|row| row.provider_risk_tier)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int8Array::from(
                rows.iter()
                    .map(|row| row.diagnosis_procedure_mismatch)
                    .collect::<Vec<_>>(),
            )),
        ],
    )?;
    write_parquet(split_dir.join("part-00000.parquet"), schema, &batch)
}

fn write_provider_peer_split(
    dataset_dir: &Path,
    split_name: &str,
    rows: &[DemoProviderPeerRow],
) -> anyhow::Result<()> {
    let split_dir = dataset_dir.join(format!("split={split_name}"));
    fs::create_dir_all(&split_dir)
        .with_context(|| format!("create provider peer split dir {}", split_dir.display()))?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("provider_id", DataType::Utf8, false),
        Field::new("cohort_key", DataType::Utf8, false),
        Field::new("service_month", DataType::Utf8, false),
        Field::new("claim_count", DataType::Int32, false),
        Field::new("avg_claim_amount", DataType::Float64, false),
        Field::new("high_cost_rate", DataType::Float64, false),
        Field::new("peer_z_score", DataType::Float64, false),
        Field::new("graph_degree", DataType::Int32, false),
        Field::new("community_id", DataType::Int32, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.provider_id).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.cohort_key).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|row| row.service_month).collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.claim_count).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.avg_claim_amount)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter()
                    .map(|row| row.high_cost_rate)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter().map(|row| row.peer_z_score).collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.graph_degree).collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.iter().map(|row| row.community_id).collect::<Vec<_>>(),
            )),
        ],
    )?;
    write_parquet(split_dir.join("part-00000.parquet"), schema, &batch)
}

fn claim_schema(include_label: bool) -> Arc<Schema> {
    let mut fields = vec![
        Field::new("claim_id", DataType::Utf8, false),
        Field::new("member_id", DataType::Utf8, false),
        Field::new("policy_id", DataType::Utf8, false),
        Field::new("provider_id", DataType::Utf8, false),
        Field::new("service_date", DataType::Utf8, false),
        Field::new("claim_amount", DataType::Float64, false),
        Field::new("amount_to_limit_ratio", DataType::Float64, false),
        Field::new("peer_percentile", DataType::Float64, false),
        Field::new("item_count", DataType::Int32, false),
        Field::new("high_cost_item_ratio", DataType::Float64, false),
        Field::new("provider_risk_tier", DataType::Int32, false),
        Field::new("diagnosis_procedure_mismatch", DataType::Int8, false),
    ];
    if include_label {
        fields.push(Field::new("confirmed_fwa", DataType::Int8, false));
    }
    Arc::new(Schema::new(fields))
}
