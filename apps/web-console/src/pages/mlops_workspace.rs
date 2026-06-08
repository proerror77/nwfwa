use crate::*;
use serde_json::{json, Value};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

#[path = "mlops_workspace_view.rs"]
mod mlops_workspace_view;
use mlops_workspace_view::MlopsWorkspaceView;

#[function_component(MlopsWorkspacePage)]
pub fn mlops_workspace_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let model_key = use_state(|| "baseline_fwa".to_string());
    let actor = use_state(|| "mlops-operator".to_string());
    let reviewer = use_state(|| "risk-model-owner".to_string());
    let promotion_decision = use_state(|| "approved".to_string());
    let monitoring_task_id = use_state(String::new);
    let monitoring_decision = use_state(|| "acknowledged".to_string());
    let alert_task_id = use_state(String::new);
    let alert_decision = use_state(|| "receipt_confirmed".to_string());
    let retraining_job_id = use_state(String::new);
    let retraining_status = use_state(|| "validation".to_string());
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
                        <label>
                            {"Model key"}
                            <input
                                value={(*model_key).clone()}
                                oninput={{
                                    let model_key = model_key.clone();
                                    Callback::from(move |event: InputEvent| {
                                        model_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
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
                        <label class="mlops-field">
                            {"Actor"}
                            <input
                                value={(*actor).clone()}
                                oninput={{
                                    let actor = actor.clone();
                                    Callback::from(move |event: InputEvent| {
                                        actor.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Reviewer"}
                            <input
                                value={(*reviewer).clone()}
                                oninput={{
                                    let reviewer = reviewer.clone();
                                    Callback::from(move |event: InputEvent| {
                                        reviewer.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Promotion decision"}
                            <select
                                value={(*promotion_decision).clone()}
                                onchange={{
                                    let promotion_decision = promotion_decision.clone();
                                    Callback::from(move |event: Event| {
                                        promotion_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="approved">{"approved"}</option>
                                <option value="rejected">{"rejected"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Monitoring task id"}
                            <input
                                value={(*monitoring_task_id).clone()}
                                oninput={{
                                    let monitoring_task_id = monitoring_task_id.clone();
                                    Callback::from(move |event: InputEvent| {
                                        monitoring_task_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Monitoring decision"}
                            <select
                                value={(*monitoring_decision).clone()}
                                onchange={{
                                    let monitoring_decision = monitoring_decision.clone();
                                    Callback::from(move |event: Event| {
                                        monitoring_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="acknowledged">{"acknowledged"}</option>
                                <option value="rejected">{"rejected"}</option>
                                <option value="prepare_retraining">{"prepare_retraining"}</option>
                                <option value="open_shadow_review">{"open_shadow_review"}</option>
                                <option value="open_rollback_review">{"open_rollback_review"}</option>
                                <option value="closed">{"closed"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Alert task id"}
                            <input
                                value={(*alert_task_id).clone()}
                                oninput={{
                                    let alert_task_id = alert_task_id.clone();
                                    Callback::from(move |event: InputEvent| {
                                        alert_task_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Alert decision"}
                            <select
                                value={(*alert_decision).clone()}
                                onchange={{
                                    let alert_decision = alert_decision.clone();
                                    Callback::from(move |event: Event| {
                                        alert_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="receipt_confirmed">{"receipt_confirmed"}</option>
                                <option value="delivery_failed">{"delivery_failed"}</option>
                                <option value="closed_no_action">{"closed_no_action"}</option>
                                <option value="escalated_for_governance_review">{"escalated_for_governance_review"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Training job id"}
                            <input
                                value={(*retraining_job_id).clone()}
                                oninput={{
                                    let retraining_job_id = retraining_job_id.clone();
                                    Callback::from(move |event: InputEvent| {
                                        retraining_job_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Training status"}
                            <select
                                value={(*retraining_status).clone()}
                                onchange={{
                                    let retraining_status = retraining_status.clone();
                                    Callback::from(move |event: Event| {
                                        retraining_status.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="running">{"running"}</option>
                                <option value="validation">{"validation"}</option>
                                <option value="failed">{"failed"}</option>
                                <option value="cancelled">{"cancelled"}</option>
                            </select>
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"External training payload"}
                            <textarea
                                value={(*training_output_payload_json).clone()}
                                oninput={{
                                    let training_output_payload_json = training_output_payload_json.clone();
                                    Callback::from(move |event: InputEvent| {
                                        training_output_payload_json.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <button class="mini-action" onclick={load_training_output_payload.clone()}>
                            {"Load provider output payload"}
                        </button>
                        <label class="mlops-field">
                            {"Candidate version"}
                            <input
                                value={(*candidate_model_version).clone()}
                                oninput={{
                                    let candidate_model_version = candidate_model_version.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_model_version.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate artifact"}
                            <input
                                value={(*candidate_artifact_uri).clone()}
                                oninput={{
                                    let candidate_artifact_uri = candidate_artifact_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_artifact_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate artifact SHA"}
                            <input
                                value={(*candidate_artifact_sha256).clone()}
                                oninput={{
                                    let candidate_artifact_sha256 = candidate_artifact_sha256.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_artifact_sha256.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Training artifact"}
                            <input
                                value={(*training_artifact_uri).clone()}
                                oninput={{
                                    let training_artifact_uri = training_artifact_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        training_artifact_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Training artifact SHA"}
                            <input
                                value={(*training_artifact_sha256).clone()}
                                oninput={{
                                    let training_artifact_sha256 = training_artifact_sha256.clone();
                                    Callback::from(move |event: InputEvent| {
                                        training_artifact_sha256.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Serving manifest"}
                            <input
                                value={(*serving_manifest_uri).clone()}
                                oninput={{
                                    let serving_manifest_uri = serving_manifest_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        serving_manifest_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate endpoint"}
                            <input
                                value={(*candidate_endpoint_url).clone()}
                                oninput={{
                                    let candidate_endpoint_url = candidate_endpoint_url.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_endpoint_url.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Validation report"}
                            <input
                                value={(*validation_report_uri).clone()}
                                oninput={{
                                    let validation_report_uri = validation_report_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        validation_report_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate AUC"}
                            <input
                                value={(*candidate_auc).clone()}
                                oninput={{
                                    let candidate_auc = candidate_auc.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_auc.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate KS"}
                            <input
                                value={(*candidate_ks).clone()}
                                oninput={{
                                    let candidate_ks = candidate_ks.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_ks.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate precision"}
                            <input
                                value={(*candidate_precision).clone()}
                                oninput={{
                                    let candidate_precision = candidate_precision.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_precision.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate recall"}
                            <input
                                value={(*candidate_recall).clone()}
                                oninput={{
                                    let candidate_recall = candidate_recall.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_recall.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate F1"}
                            <input
                                value={(*candidate_f1).clone()}
                                oninput={{
                                    let candidate_f1 = candidate_f1.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_f1.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate accuracy"}
                            <input
                                value={(*candidate_accuracy).clone()}
                                oninput={{
                                    let candidate_accuracy = candidate_accuracy.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_accuracy.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate threshold"}
                            <input
                                value={(*candidate_threshold).clone()}
                                oninput={{
                                    let candidate_threshold = candidate_threshold.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_threshold.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Feature importance URI"}
                            <input
                                value={(*candidate_feature_importance_uri).clone()}
                                oninput={{
                                    let candidate_feature_importance_uri = candidate_feature_importance_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_feature_importance_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Permutation importance URI"}
                            <input
                                value={(*candidate_permutation_importance_uri).clone()}
                                oninput={{
                                    let candidate_permutation_importance_uri = candidate_permutation_importance_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_permutation_importance_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Confusion matrix JSON"}
                            <textarea
                                value={(*candidate_confusion_matrix).clone()}
                                oninput={{
                                    let candidate_confusion_matrix = candidate_confusion_matrix.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_confusion_matrix.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Metrics JSON"}
                            <textarea
                                value={(*candidate_metrics_json).clone()}
                                oninput={{
                                    let candidate_metrics_json = candidate_metrics_json.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_metrics_json.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Draft rule candidate payload"}
                            <textarea
                                value={(*mined_rule_candidates_json).clone()}
                                oninput={{
                                    let mined_rule_candidates_json = mined_rule_candidates_json.clone();
                                    Callback::from(move |event: InputEvent| {
                                        mined_rule_candidates_json.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Anomaly candidate kind"}
                            <select
                                value={(*anomaly_candidate_kind).clone()}
                                onchange={{
                                    let anomaly_candidate_kind = anomaly_candidate_kind.clone();
                                    Callback::from(move |event: Event| {
                                        anomaly_candidate_kind.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="provider_peer_anomaly">{"provider_peer_anomaly"}</option>
                                <option value="provider_graph_anomaly">{"provider_graph_anomaly"}</option>
                                <option value="claim_entity_anomaly">{"claim_entity_anomaly"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Anomaly candidate id"}
                            <input
                                value={(*anomaly_candidate_id).clone()}
                                oninput={{
                                    let anomaly_candidate_id = anomaly_candidate_id.clone();
                                    Callback::from(move |event: InputEvent| {
                                        anomaly_candidate_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Anomaly report URI"}
                            <input
                                value={(*anomaly_source_report_uri).clone()}
                                oninput={{
                                    let anomaly_source_report_uri = anomaly_source_report_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        anomaly_source_report_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Anomaly decision"}
                            <select
                                value={(*anomaly_decision).clone()}
                                onchange={{
                                    let anomaly_decision = anomaly_decision.clone();
                                    Callback::from(move |event: Event| {
                                        anomaly_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="accepted_for_review">{"accepted_for_review"}</option>
                                <option value="rejected">{"rejected"}</option>
                                <option value="open_investigation_review">{"open_investigation_review"}</option>
                                <option value="request_more_evidence">{"request_more_evidence"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Anomaly evidence refs"}
                            <input
                                value={(*anomaly_evidence_refs).clone()}
                                oninput={{
                                    let anomaly_evidence_refs = anomaly_evidence_refs.clone();
                                    Callback::from(move |event: InputEvent| {
                                        anomaly_evidence_refs.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Anomaly candidate payload"}
                            <textarea
                                value={(*anomaly_candidate_payload).clone()}
                                oninput={{
                                    let anomaly_candidate_payload = anomaly_candidate_payload.clone();
                                    Callback::from(move |event: InputEvent| {
                                        anomaly_candidate_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Evidence refs"}
                            <input
                                value={(*evidence_refs).clone()}
                                oninput={{
                                    let evidence_refs = evidence_refs.clone();
                                    Callback::from(move |event: InputEvent| {
                                        evidence_refs.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-notes-field">
                            {"Notes"}
                            <textarea
                                value={(*action_notes).clone()}
                                oninput={{
                                    let action_notes = action_notes.clone();
                                    Callback::from(move |event: InputEvent| {
                                        action_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
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

#[derive(Properties, PartialEq)]
struct MlopsActionProps {
    state: ApiState<Value>,
}

#[function_component(MlopsActionView)]
fn mlops_action_view(props: &MlopsActionProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Choose an action only after evidence and reviewer context are ready."}</p> }
        }
        ApiState::Loading => {
            html! { <p>{"Submitting governed provider model release action..."}</p> }
        }
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <>
                <p class="empty">{"Action accepted by API. Workspace refresh has been requested."}</p>
                <pre>{pretty_json(response)}</pre>
            </>
        },
    }
}

async fn execute_mlops_governed_action(
    api_key: String,
    model_key: String,
    action: &str,
    actor: String,
    reviewer: String,
    promotion_decision: String,
    monitoring_task_id: String,
    monitoring_decision: String,
    alert_task_id: String,
    alert_decision: String,
    retraining_job_id: String,
    retraining_status: String,
    candidate_model_version: String,
    candidate_artifact_uri: String,
    candidate_artifact_sha256: String,
    training_artifact_uri: String,
    training_artifact_sha256: String,
    serving_manifest_uri: String,
    candidate_endpoint_url: String,
    validation_report_uri: String,
    candidate_auc: String,
    candidate_ks: String,
    candidate_precision: String,
    candidate_recall: String,
    candidate_f1: String,
    candidate_accuracy: String,
    candidate_threshold: String,
    candidate_confusion_matrix: String,
    candidate_feature_importance_uri: String,
    candidate_permutation_importance_uri: String,
    candidate_metrics_json: String,
    mined_rule_candidates_json: String,
    notes: String,
    mut evidence_refs: Vec<String>,
) -> Result<Value, String> {
    let model_key = model_key.trim();
    match action {
        "queue_retraining" => {
            request_json(
                &format!("/api/v1/ops/models/{model_key}/retraining-jobs"),
                api_key,
                json!({
                    "requested_by": actor.trim(),
                    "notes": notes.trim(),
                }),
            )
            .await
        }
        "monitoring_review"
        | "monitoring_reject"
        | "monitoring_prepare"
        | "monitoring_rollback" => {
            if monitoring_task_id.trim().is_empty() {
                return Err("monitoring review actions require a monitoring task id".into());
            }
            if evidence_refs.is_empty() {
                return Err("monitoring review actions require evidence refs".into());
            }
            let decision = match action {
                "monitoring_reject" => "rejected",
                "monitoring_prepare" => "prepare_retraining",
                "monitoring_rollback" => "open_rollback_review",
                _ => monitoring_decision.trim(),
            };
            request_json(
                &format!(
                    "/api/v1/ops/models/{model_key}/mlops-monitoring-review-tasks/{}/reviews",
                    monitoring_task_id.trim()
                ),
                api_key,
                json!({
                    "decision": decision,
                    "reviewer": reviewer.trim(),
                    "notes": notes.trim(),
                    "evidence_refs": evidence_refs,
                }),
            )
            .await
        }
        "alert_review" | "alert_escalate" => {
            if alert_task_id.trim().is_empty() {
                return Err("alert review actions require an alert task id".into());
            }
            if evidence_refs.is_empty() {
                return Err("alert review actions require evidence refs".into());
            }
            let decision = if action == "alert_escalate" {
                "escalated_for_governance_review"
            } else {
                alert_decision.trim()
            };
            request_json(
                &format!(
                    "/api/v1/ops/models/{model_key}/mlops-alert-delivery-tasks/{}/reviews",
                    alert_task_id.trim()
                ),
                api_key,
                json!({
                    "decision": decision,
                    "reviewer": reviewer.trim(),
                    "notes": notes.trim(),
                    "evidence_refs": evidence_refs,
                }),
            )
            .await
        }
        "claim_retraining_job" => {
            request_json(
                "/api/v1/ops/model-retraining-jobs/claim-next",
                api_key,
                json!({
                    "actor": actor.trim(),
                    "notes": notes.trim(),
                    "model_key": model_key,
                }),
            )
            .await
        }
        "update_retraining_job" => {
            if retraining_job_id.trim().is_empty() {
                return Err("training job status updates require a training job id".into());
            }
            request_json(
                &format!(
                    "/api/v1/ops/model-retraining-jobs/{}/status",
                    retraining_job_id.trim()
                ),
                api_key,
                json!({
                    "status": retraining_status.trim(),
                    "actor": actor.trim(),
                    "notes": notes.trim(),
                }),
            )
            .await
        }
        "register_retraining_output" => {
            if retraining_job_id.trim().is_empty() {
                return Err("provider output registration requires a training job id".into());
            }
            if evidence_refs.is_empty() {
                return Err("provider output registration requires evidence refs".into());
            }
            let confusion_matrix_json =
                parse_json_object(&candidate_confusion_matrix, "confusion matrix")?;
            let metrics_json = parse_json_object(&candidate_metrics_json, "metrics")?;
            let auc = parse_optional_unit_metric(&candidate_auc, "AUC")?;
            let ks = parse_optional_unit_metric(&candidate_ks, "KS")?;
            let precision = parse_optional_unit_metric(&candidate_precision, "precision")?;
            let recall = parse_optional_unit_metric(&candidate_recall, "recall")?;
            let f1 = parse_optional_unit_metric(&candidate_f1, "F1")?;
            let accuracy = parse_optional_unit_metric(&candidate_accuracy, "accuracy")?;
            let threshold = parse_optional_unit_metric(&candidate_threshold, "threshold")?;
            let artifact_sha256 = optional_trimmed_value(&candidate_artifact_sha256);
            let training_artifact_uri = optional_trimmed_value(&training_artifact_uri);
            let training_artifact_sha256 = optional_trimmed_value(&training_artifact_sha256);
            let serving_manifest_uri = optional_trimmed_value(&serving_manifest_uri);
            let endpoint_url = optional_trimmed_value(&candidate_endpoint_url);
            let feature_importance_uri = optional_trimmed_value(&candidate_feature_importance_uri);
            let permutation_importance_uri =
                optional_trimmed_value(&candidate_permutation_importance_uri);
            let mined_rule_candidates =
                parse_optional_json_array(&mined_rule_candidates_json, "mined rule candidates")?;
            let evaluation_run_id = format!(
                "eval_{}_{}",
                model_key,
                candidate_model_version
                    .trim()
                    .replace('.', "_")
                    .replace('-', "_")
            );
            evidence_refs = push_unique(
                evidence_refs,
                format!("model_retraining_jobs:{}", retraining_job_id.trim()),
            );
            evidence_refs = push_unique(
                evidence_refs,
                format!("model_artifacts:{}", candidate_artifact_uri.trim()),
            );
            if let Some(training_artifact_uri) = &training_artifact_uri {
                evidence_refs = push_unique(
                    evidence_refs,
                    format!("model_training_artifacts:{training_artifact_uri}"),
                );
            }
            if let Some(serving_manifest_uri) = &serving_manifest_uri {
                evidence_refs = push_unique(
                    evidence_refs,
                    format!("model_serving_manifests:{serving_manifest_uri}"),
                );
            }
            if let Some(permutation_importance_uri) = &permutation_importance_uri {
                evidence_refs = push_unique(
                    evidence_refs,
                    format!("model_permutation_importance:{permutation_importance_uri}"),
                );
            }
            evidence_refs = push_unique(
                evidence_refs,
                format!("model_validation_reports:{}", validation_report_uri.trim()),
            );
            evidence_refs = push_unique(
                evidence_refs,
                format!("model_evaluations:{evaluation_run_id}"),
            );
            request_json(
                &format!(
                    "/api/v1/ops/model-retraining-jobs/{}/output",
                    retraining_job_id.trim()
                ),
                api_key,
                json!({
                    "actor": actor.trim(),
                    "notes": notes.trim(),
                    "candidate_model_version": candidate_model_version.trim(),
                    "artifact_uri": candidate_artifact_uri.trim(),
                    "artifact_sha256": artifact_sha256,
                    "training_artifact_uri": training_artifact_uri,
                    "training_artifact_sha256": training_artifact_sha256,
                    "serving_manifest_uri": serving_manifest_uri,
                    "endpoint_url": endpoint_url,
                    "validation_report_uri": validation_report_uri.trim(),
                    "evaluation_run_id": evaluation_run_id,
                    "evidence_refs": evidence_refs,
                    "auc": auc,
                    "ks": ks,
                    "precision": precision,
                    "recall": recall,
                    "f1": f1,
                    "accuracy": accuracy,
                    "threshold": threshold,
                    "confusion_matrix_json": confusion_matrix_json,
                    "feature_importance_uri": feature_importance_uri,
                    "permutation_importance_uri": permutation_importance_uri,
                    "metrics_json": metrics_json,
                    "mined_rule_owner": "external-training-platform",
                    "mined_rule_candidates": mined_rule_candidates,
                }),
            )
            .await
        }
        "promotion_review" => {
            if evidence_refs.is_empty() {
                return Err("model promotion review requires evidence refs".into());
            }
            let model_version = candidate_model_version.trim();
            if model_version.is_empty() {
                return Err("model promotion review requires a candidate version".into());
            }
            request_json(
                &format!(
                    "/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-reviews"
                ),
                api_key,
                json!({
                    "decision": promotion_decision.trim(),
                    "reviewer": reviewer.trim(),
                    "notes": notes.trim(),
                    "evidence_refs": evidence_refs,
                }),
            )
            .await
        }
        "activate" => {
            if evidence_refs.is_empty() {
                return Err("model lifecycle actions require evidence refs".into());
            }
            let model_version = candidate_model_version.trim();
            if model_version.is_empty() {
                return Err("model activation requires a candidate version".into());
            }
            request_json(
                &format!("/api/v1/ops/models/{model_key}/versions/{model_version}/activate"),
                api_key,
                json!({ "evidence_refs": evidence_refs }),
            )
            .await
        }
        "rollback" => {
            if evidence_refs.is_empty() {
                return Err("model lifecycle actions require evidence refs".into());
            }
            request_json(
                &format!("/api/v1/ops/models/{model_key}/rollback"),
                api_key,
                json!({ "evidence_refs": evidence_refs }),
            )
            .await
        }
        _ => Err(format!("unknown MLOps action: {action}")),
    }
}

async fn submit_anomaly_candidate_review(
    api_key: String,
    candidate_kind: String,
    candidate_id: String,
    source_report_uri: String,
    decision: String,
    reviewer: String,
    notes: String,
    evidence_refs: Vec<String>,
    candidate_payload: Value,
) -> Result<Value, String> {
    request_json::<Value>(
        "/api/v1/ops/providers/anomaly-candidate-reviews",
        api_key,
        json!({
            "candidate_kind": candidate_kind.trim(),
            "candidate_id": candidate_id.trim(),
            "source_report_uri": source_report_uri.trim(),
            "decision": decision.trim(),
            "reviewer": reviewer.trim(),
            "notes": notes.trim(),
            "evidence_refs": evidence_refs,
            "candidate_payload": candidate_payload,
        }),
    )
    .await
}
