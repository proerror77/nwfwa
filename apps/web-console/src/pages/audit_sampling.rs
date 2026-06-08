use crate::*;
use serde_json::json;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlTextAreaElement;

#[function_component(AuditSamplingPage)]
pub fn audit_sampling_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let sample_mode = use_state(|| "risk_ranked".to_string());
    let population_definition = use_state(|| "Open high-risk leads for QA sampling".to_string());
    let inclusion_criteria = use_state(|| {
        pretty_json(&json!({
            "min_risk_score": 70,
            "rag": "RED",
            "review_mode": "pre_payment"
        }))
    });
    let sample_size = use_state(|| "5".to_string());
    let reviewer = use_state(|| "qa-reviewer-1".to_string());
    let assignment_queue = use_state(|| "qa-high-risk".to_string());
    let deterministic_seed = use_state(|| "demo-seed-2026".to_string());
    let selected_sample_id = use_state(String::new);
    let samples_state = use_state(|| ApiState::<Vec<AuditSampleRecord>>::Idle);
    let create_state = use_state(|| ApiState::<AuditSampleRecord>::Idle);
    let events_state = use_state(|| ApiState::<Vec<AuditEventRecord>>::Idle);

    let load_samples = {
        let api_key = api_key.clone();
        let samples_state = samples_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let samples_state = samples_state.clone();
            samples_state.set(ApiState::Loading);
            spawn_local(async move {
                samples_state.set(match get_audit_samples(api_key).await {
                    Ok(samples) => ApiState::Ready(samples),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let create_sample = {
        let api_key = api_key.clone();
        let sample_mode = sample_mode.clone();
        let population_definition = population_definition.clone();
        let inclusion_criteria = inclusion_criteria.clone();
        let sample_size = sample_size.clone();
        let reviewer = reviewer.clone();
        let assignment_queue = assignment_queue.clone();
        let deterministic_seed = deterministic_seed.clone();
        let create_state = create_state.clone();
        let load_samples = load_samples.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let payload = audit_sample_payload(
                (*sample_mode).clone(),
                (*population_definition).clone(),
                (*inclusion_criteria).clone(),
                (*sample_size).clone(),
                (*reviewer).clone(),
                (*assignment_queue).clone(),
                (*deterministic_seed).clone(),
            );
            let create_state = create_state.clone();
            let load_samples = load_samples.clone();
            match payload {
                Ok(payload) => {
                    create_state.set(ApiState::Loading);
                    spawn_local(async move {
                        create_state.set(match post_audit_sample(api_key, payload).await {
                            Ok(sample) => {
                                load_samples.emit(());
                                ApiState::Ready(sample)
                            }
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => create_state.set(ApiState::Failed(error)),
            }
        })
    };

    let load_events = {
        let api_key = api_key.clone();
        let selected_sample_id = selected_sample_id.clone();
        let events_state = events_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let selected_sample_id = (*selected_sample_id).clone();
            let events_state = events_state.clone();
            events_state.set(ApiState::Loading);
            spawn_local(async move {
                events_state.set(
                    match get_audit_events_for_sample(api_key, selected_sample_id).await {
                        Ok(events) => ApiState::Ready(events),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    {
        let load_samples = load_samples.clone();
        use_effect_with((), move |_| {
            load_samples.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Audit Sampling"}</h2>
                    <p>{"Create governed QA audit samples, inspect selected leads and outcome distribution, and trace audit_sample.created events by sample ID."}</p>
                </div>
                <span class="status-pill">{"QA Sampling Governance"}</span>
            </div>

            <section class="panel result-stack">
                <h3>{"Audit Sample Control"}</h3>
                <div class="form-grid">
                    {text_input("Sample mode", &sample_mode)}
                    {text_input("Population", &population_definition)}
                    {text_input("Sample size", &sample_size)}
                    {text_input("Reviewer", &reviewer)}
                    {text_input("Assignment queue", &assignment_queue)}
                    {text_input("Deterministic seed", &deterministic_seed)}
                    {text_input("Audit sample ID", &selected_sample_id)}
                </div>
                <label>
                    {"Inclusion criteria JSON"}
                    <textarea
                        class="payload-editor"
                        value={(*inclusion_criteria).clone()}
                        oninput={{
                            let inclusion_criteria = inclusion_criteria.clone();
                            Callback::from(move |event: InputEvent| {
                                inclusion_criteria.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <div class="button-row">
                    <button onclick={create_sample} disabled={matches!(&*create_state, ApiState::Loading)}>
                        {if matches!(&*create_state, ApiState::Loading) { "Creating..." } else { "Create audit sample" }}
                    </button>
                    <button onclick={{
                        let load_samples = load_samples.clone();
                        Callback::from(move |_| load_samples.emit(()))
                    }} disabled={matches!(&*samples_state, ApiState::Loading)}>
                        {if matches!(&*samples_state, ApiState::Loading) { "Refreshing..." } else { "Refresh samples" }}
                    </button>
                    <button onclick={load_events} disabled={matches!(&*events_state, ApiState::Loading)}>
                        {if matches!(&*events_state, ApiState::Loading) { "Loading..." } else { "Load sample audit events" }}
                    </button>
                </div>
                <AuditSampleCreateView state={(*create_state).clone()} />
            </section>

            <AuditSamplesView state={(*samples_state).clone()} />
            <AuditSampleEventsView state={(*events_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct AuditSampleCreateProps {
    state: ApiState<AuditSampleRecord>,
}

#[function_component(AuditSampleCreateView)]
fn audit_sample_create_view(props: &AuditSampleCreateProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Supported sample modes: risk_ranked, random_control, stratified, post_payment_audit, qa_calibration."}</p> }
        }
        ApiState::Loading => html! { <p>{"Creating audit sample..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(sample) => html! {
            <div class="summary-grid">
                <div><span>{"Sample"}</span><strong>{&sample.sample_id}</strong></div>
                <div><span>{"Mode"}</span><strong>{&sample.sample_mode}</strong></div>
                <div><span>{"Selected Leads"}</span><strong>{sample.selected_leads.len()}</strong></div>
                <div><span>{"Selection"}</span><strong>{&sample.selection_method}</strong></div>
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct AuditSamplesProps {
    state: ApiState<Vec<AuditSampleRecord>>,
}

#[function_component(AuditSamplesView)]
fn audit_samples_view(props: &AuditSamplesProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Audit Sample Inventory"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Load audit samples to inspect sampling coverage."}</p> },
                ApiState::Loading => html! { <p>{"Loading audit samples..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(samples) => html! {
                    if samples.is_empty() {
                        <p class="empty">{"No audit samples returned."}</p>
                    } else {
                        <>
                            {audit_sampling_governance_cockpit(samples)}
                            <div class="factor-card-grid">
                                {for samples.iter().take(10).map(|sample| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{format!("{} / {}", sample.sample_id, sample.sample_mode)}</strong>
                                            <span>{format!("{} / {}", sample.selection_method, sample.assignment_queue)}</span>
                                        </div>
                                        <p>{&sample.population_definition}</p>
                                        <div class="summary-grid">
                                            <div><span>{"Requested"}</span><strong>{sample.sample_size}</strong></div>
                                            <div><span>{"Selected"}</span><strong>{sample.selected_leads.len()}</strong></div>
                                            <div><span>{"Reviewer"}</span><strong>{&sample.reviewer}</strong></div>
                                            <div><span>{"Seed"}</span><strong>{sample.deterministic_seed.as_deref().unwrap_or("none")}</strong></div>
                                            <div><span>{"Created"}</span><strong>{sample.created_at.as_deref().unwrap_or("unknown")}</strong></div>
                                            <div><span>{"Criteria"}</span><strong>{payload_signal_count_label(&sample.inclusion_criteria, "criteria fields")}</strong></div>
                                        </div>
                                        <small>{format!("outcome: {}", payload_signal_count_label(&sample.outcome_distribution, "outcome fields"))}</small>
                                        <details class="data-source-detail governance-detail">
                                            <summary>{"Selected lead detail"}</summary>
                                            if sample.selected_leads.is_empty() {
                                                <p class="empty">{"No selected leads in this sample."}</p>
                                            } else {
                                                <div class="factor-card-grid">
                                                    {for sample.selected_leads.iter().take(6).map(|lead| html! {
                                                        <div class="metric-row">
                                                            <span>{format!("{} / {}", lead.lead_id, lead.claim_id)}</span>
                                                            <strong>{format!("{} / {}", lead.risk_score, lead.rag)}</strong>
                                                            <small>{format!("{} / {} / {}", lead.scheme_family, lead.review_mode, lead.risk_band)}</small>
                                                            <small>{format!("provider: {} / {} / {}", lead.provider_id, lead.provider_type, lead.provider_region)}</small>
                                                            <small>{format!("policy: {} / strata: {} / prior reviewer samples: {}", lead.policy_type, lead.strata_key, lead.prior_reviewer_sample_count)}</small>
                                                            <small>{format!("evidence: {}", refs_count_label(&lead.evidence_refs))}</small>
                                                        </div>
                                                    })}
                                                </div>
                                            }
                                        </details>
                                    </div>
                                })}
                            </div>
                        </>
                    }
                },
            }}
        </section>
    }
}

fn audit_sampling_governance_cockpit(samples: &[AuditSampleRecord]) -> Html {
    let sample = &samples[0];
    let primary_lead = sample
        .selected_leads
        .iter()
        .max_by_key(|lead| lead.risk_score);
    let lead_label = primary_lead
        .map(|lead| format!("{} / {}", lead.claim_id, lead.rag))
        .unwrap_or_else(|| "no selected lead".into());
    let risk_label = primary_lead
        .map(|lead| format!("{} / {}", lead.risk_score, lead.risk_band))
        .unwrap_or_else(|| "pending".into());
    let provider_label = primary_lead
        .map(|lead| format!("{} / {}", lead.provider_id, lead.provider_region))
        .unwrap_or_else(|| "pending".into());
    let evidence_label = primary_lead
        .map(|lead| refs_count_label(&lead.evidence_refs))
        .unwrap_or_else(|| "none".into());
    let seed_label = sample.deterministic_seed.as_deref().unwrap_or("none");
    let created_at = sample.created_at.as_deref().unwrap_or("unknown");

    html! {
        <div class="audit-sampling-cockpit">
            <div class="sampling-brief panel-soft">
                <span class="eyebrow">{"Sampling Governance Map"}</span>
                <strong>{format!("{} / {}", sample.sample_id, sample.sample_mode)}</strong>
                <p>{&sample.population_definition}</p>
                <div class="summary-grid">
                    <div><span>{"Requested"}</span><strong>{sample.sample_size}</strong></div>
                    <div><span>{"Selected leads"}</span><strong>{sample.selected_leads.len()}</strong></div>
                    <div><span>{"Reviewer"}</span><strong>{&sample.reviewer}</strong></div>
                    <div><span>{"Queue"}</span><strong>{&sample.assignment_queue}</strong></div>
                </div>
            </div>

            <div class="sampling-governance-map">
                <div class="sampling-map-title">
                    <div>
                        <span>{"QA audit sample"}</span>
                        <strong>{"Population -> Criteria -> Seed -> Leads -> QA -> Audit trace"}</strong>
                    </div>
                    <span>{format!("created {}", created_at)}</span>
                </div>
                <div class="sampling-link"></div>
                <div class="sampling-link diagonal-a"></div>
                <div class="sampling-link diagonal-b"></div>
                <div class="sampling-core">
                    <span>{"Audit Sampling"}</span>
                    <strong>{&sample.selection_method}</strong>
                </div>
                {sampling_node("Population", &sample.sample_mode, "population")}
                {sampling_node("Inclusion Criteria", &payload_signal_count_label(&sample.inclusion_criteria, "criteria fields"), "criteria")}
                {sampling_node("Deterministic seed", seed_label, "seed")}
                {sampling_node("Selected leads", &lead_label, "leads")}
                {sampling_node("QA queue", &sample.assignment_queue, "queue")}
                {sampling_node("Audit trace", "audit_sample.created", "audit")}
            </div>

            <div class="sampling-trace panel-soft">
                <span class="eyebrow">{"Controlled sample output"}</span>
                <div class="provider-signal-stack">
                    {provider_signal_row("Top selected risk", &risk_label, "danger")}
                    {provider_signal_row("Provider focus", &provider_label, "warning")}
                    {provider_signal_row("Outcome distribution", &payload_signal_count_label(&sample.outcome_distribution, "outcome fields"), "neutral")}
                    {provider_signal_row("Evidence refs", &evidence_label, "strong")}
                </div>
                <details class="data-source-detail governance-detail">
                    <summary>{"Sample output detail"}</summary>
                    <small>{format!("inclusion criteria: {}", payload_keys_label(&sample.inclusion_criteria))}</small>
                    <small>{format!("outcome distribution: {}", payload_keys_label(&sample.outcome_distribution))}</small>
                    <small>{format!("top lead evidence: {}", primary_lead.map(|lead| refs_label(&lead.evidence_refs)).unwrap_or_else(|| "none".into()))}</small>
                </details>
            </div>
        </div>
    }
}

fn sampling_node(label: &str, value: &str, position: &str) -> Html {
    html! {
        <div class={classes!("sampling-node", position.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct AuditSampleEventsProps {
    state: ApiState<Vec<AuditEventRecord>>,
}

#[function_component(AuditSampleEventsView)]
fn audit_sample_events_view(props: &AuditSampleEventsProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Audit Sample Event Trace"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Enter an audit sample ID and load sample audit events."}</p> },
                ApiState::Loading => html! { <p>{"Loading sample audit events..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(events) => html! {
                    if events.is_empty() {
                        <p class="empty">{"No audit events returned for this sample."}</p>
                    } else {
                        <ol class="audit-timeline">
                            {for events.iter().map(|event| html! {
                                <li>
                                    <div>
                                        <strong>{&event.event_type}</strong>
                                        <span>{&event.event_status}</span>
                                    </div>
                                    <p>{&event.summary}</p>
                                    <small>{format!("audit: {} / run: {} / at: {}", event.audit_id, event.run_id, event.created_at.as_deref().unwrap_or("unknown"))}</small>
                                    <small>{format!("payload: {} / evidence: {}", payload_signal_count_label(&event.payload, "payload fields"), refs_count_label(&event.evidence_refs))}</small>
                                    <details class="data-source-detail governance-detail">
                                        <summary>{"Sample audit payload detail"}</summary>
                                        <small>{format!("payload: {}", payload_keys_label(&event.payload))}</small>
                                        <small>{format!("evidence: {}", refs_label(&event.evidence_refs))}</small>
                                    </details>
                                </li>
                            })}
                        </ol>
                    }
                },
            }}
        </section>
    }
}
