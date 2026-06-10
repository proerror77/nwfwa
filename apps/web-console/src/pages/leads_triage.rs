use crate::types::*;
use crate::state::ApiState;
use crate::formatting::*;
use crate::ui_helpers::*;
use crate::case_helpers::*;
use yew::prelude::*;
use super::super::agent_investigator::AgentInvestigationView;
use super::leads_cases_view::{CaseUpdateResultView, InvestigationWritebackResultView, TriageResultView};
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

#[allow(clippy::too_many_arguments)]
pub(super) fn leads_triage_workspace(
    snapshot: &LeadsCasesSnapshot,
    selected_lead_id: &UseStateHandle<String>,
    selected_case_id: &UseStateHandle<String>,
    triage_decision: &UseStateHandle<String>,
    triage_assignee: &UseStateHandle<String>,
    triage_reviewer: &UseStateHandle<String>,
    triage_priority: &UseStateHandle<String>,
    triage_notes: &UseStateHandle<String>,
    triage_evidence_refs: &UseStateHandle<String>,
    merge_target_lead_id: &UseStateHandle<String>,
    case_status: &UseStateHandle<String>,
    case_actor: &UseStateHandle<String>,
    case_notes: &UseStateHandle<String>,
    case_evidence_refs: &UseStateHandle<String>,
    investigation_outcome: &UseStateHandle<String>,
    investigation_confirmed: &UseStateHandle<bool>,
    financial_impact_type: &UseStateHandle<String>,
    saving_amount: &UseStateHandle<String>,
    investigation_notes: &UseStateHandle<String>,
    investigation_evidence_refs: &UseStateHandle<String>,
    triage_state: &ApiState<TriageLeadRecord>,
    case_update_state: &ApiState<UpdateCaseStatusRecord>,
    investigation_state: &ApiState<PilotWritebackResponse>,
    case_agent_state: &ApiState<AgentInvestigationResponse>,
    on_triage_lead: Callback<()>,
    on_update_case: Callback<()>,
    on_write_investigation_result: Callback<()>,
    on_generate_case_package: Callback<()>,
) -> Html {
    let lead = selected_lead(snapshot, &**selected_lead_id);
    let case = selected_case(snapshot, &**selected_case_id);

    html! {
        <aside class="panel result-stack case-action-panel">
            <h3>{"Case Investigation Workspace"}</h3>

            <section class="action-card">
                <div class="selected-work-item">
                    <span>{"Selected lead"}</span>
                    <strong>{lead.map(|l| l.lead_id.as_str()).unwrap_or("none")}</strong>
                    <small>{lead.map(|l| l.reason.as_str()).unwrap_or("Lead is only the risk candidate before case work starts.")}</small>
                </div>
                <h4>{"Lead Triage"}</h4>
                <div class="form-grid action-form-grid">
                    <label>
                        {"Decision"}
                        <select onchange={{
                            let triage_decision = triage_decision.clone();
                            Callback::from(move |e: Event| triage_decision.set(e.target_unchecked_into::<HtmlSelectElement>().value()))
                        }}>
                            <option value="open_case" selected={(**triage_decision).as_str() == "open_case"}>{"Open case"}</option>
                            <option value="request_evidence" selected={(**triage_decision).as_str() == "request_evidence"}>{"Request evidence"}</option>
                            <option value="reject_lead" selected={(**triage_decision).as_str() == "reject_lead"}>{"Reject lead"}</option>
                            <option value="merge_lead" selected={(**triage_decision).as_str() == "merge_lead"}>{"Merge lead"}</option>
                        </select>
                    </label>
                    {text_input("Priority", triage_priority)}
                    {text_input("Assignee", triage_assignee)}
                    {text_input("Reviewer", triage_reviewer)}
                    {text_input("Evidence refs", triage_evidence_refs)}
                    {if (**triage_decision).as_str() == "merge_lead" {
                        html! { {text_input("Merge Target Lead ID", merge_target_lead_id)} }
                    } else {
                        html! {}
                    }}
                </div>
                <label class="compact-note">
                    {"Notes"}
                    <textarea
                        value={(**triage_notes).clone()}
                        oninput={{
                            let triage_notes = triage_notes.clone();
                            Callback::from(move |e: InputEvent| triage_notes.set(e.target_unchecked_into::<HtmlTextAreaElement>().value()))
                        }}
                    />
                </label>
                <div class="button-row">
                    <button
                        onclick={on_triage_lead.reform(|_| ())}
                        disabled={lead.is_none() || matches!(triage_state, ApiState::Loading)}
                    >
                        {if matches!(triage_state, ApiState::Loading) { "Submitting..." } else { "Submit triage" }}
                    </button>
                </div>
                <TriageResultView state={triage_state.clone()} />
            </section>

            <section class="action-card">
                <div class="selected-work-item">
                    <span>{"Selected case"}</span>
                    <strong>{case.map(|c| c.case_id.as_str()).unwrap_or("none")}</strong>
                    <small>{case.map(|c| c.routing_reason.as_str()).unwrap_or("Case is the human investigation work item.")}</small>
                </div>
                <h4>{"Case Brief"}</h4>
                {case.map(|c| html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Claim"}</span><strong>{&c.claim_id}</strong></div>
                            <div><span>{"SLA"}</span><strong>{format!("{} / {}h", sla_label(&c.sla_status), c.sla_target_hours)}</strong></div>
                            <div><span>{"Reviewer"}</span><strong>{&c.reviewer}</strong></div>
                        </div>
                        <div class="summary-grid">
                            <div><span>{"Scheme"}</span><strong>{business_label(&c.scheme_family)}</strong></div>
                            <div><span>{"Status"}</span><strong>{case_stage_label(&c.status)}</strong></div>
                            <div><span>{"Review mode"}</span><strong>{business_label(&c.review_mode)}</strong></div>
                            <div><span>{"Outcome"}</span><strong>{c.final_outcome.as_deref().map(business_label).unwrap_or_else(|| "Human review pending".into())}</strong></div>
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
                    <strong>{case.map(|c| c.claim_id.as_str()).unwrap_or("none")}</strong>
                    <small>{"Draft only: checklist, similar cases, evidence summary, and writeback hints. Reviewer decides."}</small>
                </div>
                <div class="button-row">
                    <button
                        onclick={on_generate_case_package.reform(|_| ())}
                        disabled={case.is_none() || matches!(case_agent_state, ApiState::Loading)}
                    >
                        {if matches!(case_agent_state, ApiState::Loading) { "Generating..." } else { "Generate case package" }}
                    </button>
                </div>
            </section>

            <AgentInvestigationView state={case_agent_state.clone()} />

            <section class="action-card">
                <div class="selected-work-item">
                    <span>{"Human Decision / Writeback"}</span>
                    <strong>{case.map(|c| c.claim_id.as_str()).unwrap_or("none")}</strong>
                    <small>{case.and_then(|c| c.final_outcome.as_deref()).map(business_label).unwrap_or_else(|| "No investigation result written back yet.".into())}</small>
                </div>
                <h4>{"Investigation Writeback"}</h4>
                <div class="form-grid action-form-grid">
                    <label>
                        {"Outcome"}
                        <select onchange={{
                            let investigation_outcome = investigation_outcome.clone();
                            Callback::from(move |e: Event| investigation_outcome.set(e.target_unchecked_into::<HtmlSelectElement>().value()))
                        }}>
                            <option value="confirmed_fwa_prevented_payment" selected={(**investigation_outcome).as_str() == "confirmed_fwa_prevented_payment"}>{"Confirmed FWA — prevented payment"}</option>
                            <option value="confirmed_fwa_recovered_amount" selected={(**investigation_outcome).as_str() == "confirmed_fwa_recovered_amount"}>{"Confirmed FWA — recovered amount"}</option>
                            <option value="confirmed_fwa_avoided_exposure" selected={(**investigation_outcome).as_str() == "confirmed_fwa_avoided_exposure"}>{"Confirmed FWA — avoided exposure"}</option>
                            <option value="false_positive" selected={(**investigation_outcome).as_str() == "false_positive"}>{"False positive"}</option>
                            <option value="improper_payment" selected={(**investigation_outcome).as_str() == "improper_payment"}>{"Improper payment"}</option>
                            <option value="insufficient_evidence" selected={(**investigation_outcome).as_str() == "insufficient_evidence"}>{"Insufficient evidence"}</option>
                            <option value="abuse_not_fraud" selected={(**investigation_outcome).as_str() == "abuse_not_fraud"}>{"Abuse — not fraud"}</option>
                            <option value="documentation_issue" selected={(**investigation_outcome).as_str() == "documentation_issue"}>{"Documentation issue"}</option>
                            <option value="medical_necessity_issue" selected={(**investigation_outcome).as_str() == "medical_necessity_issue"}>{"Medical necessity issue"}</option>
                            <option value="policy_exclusion" selected={(**investigation_outcome).as_str() == "policy_exclusion"}>{"Policy exclusion"}</option>
                        </select>
                    </label>
                    <label>
                        {"Impact type"}
                        <select onchange={{
                            let financial_impact_type = financial_impact_type.clone();
                            Callback::from(move |e: Event| financial_impact_type.set(e.target_unchecked_into::<HtmlSelectElement>().value()))
                        }}>
                            <option value="prevented_payment" selected={(**financial_impact_type).as_str() == "prevented_payment"}>{"Prevented payment"}</option>
                            <option value="recovered_amount" selected={(**financial_impact_type).as_str() == "recovered_amount"}>{"Recovered amount"}</option>
                            <option value="estimated_impact" selected={(**financial_impact_type).as_str() == "estimated_impact"}>{"Estimated impact"}</option>
                            <option value="avoided_future_exposure" selected={(**financial_impact_type).as_str() == "avoided_future_exposure"}>{"Avoided exposure"}</option>
                            <option value="deterrence_estimate" selected={(**financial_impact_type).as_str() == "deterrence_estimate"}>{"Deterrence estimate"}</option>
                        </select>
                    </label>
                    {text_input("Confirmed amount", saving_amount)}
                    {text_input("Evidence refs", investigation_evidence_refs)}
                </div>
                <label class="checkbox-row">
                    <input
                        type="checkbox"
                        checked={**investigation_confirmed}
                        onchange={{
                            let investigation_confirmed = investigation_confirmed.clone();
                            Callback::from(move |e: Event| investigation_confirmed.set(e.target_unchecked_into::<HtmlInputElement>().checked()))
                        }}
                    />
                    {"Confirmed by reviewer"}
                </label>
                <label class="compact-note">
                    {"Notes"}
                    <textarea
                        value={(**investigation_notes).clone()}
                        oninput={{
                            let investigation_notes = investigation_notes.clone();
                            Callback::from(move |e: InputEvent| investigation_notes.set(e.target_unchecked_into::<HtmlTextAreaElement>().value()))
                        }}
                    />
                </label>
                <div class="button-row">
                    <button
                        onclick={on_write_investigation_result.reform(|_| ())}
                        disabled={case.is_none() || matches!(investigation_state, ApiState::Loading)}
                    >
                        {if matches!(investigation_state, ApiState::Loading) { "Writing back..." } else { "Confirm and write back" }}
                    </button>
                </div>
                <InvestigationWritebackResultView state={investigation_state.clone()} />

                <details class="data-source-detail governance-detail">
                    <summary>{"Case status maintenance"}</summary>
                    <div class="form-grid action-form-grid">
                        <label>
                            {"Status"}
                            <select onchange={{
                                let case_status = case_status.clone();
                                Callback::from(move |e: Event| case_status.set(e.target_unchecked_into::<HtmlSelectElement>().value()))
                            }}>
                                <option value="triage" selected={(**case_status).as_str() == "triage"}>{"Triage"}</option>
                                <option value="investigating" selected={(**case_status).as_str() == "investigating"}>{"Investigating"}</option>
                                <option value="pending_evidence" selected={(**case_status).as_str() == "pending_evidence"}>{"Pending evidence"}</option>
                                <option value="confirmed" selected={(**case_status).as_str() == "confirmed"}>{"Confirmed"}</option>
                                <option value="rejected" selected={(**case_status).as_str() == "rejected"}>{"Rejected"}</option>
                                <option value="closed" selected={(**case_status).as_str() == "closed"}>{"Closed"}</option>
                            </select>
                        </label>
                        {text_input("Actor", case_actor)}
                        {text_input("Evidence refs", case_evidence_refs)}
                    </div>
                    <label class="compact-note">
                        {"Notes"}
                        <textarea
                            value={(**case_notes).clone()}
                            oninput={{
                                let case_notes = case_notes.clone();
                                Callback::from(move |e: InputEvent| case_notes.set(e.target_unchecked_into::<HtmlTextAreaElement>().value()))
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button
                            onclick={on_update_case.reform(|_| ())}
                            disabled={case.is_none() || matches!(case_update_state, ApiState::Loading)}
                        >
                            {if matches!(case_update_state, ApiState::Loading) { "Updating..." } else { "Update case status" }}
                        </button>
                    </div>
                    <CaseUpdateResultView state={case_update_state.clone()} />
                </details>
            </section>
        </aside>
    }
}
