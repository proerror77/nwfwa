use anyhow::{bail, Context};
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    required_manifest_str, retraining_job_output_path, safe_path_segment, ClaimedRetrainingJob,
    TrainingCommand,
};

pub(crate) fn build_training_command(
    python: &str,
    manifest_path: &str,
    artifact_base_uri: &str,
    job: &ClaimedRetrainingJob,
    actor: &str,
    trainer_workdir: Option<&str>,
    algorithm: Option<&str>,
) -> TrainingCommand {
    let mut args = vec![
        "-m".into(),
        "app.train".into(),
        "--manifest".into(),
        manifest_path.into(),
        "--artifact-base-uri".into(),
        artifact_base_uri.into(),
        "--model-key".into(),
        job.model_key.clone(),
        "--base-model-version".into(),
        job.model_version.clone(),
        "--job-id".into(),
        job.job_id.clone(),
        "--actor".into(),
        actor.into(),
    ];
    if let Some(algorithm) = algorithm
        .map(str::trim)
        .filter(|algorithm| !algorithm.is_empty())
    {
        args.push("--algorithm".into());
        args.push(algorithm.into());
    }
    TrainingCommand {
        program: python.to_string(),
        args,
        workdir: trainer_workdir
            .map(str::trim)
            .filter(|workdir| !workdir.is_empty())
            .map(PathBuf::from),
    }
}

pub fn build_training_handoff(
    manifest_path: impl AsRef<Path>,
    artifact_base_uri: &str,
    model_key: &str,
    base_model_version: &str,
    job_id: &str,
    actor: &str,
) -> anyhow::Result<serde_json::Value> {
    build_training_handoff_with_algorithm(
        manifest_path,
        artifact_base_uri,
        model_key,
        base_model_version,
        job_id,
        actor,
        "logistic_regression",
    )
}

