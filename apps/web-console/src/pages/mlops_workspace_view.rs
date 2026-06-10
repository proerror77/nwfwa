use crate::types::*;
use crate::state::ApiState;
use crate::formatting::*;
use crate::ui_helpers::*;
use crate::data_helpers::*;
use crate::data_lineage_helpers::*;
use crate::model_ui_helpers::*;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct MlopsWorkspaceProps {
    pub(crate) state: ApiState<MlopsWorkspaceSnapshot>,
    pub(crate) on_select_monitoring_task: Callback<ModelMonitoringReviewTask>,
    pub(crate) on_select_anomaly: Callback<AnomalyReviewQueueTask>,
    pub(crate) on_select_retraining_job: Callback<ModelRetrainingJobRecord>,
}

#[function_component(MlopsWorkspaceView)]
pub(crate) fn mlops_workspace_view(props: &MlopsWorkspaceProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load provider model intake to inspect candidate release evidence."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading provider model intake..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <div class="mlops-workspace-grid">
                        {provider_model_release_summary(snapshot)}
                        {mlops_model_candidates(snapshot)}
                        {mlops_promotion_gates(snapshot)}
                        {mlops_monitoring_summary(snapshot)}
                        {mlops_monitoring_review_queue(snapshot, &props.on_select_monitoring_task)}
                        {mlops_anomaly_review_queue(snapshot, &props.on_select_anomaly)}
                        {mlops_alert_delivery_queue(snapshot)}
                        {mlops_training_handoff(snapshot)}
                        {mlops_dataset_readiness(snapshot)}
                        {mlops_training_jobs(snapshot, &props.on_select_retraining_job)}
                    </div>
                },
            }}
        </>
    }
}

fn provider_model_release_summary(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    let active_model = active_model_version(&snapshot.model_ops);
    let release_decision = provider_release_decision_label(snapshot);
    html! {
        <section class="panel data-command-center">
            <div class="section-header">
                <div>
                    <h3>{"Provider Candidate Release Control"}</h3>
                    <p>{"This is the business release desk for provider-trained model candidates. Operators do not train or tune models here; they decide shadow, limited rollout, activation, rejection, or rollback from evidence."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.model_ops.gates.decision))}>{&snapshot.model_ops.gates.decision}</span>
            </div>
            <div class="ops-stat-strip">
                <div><span>{"Candidate"}</span><strong>{&snapshot.model_ops.gates.model_version}</strong><small>{&snapshot.model_ops.performance.model_key}</small></div>
                <div><span>{"Evidence"}</span><strong>{format!("{} / {}", snapshot.model_ops.gates.passed_count, snapshot.model_ops.gates.total_count)}</strong><small>{"promotion gates"}</small></div>
                <div><span>{"Current Active"}</span><strong>{active_model.map(|model| model.version.as_str()).unwrap_or("none")}</strong><small>{"serving lock target"}</small></div>
                <div><span>{"Shadow Signal"}</span><strong>{&snapshot.model_ops.performance.drift_status}</strong><small>{optional_number(snapshot.model_ops.performance.score_psi)}</small></div>
                <div><span>{"Next Decision"}</span><strong>{release_decision}</strong><small>{"human gate"}</small></div>
            </div>
        </section>
    }
}

