use crate::api::*;
use crate::types::*;
use crate::state::{use_api_key, ApiState};
use crate::case_helpers::*;
use crate::payload_helpers::*;
use yew::prelude::*;
use serde_json::{json, Value};
use wasm_bindgen_futures::spawn_local;

#[path = "leads_cases_view.rs"]
mod leads_cases_view;
use leads_cases_view::LeadsCasesView;

#[path = "leads_triage.rs"]
mod leads_triage;
use leads_triage::leads_triage_workspace;

#[function_component(LeadsCasesPage)]
pub fn leads_cases_page() -> Html {
    let api_key = use_api_key();
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
    let member_context_state = use_state(|| ApiState::<MemberProfileSummary>::Idle);
    let provider_context_state = use_state(|| ApiState::<ProviderRiskSummary>::Idle);

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
        Callback::from(move |()| {
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
        Callback::from(move |()| {
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
        Callback::from(move |()| {
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
        Callback::from(move |()| {
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
        let snapshot_state = snapshot_state.clone();
        let api_key = api_key.clone();
        let member_context_state = member_context_state.clone();
        let provider_context_state = provider_context_state.clone();
        Callback::from(move |lead_id: String| {
            selected_lead_id.set(lead_id.clone());

            let member_id = if let ApiState::Ready(snapshot) = &*snapshot_state {
                snapshot.leads.iter()
                    .find(|l| l.lead_id == lead_id)
                    .map(|l| l.member_id.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            };

            if !member_id.is_empty() {
                let api_key_m = (*api_key).clone();
                let member_context_state = member_context_state.clone();
                member_context_state.set(ApiState::Loading);
                spawn_local(async move {
                    member_context_state.set(
                        match get_member_profile_summary(api_key_m, member_id).await {
                            Ok(summary) => ApiState::Ready(summary),
                            Err(error) => ApiState::Failed(error),
                        }
                    );
                });
            }

            let api_key_p = (*api_key).clone();
            let provider_context_state = provider_context_state.clone();
            provider_context_state.set(ApiState::Loading);
            spawn_local(async move {
                provider_context_state.set(
                    match get_provider_risk_summary(api_key_p).await {
                        Ok(summary) => ApiState::Ready(summary),
                        Err(error) => ApiState::Failed(error),
                    }
                );
            });
        })
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
                        member_context_state={(*member_context_state).clone()}
                        provider_context_state={(*provider_context_state).clone()}
                    />
                </div>

                {match &*snapshot_state {
                    ApiState::Ready(snapshot) => leads_triage_workspace(
                        snapshot,
                        &selected_lead_id,
                        &selected_case_id,
                        &triage_decision,
                        &triage_assignee,
                        &triage_reviewer,
                        &triage_priority,
                        &triage_notes,
                        &triage_evidence_refs,
                        &case_status,
                        &case_actor,
                        &case_notes,
                        &case_evidence_refs,
                        &investigation_outcome,
                        &investigation_confirmed,
                        &financial_impact_type,
                        &saving_amount,
                        &investigation_notes,
                        &investigation_evidence_refs,
                        &triage_state,
                        &case_update_state,
                        &investigation_state,
                        &case_agent_state,
                        triage_lead,
                        update_case,
                        write_investigation_result,
                        generate_case_investigation_package,
                    ),
                    ApiState::Loading => html! {
                        <aside class="panel result-stack case-action-panel">
                            <p>{"Loading queue actions..."}</p>
                        </aside>
                    },
                    ApiState::Failed(_) => html! {
                        <aside class="panel result-stack case-action-panel">
                            <p class="empty">{"Fix the queue source before taking action."}</p>
                        </aside>
                    },
                    ApiState::Idle => html! {
                        <aside class="panel result-stack case-action-panel">
                            <p class="empty">{"Load the queue to select a lead or case."}</p>
                        </aside>
                    },
                }}
            </div>
        </section>
    }
}
