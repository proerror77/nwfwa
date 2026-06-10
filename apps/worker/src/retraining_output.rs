use super::{
    artifact_parent_uri, build_feature_set, build_training_command, evaluate_model_artifact,
    json_string, metric_at, mine_rule_candidates, read_json_report, run_rule_candidate_backtest,
    ClaimedRetrainingJob, CompleteRetrainingJobPayload,
};
use anyhow::{anyhow, bail, Context};
use std::{collections::BTreeSet, path::PathBuf, process::Command};

fn artifact_parent_path(artifact_uri: &str) -> PathBuf {
    let parent_uri = artifact_parent_uri(artifact_uri);
    let local_parent = parent_uri
        .strip_prefix("artifact://")
        .or_else(|| parent_uri.strip_prefix("file://"))
        .unwrap_or(parent_uri);
    PathBuf::from(local_parent)
}

pub(crate) fn required_manifest_str<'a>(
    manifest: &'a serde_json::Value,
    key: &str,
) -> anyhow::Result<&'a str> {
    manifest
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("training manifest missing {key}"))
}

pub(crate) fn build_training_retraining_output(
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
    training_manifest: &str,
    trainer_python: &str,
    trainer_workdir: Option<&str>,
    algorithm: Option<&str>,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let training_command = build_training_command(
        trainer_python,
        training_manifest,
        artifact_base_uri,
        job,
        actor,
        trainer_workdir,
        algorithm,
    );
    let mut command = Command::new(&training_command.program);
    command.args(&training_command.args);
    if let Some(workdir) = &training_command.workdir {
        command.current_dir(workdir);
    }
    let output = command
        .output()
        .with_context(|| format!("run model training command {}", training_command.program))?;
    if !output.status.success() {
        bail!(
            "model training command failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = serde_json::from_slice::<CompleteRetrainingJobPayload>(&output.stdout)
        .context("parse model training output")?;
    enrich_retraining_output_with_rust_feature_set(output, training_manifest)
}

pub(crate) fn enrich_retraining_output_with_rust_feature_set(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let feature_set_output_dir =
        artifact_parent_path(&output.artifact_uri).join("rust_feature_set");
    let feature_set_id = format!(
        "{}:{}",
        output.candidate_model_version, "rust_feature_set_v1"
    );
    let feature_set = build_feature_set(
        training_manifest,
        &feature_set_output_dir,
        Some(&feature_set_id),
    )
    .context("build Rust feature set for retraining output")?;
    let feature_set_manifest_uri = feature_set_output_dir
        .join("feature_set_manifest.json")
        .to_string_lossy()
        .into_owned();

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    if let Some(existing_hash) = metrics
        .get("feature_reproducibility_hash")
        .and_then(|value| value.as_str())
    {
        metrics.insert(
            "trainer_feature_reproducibility_hash".into(),
            serde_json::Value::String(existing_hash.to_string()),
        );
    }
    metrics.insert(
        "feature_reproducibility_hash".into(),
        serde_json::Value::String(feature_set.feature_reproducibility_hash.clone()),
    );
    metrics.insert(
        "rust_feature_set_manifest_uri".into(),
        serde_json::Value::String(feature_set_manifest_uri.clone()),
    );
    metrics.insert(
        "rust_feature_set_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "feature_store_materialization_status".into(),
        serde_json::Value::String("passed".into()),
    );

    let evidence_ref = format!("feature_set_manifests:{feature_set_manifest_uri}");
    if !output
        .evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        output.evidence_refs.push(evidence_ref);
    }
    Ok(output)
}

pub(crate) async fn enrich_retraining_output_with_model_artifact_evaluation(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let Some(serving_manifest_uri) = output.serving_manifest_uri.clone() else {
        return Ok(output);
    };
    let artifact_eval_output_dir =
        artifact_parent_path(&output.artifact_uri).join("artifact-evaluation");
    let report = evaluate_model_artifact(
        &serving_manifest_uri,
        training_manifest,
        "validation",
        &artifact_eval_output_dir,
        None,
        0.0001,
        100,
        100,
        Some(&model_artifact_evaluation_signing_key()),
    )
    .await
    .context("evaluate Rust serving artifact before retraining registration")?;
    let onnx_parity = validate_onnx_parity_for_runtime(
        report.runtime_kind.as_str(),
        output.onnx_parity_report_uri.as_deref(),
    )?;
    let gate_status = if report.gate_status == "passed"
        && onnx_parity
            .as_ref()
            .is_none_or(|parity| parity.gate_status == "passed")
    {
        "passed".to_string()
    } else {
        "blocked".to_string()
    };
    let report_uri = artifact_eval_output_dir
        .join("model_artifact_evaluation_report.json")
        .to_string_lossy()
        .into_owned();

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    metrics.insert(
        "model_artifact_evaluation_status".into(),
        serde_json::Value::String(gate_status.clone()),
    );
    metrics.insert(
        "model_artifact_evaluation_gate_status".into(),
        serde_json::Value::String(gate_status.clone()),
    );
    metrics.insert(
        "model_artifact_evaluation_report_uri".into(),
        serde_json::Value::String(report_uri.clone()),
    );
    metrics.insert(
        "rust_serving_status".into(),
        serde_json::Value::String(report.rust_serving_status),
    );
    metrics.insert(
        "rust_serving_latency_status".into(),
        serde_json::Value::String(report.latency_status),
    );
    metrics.insert(
        "rust_serving_p95_latency_ms".into(),
        serde_json::json!(report.p95_latency_ms),
    );
    metrics.insert(
        "rust_serving_runtime_kind".into(),
        serde_json::Value::String(report.runtime_kind),
    );
    if let Some(parity) = &onnx_parity {
        metrics.insert(
            "onnx_parity_gate_status".into(),
            serde_json::Value::String(parity.gate_status.clone()),
        );
        metrics.insert(
            "onnx_parity_status".into(),
            serde_json::Value::String(parity.status.clone()),
        );
        metrics.insert(
            "onnx_parity_report_uri".into(),
            serde_json::Value::String(parity.report_uri.clone()),
        );
        metrics.insert(
            "onnx_serving_runtime_kind".into(),
            serde_json::Value::String(parity.serving_runtime_kind.clone()),
        );
        metrics.insert(
            "onnx_max_abs_probability_delta".into(),
            serde_json::json!(parity.max_abs_probability_delta),
        );
        metrics.insert(
            "onnx_probability_tolerance".into(),
            serde_json::json!(parity.tolerance),
        );
    }

    let evidence_ref = format!("model_artifact_evaluations:{report_uri}");
    if !output
        .evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        output.evidence_refs.push(evidence_ref);
    }
    if let Some(parity) = onnx_parity {
        let evidence_ref = format!("model_onnx_parity_reports:{}", parity.report_uri);
        if !output
            .evidence_refs
            .iter()
            .any(|reference| reference == &evidence_ref)
        {
            output.evidence_refs.push(evidence_ref);
        }
    }
    Ok(output)
}

