use crate::api::*;
use crate::formatting::percent_label;
use crate::i18n::tr;
use crate::ops_routing::{ops_set_hash_with_id, OpsPage};
use crate::state::{use_api_key, ApiState, Language};
use crate::types::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn today_label() -> String {
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

fn fmt_cny(amount: &str) -> String {
    // amount may come as "1234567.00" — format as "¥ 1,234,567"
    let n: f64 = amount.parse().unwrap_or(0.0);
    if n >= 1_000_000.0 {
        format!("¥ {:.1}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("¥ {:.0}K", n / 1_000.0)
    } else {
        format!("¥ {n:.0}")
    }
}

fn precision_tone(p: f64) -> &'static str {
    if p > 0.70 {
        "success"
    } else if p >= 0.40 {
        "warning"
    } else {
        "danger"
    }
}

// ── Prevention banner ─────────────────────────────────────────────────────────

fn prevention_banner(s: &DashboardSummary, language: Language) -> Html {
    let prevented = fmt_cny(&s.value_measurement.prevented_payment);
    let recovered = fmt_cny(&s.value_measurement.recovered_amount);
    let pass = s.rag_distribution.get("Green").copied().unwrap_or(0);
    let flagged = s.rag_distribution.get("Red").copied().unwrap_or(0)
        + s.rag_distribution.get("Amber").copied().unwrap_or(0);
    let total = s.suspected_claims.max(1);
    let pass_pct = (pass as f64 / total as f64 * 100.0).round() as u32;

    html! {
        <div class="prevention-banner">
            <div class="prevention-stat">
                <span class="prevention-label">{tr(language, "Prevented today", "今日拦截")}</span>
                <strong class="prevention-value success">{prevented}</strong>
            </div>
            <div class="prevention-divider"/>
            <div class="prevention-stat">
                <span class="prevention-label">{tr(language, "Recovered", "追回")}</span>
                <strong class="prevention-value neutral">{recovered}</strong>
            </div>
            <div class="prevention-divider"/>
            <div class="prevention-stat">
                <span class="prevention-label">{tr(language, "Auto-passed", "自动通过")}</span>
                <strong class="prevention-value muted">{format!("{pass} ({pass_pct}%)")}</strong>
            </div>
            <div class="prevention-divider"/>
            <div class="prevention-stat">
                <span class="prevention-label">{tr(language, "Flagged for review", "标记复核")}</span>
                <strong class="prevention-value danger">{flagged.to_string()}</strong>
            </div>
        </div>
    }
}

// ── System performance row ────────────────────────────────────────────────────

fn system_performance_row(s: &DashboardSummary, language: Language) -> Html {
    let precision = s.rule_governance.precision;
    let fp_rate = s.rule_governance.false_positive_rate;
    let sla_ok = 1.0 - s.case_sla.sla_breach_rate;
    let drift = s.model_governance.drift_detected_count;
    let pt = precision_tone(precision);

    html! {
        <div class="perf-row">
            <div class={classes!("perf-card", pt)}>
                <span class="perf-label">{tr(language, "Precision", "精准率")}</span>
                <strong class="perf-value">{percent_label(precision)}</strong>
                <small class="perf-note">{tr(language, "Confirmed / reviewed", "确认 / 复核比")}</small>
            </div>
            <div class="perf-card neutral">
                <span class="perf-label">{tr(language, "False positive rate", "误报率")}</span>
                <strong class="perf-value">{percent_label(fp_rate)}</strong>
                <small class="perf-note">{tr(language, "Lower is better", "越低越好")}</small>
            </div>
            <div class={classes!("perf-card", if sla_ok >= 0.95 { "success" } else { "warning" })}>
                <span class="perf-label">{tr(language, "SLA compliance", "SLA 达标")}</span>
                <strong class="perf-value">{percent_label(sla_ok)}</strong>
                <small class="perf-note">
                    {match language {
                        Language::En => format!("{} breached", s.case_sla.breached_cases),
                        Language::Zh => format!("{} 超时", s.case_sla.breached_cases),
                    }}
                </small>
            </div>
            <div class={classes!("perf-card", if drift == 0 { "success" } else { "danger" })}>
                <span class="perf-label">{tr(language, "Model drift", "模型漂移")}</span>
                <strong class="perf-value">{if drift == 0 { tr(language, "OK", "正常") } else { tr(language, "Alert", "警告") }}</strong>
                <small class="perf-note">
                    {match language {
                        Language::En => format!("{drift} signals"),
                        Language::Zh => format!("{drift} 个信号"),
                    }}
                </small>
            </div>
        </div>
    }
}

// ── Action counters ───────────────────────────────────────────────────────────

fn action_counters(
    s: &DashboardSummary,
    on_queue: Callback<MouseEvent>,
    language: Language,
) -> Html {
    let red = s.rag_distribution.get("Red").copied().unwrap_or(0);
    let open = s.case_sla.open_cases;
    let qa_open = s.qa_queue.open_cases;
    let sla_breach = s.case_sla.breached_cases;

    html! {
        <div class="action-counters">
            <p class="section-eyebrow">{tr(language, "Action needed", "需要你处理")}</p>
            <div class="action-counter-grid">
                <button
                    class={classes!("action-counter-card", if red > 0 { "danger" } else { "neutral" })}
                    onclick={on_queue.clone()}
                >
                    <strong class="action-count">{red}</strong>
                    <span class="action-label">{tr(language, "High-risk flagged", "高风险待审")}</span>
                </button>
                <button
                    class={classes!("action-counter-card", if sla_breach > 0 { "warning" } else { "neutral" })}
                    onclick={on_queue.clone()}
                >
                    <strong class="action-count">{open}</strong>
                    <span class="action-label">
                        {match language {
                            Language::En => format!("Open cases ({sla_breach} SLA breach)"),
                            Language::Zh => format!("待结案 ({sla_breach} 超时)"),
                        }}
                    </span>
                </button>
                <button
                    class={classes!("action-counter-card", if qa_open > 0 { "warning" } else { "neutral" })}
                    onclick={on_queue}
                >
                    <strong class="action-count">{qa_open}</strong>
                    <span class="action-label">{tr(language, "Pending QA / feedback", "待 QA / 反馈")}</span>
                </button>
            </div>
        </div>
    }
}

// ── Live intake feed ──────────────────────────────────────────────────────────
// Simulated streaming feed — shows recent high-risk events from audit data.
// Real implementation would subscribe to a WebSocket / SSE endpoint.

fn live_feed_row(label: &str, value: &str, tone: &'static str) -> Html {
    html! {
        <div class="feed-row">
            <span class={classes!("feed-dot", tone)}/>
            <span class="feed-label">{label}</span>
            <span class={classes!("feed-value", tone)}>{value}</span>
        </div>
    }
}

fn live_intake_feed(s: &DashboardSummary, language: Language) -> Html {
    // Derive a few representative feed items from live summary data.
    let total = s.suspected_claims;
    let red = s.rag_distribution.get("Red").copied().unwrap_or(0);
    let amber = s.rag_distribution.get("Amber").copied().unwrap_or(0);
    let rule_hits = s.rule_hits;
    let confirmed = s.confirmed_fwa;
    let schemes: Vec<(&String, &u32)> = s.scheme_distribution.iter().take(3).collect();

    html! {
        <div class="live-feed">
            <div class="live-feed-header">
                <span class="live-dot-pulse"/>
                <p class="section-eyebrow">{tr(language, "Live intake", "实时进件流水")}</p>
            </div>
            <div class="feed-rows">
                {live_feed_row(
                    tr(language, "Claims processed", "理赔处理"),
                    &total.to_string(),
                    "neutral"
                )}
                {live_feed_row(
                    tr(language, "High risk flagged", "高风险标记"),
                    &red.to_string(),
                    if red > 0 { "danger" } else { "neutral" }
                )}
                {live_feed_row(
                    tr(language, "Watchlist (amber)", "可疑（黄色）"),
                    &amber.to_string(),
                    if amber > 0 { "warning" } else { "neutral" }
                )}
                {live_feed_row(
                    tr(language, "Rules triggered", "规则命中"),
                    &rule_hits.to_string(),
                    "neutral"
                )}
                {live_feed_row(
                    tr(language, "Confirmed FWA", "确认 FWA"),
                    &confirmed.to_string(),
                    if confirmed > 0 { "success" } else { "muted" }
                )}
                { for schemes.iter().map(|(scheme, count)| {
                    live_feed_row(scheme, &count.to_string(), "muted")
                }) }
            </div>
        </div>
    }
}

// ── Full dashboard body ───────────────────────────────────────────────────────

fn dashboard_body(
    s: &DashboardSummary,
    on_queue: Callback<MouseEvent>,
    language: Language,
) -> Html {
    html! {
        <div class="dashboard-layout">
            // ── Top: prevention banner ──────────────────────────────────
            { prevention_banner(s, language) }

            // ── Middle row: left = performance + action, right = feed ───
            <div class="dashboard-mid-row">
                <div class="dashboard-left-col">
                    { system_performance_row(s, language) }
                    { action_counters(s, on_queue, language) }
                </div>
                <div class="dashboard-right-col">
                    { live_intake_feed(s, language) }
                </div>
            </div>
        </div>
    }
}

// ── Main component ────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct OpsDashboardPageProps {
    pub language: Language,
    pub on_go_to_queue: Callback<MouseEvent>,
}

