use crate::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlTextAreaElement;

#[function_component(RuntimeScoringPage)]
pub fn runtime_scoring_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let request_payload = use_state(|| SAMPLE_RUNTIME_SCORE_REQUEST.to_string());
    let score_state = use_state(|| ApiState::<ScoreResponse>::Idle);

    let use_claim_id_template = {
        let request_payload = request_payload.clone();
        Callback::from(move |_| {
            request_payload.set(SAMPLE_RUNTIME_SCORE_REQUEST.to_string());
        })
    };

    let use_full_payload_template = {
        let request_payload = request_payload.clone();
        Callback::from(move |_| {
            request_payload.set(pretty_json(&runtime_full_payload_template()));
        })
    };

    let score = {
        let api_key = api_key.clone();
        let request_payload = request_payload.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let score_state = score_state.clone();
            match serde_json::from_str::<Value>(&request_payload) {
                Ok(payload) => {
                    score_state.set(ApiState::Loading);
                    spawn_local(async move {
                        score_state.set(match score_canonical_claim(payload, api_key).await {
                            Ok(response) => ApiState::Ready(response),
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => score_state.set(ApiState::Failed(format!(
                    "runtime scoring request JSON is invalid: {error}"
                ))),
            }
        })
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Runtime Scoring"}</h2>
                    <p>{"Validate the claim scoring contract and inspect audit-backed routing output. Business reviewers should work from Dashboard, Leads & Cases, or Review Workbench."}</p>
                </div>
                <span class="status-pill">{"Integration Tool"}</span>
            </div>

            {runtime_scoring_blueprint()}

            <div class="inbox-grid">
                <section class="panel result-stack">
                    <h3>{"Scoring Request"}</h3>
                    <label>
                        {"Dev API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={use_claim_id_template}>{"Stored claim"}</button>
                        <button onclick={use_full_payload_template}>{"Full payload"}</button>
                    </div>
                    <label>
                        {"Request JSON"}
                        <textarea
                            class="payload-editor"
                            value={(*request_payload).clone()}
                            oninput={{
                                let request_payload = request_payload.clone();
                                Callback::from(move |event: InputEvent| {
                                    request_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={score} disabled={matches!(&*score_state, ApiState::Loading)}>
                            {if matches!(&*score_state, ApiState::Loading) { "Validating..." } else { "Validate scoring contract" }}
                        </button>
                    </div>
                </section>

                <RuntimeScoreView state={(*score_state).clone()} />
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RuntimeScoreProps {
    state: ApiState<ScoreResponse>,
}

#[function_component(RuntimeScoreView)]
fn runtime_score_view(props: &RuntimeScoreProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Scoring Response"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Submit a stored claim or full payload to validate response shape, route, audit trace, and evidence references."}</p> },
                ApiState::Loading => html! { <p>{"Validating scoring contract..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                            <div><span>{"Risk Score"}</span><strong>{display_value(&response.risk_score)}</strong></div>
                            <div><span>{"RAG"}</span><strong>{response.rag.as_ref().map(display_value).unwrap_or_else(|| "none".into())}</strong></div>
                        </div>
                        {runtime_decision_visual(response)}
                        {runtime_signal_map(response)}
                        <div class="summary-grid">
                            <div><span>{"Action"}</span><strong>{response.recommended_action.as_deref().unwrap_or("review")}</strong></div>
                            <div><span>{"Decision"}</span><strong>{response.decision_outcome.as_deref().unwrap_or("manual_review")}</strong></div>
                            <div><span>{"Authority"}</span><strong>{response.decision_authority.as_deref().unwrap_or("risk_routing_policy")}</strong></div>
                            <div><span>{"Risk Level"}</span><strong>{response.risk_level.as_deref().unwrap_or("unknown")}</strong></div>
                            <div><span>{"Confidence"}</span><strong>{format!("{} / {}", response.confidence.as_deref().unwrap_or("unknown"), optional_u8(response.confidence_score))}</strong></div>
                            <div><span>{"Decision Confidence"}</span><strong>{response.decision_confidence.as_deref().unwrap_or("low")}</strong></div>
                            <div><span>{"Review Required"}</span><strong>{if response.appeal_or_review_required.unwrap_or(true) { "yes" } else { "no" }}</strong></div>
                            <div><span>{"Review Mode"}</span><strong>{response.review_mode.as_deref().unwrap_or("unknown")}</strong></div>
                            <div><span>{"Reason Code"}</span><strong>{response.reason_code.as_deref().unwrap_or("pending")}</strong></div>
                            <div><span>{"Run"}</span><strong>{response.run_id.as_deref().unwrap_or("pending")}</strong></div>
                            <div><span>{"Audit"}</span><strong>{response.audit_id.as_deref().unwrap_or("pending")}</strong></div>
                        </div>
                        <p class="empty">{response.routing_reason.as_deref().unwrap_or("No routing reason returned.")}</p>

                        <h4>{"Risk Signal Breakdown"}</h4>
                        {runtime_score_breakdown(response)}
                        <div class="factor-card-grid">
                            {for response.layers.iter().map(|layer| html! {
                                <div class="metric-row">
                                    <span>{runtime_layer_business_label(layer)}</span>
                                    <strong>{format!("{} / {}", layer.score, layer.status)}</strong>
                                    <small>{&layer.reason}</small>
                                    <small>{format!("evidence: {}", value_refs_label(&layer.evidence_refs))}</small>
                                </div>
                            })}
                        </div>

                        <h4>{"Alerts And Top Reasons"}</h4>
                        <div class="factor-card-grid">
                            {for response.alerts.iter().map(|alert| html! {
                                <div class="metric-row">
                                    <span>{&alert.alert_code}</span>
                                    <strong>{&alert.severity}</strong>
                                    <small>{&alert.reason}</small>
                                    <small>{format!("rule {} v{}", alert.rule_id, alert.rule_version)}</small>
                                    if !alert.required_evidence.is_empty() {
                                        <small>{format!("required evidence: {}", required_evidence_label(&alert.required_evidence))}</small>
                                    }
                                </div>
                            })}
                        </div>
                        if response.top_reasons.is_empty() {
                            <p class="empty">{"No top reasons returned."}</p>
                        } else {
                            <ul class="result-list">
                                {for response.top_reasons.iter().map(|reason| html! { <li>{reason}</li> })}
                            </ul>
                        }

                        <h4>{"Model Output"}</h4>
                        {runtime_model_output(response.model_score.as_ref())}

                        <h4>{"Evidence And Agent Prefill"}</h4>
                        <div class="summary-grid">
                            <div><span>{"Evidence Refs"}</span><strong>{response.evidence_refs.as_ref().map(|refs| refs.len()).unwrap_or(0)}</strong></div>
                            <div><span>{"Features"}</span><strong>{response.feature_values.len()}</strong></div>
                            <div><span>{"Similar Cases"}</span><strong>{response.similar_cases.len()}</strong></div>
                        </div>
                        <small>{format!("evidence: {}", response.evidence_refs.as_ref().map(|refs| value_refs_label(refs)).unwrap_or_else(|| "none".into()))}</small>
                        if let Some(prefill) = &response.agent_investigation_prefill {
                            <pre>{pretty_json(prefill)}</pre>
                        }
                        <details>
                            <summary>{"Routing and clinical payload"}</summary>
                            <pre>{pretty_json(&json!({
                                "routing_policy": response.routing_policy,
                                "clinical_evidence": response.clinical_evidence,
                                "provider_profile": response.provider_profile,
                                "provider_relationships": response.provider_relationships
                            }))}</pre>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

fn runtime_scoring_blueprint() -> Html {
    html! {
        <section class="panel scoring-blueprint-shell">
            <div class="blueprint-claim-card">
                <span>{"Input Contract"}</span>
                <strong>{"Stored claim ID or canonical claim payload"}</strong>
                <div class="blueprint-document">
                    <i class="wide"></i>
                    <i></i>
                    <i class="short"></i>
                    <b></b>
                </div>
            </div>
            <div class="blueprint-layer-rail contract-flow" aria-label="Scoring contract validation flow">
                {blueprint_layer("Request", "Contract", "required IDs, payload shape, tenant scope", "peer")}
                {blueprint_layer("Signals", "Risk context", "rules, model, provider, clinical evidence", "rules")}
                {blueprint_layer("Policy", "Routing", "manual review, case creation, or watchlist", "ml")}
                {blueprint_layer("Audit", "Trace", "run_id, audit_id, evidence_refs", "medical")}
                {blueprint_layer("Queue", "Human work", "reviewers decide; system never denies alone", "route")}
            </div>
            <div class="blueprint-human-card">
                <span>{"Boundary"}</span>
                <strong>{"This page validates runtime output; it is not the claim adjudication desk."}</strong>
                <small>{"Every response must carry route, reason, run_id, audit_id, and evidence_refs."}</small>
            </div>
        </section>
    }
}

fn blueprint_layer(layer: &str, label: &str, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("blueprint-layer", tone.to_string())}>
            <span>{layer}</span>
            <strong>{label}</strong>
            <small>{caption}</small>
        </div>
    }
}

fn runtime_decision_visual(response: &ScoreResponse) -> Html {
    let risk_score = numeric_value(&response.risk_score).clamp(0.0, 100.0);
    let risk_style = format!(
        "background: conic-gradient(var(--red) 0 {:.0}%, #dbe8f8 {:.0}% 100%);",
        risk_score, risk_score
    );
    let rag = response
        .rag
        .as_ref()
        .map(display_value)
        .unwrap_or_else(|| "none".into());
    let evidence_count = response.evidence_refs.as_ref().map(Vec::len).unwrap_or(0);
    html! {
        <div class="runtime-visual-cockpit">
            <div class="risk-gauge-card">
                <div class="risk-gauge" style={risk_style}>
                    <div>
                        <span>{"risk"}</span>
                        <strong>{format!("{:.0}", risk_score)}</strong>
                    </div>
                </div>
                <div class="risk-gauge-meta">
                    <span>{"Routing outcome"}</span>
                    <strong>{response.decision_outcome.as_deref().or(response.recommended_action.as_deref()).unwrap_or("manual_review")}</strong>
                    <small>{format!("{} / {}", rag, response.confidence.as_deref().unwrap_or("confidence pending"))}</small>
                </div>
            </div>
            <div class="runtime-path-card">
                {runtime_path_node("Request", "claim contract", &response.claim_id)}
                {runtime_path_node("Signals", "risk outputs", &format!("{} signals", response.layers.len()))}
                {runtime_path_node("Explain", "alerts + reasons", &format!("{} alerts", response.alerts.len()))}
                {runtime_path_node("Audit", "trace refs", &format!("{evidence_count} refs"))}
                {runtime_path_node("Queue", "human action", response.review_mode.as_deref().unwrap_or("review"))}
            </div>
        </div>
    }
}

fn runtime_signal_map(response: &ScoreResponse) -> Html {
    let model_label = response
        .model_score
        .as_ref()
        .map(|model| format!("{} {}", model.model_key, model.model_version))
        .unwrap_or_else(|| "model pending".into());
    let provider_signal = response
        .provider_profile
        .as_ref()
        .and_then(|profile| profile.get("provider_id"))
        .map(display_value)
        .unwrap_or_else(|| "provider context".into());
    let clinical_signal = response
        .clinical_evidence
        .as_ref()
        .and_then(|clinical| clinical.get("clinical_signal_count"))
        .map(display_value)
        .unwrap_or_else(|| format!("{} layers", response.layers.len()));
    let evidence_count = response.evidence_refs.as_ref().map(Vec::len).unwrap_or(0);

    html! {
        <div class="runtime-signal-map">
            <div class="signal-map-core">
                <span>{"Signal Contract Map"}</span>
                <strong>{&response.claim_id}</strong>
                <small>{response.routing_reason.as_deref().unwrap_or("policy route pending")}</small>
            </div>
            {runtime_signal_node("Controls", &format!("{} alerts", response.alerts.len()), "controls")}
            {runtime_signal_node("Model", &model_label, "model")}
            {runtime_signal_node("Clinical", &clinical_signal, "clinical")}
            {runtime_signal_node("Provider graph", &provider_signal, "graph")}
            {runtime_signal_node("Knowledge", &format!("{} similar cases", response.similar_cases.len()), "knowledge")}
            {runtime_signal_node("Evidence", &format!("{evidence_count} refs"), "evidence")}
        </div>
    }
}

fn runtime_signal_node(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("signal-map-node", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn runtime_layer_business_label(layer: &RuntimeLayerScore) -> String {
    match layer.layer_id.as_str() {
        "L1" => "Peer benchmark signal".into(),
        "L2" => "Deterministic control signal".into(),
        "L3" => "Anomaly signal".into(),
        "L4" => "Model signal".into(),
        "L5" => "Clinical reasonableness signal".into(),
        "L6" => "Provider network signal".into(),
        "L7" => "Routing policy output".into(),
        _ => layer.name.clone(),
    }
}

fn runtime_path_node(label: &str, caption: &str, value: &str) -> Html {
    html! {
        <div class="runtime-path-node">
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}
