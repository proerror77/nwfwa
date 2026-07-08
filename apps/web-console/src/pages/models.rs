use crate::api::*;
use crate::formatting::*;
use crate::model_ui_helpers::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

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
                    <h2>{"模型治理"}</h2>
                    <p>{"监控模型版本、评分漂移、发布门禁、QA 反馈闭环与重训准备度。"}</p>
                </div>
                <span class="status-pill">{"模型治理"}</span>
            </div>

            <section class="panel model-source-bar">
                <div>
                    <p class="section-eyebrow">{"当前检查对象"}</p>
                    <h3>{"模型发布准备度"}</h3>
                </div>
                <div class="form-grid">
                    <label>
                        {"模型标识"}
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
                        {if matches!(&*snapshot_state, ApiState::Loading) { "刷新中..." } else { "刷新模型治理" }}
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
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"加载模型治理数据后可检查生产准备度。"}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"正在加载模型治理数据..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {model_monitoring_cockpit(snapshot)}
                        <section class="panel result-stack">
                            <h3>{"模型清单"}</h3>
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
                                            <div><span>{"状态"}</span><strong>{&model.status}</strong></div>
                                            <div><span>{"复核模式"}</span><strong>{&model.review_mode}</strong></div>
                                            <div><span>{"端点"}</span><strong>{model.endpoint_url.as_deref().unwrap_or("无")}</strong></div>
                                        </div>
                                        <small>{format!("产物：{}", model.artifact_uri.as_deref().unwrap_or("无"))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"模型表现"}</h3>
                            {model_telemetry_visual(&snapshot.performance, &snapshot.gates, &snapshot.retraining)}
                            <div class="score-hero">
                                <div><span>{"模型"}</span><strong>{&snapshot.performance.model_key}</strong></div>
                                <div><span>{"漂移"}</span><strong>{&snapshot.performance.drift_status}</strong></div>
                                <div><span>{"评分 PSI"}</span><strong>{optional_number(snapshot.performance.score_psi)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"评分运行"}</span><strong>{snapshot.performance.scored_runs}</strong></div>
                                <div><span>{"平均分"}</span><strong>{format!("{:.1}", snapshot.performance.average_score)}</strong></div>
                                <div><span>{"高风险"}</span><strong>{snapshot.performance.high_risk_count}</strong></div>
                            </div>
                            <small>{format!("数据：{} / 最新评分：{}", snapshot.performance.data_status, snapshot.performance.latest_scored_at.as_deref().unwrap_or("无"))}</small>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"发布门禁"}</h3>
                            <div class="score-hero">
                                <div><span>{"决策"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"通过"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                                <div><span>{"评估"}</span><strong>{&snapshot.gates.latest_evaluation_id}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"数据质量"}</span><strong>{&snapshot.gates.source_data_quality_status}</strong></div>
                                <div><span>{"标签"}</span><strong>{snapshot.gates.approved_label_count}</strong></div>
                                <div><span>{"未关闭反馈"}</span><strong>{snapshot.gates.unresolved_model_feedback_count}</strong></div>
                            </div>
                            if snapshot.gates.blockers.is_empty() {
                                <p class="empty">{"没有发布阻塞项。"}</p>
                            } else {
                                <ul class="result-list compact-list">
                                    {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                            <div class="factor-card-grid">
                                {for snapshot.gates.gates.iter().map(|gate| html! {
                                    <div class="metric-row">
                                        <span>{&gate.label}</span>
                                        <strong>{if gate.passed { "通过" } else { "阻塞" }}</strong>
                                        <small>{&gate.evidence_source}</small>
                                        <small>{&gate.blocker}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"重训准备度"}</h3>
                            <div class="score-hero">
                                <div><span>{"建议"}</span><strong>{&snapshot.retraining.recommendation}</strong></div>
                                <div><span>{"漂移"}</span><strong>{&snapshot.retraining.drift_status}</strong></div>
                                <div><span>{"数据质量"}</span><strong>{&snapshot.retraining.source_data_quality_status}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"开放反馈"}</span><strong>{snapshot.retraining.open_model_feedback_count}</strong></div>
                                <div><span>{"已批准标签"}</span><strong>{snapshot.retraining.approved_label_count}</strong></div>
                                <div><span>{"待复核"}</span><strong>{snapshot.retraining.needs_review_label_count}</strong></div>
                            </div>
                            <h4>{"触发条件"}</h4>
                            if snapshot.retraining.retraining_triggers.is_empty() {
                                <p class="empty">{"没有重训触发条件。"}</p>
                            } else {
                                <ul class="result-list">
                                    {for snapshot.retraining.retraining_triggers.iter().map(|trigger| html! { <li>{trigger}</li> })}
                                </ul>
                            }
                            <h4>{"阻塞项"}</h4>
                            if snapshot.retraining.blockers.is_empty() {
                                <p class="empty">{"没有重训阻塞项。"}</p>
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
