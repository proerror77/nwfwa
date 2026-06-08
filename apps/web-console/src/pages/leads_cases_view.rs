use crate::*;

#[derive(Properties, PartialEq)]
pub(super) struct LeadsCasesProps {
    pub(super) state: ApiState<LeadsCasesSnapshot>,
    pub(super) selected_lead_id: String,
    pub(super) selected_case_id: String,
    pub(super) on_select_lead: Callback<String>,
    pub(super) on_select_case: Callback<String>,
}

#[function_component(LeadsCasesView)]
pub(super) fn leads_cases_view(props: &LeadsCasesProps) -> Html {
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
pub(super) struct TriageResultProps {
    pub(super) state: ApiState<TriageLeadRecord>,
}

#[function_component(TriageResultView)]
pub(super) fn triage_result_view(props: &TriageResultProps) -> Html {
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
pub(super) struct CaseUpdateResultProps {
    pub(super) state: ApiState<UpdateCaseStatusRecord>,
}

#[function_component(CaseUpdateResultView)]
pub(super) fn case_update_result_view(props: &CaseUpdateResultProps) -> Html {
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
pub(super) struct InvestigationWritebackResultProps {
    pub(super) state: ApiState<PilotWritebackResponse>,
}

#[function_component(InvestigationWritebackResultView)]
pub(super) fn investigation_writeback_result_view(props: &InvestigationWritebackResultProps) -> Html {
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
