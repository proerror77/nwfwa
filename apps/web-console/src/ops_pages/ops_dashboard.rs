use crate::api::*;
use crate::formatting::percent_label;
use crate::i18n::tr;
use crate::state::Language;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn today_label() -> String {
    // Date is injected at build time via JS Date; fall back to a static label
    // when the JS glue is unavailable (e.g. unit tests).
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" {
            type Date;
            #[wasm_bindgen(constructor)]
            fn new() -> Date;
            #[wasm_bindgen(method, js_name = toLocaleDateString)]
            fn to_locale_date_string(this: &Date, locale: &str) -> String;
        }
        Date::new().to_locale_date_string("zh-CN")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "今日".to_string()
    }
}

fn precision_tone(precision: f64) -> &'static str {
    if precision > 0.70 {
        "success"
    } else if precision >= 0.40 {
        "warning"
    } else {
        "danger"
    }
}

// ── Sub-views ─────────────────────────────────────────────────────────────────

fn kpi_card(label: &str, value: impl std::fmt::Display, tone: &'static str) -> Html {
    html! {
        <div class={classes!("ops-kpi-card", tone)}>
            <span class="ops-kpi-label">{label}</span>
            <strong class="ops-kpi-value">{value.to_string()}</strong>
        </div>
    }
}

fn rag_distribution_section(summary: &DashboardSummary, language: Language) -> Html {
    let dist = &summary.rag_distribution;
    let total: u32 = dist.values().sum();
    let total_f = total.max(1) as f64;

    let rows: Vec<(&'static str, &'static str, u32)> = vec![
        ("Red", "red", *dist.get("Red").unwrap_or(&0)),
        ("Amber", "amber", *dist.get("Amber").unwrap_or(&0)),
        ("Green", "green", *dist.get("Green").unwrap_or(&0)),
    ];

    html! {
        <div class="rag-bar-section">
            <p class="ops-section-label">{tr(language, "Today's risk distribution", "今日风险分布")}</p>
            { for rows.iter().map(|(label, tone, count)| {
                let pct = (*count as f64 / total_f * 100.0).clamp(0.0, 100.0);
                let width = format!("{pct:.0}%");
                let tone: &'static str = tone;
                html! {
                    <div class={classes!("rag-bar-row", tone)}>
                        <span class="rag-bar-label">{*label}</span>
                        <div class="rag-bar-track">
                            <div
                                class={classes!("rag-bar-fill", tone)}
                                style={format!("width:{width}")}
                            />
                        </div>
                        <span class="rag-bar-count">{count}</span>
                    </div>
                }
            }) }
        </div>
    }
}

fn operational_watch_section(summary: &DashboardSummary, language: Language) -> Html {
    let breached = summary.case_sla.breached_cases;
    let open_cases = summary.case_sla.open_cases;
    let qa_open = summary.qa_queue.open_cases + summary.qa_queue.unresolved_feedback_count;
    let drift_count = summary.model_governance.drift_detected_count;
    let denied_policy = summary.agent_governance.denied_policy_check_count;

    html! {
        <div class="ops-dashboard-grid">
            <div class="ops-kpi-card warning">
                <span class="ops-kpi-label">{tr(language, "SLA attention needed", "SLA 需要关注")}</span>
                <strong class="ops-kpi-value">{format!("{breached}/{open_cases}")}</strong>
                <small class="ops-kpi-note">{tr(language, "Prioritize breached or near-breach investigation cases", "优先处理超时或即将超时的调查案件")}</small>
            </div>
            <div class="ops-kpi-card neutral">
                <span class="ops-kpi-label">{tr(language, "QA / feedback open", "QA / 反馈待闭环")}</span>
                <strong class="ops-kpi-value">{qa_open}</strong>
                <small class="ops-kpi-note">{tr(language, "Use this to decide whether rules, models, or workflow need correction", "用于判断规则、模型或流程是否需要修正")}</small>
            </div>
            <div class="ops-kpi-card danger">
                <span class="ops-kpi-label">{tr(language, "Governance exceptions", "治理异常")}</span>
                <strong class="ops-kpi-value">{drift_count + denied_policy}</strong>
                <small class="ops-kpi-note">{tr(language, "Model drift or agent policy denials need second-line review", "模型漂移或 Agent policy deny 需要二线确认")}</small>
            </div>
        </div>
    }
}

fn governance_todo_section(summary: &DashboardSummary, language: Language) -> Html {
    html! {
        <div class="ops-dashboard-grid">
            <div class="ops-kpi-card warning">
                <span class="ops-kpi-label">{tr(language, "QA samples open", "待质控样本")}</span>
                <strong class="ops-kpi-value">{summary.qa_queue.open_cases}</strong>
                <small class="ops-kpi-note">{tr(language, "Route into QA sampling or reviewer disagreement handling", "进入 QA 抽样或复核分歧处理")}</small>
            </div>
            <div class="ops-kpi-card neutral">
                <span class="ops-kpi-label">{tr(language, "Labels needing review", "待审核标签")}</span>
                <strong class="ops-kpi-value">{summary.label_pool.needs_review}</strong>
                <small class="ops-kpi-note">{tr(language, "Do not feed training or rule feedback before approval", "不得直接进入训练或规则回流")}</small>
            </div>
            <div class="ops-kpi-card success">
                <span class="ops-kpi-label">{tr(language, "Audit coverage", "审计覆盖率")}</span>
                <strong class="ops-kpi-value">{percent_label(summary.audit_coverage.canonical_trace_coverage)}</strong>
                <small class="ops-kpi-note">{tr(language, "Trace coverage for the scoring path", "评分链路可追踪覆盖情况")}</small>
            </div>
        </div>
    }
}

