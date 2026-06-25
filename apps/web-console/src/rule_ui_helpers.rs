use crate::{
    percent_label, percent_width, pretty_json, refs_label, response_rule_id, rule_performance_for,
    scaled_width, ApiState, RuleBacktestResponse, RuleDiscoveryCandidate, RuleDiscoveryResponse,
    RuleOpsSnapshot, RulePerformance, RulePromotionGates, RuleSummary,
};
use serde_json::Value;
use yew::prelude::*;

pub(crate) fn rule_performance_visual(performance: &[RulePerformance]) -> Html {
    if performance.is_empty() {
        return html! {};
    }
    let max_trigger_count = performance
        .iter()
        .map(|item| item.trigger_count)
        .max()
        .unwrap_or(1);
    html! {
        <div class="visual-panel wide-visual">
            <h4>{"规则命中路径"}</h4>
            <div class="rule-bars">
                {for performance.iter().map(|item| html! {
                    <div class="rule-bar-row">
                        <div>
                            <strong>{&item.rule_id}</strong>
                            <span>{&item.alert_code}</span>
                        </div>
                        <div class="bar-track">
                            <i style={format!("width: {};", scaled_width(item.trigger_count, max_trigger_count))}></i>
                        </div>
                        <div class="dual-meter">
                            <span style={format!("width: {};", percent_width(item.precision))}></span>
                            <em style={format!("width: {};", percent_width(item.false_positive_rate))}></em>
                        </div>
                        <small>{format!("精准率 {} / 误报 {}", percent_label(item.precision), percent_label(item.false_positive_rate))}</small>
                    </div>
                })}
            </div>
        </div>
    }
}

pub(crate) fn rule_pack_matrix(snapshot: &RuleOpsSnapshot) -> Html {
    let total_rules = snapshot.rules.len();
    let active_rules = snapshot
        .rules
        .iter()
        .filter(|rule| rule.status == "active")
        .count();
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"FWA 规则包矩阵"}</h3>
                    <p>{"试点演示使用的产品化规则族：每个规则族展示当前规则库覆盖情况和运营表现。"}</p>
                </div>
                <span class="status-token strong">{"规则包"}</span>
            </div>
            <div class="rule-pack-cockpit">
                <aside class="rule-pack-brief">
                    <span class="eyebrow">{"PRD 规则覆盖"}</span>
                    <strong>{format!("{} 条启用 / {} 条已列出", active_rules, total_rules)}</strong>
                    <small>{"确定性规则必须可解释、可版本化、可回测，并经人工批准后才能进入生产路由。"}</small>
                    <div class="rule-pack-meter">
                        <i style={format!("width: {};", percent_width(rule_pack_coverage_ratio(snapshot))) }></i>
                    </div>
                    <small>{format!("已覆盖规则族：{} / 5", covered_rule_pack_count(snapshot))}</small>
                </aside>
                <div class="rule-pack-map">
                    <div class="rule-pack-link"></div>
                    <div class="rule-pack-core">
                        <span>{"L2"}</span>
                        <strong>{"规则引擎"}</strong>
                    </div>
                    {rule_pack_family_node(snapshot, "重复计费", "同服务 / 同金额", "duplicate", "top")}
                    {rule_pack_family_node(snapshot, "早期高额理赔", "新保单 + 高金额", "early", "right")}
                    {rule_pack_family_node(snapshot, "机构同侪异常", "同组机构偏离", "provider", "bottom")}
                    {rule_pack_family_node(snapshot, "诊断-项目不匹配", "编码一致性", "diagnosis", "left")}
                    {rule_pack_family_node(snapshot, "医疗必要性证据缺口", "需要病历支撑", "medical", "lower-right")}
                </div>
                <aside class="rule-pack-legend">
                    <span class="eyebrow">{"人工安全生命周期"}</span>
                    {rule_pack_lifecycle_row("草稿", "沙箱 / 回测", "neutral")}
                    {rule_pack_lifecycle_row("复核", "QA + 误报", "warning")}
                    {rule_pack_lifecycle_row("批准", "负责人确认", "strong")}
                    {rule_pack_lifecycle_row("路由", "仅建议复核", "danger")}
                </aside>
            </div>
        </section>
    }
}

