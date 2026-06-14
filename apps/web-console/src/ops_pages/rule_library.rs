use crate::api::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn performance_for<'a>(
    performance: &'a [RulePerformance],
    rule_id: &str,
) -> Option<&'a RulePerformance> {
    performance.iter().find(|p| p.rule_id == rule_id)
}

fn review_mode_badge(review_mode: &str) -> Html {
    let tone = match review_mode.to_ascii_lowercase().as_str() {
        "auto" => "info",
        "manual" => "warning",
        _ => "neutral",
    };
    html! { <span class={classes!("status-token", tone)}>{review_mode}</span> }
}

// ── KPI computations ──────────────────────────────────────────────────────────

fn kpi_active_count(rules: &[RuleSummary]) -> usize {
    rules
        .iter()
        .filter(|r| r.status.eq_ignore_ascii_case("active"))
        .count()
}

fn kpi_total_hits(performance: &[RulePerformance]) -> u32 {
    performance.iter().map(|p| p.trigger_count).sum()
}

fn kpi_avg_precision(performance: &[RulePerformance]) -> f64 {
    if performance.is_empty() {
        return 0.0;
    }
    let sum: f64 = performance.iter().map(|p| p.precision).sum();
    sum / performance.len() as f64
}

fn kpi_pending_count(rules: &[RuleSummary]) -> usize {
    rules
        .iter()
        .filter(|r| r.status.eq_ignore_ascii_case("approved"))
        .count()
}

// ── Sub-views ─────────────────────────────────────────────────────────────────

fn page_header() -> Html {
    html! {
        <div class="dashboard-header">
            <div>
                <h2>{"规则库"}</h2>
                <p class="muted">{"已激活的风险检测规则，以及待纳入的新规则建议"}</p>
            </div>
        </div>
    }
}

fn kpi_strip(rules: &[RuleSummary], performance: &[RulePerformance]) -> Html {
    let active_count = kpi_active_count(rules);
    let total_hits = kpi_total_hits(performance);
    let avg_precision = kpi_avg_precision(performance);
    let pending_count = kpi_pending_count(rules);
    html! {
        <div class="ops-kpi-strip">
            <div class="ops-kpi-card positive">
                <span>{"已激活规则"}</span>
                <strong>{active_count}</strong>
            </div>
            <div class="ops-kpi-card highlight">
                <span>{"本月拦截量"}</span>
                <strong>{total_hits}</strong>
            </div>
            <div class="ops-kpi-card">
                <span>{"平均精准率"}</span>
                <strong>{format!("{:.1}%", avg_precision * 100.0)}</strong>
            </div>
            <div class="ops-kpi-card">
                <span>{"待纳入建议"}</span>
                <strong>{pending_count}</strong>
            </div>
        </div>
    }
}

// ── Section A: rule suggestion cards ─────────────────────────────────────────

fn suggestion_section(
    rules: &[RuleSummary],
    performance: &[RulePerformance],
    on_publish: Callback<String>,
    on_defer: Callback<String>,
    on_decline: Callback<String>,
) -> Html {
    let approved: Vec<&RuleSummary> = rules
        .iter()
        .filter(|r| r.status.eq_ignore_ascii_case("approved"))
        .collect();

    html! {
        <section class="rule-library-section">
            <h3 class="ops-section-title">{"新规则建议"}</h3>
            { if approved.is_empty() {
                html! {
                    <div class="alert-card info">
                        <strong>{"暂无新规则建议"}</strong>
                        <span>{"下次推送将在月底"}</span>
                    </div>
                }
            } else {
                html! {
                    <div class="rule-suggestion-list">
                        { for approved.iter().map(|rule| {
                            let perf = performance_for(performance, &rule.rule_id);
                            let precision_pct = perf.map(|p| p.precision * 100.0).unwrap_or(0.0);
                            let rule_id_publish = rule.rule_id.clone();
                            let rule_id_defer   = rule.rule_id.clone();
                            let rule_id_decline = rule.rule_id.clone();
                            let on_publish  = on_publish.clone();
                            let on_defer    = on_defer.clone();
                            let on_decline  = on_decline.clone();
                            html! {
                                <div class="rule-suggestion-card">
                                    <div class="rule-suggestion-badge">{"待纳入"}</div>
                                    <h3>{ &rule.name }</h3>
                                    <p class="muted">{ &rule.recommended_action }</p>
                                    <div class="rule-meta-row">
                                        <span class="meta-chip">{ &rule.scheme_family }</span>
                                        <span class="meta-chip">{ format!("预估节省 {}", rule.estimated_saving) }</span>
                                        <span class="meta-chip">{ format!("精准率 {:.1}%", precision_pct) }</span>
                                    </div>
                                    <div class="rule-suggestion-actions">
                                        <button
                                            class="btn-primary"
                                            onclick={Callback::from(move |_: MouseEvent| on_publish.emit(rule_id_publish.clone()))}
                                        >
                                            {"纳入规则库"}
                                        </button>
                                        <button
                                            class="btn-secondary"
                                            onclick={Callback::from(move |_: MouseEvent| on_defer.emit(rule_id_defer.clone()))}
                                        >
                                            {"本月暂缓"}
                                        </button>
                                        <button
                                            class="btn-ghost"
                                            onclick={Callback::from(move |_: MouseEvent| on_decline.emit(rule_id_decline.clone()))}
                                        >
                                            {"拒绝"}
                                        </button>
                                    </div>
                                </div>
                            }
                        }) }
                    </div>
                }
            } }
        </section>
    }
}

