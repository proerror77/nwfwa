use crate::data_helpers::*;
use crate::formatting::*;
use crate::rule_performance_for;
use crate::rule_ui_helpers::*;
use crate::state::ApiState;
use crate::types::*;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct RulesProps {
    pub(crate) state: ApiState<RuleOpsSnapshot>,
}

#[function_component(RulesView)]
pub(crate) fn rules_view(props: &RulesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"加载规则后可检查确定性风控控制。"}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"正在加载规则运营数据..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => {
                    let selected_rule = snapshot.rules.iter().find(|rule| rule.rule_id == snapshot.gates.rule_id);
                    html! {
                        <>
                            {rule_pack_matrix(snapshot)}
                            <section class="panel result-stack">
                                <h3>{"规则库"}</h3>
                            if snapshot.rules.is_empty() {
                                    <p class="empty">{"没有返回规则。"}</p>
                                } else {
                                    <div class="factor-card-grid">
                                        {for snapshot.rules.iter().map(|rule| {
                                            let performance = rule_performance_for(&snapshot.performance, &rule.rule_id);
                                            let review_mode_tone = match rule.review_mode.as_str() {
                                                "pre_payment" | "pre" => "warning",
                                                "post_payment" | "post" => "neutral",
                                                _ => "info",
                                            };
                                            let status_tone = match rule.status.as_str() {
                                                "submitted" => "warning",
                                                "approved" => "info",
                                                "active" => "success",
                                                _ => "neutral",
                                            };
                                            html! {
                                                <div class="factor-card">
                                                    <div>
                                                        <strong>{&rule.name}</strong>
                                                        <span class={classes!("status-pill", review_mode_tone)}>{&rule.review_mode}</span>
                                                        <span class={classes!("status-pill", status_tone)}>{&rule.status}</span>
                                                        <span>{&rule.scheme_family}</span>
                                                    </div>
                                                    <div class="summary-grid">
                                                        <div><span>{"分值"}</span><strong>{rule.score}</strong></div>
                                                        <div><span>{"动作"}</span><strong>{&rule.recommended_action}</strong></div>
                                                        <div><span>{"告警"}</span><strong>{&rule.alert_code}</strong></div>
                                                        <div><span>{"负责人"}</span><strong>{&rule.owner}</strong></div>
                                                        <div><span>{"命中"}</span><strong>{performance.map(|item| item.trigger_count).unwrap_or(0)}</strong></div>
                                                        <div><span>{"证据"}</span><strong>{refs_count_label(&rule.evidence_refs)}</strong></div>
                                                    </div>
                                                    <details class="data-source-detail governance-detail">
                                                        <summary>{"规则详情"}</summary>
                                                        <small>{format!("规则：{}", rule.rule_id)}</small>
                                                        <small>{format!("版本：启用 {} / 最新 {}", optional_u32(rule.active_version), rule.latest_version)}</small>
                                                        <small>{format!("范围：{} / {} / {}", rule.applicability_scope.review_mode, rule.applicability_scope.scheme_family, rule.applicability_scope.source)}</small>
                                                        <small>{format!("证据：{}", refs_label(&rule.evidence_refs))}</small>
                                                    </details>
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            </section>

                        <section class="panel result-stack">
                            <h3>{"规则表现"}</h3>
                            {rule_performance_visual(&snapshot.performance)}
                            if snapshot.performance.is_empty() {
                                <p class="empty">{"没有返回规则表现记录。"}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.performance.iter().map(|item| html! {
                                            <div class="metric-row">
                                                <span>{format!("{} / {}", item.rule_id, item.alert_code)}</span>
                                                <strong>{format!("精准率 {}", percent_label(item.precision))}</strong>
                                                <small>{format!("命中 {} / 已复核 {} / 已确认 {}", item.trigger_count, item.reviewed_count, item.confirmed_fwa_count)}</small>
                                                <details class="data-source-detail governance-detail">
                                                    <summary>{"表现详情"}</summary>
                                                    <small>{format!("误报 {} / 误报率 {} / 节省 {} / ROI {:.2} / 标记率 {}", item.false_positive_count, percent_label(item.false_positive_rate), item.saving_amount, item.roi, percent_label(item.mark_rate))}</small>
                                                </details>
                                            </div>
                                        })}
                                    </div>
                                }
                            </section>

                        <section class="panel result-stack">
                            <h3>{"规则发布准备度"}</h3>
                            {rule_gate_pipeline(&snapshot.gates)}
                            <div class="score-hero">
                                <div><span>{"规则"}</span><strong>{&snapshot.gates.rule_id}</strong></div>
                                <div><span>{"决策"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"通过"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                                </div>
                                <div class="summary-grid">
                                    <div><span>{"状态"}</span><strong>{&snapshot.gates.status}</strong></div>
                                    <div><span>{"版本"}</span><strong>{snapshot.gates.rule_version}</strong></div>
                                    <div><span>{"复核模式"}</span><strong>{&snapshot.gates.review_mode}</strong></div>
                                    <div><span>{"命中"}</span><strong>{snapshot.gates.trigger_count}</strong></div>
                                    <div><span>{"已复核"}</span><strong>{snapshot.gates.reviewed_count}</strong></div>
                                    <div><span>{"误报率"}</span><strong>{percent_label(snapshot.gates.false_positive_rate)}</strong></div>
                                    <div><span>{"节省金额"}</span><strong>{&snapshot.gates.saving_amount}</strong></div>
                                    <div><span>{"开放反馈"}</span><strong>{snapshot.gates.open_rule_feedback_count}</strong></div>
                                    <div><span>{"未解决反馈"}</span><strong>{snapshot.gates.unresolved_rule_feedback_count}</strong></div>
                                    <div><span>{"已批准标签"}</span><strong>{snapshot.gates.approved_label_count}</strong></div>
                                    <div><span>{"待复核标签"}</span><strong>{snapshot.gates.needs_review_label_count}</strong></div>
                                    <div><span>{"所选规则"}</span><strong>{selected_rule.map(|rule| rule.name.as_str()).unwrap_or("未列出")}</strong></div>
                                </div>
                                <h4>{"回测证据"}</h4>
                                if let Some(rule) = selected_rule {
                                    <>
                                        <div class="summary-grid">
                                            <div><span>{"状态"}</span><strong>{&rule.backtest_result.status}</strong></div>
                                            <div><span>{"样本 / 命中"}</span><strong>{format!("{} / {}", rule.backtest_result.sample_count, rule.backtest_result.matched_count)}</strong></div>
                                            <div><span>{"精准率 / 召回率"}</span><strong>{format!("{} / {}", percent_label(rule.backtest_result.precision), percent_label(rule.backtest_result.recall))}</strong></div>
                                            <div><span>{"误报率"}</span><strong>{percent_label(rule.backtest_result.false_positive_rate)}</strong></div>
                                            <div><span>{"节省金额"}</span><strong>{&rule.backtest_result.estimated_saving}</strong></div>
                                            <div><span>{"证据"}</span><strong>{refs_count_label(&rule.backtest_result.evidence_refs)}</strong></div>
                                        </div>
                                        <details class="data-source-detail governance-detail">
                                            <summary>{"回测证据详情"}</summary>
                                            <small>{format!("提升：{:.2}", rule.backtest_result.lift)}</small>
                                            <small>{format!("回测时间：{}", rule.backtest_result.created_at.as_deref().unwrap_or("未运行"))}</small>
                                            <small>{format!("规则估算：{}", rule.estimated_saving)}</small>
                                            <small>{format!("回测证据：{}", refs_label(&rule.backtest_result.evidence_refs))}</small>
                                            <small>{format!("误报历史：{} / {} / {}", rule.false_positive_history.status, rule.false_positive_history.false_positive_count, percent_label(rule.false_positive_history.false_positive_rate))}</small>
                                            <small>{format!("误报证据：{}", refs_label(&rule.false_positive_history.evidence_refs))}</small>
                                        </details>
                                    </>
                                } else {
                                    <p class="empty">{"规则库列表未返回所选规则详情。"}</p>
                                }
                                <h4>{"规则发布门禁"}</h4>
                                <details class="data-source-detail governance-detail">
                                    <summary>{format!("规则发布门禁详情：{} 个门禁", snapshot.gates.gates.len())}</summary>
                                    <div class="governance-check-list">
                                        {for snapshot.gates.gates.iter().map(|gate| html! {
                                            <div>
                                                <strong>{&gate.label}</strong>
                                                <span class={classes!("status-token", if gate.passed { "success" } else { "danger" })}>{if gate.passed { "通过" } else { "阻塞" }}</span>
                                                <small>{&gate.evidence_source}</small>
                                                <small>{&gate.blocker}</small>
                                            </div>
                                        })}
                                    </div>
                                </details>
                                if snapshot.gates.blockers.is_empty() {
                                    <p class="empty">{"没有规则发布阻塞项。"}</p>
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
