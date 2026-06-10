use crate::api::*;
use crate::types::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::percent_label;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;

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

/// Parse a currency string like "CNY 1234.56" or "1234.56" → f64
fn parse_amount(s: &str) -> f64 {
    s.split_whitespace()
        .last()
        .and_then(|v| v.replace(',', "").parse::<f64>().ok())
        .unwrap_or(0.0)
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

fn rag_distribution_section(summary: &DashboardSummary) -> Html {
    let dist = &summary.rag_distribution;
    let total: u32 = dist.values().sum();
    let total_f = total.max(1) as f64;

    let rows: Vec<(&'static str, &'static str, u32)> = vec![
        ("Red",   "red",   *dist.get("Red").unwrap_or(&0)),
        ("Amber", "amber", *dist.get("Amber").unwrap_or(&0)),
        ("Green", "green", *dist.get("Green").unwrap_or(&0)),
    ];

    html! {
        <div class="rag-bar-section">
            <p class="ops-section-label">{"今日风险分布"}</p>
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

fn value_proof_section(summary: &DashboardSummary) -> Html {
    let vm = &summary.value_measurement;
    let prevented = parse_amount(&vm.prevented_payment);
    let cost = parse_amount(&vm.review_cost);
    let roi_text = if cost > 0.0 {
        format!("每投入1元审核成本，防赔{:.1}元", prevented / cost)
    } else {
        "每投入1元审核成本，防赔 — 元".to_string()
    };

    html! {
        <div class="ops-value-proof">
            <div class="ops-dashboard-grid" style="grid-template-columns:1fr 1fr;">
                <div class="ops-kpi-card success">
                    <span class="ops-kpi-label">{"本月防赔累计"}</span>
                    <strong class="ops-kpi-value">{&vm.prevented_payment}</strong>
                </div>
                <div class="ops-kpi-card neutral">
                    <span class="ops-kpi-label">{"审核投入成本"}</span>
                    <strong class="ops-kpi-value">{&vm.review_cost}</strong>
                </div>
            </div>
            <p class="ops-roi-note muted">{roi_text}</p>
        </div>
    }
}

fn dashboard_body(summary: &DashboardSummary) -> Html {
    let precision     = summary.rule_governance.precision;
    let sla_rate      = 1.0 - summary.case_sla.sla_breach_rate;
    let sla_label     = format!("{:.1}%", sla_rate * 100.0);
    let prec_label    = percent_label(precision);
    let prec_tone     = precision_tone(precision);

    html! {
        <>
            // ── Row 1 KPIs ──────────────────────────────────────────────
            <div class="ops-dashboard-grid">
                { kpi_card("今日进件",        summary.suspected_claims, "neutral") }
                { kpi_card("已拦截 (高风险)", summary.confirmed_fwa,    "danger")  }
                { kpi_card("防赔金额",        &summary.saving_amount,   "success") }
            </div>

            // ── Row 2 KPIs ──────────────────────────────────────────────
            <div class="ops-dashboard-grid">
                { kpi_card("规则命中", summary.rule_hits, "neutral")   }
                { kpi_card("精准率",   prec_label,        prec_tone)   }
                { kpi_card("SLA 达标率", sla_label,       "neutral")   }
            </div>

            // ── Risk distribution ────────────────────────────────────────
            { rag_distribution_section(summary) }

            // ── Value proof ──────────────────────────────────────────────
            { value_proof_section(summary) }
        </>
    }
}

// ── Main component ────────────────────────────────────────────────────────────

#[function_component(OpsDashboardPage)]
pub fn ops_dashboard_page() -> Html {
    let api_key      = use_api_key();
    let summary_state = use_state(|| ApiState::<DashboardSummary>::Idle);

    // Auto-load on mount
    {
        let api_key       = api_key.clone();
        let summary_state = summary_state.clone();
        use_effect_with((), move |_| {
            let api_key       = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_dashboard_summary(api_key).await {
                    Ok(s)  => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
            || ()
        });
    }

    // Refresh callback
    let refresh = {
        let api_key       = api_key.clone();
        let summary_state = summary_state.clone();
        Callback::from(move |_: MouseEvent| {
            let api_key       = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_dashboard_summary(api_key).await {
                    Ok(s)  => ApiState::Ready(s),
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
                    <h2>{"运营仪表盘"}</h2>
                    <p class="muted">{ today_label() }</p>
                </div>
                <button onclick={refresh} disabled={loading}>
                    {if loading { "刷新中..." } else { "刷新" }}
                </button>
            </div>

            // ── Body ─────────────────────────────────────────────────────
            { match &*summary_state {
                ApiState::Idle | ApiState::Loading => html! {
                    <p class="empty">{"加载中..."}</p>
                },
                ApiState::Failed(err) => html! {
                    <p class="empty">{format!("加载失败：{err}")}</p>
                },
                ApiState::Ready(summary) => dashboard_body(summary),
            } }

        </div>
    }
}