fn rule_pack_family_node(
    snapshot: &RuleOpsSnapshot,
    label: &'static str,
    caption: &'static str,
    family_key: &'static str,
    position: &'static str,
) -> Html {
    let rules = snapshot
        .rules
        .iter()
        .filter(|rule| rule_matches_family(rule, family_key))
        .collect::<Vec<_>>();
    let rule_count = rules.len();
    let trigger_count = rules
        .iter()
        .filter_map(|rule| rule_performance_for(&snapshot.performance, &rule.rule_id))
        .map(|performance| performance.trigger_count)
        .sum::<u32>();
    let precision = rules
        .iter()
        .filter_map(|rule| rule_performance_for(&snapshot.performance, &rule.rule_id))
        .map(|performance| performance.precision)
        .next();
    let tone = if rule_count > 0 { "covered" } else { "gap" };
    html! {
        <div class={classes!("rule-pack-node", position, tone)}>
            <span>{label}</span>
            <strong>{if rule_count > 0 { format!("{rule_count} 条规则") } else { "缺口".into() }}</strong>
            <small>{caption}</small>
            <em>{format!("命中 {} / 精准率 {}", trigger_count, precision.map(percent_label).unwrap_or_else(|| "无".into()))}</em>
        </div>
    }
}

fn rule_pack_lifecycle_row(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("provider-signal-row", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn covered_rule_pack_count(snapshot: &RuleOpsSnapshot) -> usize {
    ["duplicate", "early", "provider", "diagnosis", "medical"]
        .iter()
        .filter(|family| {
            snapshot
                .rules
                .iter()
                .any(|rule| rule_matches_family(rule, family))
        })
        .count()
}

fn rule_pack_coverage_ratio(snapshot: &RuleOpsSnapshot) -> f64 {
    covered_rule_pack_count(snapshot) as f64 / 5.0
}

fn rule_matches_family(rule: &RuleSummary, family_key: &str) -> bool {
    let haystack = format!(
        "{} {} {} {} {}",
        rule.rule_id,
        rule.name,
        rule.scheme_family,
        rule.alert_code,
        rule.applicability_scope.scheme_family
    )
    .to_lowercase();
    match family_key {
        "duplicate" => contains_any(&haystack, &["duplicate", "repeat", "same_service"]),
        "early" => contains_any(
            &haystack,
            &["early", "high_amount", "high_value", "short_term"],
        ),
        "provider" => contains_any(&haystack, &["provider", "peer", "outlier", "cohort"]),
        "diagnosis" => contains_any(&haystack, &["diagnosis", "procedure", "mismatch", "coding"]),
        "medical" => contains_any(
            &haystack,
            &["medical", "necessity", "evidence_gap", "documentation"],
        ),
        _ => false,
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

pub(crate) fn rule_backfill_pipeline(
    discovery_state: &UseStateHandle<ApiState<RuleDiscoveryResponse>>,
    backtest_state: &UseStateHandle<ApiState<RuleBacktestResponse>>,
    save_state: &UseStateHandle<ApiState<Value>>,
    shadow_state: &UseStateHandle<ApiState<Value>>,
    review_state: &UseStateHandle<ApiState<Value>>,
) -> Html {
    let nodes = [
        (
            "发现",
            matches!(&**discovery_state, ApiState::Ready(_)),
            state_label(discovery_state),
        ),
        (
            "回测",
            matches!(&**backtest_state, ApiState::Ready(_)),
            state_label(backtest_state),
        ),
        (
            "草稿",
            matches!(&**save_state, ApiState::Ready(_)),
            state_label(save_state),
        ),
        (
            "影子验证",
            matches!(&**shadow_state, ApiState::Ready(_)),
            state_label(shadow_state),
        ),
        (
            "审核",
            matches!(&**review_state, ApiState::Ready(_)),
            state_label(review_state),
        ),
    ];
    gate_pipeline("候选规则流程", &nodes)
}

fn state_label<T>(state: &UseStateHandle<ApiState<T>>) -> &'static str
where
    T: Clone + PartialEq + 'static,
{
    match &**state {
        ApiState::Idle => "待处理",
        ApiState::Loading => "运行中",
        ApiState::Ready(_) => "已就绪",
        ApiState::Failed(_) => "阻塞",
    }
}

pub(crate) fn rule_candidate_workflow(
    discovery_state: &UseStateHandle<ApiState<RuleDiscoveryResponse>>,
    backtest_state: &UseStateHandle<ApiState<RuleBacktestResponse>>,
    save_state: &UseStateHandle<ApiState<Value>>,
    shadow_state: &UseStateHandle<ApiState<Value>>,
    selected_candidate_id: &UseStateHandle<String>,
    accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    shadowed_candidate_ids: &UseStateHandle<Vec<String>>,
    final_accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    rejected_candidate_ids: &UseStateHandle<Vec<String>>,
) -> Html {
    html! {
        <div class="rule-candidate-workflow">
            {rule_discovery_candidates_view(
                discovery_state,
                selected_candidate_id,
                accepted_candidate_ids,
                shadowed_candidate_ids,
                final_accepted_candidate_ids,
                rejected_candidate_ids,
            )}
            {rule_backtest_view(backtest_state)}
            {rule_save_view(save_state)}
            {rule_shadow_run_state(shadow_state)}
        </div>
    }
}

fn rule_discovery_candidates_view(
    discovery_state: &UseStateHandle<ApiState<RuleDiscoveryResponse>>,
    selected_candidate_id: &UseStateHandle<String>,
    accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    shadowed_candidate_ids: &UseStateHandle<Vec<String>>,
    final_accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    rejected_candidate_ids: &UseStateHandle<Vec<String>>,
) -> Html {
    match &**discovery_state {
        ApiState::Idle => {
            html! { <p class="empty">{"运行规则发现，从可解释模型信号生成受治理的候选规则。"}</p> }
        }
        ApiState::Loading => html! { <p>{"正在发现候选规则..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="result-stack">
                <div class="summary-grid">
                    <div><span>{"样本"}</span><strong>{response.sample_count}</strong></div>
                    <div><span>{"阳性标签"}</span><strong>{response.positive_count}</strong></div>
                    <div><span>{"候选规则"}</span><strong>{response.candidates.len()}</strong></div>
                </div>
                <div class="factor-card-grid">
                    {for response.candidates.iter().map(|candidate| {
                        let candidate_id = crate::rule_candidate_id(candidate);
                        let is_selected = candidate_id == **selected_candidate_id;
                        let review_status = candidate_review_label(
                            &candidate_id,
                            accepted_candidate_ids,
                            shadowed_candidate_ids,
                            final_accepted_candidate_ids,
                            rejected_candidate_ids,
                        );
                        let review_tone = candidate_review_tone(
                            &candidate_id,
                            accepted_candidate_ids,
                            shadowed_candidate_ids,
                            final_accepted_candidate_ids,
                            rejected_candidate_ids,
                        );
                        let selected_candidate_id = selected_candidate_id.clone();
                        let candidate_id_for_click = candidate_id.clone();
                        html! {
                            <button
                                class={classes!("rule-candidate-card", review_tone, is_selected.then_some("active"))}
                                onclick={Callback::from(move |_| selected_candidate_id.set(candidate_id_for_click.clone()))}
                            >
                                <span>{candidate_id.clone()}</span>
                                <em>{review_status}</em>
                                <strong>{rule_candidate_name(candidate)}</strong>
                                <small>{&candidate.explanation}</small>
                                <div class="summary-grid compact-summary-grid">
                                    <div><span>{"支持度"}</span><strong>{candidate.support}</strong></div>
                                    <div><span>{"精准率"}</span><strong>{percent_label(candidate.precision)}</strong></div>
                                    <div><span>{"提升"}</span><strong>{format!("{:.2}", candidate.lift)}</strong></div>
                                    <div><span>{"节省金额"}</span><strong>{&candidate.estimated_saving}</strong></div>
                                </div>
                                <div class="candidate-evidence-strip">
                                    <small>{format!("匹配理赔：{}", refs_label(&candidate.matched_claim_ids))}</small>
                                    <small>{format!("证据：{}", refs_label(&candidate.evidence_refs))}</small>
                                </div>
                            </button>
                        }
                    })}
                </div>
            </div>
        },
    }
}

fn candidate_review_label(
    candidate_id: &str,
    accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    shadowed_candidate_ids: &UseStateHandle<Vec<String>>,
    final_accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    rejected_candidate_ids: &UseStateHandle<Vec<String>>,
) -> &'static str {
    if rejected_candidate_ids
        .iter()
        .any(|rejected_id| rejected_id == candidate_id)
    {
        "已拒绝"
    } else if final_accepted_candidate_ids
        .iter()
        .any(|accepted_id| accepted_id == candidate_id)
    {
        "影子复核后已接受"
    } else if shadowed_candidate_ids
        .iter()
        .any(|shadowed_id| shadowed_id == candidate_id)
    {
        "影子证据已就绪"
    } else if accepted_candidate_ids
        .iter()
        .any(|draft_id| draft_id == candidate_id)
    {
        "草稿已保存用于影子验证"
    } else {
        "需要回测"
    }
}

fn candidate_review_tone(
    candidate_id: &str,
    accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    shadowed_candidate_ids: &UseStateHandle<Vec<String>>,
    final_accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    rejected_candidate_ids: &UseStateHandle<Vec<String>>,
) -> &'static str {
    if rejected_candidate_ids
        .iter()
        .any(|rejected_id| rejected_id == candidate_id)
    {
        "rejected"
    } else if final_accepted_candidate_ids
        .iter()
        .any(|accepted_id| accepted_id == candidate_id)
    {
        "accepted"
    } else if shadowed_candidate_ids
        .iter()
        .any(|shadowed_id| shadowed_id == candidate_id)
    {
        "strong"
    } else if accepted_candidate_ids
        .iter()
        .any(|draft_id| draft_id == candidate_id)
    {
        "warning"
    } else {
        "pending-review"
    }
}

fn rule_backtest_view(backtest_state: &UseStateHandle<ApiState<RuleBacktestResponse>>) -> Html {
    match &**backtest_state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"正在回测所选候选规则..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(backtest) => html! {
            <section class="visual-panel">
                <h4>{"回测证据"}</h4>
                <div class="summary-grid">
                    <div><span>{"匹配"}</span><strong>{format!("{} / {}", backtest.matched_count, backtest.sample_count)}</strong></div>
                    <div><span>{"精准率"}</span><strong>{percent_label(backtest.precision)}</strong></div>
                    <div><span>{"召回率"}</span><strong>{percent_label(backtest.recall)}</strong></div>
                    <div><span>{"误报率"}</span><strong>{percent_label(backtest.false_positive_rate)}</strong></div>
                    <div><span>{"节省金额"}</span><strong>{&backtest.estimated_saving}</strong></div>
                    <div><span>{"建议"}</span><strong>{&backtest.promotion_recommendation}</strong></div>
                </div>
                if !backtest.blockers.is_empty() {
                    <div class="compact-list">
                        {for backtest.blockers.iter().map(|blocker| html! { <span>{blocker}</span> })}
                    </div>
                }
            </section>
        },
    }
}

fn rule_save_view(save_state: &UseStateHandle<ApiState<Value>>) -> Html {
    match &**save_state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"正在保存影子验证候选草稿..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(saved) => {
            let rule_id = response_rule_id(saved).unwrap_or_else(|| "draft rule".into());
            html! {
                <div class="success-note">
                    {format!("已将 {rule_id} 保存为影子验证候选草稿。")}
                </div>
            }
        }
    }
}

