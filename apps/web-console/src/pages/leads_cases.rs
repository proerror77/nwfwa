use crate::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

#[function_component(LeadsCasesPage)]
pub fn leads_cases_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let selected_lead_id = use_state(String::new);
    let triage_decision = use_state(|| "open_case".to_string());
    let triage_assignee = use_state(|| "investigator-1".to_string());
    let triage_reviewer = use_state(|| "lead-reviewer-1".to_string());
    let triage_priority = use_state(|| "high".to_string());
    let triage_notes = use_state(|| "Opened from Operations Studio lead triage.".to_string());
    let triage_evidence_refs = use_state(String::new);
    let selected_case_id = use_state(String::new);
    let case_status = use_state(|| "investigating".to_string());
    let case_actor = use_state(|| "case-manager-1".to_string());
    let case_notes =
        use_state(|| "Status updated from Operations Studio case workflow.".to_string());
    let case_evidence_refs = use_state(String::new);
    let investigation_outcome = use_state(|| "confirmed_fwa_prevented_payment".to_string());
    let investigation_confirmed = use_state(|| true);
    let financial_impact_type = use_state(|| "prevented_payment".to_string());
    let saving_amount = use_state(String::new);
    let investigation_notes = use_state(|| {
        "Reviewer confirmed the pre-payment FWA intervention and prevented payment.".to_string()
    });
    let investigation_evidence_refs = use_state(String::new);
    let snapshot_state = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let triage_state = use_state(|| ApiState::<TriageLeadRecord>::Idle);
    let case_update_state = use_state(|| ApiState::<UpdateCaseStatusRecord>::Idle);
    let investigation_state = use_state(|| ApiState::<PilotWritebackResponse>::Idle);
    let case_agent_state = use_state(|| ApiState::<AgentInvestigationResponse>::Idle);

    let load_cases = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_cases = load_cases.clone();
        Callback::from(move |_| load_cases.emit(()))
    };

    let triage_lead = {
        let api_key = api_key.clone();
        let selected_lead_id = selected_lead_id.clone();
        let triage_decision = triage_decision.clone();
        let triage_assignee = triage_assignee.clone();
        let triage_reviewer = triage_reviewer.clone();
        let triage_priority = triage_priority.clone();
        let triage_notes = triage_notes.clone();
        let triage_evidence_refs = triage_evidence_refs.clone();
        let snapshot_state = snapshot_state.clone();
        let triage_state = triage_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                triage_state.set(ApiState::Failed("load leads before triage".into()));
                return;
            };
            let lead = selected_lead(snapshot, &selected_lead_id);
            let Some(lead) = lead else {
                triage_state.set(ApiState::Failed("select a lead to triage".into()));
                return;
            };
            let api_key = (*api_key).clone();
            let lead_id = lead.lead_id.clone();
            let fallback_refs = lead.evidence_refs.clone();
            let payload = json!({
                "decision": (*triage_decision).clone(),
                "merge_target_lead_id": Value::Null,
                "assignee": (*triage_assignee).clone(),
                "reviewer": (*triage_reviewer).clone(),
                "priority": (*triage_priority).clone(),
                "notes": (*triage_notes).clone(),
                "evidence_refs": refs_or_fallback(&triage_evidence_refs, fallback_refs),
            });
            let triage_state = triage_state.clone();
            let snapshot_state = snapshot_state.clone();
            triage_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_triage_lead(api_key.clone(), lead_id, payload).await {
                    Ok(record) => {
                        triage_state.set(ApiState::Ready(record));
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => triage_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let update_case = {
        let api_key = api_key.clone();
        let selected_case_id = selected_case_id.clone();
        let case_status = case_status.clone();
        let case_actor = case_actor.clone();
        let case_notes = case_notes.clone();
        let case_evidence_refs = case_evidence_refs.clone();
        let snapshot_state = snapshot_state.clone();
        let case_update_state = case_update_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                case_update_state.set(ApiState::Failed("load cases before status update".into()));
                return;
            };
            let case = selected_case(snapshot, &selected_case_id);
            let Some(case) = case else {
                case_update_state.set(ApiState::Failed("select a case to update".into()));
                return;
            };
            let api_key = (*api_key).clone();
            let case_id = case.case_id.clone();
            let payload = json!({
                "status": (*case_status).clone(),
                "actor_id": (*case_actor).clone(),
                "notes": (*case_notes).clone(),
                "evidence_refs": refs_or_fallback(&case_evidence_refs, vec![format!("investigation_cases:{}", case.case_id)]),
            });
            let case_update_state = case_update_state.clone();
            let snapshot_state = snapshot_state.clone();
            case_update_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_case_status(api_key.clone(), case_id, payload).await {
                    Ok(record) => {
                        case_update_state.set(ApiState::Ready(record));
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => case_update_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let write_investigation_result = {
        let api_key = api_key.clone();
        let selected_case_id = selected_case_id.clone();
        let investigation_outcome = investigation_outcome.clone();
        let investigation_confirmed = investigation_confirmed.clone();
        let financial_impact_type = financial_impact_type.clone();
        let saving_amount = saving_amount.clone();
        let investigation_notes = investigation_notes.clone();
        let investigation_evidence_refs = investigation_evidence_refs.clone();
        let snapshot_state = snapshot_state.clone();
        let investigation_state = investigation_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                investigation_state.set(ApiState::Failed(
                    "load cases before investigation writeback".into(),
                ));
                return;
            };
            let case = selected_case(snapshot, &selected_case_id);
            let Some(case) = case else {
                investigation_state.set(ApiState::Failed("select a case to write back".into()));
                return;
            };
            if saving_amount.trim().is_empty() {
                investigation_state.set(ApiState::Failed("confirmed amount is required".into()));
                return;
            }
            let api_key = (*api_key).clone();
            let case_id = case.case_id.clone();
            let claim_id = case.claim_id.clone();
            let fallback_refs = vec![
                format!("investigation_cases:{}", case.case_id),
                format!("leads:{}", case.lead_id),
            ];
            let payload = json!({
                "case_id": case_id,
                "claim_id": claim_id,
                "investigation_id": format!("INV-UI-{}", case.case_id),
                "outcome": (*investigation_outcome).clone(),
                "confirmed_fwa": *investigation_confirmed,
                "financial_impact_type": (*financial_impact_type).clone(),
                "saving_amount": (*saving_amount).clone(),
                "currency": "CNY",
                "notes": (*investigation_notes).clone(),
                "evidence_refs": refs_or_fallback(&investigation_evidence_refs, fallback_refs),
            });
            let investigation_state = investigation_state.clone();
            let snapshot_state = snapshot_state.clone();
            investigation_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_investigation_result(api_key.clone(), payload).await {
                    Ok(record) => {
                        investigation_state.set(ApiState::Ready(record));
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => investigation_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let generate_case_investigation_package = {
        let api_key = api_key.clone();
        let selected_lead_id = selected_lead_id.clone();
        let selected_case_id = selected_case_id.clone();
        let snapshot_state = snapshot_state.clone();
        let case_agent_state = case_agent_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                case_agent_state.set(ApiState::Failed(
                    "load cases before generating the agent package".into(),
                ));
                return;
            };
            let case = selected_case(snapshot, &selected_case_id);
            let Some(case) = case else {
                case_agent_state.set(ApiState::Failed("select a case for investigation".into()));
                return;
            };
            let lead = lead_for_case(snapshot, case)
                .or_else(|| selected_lead(snapshot, &selected_lead_id));
            let top_reasons = lead
                .map(|lead| lead.reason.clone())
                .filter(|reason| !reason.trim().is_empty())
                .unwrap_or_else(|| case.routing_reason.clone());
            let payload = agent_investigation_payload(
                case.claim_id.clone(),
                lead.map(|lead| lead.risk_score.to_string())
                    .unwrap_or_else(|| "70".to_string()),
                lead.map(|lead| lead.rag.to_ascii_uppercase())
                    .unwrap_or_else(|| "RED".to_string()),
                case.scheme_family.clone(),
                top_reasons,
                "case-review".to_string(),
                case.source_system.clone(),
                format!(
                    "{},{},{},{}",
                    case.scheme_family, case.review_mode, case.lead_source, case.provider_id
                ),
            );
            let payload = match payload {
                Ok(payload) => payload,
                Err(error) => {
                    case_agent_state.set(ApiState::Failed(error));
                    return;
                }
            };
            let api_key = (*api_key).clone();
            let case_agent_state = case_agent_state.clone();
            case_agent_state.set(ApiState::Loading);
            spawn_local(async move {
                case_agent_state.set(match post_agent_investigation(api_key, payload).await {
                    Ok(response) => ApiState::Ready(response),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    {
        let load_cases = load_cases.clone();
        use_effect_with((), move |_| {
            load_cases.emit(());
            || ()
        });
    }

    let select_lead = {
        let selected_lead_id = selected_lead_id.clone();
        Callback::from(move |lead_id: String| selected_lead_id.set(lead_id))
    };

    let select_case = {
        let selected_case_id = selected_case_id.clone();
        let case_agent_state = case_agent_state.clone();
        Callback::from(move |case_id: String| {
            selected_case_id.set(case_id);
            case_agent_state.set(ApiState::Idle);
        })
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Leads & Cases"}</h2>
                    <p>{"Triage generated FWA leads into investigation cases and keep case status, SLA, reviewer, and evidence signals current."}</p>
                </div>
                <span class="status-pill">{"Case Workflow"}</span>
            </div>

            <section class="panel queue-source-panel">
                <div class="queue-source-bar">
                    <h3>{"Queue Source"}</h3>
                    <span class="status-token neutral">{"configured queue source"}</span>
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh queue" }}
                    </button>
                </div>
            </section>

            <div class="leads-cases-workflow">
                <div class="queue-column">
                    <LeadsCasesView
                        state={(*snapshot_state).clone()}
                        selected_lead_id={(*selected_lead_id).clone()}
                        selected_case_id={(*selected_case_id).clone()}
                        on_select_lead={select_lead}
                        on_select_case={select_case}
                    />
                </div>

                <aside class="panel result-stack case-action-panel">
                    <h3>{"Case Investigation Workspace"}</h3>
                    {match &*snapshot_state {
                        ApiState::Ready(snapshot) => {
                            let lead = selected_lead(snapshot, &selected_lead_id);
                            let case = selected_case(snapshot, &selected_case_id);
                            html! {
                                <>
                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Selected lead"}</span>
                                            <strong>{lead.map(|lead| lead.lead_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{lead.map(|lead| lead.reason.as_str()).unwrap_or("Lead is only the risk candidate before case work starts.")}</small>
                                        </div>
                                        <h4>{"Lead Triage"}</h4>
                                        <div class="form-grid action-form-grid">
                                            <label>
                                                {"Decision"}
                                                <select
                                                    onchange={{
                                                        let triage_decision = triage_decision.clone();
                                                        Callback::from(move |event: Event| {
                                                            triage_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                                        })
                                                    }}
                                                >
                                                    <option value="open_case" selected={(*triage_decision).as_str() == "open_case"}>{"Open case"}</option>
                                                    <option value="request_evidence" selected={(*triage_decision).as_str() == "request_evidence"}>{"Request evidence"}</option>
                                                    <option value="reject_lead" selected={(*triage_decision).as_str() == "reject_lead"}>{"Reject lead"}</option>
                                                    <option value="merge_lead" selected={(*triage_decision).as_str() == "merge_lead"}>{"Merge lead"}</option>
                                                </select>
                                            </label>
                                            {text_input("Priority", &triage_priority)}
                                            {text_input("Assignee", &triage_assignee)}
                                            {text_input("Reviewer", &triage_reviewer)}
                                            {text_input("Evidence refs", &triage_evidence_refs)}
                                        </div>
                                        <label class="compact-note">
                                            {"Notes"}
                                            <textarea
                                                value={(*triage_notes).clone()}
                                                oninput={{
                                                    let triage_notes = triage_notes.clone();
                                                    Callback::from(move |event: InputEvent| {
                                                        triage_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                                    })
                                                }}
                                            />
                                        </label>
                                        <div class="button-row">
                                            <button onclick={triage_lead} disabled={lead.is_none() || matches!(&*triage_state, ApiState::Loading)}>
                                                {if matches!(&*triage_state, ApiState::Loading) { "Submitting..." } else { "Submit triage" }}
                                            </button>
                                        </div>
                                        <TriageResultView state={(*triage_state).clone()} />
                                    </section>

                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Selected case"}</span>
                                            <strong>{case.map(|case| case.case_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{case.map(|case| case.routing_reason.as_str()).unwrap_or("Case is the human investigation work item.")}</small>
                                        </div>
                                        <h4>{"Case Brief"}</h4>
                                        {case.map(|case| html! {
                                            <>
                                                <div class="score-hero">
                                                    <div><span>{"Claim"}</span><strong>{&case.claim_id}</strong></div>
                                                    <div><span>{"SLA"}</span><strong>{format!("{} / {}h", sla_label(&case.sla_status), case.sla_target_hours)}</strong></div>
                                                    <div><span>{"Reviewer"}</span><strong>{&case.reviewer}</strong></div>
                                                </div>
                                                <div class="summary-grid">
                                                    <div><span>{"Scheme"}</span><strong>{business_label(&case.scheme_family)}</strong></div>
                                                    <div><span>{"Status"}</span><strong>{case_stage_label(&case.status)}</strong></div>
                                                    <div><span>{"Review mode"}</span><strong>{business_label(&case.review_mode)}</strong></div>
                                                    <div><span>{"Outcome"}</span><strong>{case.final_outcome.as_deref().map(business_label).unwrap_or_else(|| "Human review pending".into())}</strong></div>
                                                </div>
                                            </>
                                        }).unwrap_or_else(|| html! { <p class="empty">{"Select a case to open the investigation workspace."}</p> })}
                                        <div class="tag-grid compact-tags">
                                            <span>{"Assistive only"}</span>
                                            <span>{"Human writeback required"}</span>
                                            <span>{"Evidence refs required"}</span>
                                        </div>
                                    </section>

                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Agent-assisted case package"}</span>
                                            <strong>{case.map(|case| case.claim_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{"Draft only: checklist, similar cases, evidence summary, and writeback hints. Reviewer decides."}</small>
                                        </div>
                                        <div class="button-row">
                                            <button onclick={generate_case_investigation_package} disabled={case.is_none() || matches!(&*case_agent_state, ApiState::Loading)}>
                                                {if matches!(&*case_agent_state, ApiState::Loading) { "Generating..." } else { "Generate case package" }}
                                            </button>
                                        </div>
                                    </section>

                                    <AgentInvestigationView state={(*case_agent_state).clone()} />

                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Human Decision / Writeback"}</span>
                                            <strong>{case.map(|case| case.claim_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{case.and_then(|case| case.final_outcome.as_deref()).map(business_label).unwrap_or_else(|| "No investigation result written back yet.".into())}</small>
                                        </div>
                                        <h4>{"Investigation Writeback"}</h4>
                                        <div class="form-grid action-form-grid">
                                            {text_input("Outcome", &investigation_outcome)}
                                            <label>
                                                {"Impact type"}
                                                <select
                                                    onchange={{
                                                        let financial_impact_type = financial_impact_type.clone();
                                                        Callback::from(move |event: Event| {
                                                            financial_impact_type.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                                        })
                                                    }}
                                                >
                                                    <option value="prevented_payment" selected={(*financial_impact_type).as_str() == "prevented_payment"}>{"Prevented payment"}</option>
                                                    <option value="recovered_amount" selected={(*financial_impact_type).as_str() == "recovered_amount"}>{"Recovered amount"}</option>
                                                    <option value="estimated_impact" selected={(*financial_impact_type).as_str() == "estimated_impact"}>{"Estimated impact"}</option>
                                                    <option value="avoided_future_exposure" selected={(*financial_impact_type).as_str() == "avoided_future_exposure"}>{"Avoided exposure"}</option>
                                                    <option value="deterrence_estimate" selected={(*financial_impact_type).as_str() == "deterrence_estimate"}>{"Deterrence estimate"}</option>
                                                </select>
                                            </label>
                                            {text_input("Confirmed amount", &saving_amount)}
                                            {text_input("Evidence refs", &investigation_evidence_refs)}
                                        </div>
                                        <label class="checkbox-row">
                                            <input
                                                type="checkbox"
                                                checked={*investigation_confirmed}
                                                onchange={{
                                                    let investigation_confirmed = investigation_confirmed.clone();
                                                    Callback::from(move |event: Event| {
                                                        investigation_confirmed.set(event.target_unchecked_into::<HtmlInputElement>().checked());
                                                    })
                                                }}
                                            />
                                            {"Confirmed by reviewer"}
                                        </label>
                                        <label class="compact-note">
                                            {"Notes"}
                                            <textarea
                                                value={(*investigation_notes).clone()}
                                                oninput={{
                                                    let investigation_notes = investigation_notes.clone();
                                                    Callback::from(move |event: InputEvent| {
                                                        investigation_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                                    })
                                                }}
                                            />
                                        </label>
                                        <div class="button-row">
                                            <button onclick={write_investigation_result} disabled={case.is_none() || matches!(&*investigation_state, ApiState::Loading)}>
                                                {if matches!(&*investigation_state, ApiState::Loading) { "Writing back..." } else { "Confirm and write back" }}
                                            </button>
                                        </div>
                                        <InvestigationWritebackResultView state={(*investigation_state).clone()} />

                                        <details class="data-source-detail governance-detail">
                                            <summary>{"Case status maintenance"}</summary>
                                            <div class="form-grid action-form-grid">
                                                <label>
                                                    {"Status"}
                                                    <select
                                                        onchange={{
                                                            let case_status = case_status.clone();
                                                            Callback::from(move |event: Event| {
                                                                case_status.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                                            })
                                                        }}
                                                    >
                                                        <option value="triage" selected={(*case_status).as_str() == "triage"}>{"Triage"}</option>
                                                        <option value="investigating" selected={(*case_status).as_str() == "investigating"}>{"Investigating"}</option>
                                                        <option value="pending_evidence" selected={(*case_status).as_str() == "pending_evidence"}>{"Pending evidence"}</option>
                                                        <option value="confirmed" selected={(*case_status).as_str() == "confirmed"}>{"Confirmed"}</option>
                                                        <option value="rejected" selected={(*case_status).as_str() == "rejected"}>{"Rejected"}</option>
                                                        <option value="closed" selected={(*case_status).as_str() == "closed"}>{"Closed"}</option>
                                                    </select>
                                                </label>
                                                {text_input("Actor", &case_actor)}
                                                {text_input("Evidence refs", &case_evidence_refs)}
                                            </div>
                                            <label class="compact-note">
                                                {"Notes"}
                                                <textarea
                                                    value={(*case_notes).clone()}
                                                    oninput={{
                                                        let case_notes = case_notes.clone();
                                                        Callback::from(move |event: InputEvent| {
                                                            case_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <div class="button-row">
                                                <button onclick={update_case} disabled={case.is_none() || matches!(&*case_update_state, ApiState::Loading)}>
                                                    {if matches!(&*case_update_state, ApiState::Loading) { "Updating..." } else { "Update case status" }}
                                                </button>
                                            </div>
                                            <CaseUpdateResultView state={(*case_update_state).clone()} />
                                        </details>
                                    </section>
                                </>
                            }
                        }
                        ApiState::Loading => html! { <p>{"Loading queue actions..."}</p> },
                        ApiState::Failed(_) => html! { <p class="empty">{"Fix the queue source before taking action."}</p> },
                        ApiState::Idle => html! { <p class="empty">{"Load the queue to select a lead or case."}</p> },
                    }}
                </aside>
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct LeadsCasesProps {
    state: ApiState<LeadsCasesSnapshot>,
    selected_lead_id: String,
    selected_case_id: String,
    on_select_lead: Callback<String>,
    on_select_case: Callback<String>,
}

#[function_component(LeadsCasesView)]
fn leads_cases_view(props: &LeadsCasesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load leads and cases to inspect the investigation queue."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading leads and cases..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Investigation Control"}</h3>
                                    <p>{"Workload, urgency, and queue movement for the human investigation desk."}</p>
                                </div>
                            </div>
                            <div class="case-control-rail">
                                <div><strong>{open_lead_count(&snapshot.leads)}</strong><span>{"open leads"}</span></div>
                                <div><strong>{active_case_count(&snapshot.cases)}</strong><span>{"active cases"}</span></div>
                                <div><strong>{breached_case_count(&snapshot.cases)}</strong><span>{"SLA attention"}</span></div>
                            </div>
                            <div class="case-control-grid">
                                <div class="queue-meter-card">
                                    <span>{"Lead movement"}</span>
                                    {queue_meter("New", lead_status_count(&snapshot.leads, "new"), snapshot.leads.len(), "warning")}
                                    {queue_meter("Needs evidence", lead_status_count(&snapshot.leads, "pending_evidence"), snapshot.leads.len(), "danger")}
                                    {queue_meter("Case opened", lead_status_count(&snapshot.leads, "triaged"), snapshot.leads.len(), "success")}
                                </div>
                                <div class="queue-meter-card">
                                    <span>{"Case movement"}</span>
                                    {queue_meter("Investigating", case_status_count(&snapshot.cases, "investigating"), snapshot.cases.len(), "warning")}
                                    {queue_meter("Confirmed", case_status_count(&snapshot.cases, "confirmed"), snapshot.cases.len(), "success")}
                                    {queue_meter("Closed", case_status_count(&snapshot.cases, "closed"), snapshot.cases.len(), "neutral")}
                                </div>
                                <div class="queue-meter-card case-focus-card">
                                    <span>{"Primary pattern"}</span>
                                    <strong>{top_scheme_label(&snapshot.leads)}</strong>
                                    <small>{"use this to assign medical and SIU review capacity"}</small>
                                </div>
                            </div>
                        </section>

                        <section class="lead-case-queue-grid">
                            <div class="panel result-stack">
                                <h3>{"Generated Leads"}</h3>
                                if snapshot.leads.is_empty() {
                                    <p class="empty">{"No leads returned."}</p>
                                } else {
                                    <div class="queue-list">
                                        {for snapshot.leads.iter().take(12).enumerate().map(|(index, lead)| {
                                            let selected = props.selected_lead_id.trim();
                                            let is_active = if selected.is_empty() {
                                                index == 0
                                            } else {
                                                selected == lead.lead_id
                                            };
                                            let lead_id = lead.lead_id.clone();
                                            let on_select_lead = props.on_select_lead.clone();
                                            html! {
                                                <button
                                                    type="button"
                                                    class={classes!("row-button", "queue-row", is_active.then_some("active"))}
                                                    onclick={Callback::from(move |_| on_select_lead.emit(lead_id.clone()))}
                                                >
                                                    <div class="primary-cell">
                                                        <strong>{format!("{} / {}", lead.lead_id, lead.claim_id)}</strong>
                                                        <span>{&lead.reason}</span>
                                                        <small>{format!("{} / {} / {}", lead.scheme_family, lead.provider_id, lead.member_id)}</small>
                                                    </div>
                                                    <div class="queue-row-meta">
                                                        <span class="status-token strong">{format!("risk {}", lead.risk_score)}</span>
                                                        <span class={classes!("status-token", status_tone(&lead.rag))}>{rag_label(&lead.rag)}</span>
                                                        <span class={classes!("status-token", lead_stage_tone(&lead.status))}>{lead_stage_label(&lead.status)}</span>
                                                    </div>
                                                </button>
                                            }
                                        })}
                                    </div>
                                }
                            </div>

                            <div class="panel result-stack">
                                <h3>{"Investigation Cases"}</h3>
                                if snapshot.cases.is_empty() {
                                    <p class="empty">{"No investigation cases returned."}</p>
                                } else {
                                    <div class="queue-list">
                                        {for snapshot.cases.iter().take(12).enumerate().map(|(index, case)| {
                                            let selected = props.selected_case_id.trim();
                                            let is_active = if selected.is_empty() {
                                                index == 0
                                            } else {
                                                selected == case.case_id
                                            };
                                            let case_id = case.case_id.clone();
                                            let on_select_case = props.on_select_case.clone();
                                            html! {
                                                <button
                                                    type="button"
                                                    class={classes!("row-button", "queue-row", is_active.then_some("active"))}
                                                    onclick={Callback::from(move |_| on_select_case.emit(case_id.clone()))}
                                                >
                                                    <div class="primary-cell">
                                                        <strong>{format!("{} / {}", case.case_id, case.claim_id)}</strong>
                                                        <span>{&case.routing_reason}</span>
                                                        <small>{format!("{} / reviewer {} / lead {}", case.assignee, case.reviewer, case.lead_id)}</small>
                                                        {case.final_outcome.as_ref().map(|outcome| html! {
                                                            <small>{format!("outcome: {} / writeback {}", business_label(outcome), case.investigation_result_id.as_deref().map(business_label).unwrap_or_else(|| "Pending".into()))}</small>
                                                        }).unwrap_or_else(|| html! {})}
                                                    </div>
                                                    <div class="queue-row-meta">
                                                        <span class={classes!("status-token", priority_tone(&case.priority))}>{priority_label(&case.priority)}</span>
                                                        <span class={classes!("status-token", case_stage_tone(&case.status))}>{case_stage_label(&case.status)}</span>
                                                        <span class={classes!("status-token", sla_tone(&case.sla_status))}>{sla_label(&case.sla_status)}</span>
                                                    </div>
                                                </button>
                                            }
                                        })}
                                    </div>
                                }
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[derive(Properties, PartialEq)]
struct TriageResultProps {
    state: ApiState<TriageLeadRecord>,
}

#[function_component(TriageResultView)]
fn triage_result_view(props: &TriageResultProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Submitting lead triage..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Audit"}</span><strong>{&record.audit_id}</strong></div>
                <div><span>{"Lead"}</span><strong>{format!("{} / {}", record.lead.lead_id, lead_stage_label(&record.lead.status))}</strong></div>
                <div><span>{"Case"}</span><strong>{record.case.as_ref().map(|case| case.case_id.as_str()).unwrap_or("none")}</strong></div>
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct CaseUpdateResultProps {
    state: ApiState<UpdateCaseStatusRecord>,
}

#[function_component(CaseUpdateResultView)]
fn case_update_result_view(props: &CaseUpdateResultProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Updating case status..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Audit"}</span><strong>{&record.audit_id}</strong></div>
                <div><span>{"Case"}</span><strong>{&record.case.case_id}</strong></div>
                <div><span>{"Status"}</span><strong>{case_stage_label(&record.case.status)}</strong></div>
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct InvestigationWritebackResultProps {
    state: ApiState<PilotWritebackResponse>,
}

#[function_component(InvestigationWritebackResultView)]
fn investigation_writeback_result_view(props: &InvestigationWritebackResultProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Writing investigation result..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Claim"}</span><strong>{&record.claim_id}</strong></div>
                <div><span>{"Audit"}</span><strong>{&record.audit_id}</strong></div>
                <div><span>{"Writeback"}</span><strong>{business_label(&record.event_status)}</strong></div>
                <div><span>{"Idempotency"}</span><strong>{&record.idempotency_key}</strong></div>
            </div>
        },
    }
}