// ── Section B: active rule cards ──────────────────────────────────────────────

fn active_section(rules: &[RuleSummary], performance: &[RulePerformance]) -> Html {
    let active: Vec<&RuleSummary> = rules
        .iter()
        .filter(|r| r.status.eq_ignore_ascii_case("active"))
        .collect();
    let count = active.len();

    html! {
        <section class="rule-library-section">
            <h3 class="ops-section-title">{ format!("已激活规则 ({})", count) }</h3>
            { if active.is_empty() {
                html! {
                    <div class="alert-card">
                        <strong>{"暂无已激活规则"}</strong>
                        <span>{"请先纳入规则建议"}</span>
                    </div>
                }
            } else {
                html! {
                    <div class="rule-active-list">
                        { for active.iter().map(|rule| {
                            let perf = performance_for(performance, &rule.rule_id);
                            let hit_count          = perf.map(|p| p.trigger_count).unwrap_or(0);
                            let confirmed_fwa      = perf.map(|p| p.confirmed_fwa_count).unwrap_or(0);
                            let precision_pct      = perf.map(|p| p.precision * 100.0).unwrap_or(0.0);
                            html! {
                                <div class="rule-active-card">
                                    <div class="rule-active-left">
                                        <strong>{ &rule.name }</strong>
                                        <span class="muted">{ &rule.scheme_family }</span>
                                    </div>
                                    <div class="rule-active-stats">
                                        <span class="meta-chip">
                                            { format!("命中 {}", hit_count) }
                                        </span>
                                        <span class="meta-chip">
                                            { format!("确认FWA {}", confirmed_fwa) }
                                        </span>
                                        <span class="meta-chip">
                                            { format!("{:.1}%", precision_pct) }
                                        </span>
                                        { review_mode_badge(&rule.review_mode) }
                                    </div>
                                    <div class="rule-active-right">
                                        <span class="status-token ok">{"✓ 活跃"}</span>
                                    </div>
                                </div>
                            }
                        }) }
                    </div>
                }
            } }
        </section>
    }
}

// ── Main component ────────────────────────────────────────────────────────────

#[function_component(RuleLibraryPage)]
pub fn rule_library_page() -> Html {
    let api_key = use_api_key();
    let snapshot_state = use_state(|| ApiState::<RuleOpsSnapshot>::Idle);
    let action_state = use_state(|| ApiState::<serde_json::Value>::Idle);
    let confirm_msg = use_state(|| Option::<String>::None);

    // Auto-load on mount
    {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        use_effect_with((), move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_rule_ops_snapshot(api_key, String::new()).await {
                    Ok(s) => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
            || ()
        });
    }

    // Factory: send a lifecycle action for a rule_id
    let do_action = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let action_state = action_state.clone();
        let confirm_msg = confirm_msg.clone();
        move |rule_id: String, action: &'static str, label: &'static str| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let action_state = action_state.clone();
            let confirm_msg = confirm_msg.clone();
            action_state.set(ApiState::Loading);
            confirm_msg.set(None);
            spawn_local(async move {
                match post_rule_lifecycle(api_key.clone(), rule_id.clone(), action, vec![]).await {
                    Ok(v) => {
                        action_state.set(ApiState::Ready(v));
                        confirm_msg.set(Some(format!("规则 {} 已{}。", rule_id, label)));
                        // Refresh snapshot
                        snapshot_state.set(ApiState::Loading);
                        snapshot_state.set(
                            match get_rule_ops_snapshot(api_key, String::new()).await {
                                Ok(s) => ApiState::Ready(s),
                                Err(e) => ApiState::Failed(e),
                            },
                        );
                    }
                    Err(e) => action_state.set(ApiState::Failed(e)),
                }
            });
        }
    };

    let on_publish = {
        let do_action = do_action.clone();
        Callback::from(move |rule_id: String| do_action(rule_id, "publish", "纳入"))
    };
    let on_defer = {
        let do_action = do_action.clone();
        Callback::from(move |rule_id: String| do_action(rule_id, "defer", "暂缓"))
    };
    let on_decline = {
        let do_action = do_action.clone();
        Callback::from(move |rule_id: String| do_action(rule_id, "decline", "拒绝"))
    };

    html! {
        <div class="ops-page rule-library-page">
            { page_header() }

            // ── Action feedback banners ───────────────────────────────────────
            { if let Some(msg) = &*confirm_msg {
                html! {
                    <div class="alert-card info">
                        <strong>{"操作成功"}</strong>
                        <span>{ msg }</span>
                    </div>
                }
            } else {
                html! {}
            } }
            { if let ApiState::Failed(err) = &*action_state {
                html! {
                    <div class="alert-card critical">
                        <strong>{"操作失败"}</strong>
                        <span>{ err }</span>
                    </div>
                }
            } else {
                html! {}
            } }

            // ── Data states ───────────────────────────────────────────────────
            { match &*snapshot_state {
                ApiState::Idle | ApiState::Loading => html! {
                    <p class="empty">{"加载中..."}</p>
                },
                ApiState::Failed(err) => html! {
                    <div class="alert-card critical">
                        <strong>{"加载失败"}</strong>
                        <span>{ err }</span>
                    </div>
                },
                ApiState::Ready(snap) => html! {
                    <>
                        { kpi_strip(&snap.rules, &snap.performance) }
                        { suggestion_section(
                            &snap.rules,
                            &snap.performance,
                            on_publish,
                            on_defer,
                            on_decline,
                        ) }
                        { active_section(&snap.rules, &snap.performance) }
                    </>
                },
            } }
        </div>
    }
}