fn rule_shadow_run_state(shadow_state: &UseStateHandle<ApiState<Value>>) -> Html {
    match &**shadow_state {
        ApiState::Idle => html! {
            <p class="empty">{"先运行回测，再提交影子证据，之后才能进入发布审核。"}</p>
        },
        ApiState::Loading => html! { <p>{"正在提交规则影子证据..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="success-note">
                <span>{"影子证据已提交到发布门禁。"}</span>
                <pre>{pretty_json(response)}</pre>
            </div>
        },
    }
}

pub(crate) fn rule_candidate_review_state(review_state: &UseStateHandle<ApiState<Value>>) -> Html {
    match &**review_state {
        ApiState::Idle => html! {
            <p class="empty">{"请先运行回测、保存草稿、提交影子证据，然后接受或拒绝所选候选规则。"}</p>
        },
        ApiState::Loading => html! { <p>{"正在提交候选规则审核操作..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="success-note">
                <span>{"候选规则审核操作已受理。"}</span>
                <pre>{pretty_json(response)}</pre>
            </div>
        },
    }
}

fn rule_candidate_name(candidate: &RuleDiscoveryCandidate) -> String {
    if let Some(name) = candidate.rule.get("name").and_then(Value::as_str) {
        name.to_string()
    } else {
        crate::rule_candidate_id(candidate)
    }
}

pub(crate) fn rule_gate_pipeline(gates: &RulePromotionGates) -> Html {
    let nodes = gates
        .gates
        .iter()
        .map(|gate| {
            (
                gate.label.as_str(),
                gate.passed,
                gate.evidence_source.as_str(),
            )
        })
        .collect::<Vec<_>>();
    gate_pipeline("规则发布流程", &nodes)
}

fn gate_pipeline(title: &str, nodes: &[(&str, bool, &str)]) -> Html {
    if nodes.is_empty() {
        return html! {};
    }
    html! {
        <div class="visual-panel pipeline-panel">
            <h4>{title}</h4>
            <div class="gate-pipeline">
                {for nodes.iter().map(|(label, passed, evidence)| html! {
                    <div class={classes!("gate-node", if *passed { "passed" } else { "blocked" })}>
                        <span>{if *passed { "通过" } else { "阻塞" }}</span>
                        <strong>{label}</strong>
                        <small>{evidence}</small>
                    </div>
                })}
            </div>
        </div>
    }
}
