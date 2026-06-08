use crate::*;

pub(super) fn agent_investigator_blueprint() -> Html {
    html! {
        <section class="agent-blueprint-cockpit" aria-label="Agent investigation blueprint">
            <aside class="agent-blueprint-brief">
                <span>{"Agent investigation blueprint"}</span>
                <strong>{"assistive, evidence-bound, human-gated"}</strong>
                <dl>
                    <div><dt>{"Input"}</dt><dd>{"risk signals + top reasons"}</dd></div>
                    <div><dt>{"Tools"}</dt><dd>{"claims, rules, models, KB, documents"}</dd></div>
                    <div><dt>{"Output"}</dt><dd>{"risk summary + checklist + QA draft"}</dd></div>
                    <div><dt>{"Boundary"}</dt><dd>{"no auto denial"}</dd></div>
                </dl>
            </aside>
            <div class="agent-blueprint-map">
                <div class="agent-blueprint-rail"></div>
                <div class="agent-blueprint-node risk">
                    <span>{"Risk context"}</span>
                    <strong>{"risk signal findings"}</strong>
                    <small>{"score, RAG, reasons"}</small>
                </div>
                <div class="agent-blueprint-node evidence">
                    <span>{"Evidence collector"}</span>
                    <strong>{"source refs"}</strong>
                    <small>{"claim, rule, model, document"}</small>
                </div>
                <div class="agent-blueprint-core">
                    <span>{"Agent"}</span>
                    <strong>{"case package"}</strong>
                </div>
                <div class="agent-blueprint-node kb">
                    <span>{"Knowledge base"}</span>
                    <strong>{"similar cases"}</strong>
                    <small>{"provenance required"}</small>
                </div>
                <div class="agent-blueprint-node qa">
                    <span>{"QA draft"}</span>
                    <strong>{"review opinion"}</strong>
                    <small>{"human editable"}</small>
                </div>
                <div class="agent-blueprint-node gate">
                    <span>{"Human gate"}</span>
                    <strong>{"review only"}</strong>
                    <small>{"decision stays outside Agent"}</small>
                </div>
            </div>
            <aside class="agent-blueprint-guardrail">
                <span>{"Governance locks"}</span>
                <div class="tag-grid compact-tags">
                    <span>{"Tool allowlist"}</span>
                    <span>{"PII masking"}</span>
                    <span>{"Evidence refs"}</span>
                    <span>{"Audit events"}</span>
                    <span>{"Timeouts"}</span>
                    <span>{"Human approval"}</span>
                </div>
                <p>{"The Agent prepares investigation material. It cannot deny, approve, publish rules, or bypass audit."}</p>
            </aside>
        </section>
    }
}

#[derive(Properties, PartialEq)]
pub(crate) struct AgentInvestigationProps {
    pub(crate) state: ApiState<AgentInvestigationResponse>,
}

