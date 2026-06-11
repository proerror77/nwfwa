use super::provider_risk::provider_signal_row;
use crate::api::*;
use crate::case_helpers::*;
use crate::data_helpers::*;
use crate::formatting::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use crate::ui_helpers::*;
use serde_json::json;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[function_component(RoutingPoliciesPage)]
pub fn routing_policies_page() -> Html {
    let api_key = use_api_key();
    let policy_id = use_state(|| "fwa_risk_fusion_routing".to_string());
    let review_mode = use_state(|| "pre_payment".to_string());
    let version = use_state(|| "1".to_string());
    let evidence_refs =
        use_state(|| "routing_policies:fwa_risk_fusion_routing:v1:pre_payment".to_string());
    let snapshot_state = use_state(|| ApiState::<RoutingPolicySnapshot>::Idle);
    let action_state = use_state(|| ApiState::<RoutingPolicyRecord>::Idle);

    let load_policies = {
        let api_key = api_key.clone();
        let policy_id = policy_id.clone();
        let review_mode = review_mode.clone();
        let version = version.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let policy_id = (*policy_id).clone();
            let review_mode = (*review_mode).clone();
            let version = (*version).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_routing_policy_snapshot(api_key, policy_id, review_mode, version)
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
        let load_policies = load_policies.clone();
        Callback::from(move |_| load_policies.emit(()))
    };

    let lifecycle_action = |action: &'static str| {
        let api_key = api_key.clone();
        let policy_id = policy_id.clone();
        let review_mode = review_mode.clone();
        let version = version.clone();
        let evidence_refs = evidence_refs.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let policy_id = (*policy_id).clone();
            let review_mode = (*review_mode).clone();
            let version = (*version).clone();
            let evidence_refs = parse_tags(&evidence_refs);
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                action_state.set(
                    match update_routing_policy_lifecycle(
                        api_key,
                        policy_id,
                        review_mode,
                        version,
                        action,
                        evidence_refs,
                    )
                    .await
                    {
                        Ok(record) => ApiState::Ready(record),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    {
        let load_policies = load_policies.clone();
        use_effect_with((), move |_| {
            load_policies.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"审核分流策略"}</h2>
                    <p>{"设置不同风险等级的案件进入自动通过、抽样复核或人工审核；策略必须经过提交、批准、激活，出问题时可回滚，并且每次动作都要带 evidence_refs。这里不做最终赔付或拒赔裁决。"}</p>
                </div>
                <span class="status-pill">{"发布治理"}</span>
            </div>

            <section class="panel">
                <h3>{"策略生命周期控制"}</h3>
                <div class="form-grid">
                    {text_input("策略 ID", &policy_id)}
                    {text_input("审核模式", &review_mode)}
                    {text_input("版本", &version)}
                    {text_input("证据引用 evidence_refs", &evidence_refs)}
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "刷新中..." } else { "刷新策略" }}
                    </button>
                    <button onclick={lifecycle_action("submit")} disabled={matches!(&*action_state, ApiState::Loading)}>{"提交"}</button>
                    <button onclick={lifecycle_action("approve")} disabled={matches!(&*action_state, ApiState::Loading)}>{"批准"}</button>
                    <button onclick={lifecycle_action("activate")} disabled={matches!(&*action_state, ApiState::Loading)}>{"激活"}</button>
                    <button onclick={lifecycle_action("rollback")} disabled={matches!(&*action_state, ApiState::Loading)}>{"回滚"}</button>
                </div>
                <RoutingPolicyActionView state={(*action_state).clone()} />
            </section>

            <RoutingPoliciesView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RoutingPoliciesProps {
    state: ApiState<RoutingPolicySnapshot>,
}

#[function_component(RoutingPoliciesView)]
fn routing_policies_view(props: &RoutingPoliciesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"加载审核分流策略，查看当前路由治理状态。"}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"正在加载审核分流策略..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {routing_policy_cockpit(snapshot)}
                        <section class="panel result-stack">
                            <h3>{"策略清单"}</h3>
                            <div class="score-hero">
                                <div><span>{"策略数"}</span><strong>{snapshot.policies.len()}</strong></div>
                                <div><span>{"已激活"}</span><strong>{snapshot.policies.iter().filter(|policy| policy.status == "active").count()}</strong></div>
                                <div><span>{"审核模式"}</span><strong>{routing_review_modes(&snapshot.policies)}</strong></div>
                            </div>
                            <div class="factor-card-grid">
                                {for snapshot.policies.iter().map(|policy| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{format!("{} v{} / {}", policy.policy_id, policy.version, policy.review_mode)}</strong>
                                            <span>{format!("{} / owner {}", policy.status, policy.owner)}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Low / Medium"}</span><strong>{format!("{} / {}", policy.risk_thresholds.low_max, policy.risk_thresholds.medium_min)}</strong></div>
                                            <div><span>{"High / Critical"}</span><strong>{format!("{} / {}", policy.risk_thresholds.high_min, policy.risk_thresholds.critical_min)}</strong></div>
                                            <div><span>{"Confidence"}</span><strong>{format!("{} / {}", policy.confidence_thresholds.low_confidence_below, policy.confidence_thresholds.high_confidence_min)}</strong></div>
                                            <div><span>{"Provider 复核"}</span><strong>{policy.provider_review_threshold}</strong></div>
                                        </div>
                                        <small>{format!("activated: {} / created: {}", policy.activated_at.as_deref().unwrap_or("none"), policy.created_at.as_deref().unwrap_or("none"))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"上线门禁"}</h3>
                            <div class="score-hero">
                                <div><span>{"策略"}</span><strong>{format!("{} v{}", snapshot.gates.policy_id, snapshot.gates.version)}</strong></div>
                                <div><span>{"门禁结果"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"通过项"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"审核模式"}</span><strong>{&snapshot.gates.review_mode}</strong></div>
                                <div><span>{"状态"}</span><strong>{&snapshot.gates.status}</strong></div>
                                <div><span>{"阻塞项"}</span><strong>{snapshot.gates.blockers.len()}</strong></div>
                            </div>
                            if snapshot.gates.blockers.is_empty() {
                                <p class="empty">{"当前没有上线阻塞项。"}</p>
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
                    </>
                },
            }}
        </>
    }
}

fn routing_policy_cockpit(snapshot: &RoutingPolicySnapshot) -> Html {
    let policy = snapshot
        .policies
        .iter()
        .find(|policy| policy.status == "active")
        .or_else(|| snapshot.policies.first());

    if let Some(policy) = policy {
        let blocker_label = snapshot
            .gates
            .blockers
            .first()
            .map(String::as_str)
            .unwrap_or("no blocker");
        html! {
            <section class="panel result-stack">
                <div class="section-header">
                    <div>
                        <h3>{"分流决策图"}</h3>
                        <p>{"展示风险评分、置信度、Provider 图谱压力和上线门禁如何决定案件进入自动通过、抽样复核或人工审核；这里不做自动赔付裁决。"}</p>
                    </div>
                    <span class={classes!("status-token", status_tone(&policy.status))}>{&policy.status}</span>
                </div>
                <div class="routing-cockpit">
                    <aside class="routing-brief">
                        <span class="eyebrow">{"当前生效策略"}</span>
                        <strong>{format!("{} v{}", policy.policy_id, policy.version)}</strong>
                        <dl>
                            <div><dt>{"审核模式"}</dt><dd>{&policy.review_mode}</dd></div>
                            <div><dt>{"负责人"}</dt><dd>{&policy.owner}</dd></div>
                            <div><dt>{"上线门禁"}</dt><dd>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</dd></div>
                            <div><dt>{"门禁结果"}</dt><dd>{&snapshot.gates.decision}</dd></div>
                        </dl>
                    </aside>

                    <div class="routing-decision-map">
                        <div class="routing-map-title">
                            <span>{"风险分层与审核分流"}</span>
                            <strong>{"风险信号 + 置信度 + 策略门禁 -> 可审计分流路径"}</strong>
                        </div>
                        <div class="routing-link horizontal"></div>
                        <div class="routing-link diagonal-a"></div>
                        <div class="routing-link diagonal-b"></div>
                        <div class="routing-core">
                            <span>{"分流门禁"}</span>
                            <strong>{&policy.review_mode}</strong>
                        </div>
                        {routing_node("低风险", &format!("0-{}", policy.risk_thresholds.low_max), "low")}
                        {routing_node("中风险", &format!("{}-{}", policy.risk_thresholds.medium_min, policy.risk_thresholds.high_min.saturating_sub(1)), "medium")}
                        {routing_node("高风险", &format!("{}+", policy.risk_thresholds.high_min), "high")}
                        {routing_node("严重风险", &format!("{}+", policy.risk_thresholds.critical_min), "critical")}
                        {routing_node("置信度门槛", &format!("<{} 低 / {}+ 高", policy.confidence_thresholds.low_confidence_below, policy.confidence_thresholds.high_confidence_min), "confidence")}
                        {routing_node("Provider 复核", &format!("{}+", policy.provider_review_threshold), "provider")}
                    </div>

                    <aside class="routing-trace">
                        <span class="eyebrow">{"人工安全边界"}</span>
                        <div class="provider-signal-stack">
                            {provider_signal_row("低风险", "自动通过或抽样复核", "neutral")}
                            {provider_signal_row("中风险", "QA 抽样复核", "warning")}
                            {provider_signal_row("高风险", "人工审核", "danger")}
                            {provider_signal_row("回滚保护", blocker_label, "strong")}
                        </div>
                    </aside>
                </div>
            </section>
        }
    } else {
        html! {
            <section class="panel">
                <p class="empty">{"没有可用于分流决策图的策略。"}</p>
            </section>
        }
    }
}

fn routing_node(label: &str, value: &str, position: &str) -> Html {
    html! {
        <div class={classes!("routing-node", position.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct RoutingPolicyActionProps {
    state: ApiState<RoutingPolicyRecord>,
}

#[function_component(RoutingPolicyActionView)]
fn routing_policy_action_view(props: &RoutingPolicyActionProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"提交、批准、激活和回滚都必须带 evidence_refs，并会校验当前策略状态。"}</p> }
        }
        ApiState::Loading => html! { <p>{"正在更新审核分流策略..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"策略"}</span><strong>{format!("{} v{}", record.policy_id, record.version)}</strong></div>
                <div><span>{"审核模式"}</span><strong>{&record.review_mode}</strong></div>
                <div><span>{"状态"}</span><strong>{&record.status}</strong></div>
                <div><span>{"负责人"}</span><strong>{&record.owner}</strong></div>
            </div>
        },
    }
}

async fn get_routing_policy_snapshot(
    api_key: String,
    policy_id: String,
    review_mode: String,
    version: String,
) -> Result<RoutingPolicySnapshot, String> {
    let policies = request_get_json::<RoutingPolicyListResponse>(
        "/api/v1/ops/routing-policies",
        api_key.clone(),
    )
    .await?
    .policies;
    let version = parse_u32(&version, "routing policy version")?;
    let gates = request_get_json::<RoutingPolicyPromotionGates>(
        &format!(
            "/api/v1/ops/routing-policies/{}/{}/{}/promotion-gates",
            policy_id.trim(),
            review_mode.trim(),
            version
        ),
        api_key,
    )
    .await?;
    Ok(RoutingPolicySnapshot { policies, gates })
}

async fn update_routing_policy_lifecycle(
    api_key: String,
    policy_id: String,
    review_mode: String,
    version: String,
    action: &str,
    evidence_refs: Vec<String>,
) -> Result<RoutingPolicyRecord, String> {
    if evidence_refs.is_empty() {
        return Err("routing policy lifecycle actions require evidence refs".into());
    }
    let version = parse_u32(&version, "routing policy version")?;
    request_json(
        &format!(
            "/api/v1/ops/routing-policies/{}/{}/{}/{}",
            policy_id.trim(),
            review_mode.trim(),
            version,
            action
        ),
        api_key,
        json!({ "evidence_refs": evidence_refs }),
    )
    .await
}
