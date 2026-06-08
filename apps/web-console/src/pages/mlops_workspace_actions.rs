use crate::*;

#[derive(Properties, PartialEq)]
pub(crate) struct MlopsActionProps {
    pub(crate) state: ApiState<Value>,
}

#[function_component(MlopsActionView)]
pub(crate) fn mlops_action_view(props: &MlopsActionProps) -> Html {
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

pub(crate) async fn execute_mlops_governed_action(
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

pub(crate) async fn submit_anomaly_candidate_review(
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
