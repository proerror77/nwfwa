use crate::*;
use wasm_bindgen_futures::spawn_local;

#[function_component(RoutingPoliciesPage)]
pub fn routing_policies_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
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
                    <h2>{"Routing Policies"}</h2>
                    <p>{"Govern risk bands, confidence gates, provider review thresholds, approvals, activation, and rollback evidence before routing affects claim handling."}</p>
                </div>
                <span class="status-pill">{"Risk Fusion Routing"}</span>
            </div>

            <section class="panel">
                <h3>{"Routing Policy Control"}</h3>
                <div class="form-grid">
                    {text_input("Policy ID", &policy_id)}
                    {text_input("Review mode", &review_mode)}
                    {text_input("Version", &version)}
                    {text_input("Evidence refs", &evidence_refs)}
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh routing policies" }}
                    </button>
                    <button onclick={lifecycle_action("submit")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit"}</button>
                    <button onclick={lifecycle_action("approve")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Approve"}</button>
                    <button onclick={lifecycle_action("activate")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Activate"}</button>
                    <button onclick={lifecycle_action("rollback")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Rollback"}</button>
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
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load routing policies to inspect routing governance."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading routing policies..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {routing_policy_cockpit(snapshot)}
                        <section class="panel result-stack">
                            <h3>{"Routing Policy Inventory"}</h3>
                            <div class="score-hero">
                                <div><span>{"Policies"}</span><strong>{snapshot.policies.len()}</strong></div>
                                <div><span>{"Active"}</span><strong>{snapshot.policies.iter().filter(|policy| policy.status == "active").count()}</strong></div>
                                <div><span>{"Review Modes"}</span><strong>{routing_review_modes(&snapshot.policies)}</strong></div>
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
                                            <div><span>{"Provider Review"}</span><strong>{policy.provider_review_threshold}</strong></div>
                                        </div>
                                        <small>{format!("activated: {} / created: {}", policy.activated_at.as_deref().unwrap_or("none"), policy.created_at.as_deref().unwrap_or("none"))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Routing Promotion Gates"}</h3>
                            <div class="score-hero">
                                <div><span>{"Policy"}</span><strong>{format!("{} v{}", snapshot.gates.policy_id, snapshot.gates.version)}</strong></div>
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Review Mode"}</span><strong>{&snapshot.gates.review_mode}</strong></div>
                                <div><span>{"Status"}</span><strong>{&snapshot.gates.status}</strong></div>
                                <div><span>{"Blockers"}</span><strong>{snapshot.gates.blockers.len()}</strong></div>
                            </div>
                            if snapshot.gates.blockers.is_empty() {
                                <p class="empty">{"No routing policy blockers."}</p>
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
                        <h3>{"Routing Decision Map"}</h3>
                        <p>{"How fused risk score, confidence, provider graph pressure, and governance gates route claims without automatic adjudication."}</p>
                    </div>
                    <span class={classes!("status-token", status_tone(&policy.status))}>{&policy.status}</span>
                </div>
                <div class="routing-cockpit">
                    <aside class="routing-brief">
                        <span class="eyebrow">{"Active routing policy"}</span>
                        <strong>{format!("{} v{}", policy.policy_id, policy.version)}</strong>
                        <dl>
                            <div><dt>{"Review mode"}</dt><dd>{&policy.review_mode}</dd></div>
                            <div><dt>{"Owner"}</dt><dd>{&policy.owner}</dd></div>
                            <div><dt>{"Promotion"}</dt><dd>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</dd></div>
                            <div><dt>{"Decision"}</dt><dd>{&snapshot.gates.decision}</dd></div>
                        </dl>
                    </aside>

                    <div class="routing-decision-map">
                        <div class="routing-map-title">
                            <span>{"Risk fusion and routing"}</span>
                            <strong>{"risk signals + confidence + policy gate -> human-safe route"}</strong>
                        </div>
                        <div class="routing-link horizontal"></div>
                        <div class="routing-link diagonal-a"></div>
                        <div class="routing-link diagonal-b"></div>
                        <div class="routing-core">
                            <span>{"Routing gate"}</span>
                            <strong>{&policy.review_mode}</strong>
                        </div>
                        {routing_node("Green band", &format!("0-{}", policy.risk_thresholds.low_max), "low")}
                        {routing_node("Amber band", &format!("{}-{}", policy.risk_thresholds.medium_min, policy.risk_thresholds.high_min.saturating_sub(1)), "medium")}
                        {routing_node("Red band", &format!("{}+", policy.risk_thresholds.high_min), "high")}
                        {routing_node("Critical route", &format!("{}+", policy.risk_thresholds.critical_min), "critical")}
                        {routing_node("Confidence gate", &format!("<{} low / {}+ high", policy.confidence_thresholds.low_confidence_below, policy.confidence_thresholds.high_confidence_min), "confidence")}
                        {routing_node("Provider review", &format!("{}+", policy.provider_review_threshold), "provider")}
                    </div>

                    <aside class="routing-trace">
                        <span class="eyebrow">{"Human-safe route"}</span>
                        <div class="provider-signal-stack">
                            {provider_signal_row("Low", "STP or sample QA", "neutral")}
                            {provider_signal_row("Medium", "QA sampling", "warning")}
                            {provider_signal_row("High", "Manual review", "danger")}
                            {provider_signal_row("Rollback gate", blocker_label, "strong")}
                        </div>
                    </aside>
                </div>
            </section>
        }
    } else {
        html! {
            <section class="panel">
                <p class="empty">{"No routing policy available for routing decision map."}</p>
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
            html! { <p class="empty">{"Lifecycle actions require evidence refs and enforce current policy status."}</p> }
        }
        ApiState::Loading => html! { <p>{"Updating routing policy..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Policy"}</span><strong>{format!("{} v{}", record.policy_id, record.version)}</strong></div>
                <div><span>{"Review Mode"}</span><strong>{&record.review_mode}</strong></div>
                <div><span>{"Status"}</span><strong>{&record.status}</strong></div>
                <div><span>{"Owner"}</span><strong>{&record.owner}</strong></div>
            </div>
        },
    }
}