pub fn build_training_handoff_with_algorithm(
    manifest_path: impl AsRef<Path>,
    artifact_base_uri: &str,
    model_key: &str,
    base_model_version: &str,
    job_id: &str,
    actor: &str,
    algorithm: &str,
) -> anyhow::Result<serde_json::Value> {
    if artifact_base_uri.trim().is_empty() {
        bail!("artifact_base_uri is required");
    }
    let algorithm = normalize_training_algorithm(algorithm)?;
    let manifest_path = manifest_path.as_ref();
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read training manifest {}", manifest_path.display()))?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_json).context("parse training manifest")?;
    let dataset_key = required_manifest_str(&manifest, "dataset_key")?;
    let dataset_version = required_manifest_str(&manifest, "dataset_version")?;
    let label_column = required_manifest_str(&manifest, "label_column")?;
    let time_split_field = required_manifest_str(&manifest, "time_split_field")?;
    let entity_keys = manifest
        .get("entity_keys")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let group_split_fields = manifest
        .get("group_split_fields")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let splits = manifest
        .get("splits")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if splits.is_empty() {
        bail!("training manifest must include splits");
    }

    let candidate_model_version = training_candidate_version(base_model_version, job_id, algorithm);
    let artifact_root = artifact_base_uri.trim().trim_end_matches('/');
    let safe_model_key = safe_path_segment(model_key);
    let artifact_dir = format!("{artifact_root}/{safe_model_key}/{candidate_model_version}");
    let onnx_algorithm = matches!(algorithm, "xgboost" | "lightgbm");
    let rust_native_algorithm = algorithm == "logistic_regression";
    let serving_artifact_uri = match algorithm {
        "xgboost" | "lightgbm" => format!("{artifact_dir}/model.onnx"),
        "deep_learning" => format!("{artifact_dir}/model.joblib"),
        "logistic_regression" => format!("{artifact_dir}/rust_serving_artifact.json"),
        _ => unreachable!("algorithm normalized"),
    };
    let runtime_kind = match algorithm {
        "logistic_regression" => "rust_logistic_regression",
        "xgboost" => "xgboost_onnx",
        "lightgbm" => "lightgbm_onnx",
        "deep_learning" => "deep_learning_sklearn_mlp",
        _ => unreachable!("algorithm normalized"),
    };
    let mut required_evidence_refs = vec![
        "model_retraining_jobs:<job_id>".to_string(),
        "model_artifacts:<serving_artifact_uri>".to_string(),
        "feature_set_manifests:<rust_feature_set_manifest_uri>".to_string(),
        "model_feature_importance:<feature_importance_uri>".to_string(),
        "model_permutation_importance:<permutation_importance_uri>".to_string(),
        "model_validation_reports:<validation_report_uri>".to_string(),
        "model_evaluations:<evaluation_run_id>".to_string(),
        "rule_candidate_mining_plans:<rule_candidate_mining_plan_uri>".to_string(),
        "rule_candidate_backtests:<rule_candidate_backtest_report_uri>".to_string(),
        "rule_candidate_review_tasks:<rule_candidate_review_tasks_uri>".to_string(),
    ];
    if onnx_algorithm {
        required_evidence_refs.push("model_onnx_parity_reports:<onnx_parity_report_uri>".into());
    }

    Ok(serde_json::json!({
        "handoff_kind": "external_training_platform",
        "handoff_version": 2,
        "data_contract": {
            "source": "same_parquet_dataset_manifest",
            "manifest_uri": manifest_path.to_string_lossy(),
            "forbidden_sources": ["application_tables", "ad_hoc_feature_definitions"]
        },
        "dataset": {
            "dataset_key": dataset_key,
            "dataset_version": dataset_version,
            "manifest_uri": manifest_path.to_string_lossy(),
            "label_column": label_column,
            "entity_keys": entity_keys,
            "time_split_field": time_split_field,
            "group_split_fields": group_split_fields,
            "splits": splits
        },
        "training_job": {
            "model_key": model_key,
            "base_model_version": base_model_version,
            "candidate_model_version": candidate_model_version,
            "job_id": job_id,
            "actor": actor,
            "algorithm": algorithm,
            "runtime_kind": runtime_kind
        },
        "artifact_contract": {
            "artifact_dir": artifact_dir,
            "serving_artifact_uri": serving_artifact_uri,
            "serving_artifact_format": match algorithm {
                "xgboost" | "lightgbm" => "onnx",
                "deep_learning" => "joblib",
                "logistic_regression" => "rust_json",
                _ => unreachable!("algorithm normalized"),
            },
            "rust_serving_artifact_uri": if rust_native_algorithm {
                serde_json::Value::String(format!("{artifact_dir}/rust_serving_artifact.json"))
            } else {
                serde_json::Value::Null
            },
            "onnx_artifact_uri": if onnx_algorithm {
                serde_json::Value::String(format!("{artifact_dir}/model.onnx"))
            } else {
                serde_json::Value::Null
            },
            "training_artifact_uri": format!("{artifact_dir}/model.joblib"),
            "serving_manifest_uri": format!("{artifact_dir}/serving_manifest.json"),
            "onnx_parity_report_uri": if onnx_algorithm {
                serde_json::Value::String(format!("{artifact_dir}/onnx_parity_report.json"))
            } else {
                serde_json::Value::Null
            },
            "validation_report_uri": format!("{artifact_dir}/validation.json"),
            "feature_importance_uri": format!("{artifact_dir}/feature_importance.parquet"),
            "permutation_importance_uri": format!("{artifact_dir}/permutation_importance.parquet"),
            "rust_feature_set_manifest_uri": format!("{artifact_dir}/rust_feature_set/feature_set_manifest.json"),
            "feature_store_manifest_uri": format!("{artifact_dir}/feature_store_manifest.json"),
            "rule_candidate_mining_plan_uri": format!("{artifact_dir}/rule-candidates/rule_candidate_mining_plan.json"),
            "rule_candidate_review_tasks_uri": format!("{artifact_dir}/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"),
            "rule_candidate_backtest_report_uri": format!("{artifact_dir}/rule-candidates/backtest/rule_candidate_backtest_report.json"),
            "shadow_report_uri": format!("{artifact_dir}/shadow_report.json"),
            "drift_report_uri": format!("{artifact_dir}/drift_report.json"),
            "fairness_report_uri": format!("{artifact_dir}/fairness_report.json")
        },
        "feature_set_contract": {
            "builder": "worker build-feature-set",
            "required_hash_field": "metrics_json.feature_reproducibility_hash",
            "required_manifest_field": "metrics_json.rust_feature_set_manifest_uri",
            "excluded_columns": ["dataset.entity_keys", "dataset.label_column"],
            "evidence_ref": "feature_set_manifests:<rust_feature_set_manifest_uri>"
        },
        "rule_candidate_workflow_contract": {
            "candidate_builder": "worker mine-rule-candidates",
            "backtest_builder": "worker run-rule-candidate-backtest",
            "validation_report_uri": "artifact_contract.validation_report_uri",
            "feature_importance_uri": "artifact_contract.feature_importance_uri",
            "training_manifest_uri": "data_contract.manifest_uri",
            "required_metrics_fields": [
                "metrics_json.rule_candidate_mining_status",
                "metrics_json.rule_candidate_backtest_status",
                "metrics_json.rule_candidate_backtest_report_uri",
                "metrics_json.rule_candidate_review_tasks_uri",
                "metrics_json.rule_library_writeback_status"
            ],
            "required_evidence_refs": [
                "rule_candidate_mining_plans:<rule_candidate_mining_plan_uri>",
                "rule_candidate_backtests:<rule_candidate_backtest_report_uri>",
                "rule_candidate_review_tasks:<rule_candidate_review_tasks_uri>"
            ],
            "writeback_boundary": "human_review_required_before_rule_library_writeback"
        },
        "output_contract": {
            "submit_path": retraining_job_output_path(job_id),
            "artifact_uri": "artifact_contract.serving_artifact_uri",
            "feature_importance_uri": "artifact_contract.feature_importance_uri",
            "permutation_importance_uri": "artifact_contract.permutation_importance_uri",
            "serving_manifest_uri": "artifact_contract.serving_manifest_uri",
            "required_metrics_fields": [
                "metrics_json.time_group_split_status",
                "metrics_json.time_split_field",
                "metrics_json.group_split_fields",
                "metrics_json.leakage_check_status",
                "metrics_json.out_of_time_validation_status",
                "metrics_json.score_stability_status",
                "metrics_json.feature_stability_status",
                "metrics_json.overfitting_diagnostics_status",
                "metrics_json.overfitting_diagnostics_report_uri",
                "metrics_json.out_of_time_auc",
                "metrics_json.out_of_time_precision",
                "metrics_json.out_of_time_recall",
                "metrics_json.score_psi",
                "metrics_json.max_feature_psi",
                "metrics_json.feature_reproducibility_hash"
            ],
            "onnx_parity_report_uri": if onnx_algorithm {
                serde_json::Value::String("artifact_contract.onnx_parity_report_uri".into())
            } else {
                serde_json::Value::Null
            },
            "required_evidence_refs": required_evidence_refs
        }
    }))
}

fn normalize_training_algorithm(algorithm: &str) -> anyhow::Result<&'static str> {
    match algorithm.trim() {
        "" | "logistic_regression" => Ok("logistic_regression"),
        "xgboost" => Ok("xgboost"),
        "lightgbm" => Ok("lightgbm"),
        "deep_learning" => Ok("deep_learning"),
        other => bail!("unsupported training algorithm: {other}"),
    }
}

fn training_candidate_version(base_model_version: &str, job_id: &str, algorithm: &str) -> String {
    let base = safe_path_segment(base_model_version);
    let job = safe_path_segment(job_id);
    if algorithm == "logistic_regression" {
        format!("{base}-candidate-{job}")
    } else {
        format!("{base}-{}-candidate-{job}", safe_path_segment(algorithm))
    }
}
