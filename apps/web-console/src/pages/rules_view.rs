use crate::*;

#[derive(Properties, PartialEq)]
pub(crate) struct RulesProps {
    pub(crate) state: ApiState<RuleOpsSnapshot>,
}

#[function_component(RulesView)]
pub(crate) fn rules_view(props: &RulesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load rules to inspect deterministic detection controls."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading rule operations..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => {
                    let selected_rule = snapshot.rules.iter().find(|rule| rule.rule_id == snapshot.gates.rule_id);
                    html! {
                        <>
                            {rule_pack_matrix(snapshot)}
                            <section class="panel result-stack">
                                <h3>{"Rule Library"}</h3>
                                if snapshot.rules.is_empty() {
                                    <p class="empty">{"No rules returned."}</p>
                                } else {
                                    <div class="factor-card-grid">
                                        {for snapshot.rules.iter().map(|rule| {
                                            let performance = rule_performance_for(&snapshot.performance, &rule.rule_id);
                                            html! {
                                                <div class="factor-card">
                                                    <div>
                                                        <strong>{&rule.name}</strong>
                                                        <span>{format!("{} / {} / {}", rule.status, rule.review_mode, rule.scheme_family)}</span>
                                                    </div>
                                                    <div class="summary-grid">
                                                        <div><span>{"Score"}</span><strong>{rule.score}</strong></div>
                                                        <div><span>{"Action"}</span><strong>{&rule.recommended_action}</strong></div>
                                                        <div><span>{"Alert"}</span><strong>{&rule.alert_code}</strong></div>
                                                        <div><span>{"Owner"}</span><strong>{&rule.owner}</strong></div>
                                                        <div><span>{"Triggers"}</span><strong>{performance.map(|item| item.trigger_count).unwrap_or(0)}</strong></div>
                                                        <div><span>{"Evidence"}</span><strong>{refs_count_label(&rule.evidence_refs)}</strong></div>
                                                    </div>
                                                    <details class="data-source-detail governance-detail">
                                                        <summary>{"Rule library detail"}</summary>
                                                        <small>{format!("rule: {}", rule.rule_id)}</small>
                                                        <small>{format!("version: active {} / latest {}", optional_u32(rule.active_version), rule.latest_version)}</small>
                                                        <small>{format!("scope: {} / {} / {}", rule.applicability_scope.review_mode, rule.applicability_scope.scheme_family, rule.applicability_scope.source)}</small>
                                                        <small>{format!("evidence: {}", refs_label(&rule.evidence_refs))}</small>
                                                    </details>
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            </section>

                        <section class="panel result-stack">
                            <h3>{"Rule Performance"}</h3>
                            {rule_performance_visual(&snapshot.performance)}
                            if snapshot.performance.is_empty() {
                                <p class="empty">{"No rule performance records returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.performance.iter().map(|item| html! {
                                            <div class="metric-row">
                                                <span>{format!("{} / {}", item.rule_id, item.alert_code)}</span>
                                                <strong>{format!("precision {}", percent_label(item.precision))}</strong>
                                                <small>{format!("triggers {} / reviewed {} / confirmed {}", item.trigger_count, item.reviewed_count, item.confirmed_fwa_count)}</small>
                                                <details class="data-source-detail governance-detail">
                                                    <summary>{"Performance detail"}</summary>
                                                    <small>{format!("FP {} / rate {} / saving {} / ROI {:.2} / mark {}", item.false_positive_count, percent_label(item.false_positive_rate), item.saving_amount, item.roi, percent_label(item.mark_rate))}</small>
                                                </details>
                                            </div>
                                        })}
                                    </div>
                                }
                            </section>

                        <section class="panel result-stack">
                            <h3>{"Rule Promotion Readiness"}</h3>
                            {rule_gate_pipeline(&snapshot.gates)}
                            <div class="score-hero">
                                <div><span>{"Rule"}</span><strong>{&snapshot.gates.rule_id}</strong></div>
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                                </div>
                                <div class="summary-grid">
                                    <div><span>{"Status"}</span><strong>{&snapshot.gates.status}</strong></div>
                                    <div><span>{"Version"}</span><strong>{snapshot.gates.rule_version}</strong></div>
                                    <div><span>{"Review Mode"}</span><strong>{&snapshot.gates.review_mode}</strong></div>
                                    <div><span>{"Triggers"}</span><strong>{snapshot.gates.trigger_count}</strong></div>
                                    <div><span>{"Reviewed"}</span><strong>{snapshot.gates.reviewed_count}</strong></div>
                                    <div><span>{"False Positive Rate"}</span><strong>{percent_label(snapshot.gates.false_positive_rate)}</strong></div>
                                    <div><span>{"Saving"}</span><strong>{&snapshot.gates.saving_amount}</strong></div>
                                    <div><span>{"Open Feedback"}</span><strong>{snapshot.gates.open_rule_feedback_count}</strong></div>
                                    <div><span>{"Unresolved Feedback"}</span><strong>{snapshot.gates.unresolved_rule_feedback_count}</strong></div>
                                    <div><span>{"Approved Labels"}</span><strong>{snapshot.gates.approved_label_count}</strong></div>
                                    <div><span>{"Needs Review Labels"}</span><strong>{snapshot.gates.needs_review_label_count}</strong></div>
                                    <div><span>{"Selected Rule"}</span><strong>{selected_rule.map(|rule| rule.name.as_str()).unwrap_or("not listed")}</strong></div>
                                </div>
                                <h4>{"Backtest Evidence"}</h4>
                                if let Some(rule) = selected_rule {
                                    <>
                                        <div class="summary-grid">
                                            <div><span>{"Status"}</span><strong>{&rule.backtest_result.status}</strong></div>
                                            <div><span>{"Sample / Matched"}</span><strong>{format!("{} / {}", rule.backtest_result.sample_count, rule.backtest_result.matched_count)}</strong></div>
                                            <div><span>{"Precision / Recall"}</span><strong>{format!("{} / {}", percent_label(rule.backtest_result.precision), percent_label(rule.backtest_result.recall))}</strong></div>
                                            <div><span>{"FP Rate"}</span><strong>{percent_label(rule.backtest_result.false_positive_rate)}</strong></div>
                                            <div><span>{"Saving"}</span><strong>{&rule.backtest_result.estimated_saving}</strong></div>
                                            <div><span>{"Evidence"}</span><strong>{refs_count_label(&rule.backtest_result.evidence_refs)}</strong></div>
                                        </div>
                                        <details class="data-source-detail governance-detail">
                                            <summary>{"Backtest evidence detail"}</summary>
                                            <small>{format!("lift: {:.2}", rule.backtest_result.lift)}</small>
                                            <small>{format!("backtest at: {}", rule.backtest_result.created_at.as_deref().unwrap_or("not_run"))}</small>
                                            <small>{format!("rule estimate: {}", rule.estimated_saving)}</small>
                                            <small>{format!("backtest evidence: {}", refs_label(&rule.backtest_result.evidence_refs))}</small>
                                            <small>{format!("FP history: {} / {} / {}", rule.false_positive_history.status, rule.false_positive_history.false_positive_count, percent_label(rule.false_positive_history.false_positive_rate))}</small>
                                            <small>{format!("FP evidence: {}", refs_label(&rule.false_positive_history.evidence_refs))}</small>
                                        </details>
                                    </>
                                } else {
                                    <p class="empty">{"Selected rule details were not returned in the library list."}</p>
                                }
                                <h4>{"Rule Promotion Gates"}</h4>
                                <details class="data-source-detail governance-detail">
                                    <summary>{format!("Rule promotion gate detail: {} gates", snapshot.gates.gates.len())}</summary>
                                    <div class="governance-check-list">
                                        {for snapshot.gates.gates.iter().map(|gate| html! {
                                            <div>
                                                <strong>{&gate.label}</strong>
                                                <span class={classes!("status-token", if gate.passed { "success" } else { "danger" })}>{if gate.passed { "passed" } else { "blocked" }}</span>
                                                <small>{&gate.evidence_source}</small>
                                                <small>{&gate.blocker}</small>
                                            </div>
                                        })}
                                    </div>
                                </details>
                                if snapshot.gates.blockers.is_empty() {
                                    <p class="empty">{"No rule promotion blockers."}</p>
                                } else {
                                    <ul class="result-list">
                                        {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                    </ul>
                                }
                            </section>
                        </>
                    }
                },
            }}
        </>
    }
}
