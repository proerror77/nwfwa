use crate::api::*;
use crate::types::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::model_ui_helpers::*;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

#[function_component(ModelsPage)]
pub fn models_page() -> Html {
    let api_key = use_api_key();
    let model_key = use_state(|| "baseline_fwa".to_string());
    let snapshot_state = use_state(|| ApiState::<ModelOpsSnapshot>::Idle);

    let load_models = {
        let api_key = api_key.clone();
        let model_key = model_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let model_key = (*model_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_model_ops_snapshot(api_key, model_key, None).await {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let refresh = {
        let load_models = load_models.clone();
        Callback::from(move |_| load_models.emit(()))
    };

    {
        let load_models = load_models.clone();
        use_effect_with((), move |_| {
            load_models.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Models"}</h2>
                    <p>{"Monitor model versions, scoring drift, promotion gates, QA feedback closure, and retraining readiness."}</p>
                </div>
                <span class="status-pill">{"Model Governance"}</span>
            </div>

            <section class="panel">
                <h3>{"Model Source"}</h3>
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
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh model governance" }}
                    </button>
                </div>
            </section>

            <ModelOpsView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ModelOpsProps {
    state: ApiState<ModelOpsSnapshot>,
}

#[function_component(ModelOpsView)]
fn model_ops_view(props: &ModelOpsProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load model governance to inspect production readiness."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading model governance..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {model_monitoring_cockpit(snapshot)}
                        <section class="panel result-stack">
                            <h3>{"Model Inventory"}</h3>
                            <div class="factor-card-grid">
                                {for snapshot.models.iter().map(|model| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{format!("{} {}", model.model_key, model.version)}</strong>
                                            {{
                                                let review_mode_tone = match model.review_mode.as_str() {
                                                    "pre_payment" | "pre" => "warning",
                                                    "post_payment" | "post" => "neutral",
                                                    _ => "info",
                                                };
                                                html! { <span class={classes!("status-pill", review_mode_tone)}>{&model.review_mode}</span> }
                                            }}
                                            <span>{format!("{} / {} / {}", model.model_type, model.runtime_kind, model.execution_provider)}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Status"}</span><strong>{&model.status}</strong></div>
                                            <div><span>{"Review Mode"}</span><strong>{&model.review_mode}</strong></div>
                                            <div><span>{"Endpoint"}</span><strong>{model.endpoint_url.as_deref().unwrap_or("none")}</strong></div>
                                        </div>
                                        <small>{format!("artifact: {}", model.artifact_uri.as_deref().unwrap_or("none"))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Model Performance"}</h3>
                            {model_telemetry_visual(&snapshot.performance, &snapshot.gates, &snapshot.retraining)}
                            <div class="score-hero">
                                <div><span>{"Model"}</span><strong>{&snapshot.performance.model_key}</strong></div>
                                <div><span>{"Drift"}</span><strong>{&snapshot.performance.drift_status}</strong></div>
                                <div><span>{"Score PSI"}</span><strong>{optional_number(snapshot.performance.score_psi)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Scored Runs"}</span><strong>{snapshot.performance.scored_runs}</strong></div>
                                <div><span>{"Avg Score"}</span><strong>{format!("{:.1}", snapshot.performance.average_score)}</strong></div>
                                <div><span>{"High Risk"}</span><strong>{snapshot.performance.high_risk_count}</strong></div>
                            </div>
                            <small>{format!("data: {} / latest scored: {}", snapshot.performance.data_status, snapshot.performance.latest_scored_at.as_deref().unwrap_or("none"))}</small>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Promotion Gates"}</h3>
                            <div class="score-hero">
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                                <div><span>{"Evaluation"}</span><strong>{&snapshot.gates.latest_evaluation_id}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Data Quality"}</span><strong>{&snapshot.gates.source_data_quality_status}</strong></div>
                                <div><span>{"Labels"}</span><strong>{snapshot.gates.approved_label_count}</strong></div>
                                <div><span>{"Open Feedback"}</span><strong>{snapshot.gates.unresolved_model_feedback_count}</strong></div>
                            </div>
                            if snapshot.gates.blockers.is_empty() {
                                <p class="empty">{"No promotion blockers."}</p>
                            } else {
                                <ul class="result-list compact-list">
                                    {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                            <div class="factor-card-grid">
                                {for snapshot.gates.gates.iter().map(|gate| html! {
                                    <div class="metric-row">
                                        <span>{&gate.label}</span>
                                        <strong>{if gate.passed { "passed" } else { "blocked" }}</strong>
                                        <small>{&gate.evidence_source}</small>
                                        <small>{&gate.blocker}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Retraining Readiness"}</h3>
                            <div class="score-hero">
                                <div><span>{"Recommendation"}</span><strong>{&snapshot.retraining.recommendation}</strong></div>
                                <div><span>{"Drift"}</span><strong>{&snapshot.retraining.drift_status}</strong></div>
                                <div><span>{"Data Quality"}</span><strong>{&snapshot.retraining.source_data_quality_status}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Open Feedback"}</span><strong>{snapshot.retraining.open_model_feedback_count}</strong></div>
                                <div><span>{"Approved Labels"}</span><strong>{snapshot.retraining.approved_label_count}</strong></div>
                                <div><span>{"Needs Review"}</span><strong>{snapshot.retraining.needs_review_label_count}</strong></div>
                            </div>
                            <h4>{"Triggers"}</h4>
                            if snapshot.retraining.retraining_triggers.is_empty() {
                                <p class="empty">{"No retraining triggers."}</p>
                            } else {
                                <ul class="result-list">
                                    {for snapshot.retraining.retraining_triggers.iter().map(|trigger| html! { <li>{trigger}</li> })}
                                </ul>
                            }
                            <h4>{"Blockers"}</h4>
                            if snapshot.retraining.blockers.is_empty() {
                                <p class="empty">{"No retraining blockers."}</p>
                            } else {
                                <ul class="result-list">
                                    {for snapshot.retraining.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}