fn mlops_training_handoff(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    let dataset = latest_dataset(&snapshot.data_sources.datasets);
    let active_model = active_model_version(&snapshot.model_ops);
    html! {
        <section class="panel result-stack mlops-handoff-panel">
            <div class="section-header">
                <div>
                    <h3>{"Offline Training Handoff"}</h3>
                    <p>{"The UI exposes the contract that an external training platform must consume and return. Training remains offline; promotion remains human-governed."}</p>
                </div>
                <span class="status-token strong">{"human review required"}</span>
            </div>
            <details class="data-source-detail governance-detail release-evidence-detail">
                <summary>{"Provider training handoff detail"}</summary>
                <div class="summary-grid">
                    <div><span>{"Dataset manifest"}</span><strong>{dataset.map(|item| item.manifest_uri.as_str()).unwrap_or("missing")}</strong></div>
                    <div><span>{"Dataset version"}</span><strong>{dataset.map(dataset_version_label).unwrap_or_else(|| "missing".into())}</strong></div>
                    <div><span>{"Model key"}</span><strong>{&snapshot.model_ops.performance.model_key}</strong></div>
                    <div><span>{"Base version"}</span><strong>{active_model.map(|model| model.version.as_str()).unwrap_or("none")}</strong></div>
                    <div><span>{"Expected output"}</span><strong>{"/api/v1/ops/model-retraining-jobs/{job_id}/output"}</strong></div>
                    <div><span>{"Artifact boundary"}</span><strong>{active_model.and_then(|model| model.artifact_uri.as_deref()).unwrap_or("candidate artifact pending")}</strong></div>
                </div>
                <div class="factor-card-grid mlops-stage-grid">
                    {mlops_handoff_step("1", "Dataset approval", "Use a governed Parquet manifest with time and group split evidence.")}
                    {mlops_handoff_step("2", "Provider training", "External platform writes model, validation, feature, shadow, drift, and fairness artifacts.")}
                    {mlops_handoff_step("3", "Candidate registration", "Training output creates a candidate model and evaluation through the API.")}
                    {mlops_handoff_step("4", "Human release", "Promotion gates and reviewer decision decide shadow, activation, or rejection.")}
                </div>
            </details>
        </section>
    }
}

fn mlops_handoff_step(step: &str, label: &str, detail: &str) -> Html {
    html! {
        <div class="metric-row">
            <span>{format!("Step {step}")}</span>
            <strong>{label}</strong>
            <small>{detail}</small>
        </div>
    }
}