#[function_component(AgentInvestigationView)]
pub(crate) fn agent_investigation_view(props: &AgentInvestigationProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Investigation Package"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Generate an investigation package to inspect findings, checklist, similar cases, QA draft, and evidence sufficiency."}</p> },
                ApiState::Loading => html! { <p>{"Generating investigation package..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        {agent_investigation_cockpit(response)}
                        <div class="score-hero">
                            <div><span>{"Agent Run"}</span><strong>{&response.agent_run_id}</strong></div>
                            <div><span>{"Boundary"}</span><strong>{business_label(&response.decision_boundary)}</strong></div>
                            <div><span>{"Evidence"}</span><strong>{response.evidence_refs.len()}</strong></div>
                        </div>
                        <p>{&response.risk_summary}</p>
                        <div class="summary-grid">
                            <div><span>{"Evidence Status"}</span><strong>{business_label(&response.evidence_sufficiency.status)}</strong></div>
                            <div><span>{"Scheme"}</span><strong>{business_label(&response.evidence_sufficiency.scheme_family)}</strong></div>
                            <div><span>{"Present"}</span><strong>{response.evidence_sufficiency.present_evidence.len()}</strong></div>
                            <div><span>{"Missing"}</span><strong>{response.evidence_sufficiency.missing_evidence.len()}</strong></div>
                        </div>

                        <h4>{"Findings"}</h4>
                        <div class="factor-card-grid">
                            {for response.findings.iter().map(|finding| html! {
                                <div class="metric-row">
                                    <span>{&finding.finding}</span>
                                    <strong>{refs_count_label(&finding.evidence_refs)}</strong>
                                </div>
                            })}
                        </div>

                        <h4>{"Investigation Checklist"}</h4>
                        <ul class="result-list">
                            {for response.investigation_checklist.iter().map(|item| html! { <li>{item}</li> })}
                        </ul>

                        <h4>{"Similar Cases"}</h4>
                        if response.similar_cases.is_empty() {
                            <p class="empty">{"No similar cases returned."}</p>
                        } else {
                            <div class="factor-card-grid">
                                {for response.similar_cases.iter().map(|case| html! {
                                    <div class="metric-row">
                                        <span>{&case.case_id}</span>
                                        <strong>{format!("{:.2}", case.similarity_score)}</strong>
                                        <small>{format!("signals: {}", refs_count_label(&case.matched_signals))}</small>
                                        <small>{format!("provenance: {}", refs_count_label(&case.provenance_refs))}</small>
                                    </div>
                                })}
                            </div>
                        }

                        <h4>{"QA Opinion Draft"}</h4>
                        <p>{&response.qa_opinion_draft}</p>

                        <h4>{"Evidence Buckets"}</h4>
                        <div class="summary-grid">
                            <div><span>{"Claim"}</span><strong>{response.evidence_refs_by_type.claim.len()}</strong></div>
                            <div><span>{"Rule"}</span><strong>{response.evidence_refs_by_type.rule.len()}</strong></div>
                            <div><span>{"Model"}</span><strong>{response.evidence_refs_by_type.model.len()}</strong></div>
                            <div><span>{"Anomaly"}</span><strong>{response.evidence_refs_by_type.anomaly.len()}</strong></div>
                            <div><span>{"Document"}</span><strong>{response.evidence_refs_by_type.document.len()}</strong></div>
                            <div><span>{"Similar Case"}</span><strong>{response.evidence_refs_by_type.similar_case.len()}</strong></div>
                        </div>
                        <small>{format!("evidence: {}", refs_count_label(&response.evidence_refs))}</small>
                        <details class="data-source-detail governance-detail">
                            <summary>{"Investigation evidence detail"}</summary>
                            <small>{refs_label(&response.evidence_refs)}</small>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

fn agent_investigation_cockpit(response: &AgentInvestigationResponse) -> Html {
    let top_finding = response
        .findings
        .first()
        .map(|finding| finding.finding.as_str())
        .unwrap_or("finding pending");
    let similar_case = response
        .similar_cases
        .first()
        .map(|case| case.case_id.as_str())
        .unwrap_or("no similar case");
    let missing_evidence = response
        .evidence_sufficiency
        .missing_evidence
        .first()
        .map(String::as_str)
        .unwrap_or("none");
    html! {
        <div class="agent-cockpit">
            <aside class="case-brief agent-brief">
                <span>{"Agent investigation command"}</span>
                <strong>{&response.agent_run_id}</strong>
                <dl>
                    <div><dt>{"Boundary"}</dt><dd>{business_label(&response.decision_boundary)}</dd></div>
                    <div><dt>{"Scheme"}</dt><dd>{business_label(&response.evidence_sufficiency.scheme_family)}</dd></div>
                    <div><dt>{"Evidence"}</dt><dd>{response.evidence_refs.len()}</dd></div>
                    <div><dt>{"Status"}</dt><dd>{business_label(&response.evidence_sufficiency.status)}</dd></div>
                </dl>
                <div class="tag-grid compact-tags">
                    <span>{format!("findings {}", response.findings.len())}</span>
                    <span>{format!("checklist {}", response.investigation_checklist.len())}</span>
                    <span>{format!("similar {}", response.similar_cases.len())}</span>
                </div>
            </aside>

            <div class="agent-evidence-map">
                <div class="agent-map-title">
                    <span>{"Agent evidence orchestration"}</span>
                    <strong>{"assistive package only"}</strong>
                </div>
                <div class="agent-map-link horizontal"></div>
                <div class="agent-map-link diagonal-a"></div>
                <div class="agent-map-link diagonal-b"></div>
                <div class="agent-node risk">
                    <span>{"7-layer risk"}</span>
                    <strong>{top_finding}</strong>
                </div>
                <div class="agent-node evidence">
                    <span>{"Evidence buckets"}</span>
                    <strong>{format!(
                        "claim {} / rule {} / model {}",
                        response.evidence_refs_by_type.claim.len(),
                        response.evidence_refs_by_type.rule.len(),
                        response.evidence_refs_by_type.model.len()
                    )}</strong>
                </div>
                <div class="agent-node kb">
                    <span>{"Knowledge memory"}</span>
                    <strong>{similar_case}</strong>
                </div>
                <div class="agent-node qa">
                    <span>{"QA draft"}</span>
                    <strong>{&response.qa_opinion_draft}</strong>
                </div>
                <div class="agent-node human">
                    <span>{"Human gate"}</span>
                    <strong>{missing_evidence}</strong>
                </div>
                <div class="agent-core">
                    <span>{"Agent"}</span>
                    <strong>{"evidence pack"}</strong>
                </div>
            </div>

            <aside class="case-timeline agent-guardrail">
                <h4>{"Guardrail path"}</h4>
                {timeline_item("Input", "risk output + evidence refs", "done")}
                {timeline_item("Tools", "allowlisted retrieval", "done")}
                {timeline_item("Output", "structured summary", "ready")}
                {timeline_item("Action", "human approval required", "review")}
            </aside>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub(super) struct AgentRunsProps {
    pub(super) state: ApiState<Vec<AgentRunRecord>>,
}

#[function_component(AgentRunsView)]
pub(super) fn agent_runs_view(props: &AgentRunsProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Agent Run Evidence Trail"}</h3>
            <p class="empty">{"Assistive Boundary: Agent outputs support investigation and require human approval before high-impact downstream action."}</p>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Refresh Agent runs to inspect evidence trail."}</p> },
                ApiState::Loading => html! { <p>{"Loading Agent runs..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(runs) => html! {
                    if runs.is_empty() {
                        <p class="empty">{"No Agent runs returned."}</p>
                    } else {
                        <>
                            {agent_run_governance_cockpit(&runs[0])}
                            <div class="factor-card-grid">
                                {for runs.iter().take(8).map(|run| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{&run.claim_id}</strong>
                                            <span>{format!("{} / {}", business_label(&run.status), business_label(&run.decision_boundary))}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Steps"}</span><strong>{run.steps.len()}</strong></div>
                                            <div><span>{"Tool Calls"}</span><strong>{run.tool_calls.len()}</strong></div>
                                            <div><span>{"Policy Checks"}</span><strong>{run.policy_checks.len()}</strong></div>
                                            <div><span>{"Approvals"}</span><strong>{run.approvals.len()}</strong></div>
                                            <div><span>{"Evidence"}</span><strong>{refs_count_label(&run.evidence_refs)}</strong></div>
                                        </div>
                                        <small>{format!("created: {} / completed: {}", run.created_at.as_deref().unwrap_or("unknown"), run.completed_at.as_deref().unwrap_or("pending"))}</small>
                                        <small>{format!("approval: {}", approval_count_label(&run.approvals))}</small>
                                        <details class="data-source-detail governance-detail">
                                            <summary>{"Agent run evidence detail"}</summary>
                                            <small>{format!("agent run: {}", run.agent_run_id)}</small>
                                            <small>{format!("evidence: {}", refs_label(&run.evidence_refs))}</small>
                                            <small>{format!("approval: {}", approval_summary(&run.approvals))}</small>
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

fn agent_run_governance_cockpit(run: &AgentRunRecord) -> Html {
    let policy_label = run
        .policy_checks
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no policy check".into());
    let tool_label = run
        .tool_calls
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no tool call".into());
    let result_label = run
        .tool_results
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no tool result".into());
    let context_label = run
        .context_snapshots
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no context snapshot".into());
    let step_label = run
        .steps
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no step".into());
    let approval_label = if run.approvals.is_empty() {
        "no approval record".into()
    } else {
        format!("{} approval records", run.approvals.len())
    };
    let evidence_label = format!("{} evidence refs", run.evidence_refs.len());
    let output_label = compact_payload_label(&run.output_json);

    html! {
        <div class="agent-run-cockpit">
            <aside class="agent-run-brief">
                <span class="eyebrow">{"Agent Run Governance Map"}</span>
                <strong>{&run.agent_run_id}</strong>
                <dl>
                    <div><dt>{"Claim"}</dt><dd>{&run.claim_id}</dd></div>
                    <div><dt>{"Status"}</dt><dd>{business_label(&run.status)}</dd></div>
                    <div><dt>{"Boundary"}</dt><dd>{business_label(&run.decision_boundary)}</dd></div>
                    <div><dt>{"Evidence"}</dt><dd>{run.evidence_refs.len()}</dd></div>
                </dl>
            </aside>

            <div class="agent-run-map">
                <div class="agent-run-map-title">
                    <span>{"Governed agent execution"}</span>
                    <strong>{"context -> policy check -> tool allowlist -> result -> human approval -> audit"}</strong>
                </div>
                <div class="agent-run-link"></div>
                <div class="agent-run-link diagonal-a"></div>
                <div class="agent-run-link diagonal-b"></div>
                <div class="agent-run-core">
                    <span>{"Assistive Only"}</span>
                    <strong>{business_label(&run.status)}</strong>
                </div>
                {agent_run_node("Context snapshot", &context_label, "context")}
                {agent_run_node("Policy check", &policy_label, "policy")}
                {agent_run_node("Tool allowlist", &tool_label, "tool")}
                {agent_run_node("Tool result", &result_label, "result")}
                {agent_run_node("Human approval gate", &approval_label, "approval")}
                {agent_run_node("Evidence audit trail", &evidence_label, "audit")}
            </div>

            <aside class="agent-run-trace">
                <span class="eyebrow">{"Execution counters"}</span>
                <div class="provider-signal-stack">
                    {provider_signal_row("Steps", &format!("{} / {}", run.steps.len(), step_label), "neutral")}
                    {provider_signal_row("Policy checks", &run.policy_checks.len().to_string(), "strong")}
                    {provider_signal_row("Tool calls", &run.tool_calls.len().to_string(), "warning")}
                    {provider_signal_row("Approvals", &run.approvals.len().to_string(), "danger")}
                    {provider_signal_row("Output JSON", &output_label, "neutral")}
                </div>
            </aside>
        </div>
    }
}

fn agent_run_node(label: &str, value: &str, position: &str) -> Html {
    html! {
        <div class={classes!("agent-run-node", position.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}