pub(crate) fn enrich_retraining_output_with_rule_candidate_workflow(
    mut output: CompleteRetrainingJobPayload,
    training_manifest: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    let Some(feature_importance_uri) = output.feature_importance_uri.clone() else {
        return Ok(output);
    };
    let training_platform_candidates = output.mined_rule_candidates.clone();
    let training_platform_candidate_count = training_platform_candidates.len();
    let rule_candidate_dir = artifact_parent_path(&output.artifact_uri).join("rule-candidates");
    let plan = mine_rule_candidates(
        &output.validation_report_uri,
        &feature_importance_uri,
        &rule_candidate_dir,
    )
    .context("mine explainable rule candidates before retraining output registration")?;
    let candidate_plan_uri = rule_candidate_dir
        .join("rule_candidate_mining_plan.json")
        .to_string_lossy()
        .into_owned();
    let candidate_review_tasks_uri = rule_candidate_dir
        .join("rule_candidate_review_tasks.json")
        .to_string_lossy()
        .into_owned();
    let backtest_dir = rule_candidate_dir.join("backtest");
    let backtest =
        run_rule_candidate_backtest(&candidate_plan_uri, training_manifest, &backtest_dir)
            .context(
                "backtest explainable rule candidates before retraining output registration",
            )?;
    let backtest_report_uri = backtest_dir
        .join("rule_candidate_backtest_report.json")
        .to_string_lossy()
        .into_owned();
    let backtest_review_tasks_uri = backtest_dir
        .join("rule_candidate_backtest_review_tasks.json")
        .to_string_lossy()
        .into_owned();
    let mut rule_candidates = training_platform_candidates;
    let mut existing_rule_ids = rule_candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .get("rule_id")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();
    let backtested_rule_candidates = backtest
        .candidate_results
        .iter()
        .map(|result| result.rule_library_writeback_template.clone())
        .collect::<Vec<_>>();
    let backtested_rule_candidate_count = backtested_rule_candidates.len();
    for candidate in backtested_rule_candidates {
        let Some(rule_id) = candidate.get("rule_id").and_then(|value| value.as_str()) else {
            continue;
        };
        if existing_rule_ids.insert(rule_id.to_string()) {
            rule_candidates.push(candidate);
        }
    }
    output.mined_rule_candidates = rule_candidates;

    let Some(metrics) = output.metrics_json.as_object_mut() else {
        bail!("training output metrics_json must be an object");
    };
    metrics.insert(
        "rule_candidate_mining_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "rule_candidate_mining_plan_uri".into(),
        serde_json::Value::String(candidate_plan_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_source_count".into(),
        serde_json::json!(plan.candidate_rules.len()),
    );
    metrics.insert(
        "rule_candidate_backtest_status".into(),
        serde_json::Value::String("passed".into()),
    );
    metrics.insert(
        "rule_candidate_backtest_report_uri".into(),
        serde_json::Value::String(backtest_report_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_review_tasks_uri".into(),
        serde_json::Value::String(backtest_review_tasks_uri.clone()),
    );
    metrics.insert(
        "rule_candidate_review_task_count".into(),
        serde_json::json!(backtest.review_tasks.len()),
    );
    metrics.insert(
        "mined_rule_candidates_source".into(),
        serde_json::Value::String(
            "training_platform_and_deterministic_rule_candidate_backtest".into(),
        ),
    );
    metrics.insert(
        "training_platform_mined_rule_candidate_count".into(),
        serde_json::json!(training_platform_candidate_count),
    );
    metrics.insert(
        "mined_rule_candidates_backtested_count".into(),
        serde_json::json!(backtested_rule_candidate_count),
    );
    metrics.insert(
        "rule_library_writeback_status".into(),
        serde_json::Value::String(backtest.rule_library_writeback_status.clone()),
    );
    metrics.insert(
        "rule_candidate_workflow_boundary".into(),
        serde_json::Value::String(
            "rule candidates are backtested and handed to human review only; worker must not write active rules".into(),
        ),
    );

    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_mining_plans:{candidate_plan_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_review_tasks:{candidate_review_tasks_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_backtests:{backtest_report_uri}"),
    );
    push_unique_evidence_ref(
        &mut output.evidence_refs,
        format!("rule_candidate_review_tasks:{backtest_review_tasks_uri}"),
    );
    Ok(output)
}

fn push_unique_evidence_ref(evidence_refs: &mut Vec<String>, evidence_ref: String) {
    if !evidence_refs
        .iter()
        .any(|reference| reference == &evidence_ref)
    {
        evidence_refs.push(evidence_ref);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OnnxParityGate {
    pub(crate) report_uri: String,
    pub(crate) gate_status: String,
    pub(crate) status: String,
    pub(crate) serving_runtime_kind: String,
    pub(crate) max_abs_probability_delta: Option<f64>,
    pub(crate) tolerance: Option<f64>,
}

pub(crate) fn validate_onnx_parity_for_runtime(
    runtime_kind: &str,
    onnx_parity_report_uri: Option<&str>,
) -> anyhow::Result<Option<OnnxParityGate>> {
    if !runtime_kind.ends_with("_onnx") {
        return Ok(None);
    }
    let report_uri = onnx_parity_report_uri
        .filter(|uri| !uri.trim().is_empty())
        .ok_or_else(|| anyhow!("ONNX runtime {runtime_kind} requires onnx_parity_report_uri"))?;
    let report = read_json_report(report_uri)?;
    if report["report_kind"] != "onnx_probability_parity" {
        bail!("ONNX parity report {report_uri} has invalid report_kind");
    }
    let serving_runtime_kind =
        json_string(&report, "serving_runtime_kind").unwrap_or_else(|| "missing".into());
    let status = json_string(&report, "status").unwrap_or_else(|| "missing".into());
    let max_abs_probability_delta = metric_at(&report, "max_abs_probability_delta");
    let tolerance = metric_at(&report, "tolerance");
    let gate_status = if status == "passed" && serving_runtime_kind == runtime_kind {
        "passed"
    } else {
        "blocked"
    }
    .to_string();
    Ok(Some(OnnxParityGate {
        report_uri: report_uri.into(),
        gate_status,
        status,
        serving_runtime_kind,
        max_abs_probability_delta,
        tolerance,
    }))
}

fn model_artifact_evaluation_signing_key() -> String {
    std::env::var("FWA_MODEL_SIGNATURE_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local-dev-model-signing-key".into())
}

pub(crate) fn build_mock_retraining_output(
    job: &ClaimedRetrainingJob,
    actor: &str,
    artifact_base_uri: &str,
) -> anyhow::Result<CompleteRetrainingJobPayload> {
    if artifact_base_uri.trim().is_empty() {
        bail!("artifact_base_uri is required");
    }
    let safe_model_key = safe_path_segment(&job.model_key);
    let candidate_model_version = format!(
        "{}-candidate-{}",
        safe_path_segment(&job.model_version),
        safe_path_segment(&job.job_id)
    );
    let artifact_root = artifact_base_uri.trim().trim_end_matches('/');
    let artifact_uri =
        format!("{artifact_root}/{safe_model_key}/{candidate_model_version}/model.onnx");
    let validation_report_uri =
        format!("{artifact_root}/{safe_model_key}/{candidate_model_version}/validation.json");
    let feature_importance_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/feature_importance.parquet"
    );
    let permutation_importance_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/permutation_importance.parquet"
    );
    let artifact_evaluation_report_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/artifact-evaluation/model_artifact_evaluation_report.json"
    );
    let rule_backtest_report_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/rule-candidates/backtest/rule_candidate_backtest_report.json"
    );
    let rule_review_tasks_uri = format!(
        "{artifact_root}/{safe_model_key}/{candidate_model_version}/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"
    );
    let evaluation_run_id = format!(
        "eval_{}_{}",
        safe_id_segment(&job.model_key),
        safe_id_segment(&candidate_model_version)
    );
    let evidence_refs = vec![
        format!("model_retraining_jobs:{}", job.job_id),
        format!("model_artifacts:{artifact_uri}"),
        format!("model_validation_reports:{validation_report_uri}"),
        format!("model_feature_importance:{feature_importance_uri}"),
        format!("model_permutation_importance:{permutation_importance_uri}"),
        format!("model_artifact_evaluations:{artifact_evaluation_report_uri}"),
        format!("rule_candidate_backtests:{rule_backtest_report_uri}"),
        format!("rule_candidate_review_tasks:{rule_review_tasks_uri}"),
        format!("model_evaluations:{evaluation_run_id}"),
    ];

    Ok(CompleteRetrainingJobPayload {
        actor: actor.to_string(),
        notes: "Candidate model and validation report registered by worker.".into(),
        candidate_model_version,
        artifact_uri,
        artifact_sha256: None,
        training_artifact_uri: None,
        training_artifact_sha256: None,
        serving_manifest_uri: None,
        onnx_parity_report_uri: None,
        endpoint_url: None,
        validation_report_uri,
        evaluation_run_id,
        auc: Some("0.86".into()),
        ks: Some("0.48".into()),
        precision: Some("0.78".into()),
        recall: Some("0.71".into()),
        f1: Some("0.74".into()),
        accuracy: Some("0.79".into()),
        threshold: Some("0.52".into()),
        confusion_matrix_json: serde_json::json!({
            "tp": 24,
            "fp": 6,
            "tn": 52,
            "fn": 8
        }),
        feature_importance_uri: Some(feature_importance_uri),
        permutation_importance_uri: Some(permutation_importance_uri),
        metrics_json: serde_json::json!({
            "out_of_time_auc": 0.82,
            "out_of_time_precision": 0.76,
            "out_of_time_recall": 0.71,
            "score_psi": 0.04,
            "max_feature_psi": 0.08,
            "leakage_check_status": "passed",
            "time_group_split_status": "passed",
            "time_split_field": "service_date",
            "group_split_fields": ["member_id", "policy_id", "provider_id"],
            "feature_reproducibility_hash": "sha256:demo-retraining-feature-reproducibility",
            "label_provenance_status": "passed",
            "label_reviewer_source": "investigation_results",
            "pilot_validation_status": "passed",
            "shadow_comparison_status": "passed",
            "serving_version_lock_status": "passed",
            "artifact_integrity_status": "passed",
            "feature_store_materialization_status": "passed",
            "segment_fairness_status": "passed",
            "review_capacity_threshold_status": "passed",
            "model_artifact_evaluation_status": "passed",
            "model_artifact_evaluation_report_uri": artifact_evaluation_report_uri,
            "rule_candidate_backtest_status": "passed",
            "rule_candidate_backtest_report_uri": rule_backtest_report_uri,
            "rule_candidate_review_tasks_uri": rule_review_tasks_uri,
            "rule_library_writeback_status": "blocked_pending_human_review_and_policy_governance_approval"
        }),
        evidence_refs,
        mined_rule_owner: None,
        mined_rule_candidates: Vec::new(),
    })
}

pub(crate) fn safe_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}

pub(crate) fn safe_id_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}