fn mlops_dataset_readiness(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-datasets-panel">
            <div class="section-header">
                <div>
                    <h3>{"Datasets"}</h3>
                    <p>{"Training data must show source scope, label policy, split quality, schema health, and production-evidence boundary before promotion."}</p>
                </div>
            </div>
            if snapshot.data_sources.datasets.is_empty() {
                <p class="empty">{"No datasets registered for provider model review."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail">
                    <summary>{format!("Release dataset evidence detail: {} datasets", snapshot.data_sources.datasets.len())}</summary>
                    <div class="factor-card-grid">
                        {for snapshot.data_sources.datasets.iter().map(|dataset| {
                            let health = health_for_dataset(&snapshot.data_sources.health, &dataset.dataset_id);
                            html! {
                                <div class="factor-card">
                                    <div>
                                        <strong>{dataset_version_label(dataset)}</strong>
                                        <span>{format!("{} / {} / {}", dataset.business_domain, dataset.sample_grain, dataset.storage_format)}</span>
                                    </div>
                                    <div class="summary-grid">
                                        <div><span>{"Rows"}</span><strong>{dataset.row_count}</strong></div>
                                        <div><span>{"Splits"}</span><strong>{dataset.splits.len()}</strong></div>
                                        <div><span>{"Fields"}</span><strong>{dataset.fields.len()}</strong></div>
                                        <div><span>{"Mappings"}</span><strong>{dataset.mappings.len()}</strong></div>
                                        <div><span>{"Label"}</span><strong>{empty_label(&dataset.label_column)}</strong></div>
                                        <div><span>{"Quality"}</span><strong>{health.map(|item| item.data_quality_status.as_str()).unwrap_or("missing")}</strong></div>
                                    </div>
                                    <small>{format!("manifest: {}", dataset.manifest_uri)}</small>
                                </div>
                            }
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

fn mlops_training_jobs(
    snapshot: &MlopsWorkspaceSnapshot,
    on_select_job: &Callback<ModelRetrainingJobRecord>,
) -> Html {
    html! {
        <section class="panel result-stack mlops-training-panel">
            <div class="section-header">
                <div>
                    <h3>{"Provider Output Handoff"}</h3>
                    <p>{"External training status is read here only as handoff evidence. FWA registers completed candidate outputs; it does not operate the training platform."}</p>
                </div>
                <span class="status-token neutral">{format!("{} jobs", snapshot.retraining_jobs.len())}</span>
            </div>
            if snapshot.retraining_jobs.is_empty() {
                <p class="empty">{"No retraining jobs returned for this model."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail">
                    <summary>{format!("Provider training job detail: {} jobs", snapshot.retraining_jobs.len())}</summary>
                    <div class="ops-table">
                        <div class="ops-table-head">
                            <span>{"Job"}</span>
                            <span>{"Status"}</span>
                            <span>{"Dataset"}</span>
                            <span>{"Candidate"}</span>
                            <span>{"Updated"}</span>
                        </div>
                        {for snapshot.retraining_jobs.iter().map(|job| html! {
                            <div class="ops-table-row">
                                <div class="primary-cell">
                                    <strong>{&job.job_id}</strong>
                                    <span>{format!("{} {} / requested by {}", job.model_key, job.model_version, job.requested_by)}</span>
                                </div>
                                <span class={classes!("status-token", status_tone(&job.status))}>{&job.status}</span>
                                <span>{format!("{} / {}", job.source_dataset_id, job.source_data_quality_status)}</span>
                                <span>{job.candidate_model_version.as_deref().unwrap_or("pending")}</span>
                                <span>{job.updated_at.as_deref().unwrap_or("missing")}</span>
                                <small class="row-detail">{format!("trigger {} / blocker {} / output {}", refs_label(&job.trigger_summary), refs_label(&job.blocker_summary), job.output_evaluation_id.as_deref().unwrap_or("none"))}</small>
                                <button
                                    class="mini-action"
                                    onclick={{
                                        let on_select_job = on_select_job.clone();
                                        let job = job.clone();
                                        Callback::from(move |_| on_select_job.emit(job.clone()))
                                    }}
                                >
                                    {"Use for output registration"}
                                </button>
                            </div>
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

fn mlops_model_candidates(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-candidates-panel">
            <div class="section-header">
                <div>
                    <h3>{"Model Candidates"}</h3>
                    <p>{"Active and candidate versions are inspected through runtime kind, artifact URI, evaluation lineage, and deployment status."}</p>
                </div>
            </div>
            if snapshot.model_ops.models.is_empty() {
                <p class="empty">{"No model versions returned."}</p>
            } else {
                <div class="factor-card-grid">
                    {for snapshot.model_ops.models.iter().map(|model| html! {
                        <div class="factor-card">
                            <div>
                                <strong>{format!("{} {}", model.model_key, model.version)}</strong>
                                <span>{format!("{} / {} / {}", model.status, model.runtime_kind, model.execution_provider)}</span>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Type"}</span><strong>{&model.model_type}</strong></div>
                                <div><span>{"Review Mode"}</span><strong>{&model.review_mode}</strong></div>
                                <div><span>{"Endpoint"}</span><strong>{model.endpoint_url.as_deref().unwrap_or("none")}</strong></div>
                            </div>
                            <small>{format!("artifact: {}", model.artifact_uri.as_deref().unwrap_or("none"))}</small>
                        </div>
                    })}
                </div>
            }
        </section>
    }
}

fn mlops_promotion_gates(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-promotion-panel">
            <div class="section-header">
                <div>
                    <h3>{"Promotion Gates"}</h3>
                    <p>{"Promotion gates keep data quality, label provenance, shadow evidence, drift, fairness, and approval requirements visible before activation."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.model_ops.gates.decision))}>{&snapshot.model_ops.gates.decision}</span>
            </div>
            <div class="score-hero">
                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.model_ops.gates.passed_count, snapshot.model_ops.gates.total_count)}</strong></div>
                <div><span>{"Evaluation"}</span><strong>{&snapshot.model_ops.gates.latest_evaluation_id}</strong></div>
                <div><span>{"Approved Labels"}</span><strong>{snapshot.model_ops.gates.approved_label_count}</strong></div>
            </div>
            <div class="summary-grid">
                <div><span>{"Serving Manifest"}</span><strong>{snapshot.model_ops.gates.artifact_evidence.serving_manifest_uri.as_deref().unwrap_or("missing")}</strong></div>
                <div><span>{"Artifact Report"}</span><strong>{snapshot.model_ops.gates.artifact_evidence.model_artifact_evaluation_report_uri.as_deref().unwrap_or("missing")}</strong></div>
                <div><span>{"Rust Serving"}</span><strong>{snapshot.model_ops.gates.artifact_evidence.rust_serving_status.as_deref().unwrap_or("missing")}</strong></div>
                <div><span>{"P95 Latency"}</span><strong>{model_latency_label(&snapshot.model_ops.gates.artifact_evidence)}</strong></div>
            </div>
            if snapshot.model_ops.gates.blockers.is_empty() {
                <p class="empty">{"No promotion blockers returned."}</p>
            } else {
                <ul class="result-list compact-list">
                    {for snapshot.model_ops.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                </ul>
            }
            <details class="data-source-detail governance-detail release-evidence-detail">
                <summary>{format!("Promotion gate detail: {} gates", snapshot.model_ops.gates.gates.len())}</summary>
                <div class="governance-check-list">
                    {for snapshot.model_ops.gates.gates.iter().map(|gate| html! {
                        <div>
                            <strong>{&gate.label}</strong>
                            <span class={classes!("status-token", if gate.passed { "success" } else { "danger" })}>{if gate.passed { "passed" } else { "blocked" }}</span>
                            <small>{&gate.evidence_source}</small>
                            <small>{&gate.blocker}</small>
                        </div>
                    })}
                </div>
            </details>
        </section>
    }
}

fn model_latency_label(evidence: &ModelArtifactEvidence) -> String {
    let latency = match (
        evidence.rust_serving_latency_status.as_deref(),
        evidence.rust_serving_p95_latency_ms,
    ) {
        (Some(status), Some(ms)) => format!("{status} / {ms}ms"),
        (Some(status), None) => status.to_string(),
        (None, Some(ms)) => format!("{ms}ms"),
        (None, None) => "missing".into(),
    };
    match (
        evidence.rust_serving_latency_measurement_kind.as_deref(),
        evidence.rust_serving_latency_sample_count,
    ) {
        (Some(kind), Some(count)) => format!("{latency} ({kind}, n={count})"),
        (Some(kind), None) => format!("{latency} ({kind})"),
        (None, Some(count)) => format!("{latency} (n={count})"),
        (None, None) => latency,
    }
}

fn mlops_monitoring_summary(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-monitoring-panel">
            <div class="section-header">
                <div>
                    <h3>{"Monitoring"}</h3>
                    <p>{"Monitoring should trigger retraining readiness, shadow review, or rollback review. It must not automatically promote a model."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.model_ops.retraining.recommendation))}>{&snapshot.model_ops.retraining.recommendation}</span>
            </div>
            <div class="summary-grid">
                <div><span>{"Scored Runs"}</span><strong>{snapshot.model_ops.performance.scored_runs}</strong></div>
                <div><span>{"Average Score"}</span><strong>{format!("{:.1}", snapshot.model_ops.performance.average_score)}</strong></div>
                <div><span>{"High Risk"}</span><strong>{snapshot.model_ops.performance.high_risk_count}</strong></div>
                <div><span>{"Score PSI"}</span><strong>{optional_number(snapshot.model_ops.performance.score_psi)}</strong></div>
                <div><span>{"Drift"}</span><strong>{&snapshot.model_ops.performance.drift_status}</strong></div>
                <div><span>{"Open Feedback"}</span><strong>{snapshot.model_ops.retraining.open_model_feedback_count}</strong></div>
                <div><span>{"Needs Review"}</span><strong>{snapshot.model_ops.retraining.needs_review_label_count}</strong></div>
                <div><span>{"Data Quality"}</span><strong>{&snapshot.model_ops.retraining.source_data_quality_status}</strong></div>
            </div>
            <h4>{"Retraining Triggers"}</h4>
            if snapshot.model_ops.retraining.retraining_triggers.is_empty() {
                <p class="empty">{"No retraining triggers."}</p>
            } else {
                <ul class="result-list compact-list">
                    {for snapshot.model_ops.retraining.retraining_triggers.iter().map(|trigger| html! { <li>{trigger}</li> })}
                </ul>
            }
        </section>
    }
}

fn mlops_monitoring_review_queue(
    snapshot: &MlopsWorkspaceSnapshot,
    on_select_monitoring_task: &Callback<ModelMonitoringReviewTask>,
) -> Html {
    html! {
        <section class="panel result-stack mlops-monitoring-panel">
            <div class="section-header">
                <div>
                    <h3>{"Monitoring Review Queue"}</h3>
                    <p>{"Submitted monitoring reports open human review tasks for drift, shadow, serving, or fairness signals before retraining or rollback can proceed."}</p>
                </div>
                <span class="status-token neutral">{format!("{} tasks", snapshot.monitoring_review_tasks.len())}</span>
            </div>
            if snapshot.monitoring_review_tasks.is_empty() {
                <p class="empty">{"No monitoring review tasks returned for this model."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail" open=true>
                    <summary>{format!("Open monitoring review detail: {} tasks", snapshot.monitoring_review_tasks.len())}</summary>
                    <div class="ops-table">
                        <div class="ops-table-head">
                            <span>{"Task"}</span>
                            <span>{"Trigger"}</span>
                            <span>{"Status"}</span>
                            <span>{"Recommendation"}</span>
                            <span>{"Evidence"}</span>
                        </div>
                        {for snapshot.monitoring_review_tasks.iter().map(|task| html! {
                            <div class="ops-table-row">
                                <div class="primary-cell">
                                    <strong>{&task.task_kind}</strong>
                                    <span>{format!("{} {} / {}", task.model_key, task.model_version, task.audit_id)}</span>
                                </div>
                                <span>{empty_label(&task.trigger)}</span>
                                <span class={classes!("status-token", status_tone(&task.review_status))}>{&task.review_status}</span>
                                <span>{format!("{} / {}", task.monitoring_status, task.retraining_recommendation)}</span>
                                <span>{refs_label(&task.evidence_refs)}</span>
                                <small class="row-detail">{format!("required refs model_versions:{}:{}; model_monitoring_reports:{}; model_monitoring_review_tasks:{}", task.model_key, task.model_version, task.report_uri, task.task_id)}</small>
                                <button
                                    class="mini-action"
                                    onclick={{
                                        let on_select_monitoring_task = on_select_monitoring_task.clone();
                                        let task = task.clone();
                                        Callback::from(move |_| on_select_monitoring_task.emit(task.clone()))
                                    }}
                                >
                                    {"Use for monitoring review"}
                                </button>
                            </div>
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

fn mlops_anomaly_review_queue(
    snapshot: &MlopsWorkspaceSnapshot,
    on_select_anomaly: &Callback<AnomalyReviewQueueTask>,
) -> Html {
    html! {
        <section class="panel result-stack mlops-monitoring-panel">
            <div class="section-header">
                <div>
                    <h3>{"Anomaly Review Queue"}</h3>
                    <p>{"Unsupervised clustering can only open explainable anomaly candidates for human review. Tasks stay pending_human_review until a reviewer records accepted_for_review, rejected, open_investigation_review, or request_more_evidence; decisions here do not create cases, assign labels, activate models, or write rules."}</p>
                </div>
                <span class="status-token neutral">{format!("{} candidates", snapshot.anomaly_review_tasks.len())}</span>
            </div>
            if snapshot.anomaly_review_tasks.is_empty() {
                <p class="empty">{"No anomaly review tasks returned."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail" open=true>
                    <summary>{format!("Anomaly review detail: {} candidates", snapshot.anomaly_review_tasks.len())}</summary>
                    <div class="ops-table">
                        <div class="ops-table-head">
                            <span>{"Candidate"}</span>
                            <span>{"Kind"}</span>
                            <span>{"Status"}</span>
                            <span>{"Decision"}</span>
                            <span>{"Evidence"}</span>
                        </div>
                        {for snapshot.anomaly_review_tasks.iter().map(|task| html! {
                            <div class="ops-table-row">
                                <div class="primary-cell">
                                    <strong>{&task.candidate_id}</strong>
                                    <span>{format!("{} / {}", task.dataset_key, task.dataset_version)}</span>
                                </div>
                                <span>{format!("{} / {}", task.candidate_kind, task.report_kind)}</span>
                                <span class={classes!("status-token", status_tone(&task.review_status))}>{&task.review_status}</span>
                                <span>{task.decision.as_deref().unwrap_or("pending decision")}</span>
                                <span>{refs_label(&task.evidence_refs)}</span>
                                <small class="row-detail">{format!("queue {} / required {} / report {}", task.review_queue, task.required_review, task.source_report_uri)}</small>
                                <small class="row-detail">{format!("options: {}", refs_label(&task.decision_options))}</small>
                                <button
                                    class="mini-action"
                                    onclick={{
                                        let on_select_anomaly = on_select_anomaly.clone();
                                        let task = task.clone();
                                        Callback::from(move |_| on_select_anomaly.emit(task.clone()))
                                    }}
                                >
                                    {"Use for review"}
                                </button>
                            </div>
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

fn mlops_alert_delivery_queue(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-monitoring-panel">
            <div class="section-header">
                <div>
                    <h3>{"Alert Delivery Queue"}</h3>
                    <p>{"Alert delivery tasks track customer alert-router handoff and receipt confirmation before any governance escalation."}</p>
                </div>
                <span class="status-token neutral">{format!("{} alerts", snapshot.alert_delivery_tasks.len())}</span>
            </div>
            if snapshot.alert_delivery_tasks.is_empty() {
                <p class="empty">{"No alert delivery tasks returned for this model."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail" open=true>
                    <summary>{format!("Alert delivery detail: {} tasks", snapshot.alert_delivery_tasks.len())}</summary>
                    <div class="ops-table">
                        <div class="ops-table-head">
                            <span>{"Task"}</span>
                            <span>{"Route"}</span>
                            <span>{"Delivery"}</span>
                            <span>{"Receipt"}</span>
                            <span>{"Evidence"}</span>
                        </div>
                        {for snapshot.alert_delivery_tasks.iter().map(|task| html! {
                            <div class="ops-table-row">
                                <div class="primary-cell">
                                    <strong>{&task.task_kind}</strong>
                                    <span>{format!("{} {} / {}", task.model_key, task.model_version, task.audit_id)}</span>
                                </div>
                                <span>{format!("{} / {}", empty_label(&task.trigger), empty_label(&task.route_key))}</span>
                                <span class={classes!("status-token", status_tone(&task.delivery_status))}>{&task.delivery_status}</span>
                                <span class={classes!("status-token", status_tone(&task.review_status))}>{&task.review_status}</span>
                                <span>{refs_label(&task.evidence_refs)}</span>
                                <small class="row-detail">{format!("required refs model_versions:{}:{}; mlops_scheduler_execution_reports:{}; mlops_alert_delivery_tasks:{}", task.model_key, task.model_version, task.scheduler_execution_report_uri, task.task_id)}</small>
                            </div>
                        })}
                    </div>
                </details>
            }
        </section>
    }
}
