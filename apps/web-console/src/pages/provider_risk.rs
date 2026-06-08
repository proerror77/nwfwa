use crate::*;
use wasm_bindgen_futures::spawn_local;

#[function_component(ProviderRiskPage)]
pub fn provider_risk_page() -> Html {
    let api_key = use_api_key();
    let summary_state = use_state(|| ApiState::<ProviderRiskSummary>::Idle);

    let load_summary = {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_provider_risk_summary(api_key).await {
                    Ok(summary) => ApiState::Ready(summary),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_summary = load_summary.clone();
        Callback::from(move |_| load_summary.emit(()))
    };

    {
        let load_summary = load_summary.clone();
        use_effect_with((), move |_| {
            load_summary.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Provider Risk"}</h2>
                    <p>{"Inspect provider network and graph risk profiles, review routing, outlier flags, graph reasons, and evidence refs for provider-focused investigation."}</p>
                </div>
                <span class="status-pill">{"Provider Graph Risk"}</span>
            </div>

            <section class="panel">
                <h3>{"Provider Risk Source"}</h3>
                <p class="empty">{"Using the configured provider-risk workspace for graph and peer-pattern signals."}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*summary_state, ApiState::Loading)}>
                        {if matches!(&*summary_state, ApiState::Loading) { "Refreshing..." } else { "Refresh provider risk" }}
                    </button>
                </div>
            </section>

            <ProviderRiskView state={(*summary_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ProviderRiskProps {
    state: ApiState<ProviderRiskSummary>,
}

#[function_component(ProviderRiskView)]
fn provider_risk_view(props: &ProviderRiskProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load provider risk to inspect provider graph signals."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading provider risk..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(summary) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Provider Risk Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Providers"}</span><strong>{summary.provider_count}</strong></div>
                                <div><span>{"Review Required"}</span><strong>{summary.review_required_count}</strong></div>
                                <div><span>{"High Risk"}</span><strong>{summary.high_risk_count}</strong></div>
                            </div>
                            {provider_graph_cockpit(summary)}
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Provider Risk Profiles"}</h3>
                            if summary.providers.is_empty() {
                                <p class="empty">{"No provider risk profiles returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for summary.providers.iter().map(|provider| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", provider.provider_id, provider.risk_tier)}</strong>
                                                <span>{format!("score {} / route {}", provider.risk_score, provider.review_route)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Review"}</span><strong>{yes_no(provider.review_required)}</strong></div>
                                                <div><span>{"Claims"}</span><strong>{provider.claim_count}</strong></div>
                                                <div><span>{"Network Risk"}</span><strong>{optional_u8(provider.network_risk_score)}</strong></div>
                                                <div><span>{"Failures"}</span><strong>{provider.review_failure_count}</strong></div>
                                                <div><span>{"Confirmed FWA"}</span><strong>{provider.confirmed_fwa_count}</strong></div>
                                                <div><span>{"False Positives"}</span><strong>{provider.false_positive_count}</strong></div>
                                                <div><span>{"Specialty"}</span><strong>{provider.specialty.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"Network"}</span><strong>{provider.network_status.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"Latest Claim"}</span><strong>{provider.latest_claim_id.as_deref().unwrap_or("none")}</strong></div>
                                            </div>
                                            <small>{format!("outliers: {}", refs_label(&provider.outlier_flags))}</small>
                                            <small>{format!("graph reasons: {}", refs_label(&provider.graph_reasons))}</small>
                                            <small>{format!("evidence: {}", refs_label(&provider.evidence_refs))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn provider_graph_cockpit(summary: &ProviderRiskSummary) -> Html {
    let primary = summary
        .providers
        .iter()
        .max_by_key(|provider| provider.risk_score);

    if let Some(provider) = primary {
        let network_score = provider
            .network_risk_score
            .map(|score| score.to_string())
            .unwrap_or_else(|| "n/a".into());
        let outlier_label = provider
            .outlier_flags
            .first()
            .cloned()
            .unwrap_or_else(|| "no outlier flag".into());
        let graph_reason = provider
            .graph_reasons
            .first()
            .cloned()
            .unwrap_or_else(|| "graph reason pending".into());
        html! {
            <div class="provider-risk-cockpit">
                <div class="relationship-graph provider-relationship-graph">
                    <div class="graph-ring"></div>
                    <div class="graph-ring inner"></div>
                    <div class="graph-center provider-risk-center">
                        <span>{"Provider Network"}</span>
                        <strong>{&provider.provider_id}</strong>
                    </div>
                    {provider_graph_entity("Risk tier", &provider.risk_tier, "top", "lead")}
                    {provider_graph_entity("Network risk", &network_score, "right", "provider")}
                    {provider_graph_entity("Review route", &provider.review_route, "bottom", "case")}
                    {provider_graph_entity("Latest claim", provider.latest_claim_id.as_deref().unwrap_or("none"), "left", "claim")}
                    {provider_graph_entity("Outlier flag", &outlier_label, "lower-right", "lead")}
                    {provider_graph_entity("Evidence refs", &provider.evidence_refs.len().to_string(), "lower-left", "reviewer")}
                </div>
                <div class="provider-graph-panel">
                    <div>
                        <span>{"Graph Risk Focus"}</span>
                        <strong>{format!("score {} / claims {}", provider.risk_score, provider.claim_count)}</strong>
                        <small>{graph_reason}</small>
                    </div>
                    <div class="provider-signal-stack">
                        {provider_signal_row("Confirmed FWA", &provider.confirmed_fwa_count.to_string(), "danger")}
                        {provider_signal_row("Review failures", &provider.review_failure_count.to_string(), "warning")}
                        {provider_signal_row("False positives", &provider.false_positive_count.to_string(), "neutral")}
                        {provider_signal_row("Network status", provider.network_status.as_deref().unwrap_or("unknown"), "strong")}
                    </div>
                    <small>{format!("evidence: {}", refs_label(&provider.evidence_refs))}</small>
                </div>
            </div>
        }
    } else {
        html! { <p class="empty">{"No provider graph cockpit available until provider risk profiles are returned."}</p> }
    }
}

fn provider_graph_entity(label: &str, value: &str, position: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("graph-entity", position.to_string(), tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

pub(crate) fn provider_signal_row(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("provider-signal-row", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}