fn next_action_section(summary: &DashboardSummary, language: Language) -> Html {
    let red_count = summary.rag_distribution.get("Red").copied().unwrap_or(0);
    let first = match language {
        Language::En => format!("Start with {red_count} Red-risk claims: decide whether to request evidence, route to investigation, or archive low-risk items."),
        Language::Zh => format!("先处理 {red_count} 个 Red 风险进件，判断补件、转调查或低风险归档。"),
    };
    let second = match language {
        Language::En => format!(
            "Then work {count} open investigation cases, sorted by SLA and priority.",
            count = summary.case_sla.open_cases
        ),
        Language::Zh => format!(
            "再处理 {} 个开放调查案件，优先按 SLA 和优先级排序。",
            summary.case_sla.open_cases
        ),
    };
    let third = match language {
        Language::En => format!("Finally close {count} QA/governance feedback items so rules and models do not amplify errors.", count = summary.qa_queue.unresolved_feedback_count),
        Language::Zh => format!("最后闭环 {} 条 QA/治理反馈，避免规则和模型继续放大误差。", summary.qa_queue.unresolved_feedback_count),
    };
    html! {
        <div class="ops-command-panel">
            <div>
                <p class="ops-section-label">{tr(language, "Today's work order", "今日处理顺序")}</p>
                <ol>
                    <li>{first}</li>
                    <li>{second}</li>
                    <li>{third}</li>
                </ol>
            </div>
            <div>
                <p class="ops-section-label">{tr(language, "Dashboard boundary", "看板边界")}</p>
                <p class="muted">{tr(language, "The dashboard only answers where humans should work today. Triage belongs in Claims Queue, evidence judgment belongs in Investigation Workbench, and configuration changes belong in Quality & Governance.", "运营仪表盘只回答今天哪里需要人处理；进件分流去理赔队列，证据判断去调查工作台，配置变更去质控与治理。")}</p>
            </div>
        </div>
    }
}

fn dashboard_body(summary: &DashboardSummary, language: Language) -> Html {
    let precision = summary.rule_governance.precision;
    let sla_rate = 1.0 - summary.case_sla.sla_breach_rate;
    let sla_label = format!("{:.1}%", sla_rate * 100.0);
    let prec_label = percent_label(precision);
    let prec_tone = precision_tone(precision);

    html! {
        <>
            // ── Row 1 KPIs ──────────────────────────────────────────────
            <div class="ops-dashboard-grid">
                { kpi_card(tr(language, "Claims today", "今日进件"), summary.suspected_claims, "neutral") }
                { kpi_card(tr(language, "Red-risk claims", "Red 风险进件"),    summary.rag_distribution.get("Red").copied().unwrap_or(0), "danger")  }
                { kpi_card(tr(language, "Open investigations", "开放调查"),        summary.case_sla.open_cases, "warning") }
            </div>

            // ── Row 2 KPIs ──────────────────────────────────────────────
            <div class="ops-dashboard-grid">
            { kpi_card(tr(language, "Rule hits", "规则命中"), summary.rule_hits, "neutral")   }
            { kpi_card(tr(language, "Precision", "精准率"),   prec_label,        prec_tone)   }
            { kpi_card(tr(language, "SLA compliance", "SLA 达标率"), sla_label,       "neutral")   }
            </div>

            // ── Operational watchlist ───────────────────────────────────
            { operational_watch_section(summary, language) }

            // ── Governance todo ──────────────────────────────────────────
            { governance_todo_section(summary, language) }

            // ── Next actions ────────────────────────────────────────────
            { next_action_section(summary, language) }

            // ── Risk distribution ────────────────────────────────────────
            { rag_distribution_section(summary, language) }
        </>
    }
}

// ── Main component ────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct OpsDashboardPageProps {
    pub language: Language,
}

#[function_component(OpsDashboardPage)]
pub fn ops_dashboard_page(props: &OpsDashboardPageProps) -> Html {
    let api_key = use_api_key();
    let summary_state = use_state(|| ApiState::<DashboardSummary>::Idle);

    // Auto-load on mount
    {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        use_effect_with((), move |_| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_dashboard_summary(api_key).await {
                    Ok(s) => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
            || ()
        });
    }

    // Refresh callback
    let refresh = {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        Callback::from(move |_: MouseEvent| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_dashboard_summary(api_key).await {
                    Ok(s) => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
        })
    };

    let loading = matches!(&*summary_state, ApiState::Loading);

    html! {
        <div class="ops-page ops-dashboard-page">

            // ── Header ───────────────────────────────────────────────────
            <div class="ops-page-header">
                <div>
                    <h2>{tr(props.language, "Operations Dashboard", "运营仪表盘")}</h2>
                    <p class="muted">{match props.language {
                        Language::En => format!("{} · Daily priority, SLA risk, queue load, and governance exceptions", today_label()),
                        Language::Zh => format!("{} · 看今日优先级、SLA 风险、队列负载与治理异常", today_label()),
                    }}</p>
                </div>
                <button onclick={refresh} disabled={loading}>
                    {if loading { tr(props.language, "Refreshing...", "刷新中...") } else { tr(props.language, "Refresh", "刷新") }}
                </button>
            </div>

            // ── Body ─────────────────────────────────────────────────────
            { match &*summary_state {
                ApiState::Idle | ApiState::Loading => html! {
                    <p class="empty">{tr(props.language, "Loading...", "加载中...")}</p>
                },
                ApiState::Failed(err) => html! {
                    <p class="empty">{match props.language {
                        Language::En => format!("Load failed: {err}"),
                        Language::Zh => format!("加载失败：{err}"),
                    }}</p>
                },
                ApiState::Ready(summary) => dashboard_body(summary, props.language),
            } }

        </div>
    }
}