#[function_component(OpsDashboardPage)]
pub fn ops_dashboard_page(props: &OpsDashboardPageProps) -> Html {
    let api_key = use_api_key();
    let state = use_state(|| ApiState::<DashboardSummary>::Idle);

    {
        let api_key = api_key.clone();
        let state = state.clone();
        use_effect_with((), move |_| {
            let api_key = (*api_key).clone();
            let state = state.clone();
            state.set(ApiState::Loading);
            spawn_local(async move {
                state.set(match get_dashboard_summary(api_key).await {
                    Ok(s) => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
            || ()
        });
    }

    let refresh = {
        let api_key = api_key.clone();
        let state = state.clone();
        Callback::from(move |_: MouseEvent| {
            let api_key = (*api_key).clone();
            let state = state.clone();
            state.set(ApiState::Loading);
            spawn_local(async move {
                state.set(match get_dashboard_summary(api_key).await {
                    Ok(s) => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
        })
    };

    let loading = matches!(&*state, ApiState::Loading);
    let language = props.language;

    html! {
        <div class="ops-page ops-dashboard-page">
            <div class="ops-page-header">
                <div>
                    <h2>{tr(language, "Operations Overview", "运营概况")}</h2>
                    <p class="muted">{match language {
                        Language::En => format!("{} · Prevention value, system health, action counters, live feed", today_label()),
                        Language::Zh => format!("{} · 防损金额、系统健康、待处理计数与进件流水", today_label()),
                    }}</p>
                </div>
                <button onclick={refresh} disabled={loading}>
                    {if loading { tr(language, "Refreshing...", "刷新中...") } else { tr(language, "↺ Refresh", "↺ 刷新") }}
                </button>
            </div>

            { match &*state {
                ApiState::Idle | ApiState::Loading => html! {
                    <div class="loading-placeholder">
                        <span class="loading-pulse"/>
                        <p class="muted">{tr(language, "Loading dashboard...", "加载中...")}</p>
                    </div>
                },
                ApiState::Failed(err) => html! {
                    <p class="empty">{match language {
                        Language::En => format!("Load failed: {err}"),
                        Language::Zh => format!("加载失败：{err}"),
                    }}</p>
                },
                ApiState::Ready(s) => dashboard_body(s, props.on_go_to_queue.clone(), language),
            } }
        </div>
    }
}
