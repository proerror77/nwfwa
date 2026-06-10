use crate::api::*;
use crate::types::*;
use crate::constants::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::ui_helpers::*;
use crate::visual_helpers::*;
use crate::case_helpers::*;
use crate::rule_helpers::*;
use crate::rule_ui_helpers::*;
use crate::inbox_helpers::*;
use crate::payload_helpers::*;
use crate::data_helpers::*;
use crate::data_lineage_helpers::*;
use crate::medical_review_helpers::*;
use crate::model_ui_helpers::*;
use crate::runtime_helpers::*;
use yew::prelude::*;
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;

#[path = "mlops_workspace_view.rs"]
mod mlops_workspace_view;
use mlops_workspace_view::MlopsWorkspaceView;
#[path = "mlops_workspace_actions.rs"]
mod mlops_workspace_actions;
use mlops_workspace_actions::{
    execute_mlops_governed_action, submit_anomaly_candidate_review, MlopsActionView,
};
#[path = "mlops_workspace_fields.rs"]
mod mlops_workspace_fields;
use mlops_workspace_fields::{
    mlops_select_field, mlops_text_field, mlops_text_field_with_class, mlops_textarea_field,
};

#[function_component(MlopsWorkspacePage)]
pub fn mlops_workspace_page() -> Html {
    // ── Common ───────────────────────────────────────────────────────────────
    let api_key = use_api_key();
    let model_key = use_state(|| "baseline_fwa".to_string());
    let actor = use_state(|| "mlops-operator".to_string());
    let reviewer = use_state(|| "risk-model-owner".to_string());

    // ── Monitoring task ──────────────────────────────────────────────────────
    let monitoring_task_id = use_state(String::new);
    let monitoring_decision = use_state(|| "acknowledged".to_string());

    // ── Alert task ───────────────────────────────────────────────────────────
    let alert_task_id = use_state(String::new);
    let alert_decision = use_state(|| "receipt_confirmed".to_string());

    // ── Retraining job ───────────────────────────────────────────────────────
    let retraining_job_id = use_state(String::new);
    let retraining_status = use_state(|| "validation".to_string());
    // ── Candidate / promotion ────────────────────────────────────────────────
    let promotion_decision = use_state(|| "approved".to_string());
    let candidate_model_version = use_state(|| "0.2.0-candidate".to_string());
    let candidate_artifact_uri = use_state(|| {
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json".to_string()
    });
    let candidate_artifact_sha256 = use_state(|| "sha256:rust-serving-artifact".to_string());
    let training_artifact_uri =
        use_state(|| "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib".to_string());
    let training_artifact_sha256 = use_state(|| "sha256:training-artifact".to_string());
    let serving_manifest_uri = use_state(|| {
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json".to_string()
    });
    let candidate_endpoint_url =
        use_state(|| "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-candidate".to_string());
    let validation_report_uri =
        use_state(|| "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json".to_string());
    let candidate_auc = use_state(|| "0.92".to_string());
    let candidate_ks = use_state(|| "0.51".to_string());
    let candidate_precision = use_state(|| "0.78".to_string());
    let candidate_recall = use_state(|| "0.64".to_string());
    let candidate_f1 = use_state(|| "0.70".to_string());
    let candidate_accuracy = use_state(|| "0.89".to_string());
    let candidate_threshold = use_state(|| "0.70".to_string());
    let candidate_confusion_matrix =
        use_state(|| r#"{"tp": 64, "fp": 18, "tn": 820, "fn": 36}"#.to_string());
    let candidate_feature_importance_uri = use_state(|| {
        "data/eval/provider_retraining_candidate/feature_importance.parquet".to_string()
    });
    let candidate_permutation_importance_uri = use_state(|| {
        "data/eval/provider_retraining_candidate/permutation_importance.parquet".to_string()
    });
    let candidate_metrics_json = use_state(|| {
        r#"{"data_quality_status":"passed","split_strategy":"time_group_split","shadow_comparison_status":"passed","review_capacity_threshold_status":"passed"}"#.to_string()
    });
    let mined_rule_candidates_json = use_state(|| {
        r#"[{"rule_id":"candidate_retraining_amount_ratio","version":1,"name":"Retraining mined amount ratio candidate","review_mode":"both","scheme_family":"high_risk_claim","conditions":[{"field":"claim_amount_to_limit_ratio","operator":">=","value":0.82}],"action":{"score":22,"alert_code":"RETRAINING_AMOUNT_RATIO_CANDIDATE","recommended_action":"ManualReview","reason":"External training platform mined this explainable candidate from feature importance and backtest evidence."}}]"#.to_string()
    });
    let training_output_payload_json = use_state(|| {
        r#"{"candidate_model_version":"0.2.0-candidate","artifact_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json","artifact_sha256":"sha256:rust-serving-artifact","training_artifact_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib","training_artifact_sha256":"sha256:training-artifact","serving_manifest_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json","endpoint_url":"http://127.0.0.1:8001/score/baseline_fwa/0.2.0-candidate","validation_report_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json","feature_importance_uri":"data/eval/provider_retraining_candidate/feature_importance.parquet","permutation_importance_uri":"data/eval/provider_retraining_candidate/permutation_importance.parquet","metrics_json":{"shadow_comparison_status":"passed","review_capacity_threshold_status":"passed","model_artifact_evaluation_status":"passed","model_artifact_evaluation_report_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json","rule_candidate_backtest_status":"passed","rule_candidate_backtest_report_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json","rule_candidate_review_tasks_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json","rule_library_writeback_status":"blocked_pending_human_review_and_policy_governance_approval"},"confusion_matrix_json":{"tp":64,"fp":18,"tn":820,"fn":36},"evidence_refs":["model_artifact_evaluations:s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json","rule_candidate_backtests:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json","rule_candidate_review_tasks:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"],"mined_rule_candidates":[]}"#.to_string()
    });
    // ── Anomaly candidate review ─────────────────────────────────────────────
    let anomaly_candidate_kind = use_state(|| "provider_peer_anomaly".to_string());
    let anomaly_candidate_id = use_state(|| "provider_peer:PRV-042:2026-05".to_string());
    let anomaly_source_report_uri = use_state(|| {
        "data/rust-automl-demo/unlabeled_provider_peer_clustering/clusters/provider_peer_clustering_report.json".to_string()
    });
    let anomaly_decision = use_state(|| "accepted_for_review".to_string());
    let anomaly_evidence_refs = use_state(|| {
        "anomaly_clustering_reports:data/rust-automl-demo/unlabeled_provider_peer_clustering/clusters/provider_peer_clustering_report.json, provider_peer_anomaly:PRV-042:2026-05".to_string()
    });
    let anomaly_candidate_payload = use_state(|| {
        r#"{"provider_id":"PRV-042","outlier_score":0.93,"reason":"peer z-score and high-cost rate exceed cohort threshold"}"#.to_string()
    });
    let action_notes = use_state(|| {
        "non-PII governed provider model release review for demo evidence".to_string()
    });
    let evidence_refs = use_state(|| "model_versions:baseline_fwa:v1".to_string());
    let snapshot_state = use_state(|| ApiState::<MlopsWorkspaceSnapshot>::Idle);
    let action_state = use_state(|| ApiState::<Value>::Idle);

    // ── Callbacks ────────────────────────────────────────────────────────────
    let load_workspace = {
        let api_key = api_key.clone();
        let model_key = model_key.clone();
        let candidate_model_version = candidate_model_version.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let model_key = (*model_key).clone();
            let candidate_model_version = (*candidate_model_version).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_mlops_workspace_snapshot(api_key, model_key, candidate_model_version)
                        .await
                    {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let refresh = {
        let load_workspace = load_workspace.clone();
        Callback::from(move |_| load_workspace.emit(()))
    };

    let governed_action = |action: &'static str| {
        let api_key = api_key.clone();
        let model_key = model_key.clone();
        let actor = actor.clone();
        let reviewer = reviewer.clone();
        let promotion_decision = promotion_decision.clone();
        let monitoring_task_id = monitoring_task_id.clone();
        let monitoring_decision = monitoring_decision.clone();
        let alert_task_id = alert_task_id.clone();
        let alert_decision = alert_decision.clone();
        let retraining_job_id = retraining_job_id.clone();
        let retraining_status = retraining_status.clone();
        let candidate_model_version = candidate_model_version.clone();
        let candidate_artifact_uri = candidate_artifact_uri.clone();
        let candidate_artifact_sha256 = candidate_artifact_sha256.clone();
        let training_artifact_uri = training_artifact_uri.clone();
        let training_artifact_sha256 = training_artifact_sha256.clone();
        let serving_manifest_uri = serving_manifest_uri.clone();
        let candidate_endpoint_url = candidate_endpoint_url.clone();
        let validation_report_uri = validation_report_uri.clone();
        let candidate_auc = candidate_auc.clone();
        let candidate_ks = candidate_ks.clone();
        let candidate_precision = candidate_precision.clone();
        let candidate_recall = candidate_recall.clone();
        let candidate_f1 = candidate_f1.clone();
        let candidate_accuracy = candidate_accuracy.clone();
        let candidate_threshold = candidate_threshold.clone();
        let candidate_confusion_matrix = candidate_confusion_matrix.clone();
        let candidate_feature_importance_uri = candidate_feature_importance_uri.clone();
        let candidate_permutation_importance_uri = candidate_permutation_importance_uri.clone();
        let candidate_metrics_json = candidate_metrics_json.clone();
        let mined_rule_candidates_json = mined_rule_candidates_json.clone();
        let action_notes = action_notes.clone();
        let evidence_refs = evidence_refs.clone();
        let action_state = action_state.clone();
        let load_workspace = load_workspace.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let model_key = (*model_key).clone();
            let actor = (*actor).clone();
            let reviewer = (*reviewer).clone();
            let promotion_decision = (*promotion_decision).clone();
            let monitoring_task_id = (*monitoring_task_id).clone();
            let monitoring_decision = (*monitoring_decision).clone();
            let alert_task_id = (*alert_task_id).clone();
            let alert_decision = (*alert_decision).clone();
            let retraining_job_id_state = retraining_job_id.clone();
            let retraining_job_id = (*retraining_job_id).clone();
            let retraining_status = (*retraining_status).clone();
            let candidate_model_version = (*candidate_model_version).clone();
            let candidate_artifact_uri = (*candidate_artifact_uri).clone();
            let candidate_artifact_sha256 = (*candidate_artifact_sha256).clone();
            let training_artifact_uri = (*training_artifact_uri).clone();
            let training_artifact_sha256 = (*training_artifact_sha256).clone();
            let serving_manifest_uri = (*serving_manifest_uri).clone();
            let candidate_endpoint_url = (*candidate_endpoint_url).clone();
            let validation_report_uri = (*validation_report_uri).clone();
            let candidate_auc = (*candidate_auc).clone();
            let candidate_ks = (*candidate_ks).clone();
            let candidate_precision = (*candidate_precision).clone();
            let candidate_recall = (*candidate_recall).clone();
            let candidate_f1 = (*candidate_f1).clone();
            let candidate_accuracy = (*candidate_accuracy).clone();
            let candidate_threshold = (*candidate_threshold).clone();
            let candidate_confusion_matrix = (*candidate_confusion_matrix).clone();
            let candidate_feature_importance_uri = (*candidate_feature_importance_uri).clone();
            let candidate_permutation_importance_uri =
                (*candidate_permutation_importance_uri).clone();
            let candidate_metrics_json = (*candidate_metrics_json).clone();
            let mined_rule_candidates_json = (*mined_rule_candidates_json).clone();
            let action_notes = (*action_notes).clone();
            let evidence_refs = parse_tags(&evidence_refs);
            let action_state = action_state.clone();
            let load_workspace = load_workspace.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                let result = execute_mlops_governed_action(
                    api_key,
                    model_key,
                    action,
                    actor,
                    reviewer,
                    promotion_decision,
                    monitoring_task_id,
                    monitoring_decision,
                    alert_task_id,
                    alert_decision,
                    retraining_job_id,
                    retraining_status,
                    candidate_model_version,
                    candidate_artifact_uri,
                    candidate_artifact_sha256,
                    training_artifact_uri,
                    training_artifact_sha256,
                    serving_manifest_uri,
                    candidate_endpoint_url,
                    validation_report_uri,
                    candidate_auc,
                    candidate_ks,
                    candidate_precision,
                    candidate_recall,
                    candidate_f1,
                    candidate_accuracy,
                    candidate_threshold,
                    candidate_confusion_matrix,
                    candidate_feature_importance_uri,
                    candidate_permutation_importance_uri,
                    candidate_metrics_json,
                    mined_rule_candidates_json,
                    action_notes,
                    evidence_refs,
                )
                .await;
                match result {
                    Ok(response) => {
                        if let Some(job_id) = response_retraining_job_id(&response) {
                            retraining_job_id_state.set(job_id);
                        }
                        action_state.set(ApiState::Ready(response));
                        load_workspace.emit(());
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let review_anomaly_candidate = {
        let api_key = api_key.clone();
        let reviewer = reviewer.clone();
        let action_notes = action_notes.clone();
        let anomaly_candidate_kind = anomaly_candidate_kind.clone();
        let anomaly_candidate_id = anomaly_candidate_id.clone();
        let anomaly_source_report_uri = anomaly_source_report_uri.clone();
        let anomaly_decision = anomaly_decision.clone();
        let anomaly_evidence_refs = anomaly_evidence_refs.clone();
        let anomaly_candidate_payload = anomaly_candidate_payload.clone();
        let action_state = action_state.clone();
        let load_workspace = load_workspace.clone();
        Callback::from(move |_| {
            let payload =
                match parse_json_object(&anomaly_candidate_payload, "anomaly candidate payload") {
                    Ok(payload) => payload,
                    Err(error) => {
                        action_state.set(ApiState::Failed(error));
                        return;
                    }
                };
            let api_key = (*api_key).clone();
            let reviewer = (*reviewer).clone();
            let notes = (*action_notes).clone();
            let candidate_kind = (*anomaly_candidate_kind).clone();
            let candidate_id = (*anomaly_candidate_id).clone();
            let source_report_uri = (*anomaly_source_report_uri).clone();
            let decision = (*anomaly_decision).clone();
            let evidence_refs = parse_tags(&anomaly_evidence_refs);
            let action_state = action_state.clone();
            let load_workspace = load_workspace.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                match submit_anomaly_candidate_review(
                    api_key,
                    candidate_kind,
                    candidate_id,
                    source_report_uri,
                    decision,
                    reviewer,
                    notes,
                    evidence_refs,
                    payload,
                )
                .await
                {
                    Ok(response) => {
                        action_state.set(ApiState::Ready(response));
                        load_workspace.emit(());
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let load_training_output_payload = {
        let training_output_payload_json = training_output_payload_json.clone();
        let candidate_model_version = candidate_model_version.clone();
        let candidate_artifact_uri = candidate_artifact_uri.clone();
        let candidate_artifact_sha256 = candidate_artifact_sha256.clone();
        let training_artifact_uri = training_artifact_uri.clone();
        let training_artifact_sha256 = training_artifact_sha256.clone();
        let serving_manifest_uri = serving_manifest_uri.clone();
        let candidate_endpoint_url = candidate_endpoint_url.clone();
        let validation_report_uri = validation_report_uri.clone();
        let candidate_auc = candidate_auc.clone();
        let candidate_ks = candidate_ks.clone();
        let candidate_precision = candidate_precision.clone();
        let candidate_recall = candidate_recall.clone();
        let candidate_f1 = candidate_f1.clone();
        let candidate_accuracy = candidate_accuracy.clone();
        let candidate_threshold = candidate_threshold.clone();
        let candidate_confusion_matrix = candidate_confusion_matrix.clone();
        let candidate_feature_importance_uri = candidate_feature_importance_uri.clone();
        let candidate_permutation_importance_uri = candidate_permutation_importance_uri.clone();
        let candidate_metrics_json = candidate_metrics_json.clone();
        let mined_rule_candidates_json = mined_rule_candidates_json.clone();
        let evidence_refs = evidence_refs.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let payload =
                match parse_json_object(&training_output_payload_json, "external training payload")
                {
                    Ok(payload) => payload,
                    Err(error) => {
                        action_state.set(ApiState::Failed(error));
                        return;
                    }
                };
            if let Some(value) = json_string_field(&payload, "candidate_model_version") {
                candidate_model_version.set(value);
            }
            if let Some(value) = json_string_field(&payload, "artifact_uri") {
                candidate_artifact_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "artifact_sha256") {
                candidate_artifact_sha256.set(value);
            }
            if let Some(value) = json_string_field(&payload, "training_artifact_uri") {
                training_artifact_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "training_artifact_sha256") {
                training_artifact_sha256.set(value);
            }
            if let Some(value) = json_string_field(&payload, "serving_manifest_uri") {
                serving_manifest_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "endpoint_url") {
                candidate_endpoint_url.set(value);
            }
            if let Some(value) = json_string_field(&payload, "validation_report_uri") {
                validation_report_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "feature_importance_uri") {
                candidate_feature_importance_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "permutation_importance_uri") {
                candidate_permutation_importance_uri.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "auc") {
                candidate_auc.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "ks") {
                candidate_ks.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "precision") {
                candidate_precision.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "recall") {
                candidate_recall.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "f1") {
                candidate_f1.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "accuracy") {
                candidate_accuracy.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "threshold") {
                candidate_threshold.set(value);
            }
            if let Some(confusion_matrix) = payload.get("confusion_matrix_json") {
                candidate_confusion_matrix.set(pretty_json(confusion_matrix));
            }
            if let Some(metrics) = payload.get("metrics_json") {
                candidate_metrics_json.set(pretty_json(metrics));
            }
            if let Some(candidates) = payload.get("mined_rule_candidates") {
                mined_rule_candidates_json.set(pretty_json(candidates));
            }
            if let Some(refs) = payload
                .get("evidence_refs")
                .and_then(|value| value.as_array())
            {
                let refs = refs
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect::<Vec<_>>();
                evidence_refs.set(refs.join(", "));
            }
            action_state.set(ApiState::Idle);
        })
    };

    let select_anomaly_review_task = {
        let anomaly_candidate_kind = anomaly_candidate_kind.clone();
        let anomaly_candidate_id = anomaly_candidate_id.clone();
        let anomaly_source_report_uri = anomaly_source_report_uri.clone();
        let anomaly_decision = anomaly_decision.clone();
        let anomaly_evidence_refs = anomaly_evidence_refs.clone();
        let anomaly_candidate_payload = anomaly_candidate_payload.clone();
        Callback::from(move |task: AnomalyReviewQueueTask| {
            anomaly_candidate_kind.set(task.candidate_kind);
            anomaly_candidate_id.set(task.candidate_id);
            anomaly_source_report_uri.set(task.source_report_uri);
            anomaly_decision.set("request_more_evidence".into());
            anomaly_evidence_refs.set(task.evidence_refs.join(", "));
            anomaly_candidate_payload.set(
                serde_json::to_string_pretty(&task.candidate_payload)
                    .unwrap_or_else(|_| "{}".into()),
            );
        })
    };

    let select_monitoring_review_task = {
        let monitoring_task_id = monitoring_task_id.clone();
        let monitoring_decision = monitoring_decision.clone();
        let evidence_refs = evidence_refs.clone();
        Callback::from(move |task: ModelMonitoringReviewTask| {
            let mut refs = vec![
                format!("model_versions:{}:{}", task.model_key, task.model_version),
                format!("model_monitoring_reports:{}", task.report_uri),
                format!("model_monitoring_review_tasks:{}", task.task_id),
            ];
            for evidence_ref in task.evidence_refs {
                refs = push_unique(refs, evidence_ref);
            }
            let decision = if task.retraining_recommendation == "prepare_retraining" {
                "prepare_retraining"
            } else if task.task_kind.contains("shadow") || task.trigger.contains("shadow") {
                "open_shadow_review"
            } else {
                "acknowledged"
            };
            monitoring_task_id.set(task.task_id);
            monitoring_decision.set(decision.into());
            evidence_refs.set(refs.join(", "));
        })
    };

    let select_retraining_job = {
        let retraining_job_id = retraining_job_id.clone();
        let retraining_status = retraining_status.clone();
        let candidate_model_version = candidate_model_version.clone();
        let candidate_artifact_uri = candidate_artifact_uri.clone();
        let candidate_endpoint_url = candidate_endpoint_url.clone();
        let validation_report_uri = validation_report_uri.clone();
        let evidence_refs = evidence_refs.clone();
        Callback::from(move |job: ModelRetrainingJobRecord| {
            retraining_job_id.set(job.job_id.clone());
            retraining_status.set(job.status.clone());
            let evidence_model_version = job
                .candidate_model_version
                .clone()
                .unwrap_or_else(|| job.model_version.clone());
            if let Some(version) = job.candidate_model_version {
                candidate_model_version.set(version);
            }
            if let Some(uri) = job.candidate_artifact_uri {
                candidate_artifact_uri.set(uri);
            }
            if let Some(url) = job.candidate_endpoint_url {
                candidate_endpoint_url.set(url);
            }
            if let Some(uri) = job.validation_report_uri {
                validation_report_uri.set(uri);
            }
            let mut refs = vec![
                format!("model_retraining_jobs:{}", job.job_id),
                format!(
                    "model_versions:{}:{}",
                    job.model_key, evidence_model_version
                ),
            ];
            if let Some(evaluation_id) = job.output_evaluation_id {
                refs = push_unique(refs, format!("model_evaluations:{evaluation_id}"));
            }
            evidence_refs.set(refs.join(", "));
        })
    };

    let (activation_allowed, activation_blockers) = match &*snapshot_state {
        ApiState::Ready(snapshot) => (
            snapshot.model_ops.gates.blockers.is_empty(),
            snapshot.model_ops.gates.blockers.clone(),
        ),
        ApiState::Idle | ApiState::Loading => {
            (false, vec!["load promotion gates before activation".into()])
        }
        ApiState::Failed(error) => (false, vec![error.clone()]),
    };

    {
        let load_workspace = load_workspace.clone();
        use_effect_with((), move |_| {
            load_workspace.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Provider Model Intake"}</h2>
                    <p>{"Review provider-delivered model candidates after offline training. Operators compare evidence and decide shadow, limited rollout, activation, rejection, or rollback."}</p>
                </div>
                <span class="status-pill">{"Provider candidate release"}</span>
            </div>

            <div class="mlops-cockpit">
                <section class="panel mlops-source-panel">
                    <div class="section-header compact">
                        <div>
                            <h3>{"Candidate Source"}</h3>
                            <p>{"Select the provider-trained model candidate to inspect."}</p>
                        </div>
                    </div>
                    <div class="form-grid">
                        {text_input("Model key", &model_key)}
                    </div>
                    <div class="button-row">
                        <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                            {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh workspace" }}
                        </button>
                    </div>
                </section>

                <section class="panel result-stack mlops-action-panel">
                    <div class="section-header compact">
                        <div>
                            <h3>{"Governed Actions"}</h3>
                            <p>{"Lifecycle actions require reviewer context and evidence refs before backend gates accept them."}</p>
                        </div>
                        <span class="status-token strong">{"manual evidence required"}</span>
                    </div>
                    <div class="mlops-action-grid">
                        {mlops_text_field("Actor", &actor)}
                        {mlops_text_field("Reviewer", &reviewer)}
                        {mlops_select_field("Promotion decision", &promotion_decision, &["approved", "rejected"])}
                        {mlops_text_field("Monitoring task id", &monitoring_task_id)}
                        {mlops_select_field("Monitoring decision", &monitoring_decision, &["acknowledged", "rejected", "prepare_retraining", "open_shadow_review", "open_rollback_review", "closed"])}
                        {mlops_text_field("Alert task id", &alert_task_id)}
                        {mlops_select_field("Alert decision", &alert_decision, &["receipt_confirmed", "delivery_failed", "closed_no_action", "escalated_for_governance_review"])}
                        {mlops_text_field("Training job id", &retraining_job_id)}
                        {mlops_select_field("Training status", &retraining_status, &["running", "validation", "failed", "cancelled"])}
                        {mlops_textarea_field("External training payload", &training_output_payload_json, "mlops-evidence-field")}
                        <button class="mini-action" onclick={load_training_output_payload.clone()}>
                            {"Load provider output payload"}
                        </button>
                        {mlops_text_field("Candidate version", &candidate_model_version)}
                        {mlops_text_field("Candidate artifact", &candidate_artifact_uri)}
                        {mlops_text_field("Candidate artifact SHA", &candidate_artifact_sha256)}
                        {mlops_text_field("Training artifact", &training_artifact_uri)}
                        {mlops_text_field("Training artifact SHA", &training_artifact_sha256)}
                        {mlops_text_field("Serving manifest", &serving_manifest_uri)}
                        {mlops_text_field("Candidate endpoint", &candidate_endpoint_url)}
                        {mlops_text_field("Validation report", &validation_report_uri)}
                        {mlops_text_field("Candidate AUC", &candidate_auc)}
                        {mlops_text_field("Candidate KS", &candidate_ks)}
                        {mlops_text_field("Candidate precision", &candidate_precision)}
                        {mlops_text_field("Candidate recall", &candidate_recall)}
                        {mlops_text_field("Candidate F1", &candidate_f1)}
                        {mlops_text_field("Candidate accuracy", &candidate_accuracy)}
                        {mlops_text_field("Candidate threshold", &candidate_threshold)}
                        {mlops_text_field("Feature importance URI", &candidate_feature_importance_uri)}
                        {mlops_text_field("Permutation importance URI", &candidate_permutation_importance_uri)}
                        {mlops_textarea_field("Confusion matrix JSON", &candidate_confusion_matrix, "mlops-evidence-field")}
                        {mlops_textarea_field("Metrics JSON", &candidate_metrics_json, "mlops-evidence-field")}
                        {mlops_textarea_field("Draft rule candidate payload", &mined_rule_candidates_json, "mlops-evidence-field")}
                        {mlops_select_field("Anomaly candidate kind", &anomaly_candidate_kind, &["provider_peer_anomaly", "provider_graph_anomaly", "claim_entity_anomaly"])}
                        {mlops_text_field("Anomaly candidate id", &anomaly_candidate_id)}
                        {mlops_text_field("Anomaly report URI", &anomaly_source_report_uri)}
                        {mlops_select_field("Anomaly decision", &anomaly_decision, &["accepted_for_review", "rejected", "open_investigation_review", "request_more_evidence"])}
                        {mlops_text_field("Anomaly evidence refs", &anomaly_evidence_refs)}
                        {mlops_textarea_field("Anomaly candidate payload", &anomaly_candidate_payload, "mlops-evidence-field")}
                        {mlops_text_field_with_class("Evidence refs", &evidence_refs, "mlops-evidence-field")}
                        {mlops_textarea_field("Notes", &action_notes, "mlops-notes-field")}
                        <div class="mlops-boundary-card">
                            <span>{"Boundary"}</span>
                            <strong>{"Evidence before action"}</strong>
                            <small>{"Provider rule candidates are saved as drafts only. Review them one by one in Discovery Review with backtest and shadow evidence before accepting or rejecting."}</small>
                        </div>
                    </div>
                    <div class="button-row mlops-action-buttons">
                        <button onclick={governed_action("queue_retraining")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Request provider retraining"}</button>
                        <button onclick={governed_action("monitoring_review")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit monitoring decision"}</button>
                        <button onclick={governed_action("monitoring_reject")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Reject task"}</button>
                        <button onclick={governed_action("monitoring_prepare")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Prepare retraining from task"}</button>
                        <button onclick={governed_action("monitoring_rollback")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Open rollback review"}</button>
                        <button onclick={governed_action("alert_review")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit alert decision"}</button>
                        <button onclick={governed_action("alert_escalate")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Escalate alert to review"}</button>
                        <button onclick={governed_action("register_retraining_output")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Register completed provider output"}</button>
                        <button onclick={review_anomaly_candidate} disabled={matches!(&*action_state, ApiState::Loading)}>{"Review anomaly candidate"}</button>
                        <button onclick={governed_action("promotion_review")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit release review"}</button>
                        <button onclick={governed_action("activate")} disabled={!activation_allowed || matches!(&*action_state, ApiState::Loading)}>{"Activate approved candidate"}</button>
                        <button onclick={governed_action("rollback")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Rollback active model"}</button>
                    </div>
                    if !activation_allowed {
                        <div class="compact-list">
                            <span>{"Activation blocked by promotion gates"}</span>
                            {for activation_blockers.iter().map(|blocker| html! { <span>{blocker}</span> })}
                        </div>
                    }
                    <MlopsActionView state={(*action_state).clone()} />
                </section>
            </div>

            <MlopsWorkspaceView
                state={(*snapshot_state).clone()}
                on_select_monitoring_task={select_monitoring_review_task}
                on_select_anomaly={select_anomaly_review_task}
                on_select_retraining_job={select_retraining_job}
            />
        </section>
    }
}
