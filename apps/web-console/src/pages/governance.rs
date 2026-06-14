use crate::api::*;
use crate::data_helpers::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use crate::ui_helpers::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[function_component(GovernancePage)]
pub fn governance_page() -> Html {
    let api_key = use_api_key();
    let event_group = use_state(|| "governance".to_string());
    let snapshot_state = use_state(|| ApiState::<GovernanceSnapshot>::Idle);

    let load_governance = {
        let api_key = api_key.clone();
        let event_group = event_group.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let event_group = (*event_group).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_governance_snapshot(api_key, event_group).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_governance = load_governance.clone();
        Callback::from(move |_| load_governance.emit(()))
    };

    {
        let load_governance = load_governance.clone();
        use_effect_with((), move |_| {
            load_governance.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Governance"}</h2>
                    <p>{"Review audit events, API call records, and assistive Agent run logs with evidence references before operational approval."}</p>
                </div>
                <span class="status-pill">{"Audit Coverage"}</span>
            </div>

            <section class="panel">
                <h3>{"Governance Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"Audit event group"}
                        <input
                            value={(*event_group).clone()}
                            oninput={{
                                let event_group = event_group.clone();
                                Callback::from(move |event: InputEvent| {
                                    event_group.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh governance" }}
                    </button>
                </div>
            </section>

            <GovernanceView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct GovernanceProps {
    state: ApiState<GovernanceSnapshot>,
}

#[function_component(GovernanceView)]
fn governance_view(props: &GovernanceProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load governance logs to inspect audit and Agent controls."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading governance records..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {governance_control_tower(snapshot)}

                        <section class="panel result-stack">
                            <h3>{"Pilot Security Readiness"}</h3>
                            {pilot_readiness_cockpit(&snapshot.health)}
                            <div class="score-hero">
                                <div><span>{"Pilot Gate"}</span><strong>{&snapshot.health.pilot_readiness.status}</strong></div>
                                <div><span>{"Customer Pilot"}</span><strong>{if snapshot.health.pilot_readiness.ready_for_customer_pilot { "ready" } else { "blocked" }}</strong></div>
                                <div><span>{"Ready Checks"}</span><strong>{format!("{} / {}", snapshot.health.pilot_readiness.ready_check_count, snapshot.health.pilot_readiness.required_check_count)}</strong></div>
                                <div><span>{"Blocking Checks"}</span><strong>{snapshot.health.pilot_readiness.blocking_check_count}</strong></div>
                                <div><span>{"Health Checks"}</span><strong>{snapshot.health.checks.len()}</strong></div>
                                <div><span>{"Service"}</span><strong>{format!("{} {}", snapshot.health.service, snapshot.health.version)}</strong></div>
                            </div>
                            if snapshot.health.pilot_readiness.blocking_checks.is_empty() {
                                <p class="empty">{"All pilot configuration gates are configured for this environment."}</p>
                            } else {
                                <>
                                    <div class="factor-card-grid">
                                        {for snapshot.health.pilot_readiness.blocking_checks.iter().map(|check| html! {
                                            <div class="factor-card">
                                                <div>
                                                    <strong>{&check.name}</strong>
                                                    <span>{&check.status}</span>
                                                </div>
                                                <small>{format!("runtime: {}", check.runtime_kind.as_deref().unwrap_or("n/a"))}</small>
                                                if let Some(remediation) = &check.remediation {
                                                    <small>{remediation}</small>
                                                }
                                            </div>
                                        })}
                                    </div>
                                    <details class="data-source-detail governance-detail">
                                        <summary>{format!("All blocking check detail: {} checks", snapshot.health.pilot_readiness.blocking_checks.len())}</summary>
                                        <div class="governance-check-list">
                                            {for snapshot.health.pilot_readiness.blocking_checks.iter().map(|check| html! {
                                                <div>
                                                    <strong>{&check.name}</strong>
                                                    <span class={classes!("status-token", status_tone(&check.status))}>{&check.status}</span>
                                                    <small>{format!("runtime: {}", check.runtime_kind.as_deref().unwrap_or("n/a"))}</small>
                                                    if let Some(remediation) = &check.remediation {
                                                        <small>{remediation}</small>
                                                    }
                                                </div>
                                            })}
                                        </div>
                                    </details>
                                </>
                            }
                            {pilot_configuration_summary(&snapshot.health)}
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Audit Event Log"}</h3>
                            <div class="score-hero">
                                <div><span>{"Audit Events"}</span><strong>{snapshot.audit_events.len()}</strong></div>
                                <div><span>{"API Call Records"}</span><strong>{snapshot.api_calls.len()}</strong></div>
                                <div><span>{"Agent Run Logs"}</span><strong>{snapshot.agent_runs.len()}</strong></div>
                            </div>
                            if snapshot.audit_events.is_empty() {
                                <p class="empty">{"No audit events returned for this filter."}</p>
                            } else {
                                <ol class="audit-timeline">
                                    {for snapshot.audit_events.iter().map(|event| html! {
                                        <li>
                                            <div>
                                                <strong>{&event.event_type}</strong>
                                                <span>{&event.event_status}</span>
                                            </div>
                                            <p>{&event.summary}</p>
                                            <small>{format!("audit: {} / run: {} / at: {}", event.audit_id, event.run_id, event.created_at.as_deref().unwrap_or("unknown"))}</small>
                                            <small>{format!("evidence: {}", refs_count_label(&event.evidence_refs))}</small>
                                            <details class="inline-detail data-source-detail governance-detail">
                                                <summary>{"Payload trace detail"}</summary>
                                                <small>{payload_keys_label(&event.payload)}</small>
                                            </details>
                                        </li>
                                    })}
                                </ol>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"API Call Records"}</h3>
                            if snapshot.api_calls.is_empty() {
                                <p class="empty">{"No API call records returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.api_calls.iter().map(|call| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} {}", call.method, call.endpoint)}</strong>
                                                <span>{format!("{} / {} / {}", call.status_code, call.result, call.source_system)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Claim"}</span><strong>{empty_label(&call.claim_id)}</strong></div>
                                                <div><span>{"Event"}</span><strong>{&call.event_type}</strong></div>
                                                <div><span>{"Result"}</span><strong>{&call.result}</strong></div>
                                                <div><span>{"Evidence"}</span><strong>{refs_count_label(&call.evidence_refs)}</strong></div>
                                                <div><span>{"Observed"}</span><strong>{call.observed_at.as_deref().unwrap_or("unknown")}</strong></div>
                                            </div>
                                            <details class="data-source-detail governance-detail">
                                                <summary>{"API evidence detail"}</summary>
                                                <small>{format!("call: {}", call.call_id)}</small>
                                                <small>{format!("run: {}", call.run_id)}</small>
                                                <small>{format!("audit: {}", call.audit_id)}</small>
                                                <small>{format!("idempotency: {}", call.idempotency_key.as_deref().unwrap_or("none"))}</small>
                                                <small>{format!("evidence: {}", refs_label(&call.evidence_refs))}</small>
                                            </details>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Agent Run Logs"}</h3>
                            <p class="empty">{"Assistive Boundary: Agent outputs remain investigation support and require human approval for high-impact actions."}</p>
                            if snapshot.agent_runs.is_empty() {
                                <p class="empty">{"No Agent run logs returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.agent_runs.iter().map(|run| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{&run.claim_id}</strong>
                                                <span>{format!("{} / {}", run.status, run.decision_boundary)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Steps"}</span><strong>{run.steps.len()}</strong></div>
                                                <div><span>{"Tool Calls"}</span><strong>{run.tool_calls.len()}</strong></div>
                                                <div><span>{"Tool Results"}</span><strong>{run.tool_results.len()}</strong></div>
                                                <div><span>{"Policy Checks"}</span><strong>{run.policy_checks.len()}</strong></div>
                                                <div><span>{"Context Snapshots"}</span><strong>{run.context_snapshots.len()}</strong></div>
                                                <div><span>{"Approvals"}</span><strong>{run.approvals.len()}</strong></div>
                                            </div>
                                            <small>{format!("created: {} / completed: {}", run.created_at.as_deref().unwrap_or("unknown"), run.completed_at.as_deref().unwrap_or("pending"))}</small>
                                            <small>{format!("evidence: {}", refs_count_label(&run.evidence_refs))}</small>
                                            <small>{format!("approval: {}", approval_summary(&run.approvals))}</small>
                                            <details class="data-source-detail governance-detail">
                                                <summary>{"Agent run detail"}</summary>
                                                <small>{format!("agent run: {}", run.agent_run_id)}</small>
                                                <small>{format!("output: {}", payload_signal_count_label(&run.output_json, "output fields"))}</small>
                                                <small>{format!("evidence: {}", refs_label(&run.evidence_refs))}</small>
                                            </details>
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

fn pilot_readiness_cockpit(health: &HealthResponse) -> Html {
    let readiness = &health.pilot_readiness;
    let ready_count = readiness.ready_check_count;
    let required_count = readiness.required_check_count;
    let blocked_count = readiness.blocking_check_count;
    let ready_pct = if required_count == 0 {
        0
    } else {
        ((ready_count * 100) / required_count).min(100)
    };
    let blocker_label = readiness
        .blocking_checks
        .first()
        .map(|check| check.name.as_str())
        .unwrap_or("no active blocker");
    let ready_label = readiness
        .ready_checks
        .first()
        .map(|check| check.name.as_str())
        .unwrap_or("no ready checks");
    let required_label = readiness
        .required_check_names
        .first()
        .map(String::as_str)
        .unwrap_or("required checks not reported");
    let customer_pilot_label = if readiness.ready_for_customer_pilot {
        "ready for customer pilot"
    } else {
        "blocked for customer pilot"
    };

    html! {
        <div class="pilot-readiness-cockpit">
            <aside class="pilot-readiness-brief">
                <span class="eyebrow">{"Pilot gate status"}</span>
                <strong>{customer_pilot_label}</strong>
                <dl>
                    <div><dt>{"Ready"}</dt><dd>{format!("{ready_count} / {required_count}")}</dd></div>
                    <div><dt>{"Blocked"}</dt><dd>{blocked_count}</dd></div>
                    <div><dt>{"Decision"}</dt><dd>{&readiness.status}</dd></div>
                    <div><dt>{"Health"}</dt><dd>{health.checks.len()}</dd></div>
                    <div><dt>{"Service"}</dt><dd>{format!("{} {}", health.service, health.version)}</dd></div>
                </dl>
            </aside>

            <div class="pilot-readiness-map">
                <div class="readiness-track"></div>
                <div class="readiness-progress" style={format!("width: {ready_pct}%;")}></div>
                {readiness_node("Required", &required_count.to_string(), required_label, "required")}
                {readiness_node("Ready", &format!("{ready_pct}%"), ready_label, "ready")}
                {readiness_node("Blocked", &blocked_count.to_string(), blocker_label, "blocked")}
                {readiness_node("Decision", customer_pilot_label, "worker check-pilot-readiness", "decision")}
            </div>

            <aside class="pilot-readiness-actions">
                <span class="eyebrow">{"Next blocker"}</span>
                <strong>{
                    readiness
                        .blocking_check_names
                        .first()
                        .map(String::as_str)
                        .unwrap_or(blocker_label)
                }</strong>
                if let Some(remediation) = readiness.remediation_summary.first() {
                    <small>{remediation}</small>
                } else if let Some(check) = readiness.blocking_checks.first() {
                    <small>{check.remediation.as_deref().unwrap_or("no remediation returned")}</small>
                } else {
                    <small>{"Pilot readiness has no blocking configuration checks."}</small>
                }
            </aside>
        </div>
    }
}

fn pilot_configuration_summary(health: &HealthResponse) -> Html {
    let configuration_checks = health
        .checks
        .iter()
        .filter(|check| check.name.ends_with("_configuration"))
        .collect::<Vec<_>>();
    let configured_count = configuration_checks
        .iter()
        .filter(|check| status_tone(&check.status) == "success")
        .count();
    let needs_setup_count = configuration_checks.len().saturating_sub(configured_count);

    html! {
        <>
            <div class="summary-grid">
                <div><span>{"Configuration checks"}</span><strong>{configuration_checks.len()}</strong></div>
                <div><span>{"Configured"}</span><strong>{configured_count}</strong></div>
                <div><span>{"Needs setup"}</span><strong>{needs_setup_count}</strong></div>
            </div>
            <details class="data-source-detail governance-detail">
                <summary>{"Configuration check detail"}</summary>
                <div class="governance-check-list">
                    {for configuration_checks.iter().map(|check| html! {
                        <div>
                            <strong>{&check.name}</strong>
                            <span class={classes!("status-token", status_tone(&check.status))}>{&check.status}</span>
                            if let Some(remediation) = &check.remediation {
                                <small>{remediation}</small>
                            }
                        </div>
                    })}
                </div>
            </details>
        </>
    }
}

fn readiness_node(label: &str, value: &str, detail: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("readiness-node", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{detail}</small>
        </div>
    }
}

fn governance_control_tower(snapshot: &GovernanceSnapshot) -> Html {
    let pilot_status = snapshot.health.pilot_readiness.status.as_str();
    let first_blocker = snapshot
        .health
        .pilot_readiness
        .blocking_checks
        .first()
        .map(|check| check.name.as_str())
        .unwrap_or("no blocking checks");
    let first_audit = snapshot
        .audit_events
        .first()
        .map(|event| event.audit_id.as_str())
        .unwrap_or("audit pending");
    let first_api = snapshot
        .api_calls
        .first()
        .map(|call| call.endpoint.as_str())
        .unwrap_or("api call pending");
    let first_agent = snapshot
        .agent_runs
        .first()
        .map(|run| run.agent_run_id.as_str())
        .unwrap_or("agent run pending");
    let config_count = snapshot
        .health
        .checks
        .iter()
        .filter(|check| check.name.ends_with("_configuration"))
        .count();
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Governance control tower"}</h3>
                    <p>{"Audit-by-design map for pilot readiness, API access, Agent boundaries, and evidence trace coverage."}</p>
                </div>
                <span class={classes!("status-token", status_tone(pilot_status))}>{pilot_status}</span>
            </div>
            <div class="governance-cockpit">
                <aside class="case-brief governance-brief">
                    <span>{"Pilot readiness gate"}</span>
                    <strong>{pilot_status}</strong>
                    <dl>
                        <div><dt>{"Service"}</dt><dd>{format!("{} {}", snapshot.health.service, snapshot.health.version)}</dd></div>
                        <div><dt>{"Blockers"}</dt><dd>{snapshot.health.pilot_readiness.blocking_checks.len()}</dd></div>
                        <div><dt>{"Checks"}</dt><dd>{snapshot.health.checks.len()}</dd></div>
                        <div><dt>{"Configs"}</dt><dd>{format!("{} checks", config_count)}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("audit {}", snapshot.audit_events.len())}</span>
                        <span>{format!("api {}", snapshot.api_calls.len())}</span>
                        <span>{format!("agent {}", snapshot.agent_runs.len())}</span>
                    </div>
                </aside>

                <div class="governance-map">
                    <div class="governance-map-title">
                        <span>{"Audit-by-design map"}</span>
                        <strong>{"Evidence Trace Hub"}</strong>
                    </div>
                    <div class="governance-link horizontal"></div>
                    <div class="governance-link diagonal-a"></div>
                    <div class="governance-link diagonal-b"></div>
                    <div class="governance-core">
                        <span>{"Governance"}</span>
                        <strong>{"audit trail"}</strong>
                    </div>
                    <div class="governance-node readiness">
                        <span>{"Pilot gate"}</span>
                        <strong>{first_blocker}</strong>
                    </div>
                    <div class="governance-node api">
                        <span>{"API access"}</span>
                        <strong>{first_api}</strong>
                    </div>
                    <div class="governance-node audit">
                        <span>{"Audit event"}</span>
                        <strong>{first_audit}</strong>
                    </div>
                    <div class="governance-node agent">
                        <span>{"Agent boundary"}</span>
                        <strong>{first_agent}</strong>
                    </div>
                    <div class="governance-node evidence">
                        <span>{"Evidence refs"}</span>
                        <strong>{format!(
                            "{} audit / {} agent",
                            snapshot.audit_events.iter().filter(|event| !event.evidence_refs.is_empty()).count(),
                            snapshot.agent_runs.iter().filter(|run| !run.evidence_refs.is_empty()).count()
                        )}</strong>
                    </div>
                </div>

                <aside class="case-timeline governance-trace">
                    <h4>{"Control path"}</h4>
                    {timeline_item("Readiness", pilot_status, pilot_status)}
                    {timeline_item("API", &format!("{} records", snapshot.api_calls.len()), "done")}
                    {timeline_item("Audit", &format!("{} events", snapshot.audit_events.len()), "done")}
                    {timeline_item("Agent", "human approval boundary", "review")}
                </aside>
            </div>
        </section>
    }
}
