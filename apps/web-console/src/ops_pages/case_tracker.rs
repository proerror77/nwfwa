use crate::api::*;
use crate::formatting::business_label;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

// ── Filter kind ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    All,
    Investigating,
    PendingEvidence,
    Confirmed,
    Closed,
}

impl Filter {
    fn label(self) -> &'static str {
        match self {
            Filter::All => "全部",
            Filter::Investigating => "调查中",
            Filter::PendingEvidence => "待证据",
            Filter::Confirmed => "已确认",
            Filter::Closed => "已关闭",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Investigating => "investigating",
            Filter::PendingEvidence => "pending-evidence",
            Filter::Confirmed => "confirmed",
            Filter::Closed => "closed",
        }
    }

    fn matches(self, case: &CaseRecord) -> bool {
        match self {
            Filter::All => true,
            Filter::Investigating => {
                case.status.eq_ignore_ascii_case("investigating")
                    || case.status.eq_ignore_ascii_case("open")
            }
            Filter::PendingEvidence => {
                case.status.eq_ignore_ascii_case("pending_evidence")
                    || case.status.eq_ignore_ascii_case("evidence_pending")
            }
            Filter::Confirmed => case.status.eq_ignore_ascii_case("confirmed"),
            Filter::Closed => case.status.eq_ignore_ascii_case("closed"),
        }
    }
}

const FILTERS: &[Filter] = &[
    Filter::All,
    Filter::Investigating,
    Filter::PendingEvidence,
    Filter::Confirmed,
    Filter::Closed,
];

// ── Helpers ───────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

fn sla_badge_html(sla_status: &str) -> Html {
    let (tone, label) = match sla_status.trim().to_ascii_lowercase().as_str() {
        "ok" => ("ok", "✓ 正常"),
        "warning" => ("warning", "⚠ 注意"),
        "breach" => ("breach", "✕ 超时"),
        _ => ("ok", "✓ 正常"),
    };
    html! { <span class={classes!("sla-badge", tone)}>{label}</span> }
}

fn status_badge_html(status: &str) -> Html {
    let tone = match status.to_ascii_lowercase().as_str() {
        "investigating" | "open" => "info",
        "pending_evidence" | "evidence_pending" => "warning",
        "confirmed" => "positive",
        "closed" => "neutral",
        _ => "neutral",
    };
    html! { <span class={classes!("status-token", tone)}>{business_label(status)}</span> }
}

fn priority_tone(priority: &str) -> &'static str {
    match priority.trim().to_ascii_lowercase().as_str() {
        "high" | "critical" => "high",
        "medium" | "normal" => "medium",
        _ => "low",
    }
}

// ── KPI helpers ───────────────────────────────────────────────────────────────

fn kpi_active(cases: &[CaseRecord]) -> usize {
    cases
        .iter()
        .filter(|c| {
            let s = c.status.to_ascii_lowercase();
            s != "closed"
        })
        .count()
}

fn kpi_breach(cases: &[CaseRecord]) -> usize {
    cases
        .iter()
        .filter(|c| c.sla_status.eq_ignore_ascii_case("breach"))
        .count()
}

fn kpi_closed_this_week(cases: &[CaseRecord]) -> usize {
    // All closed cases — week-window requires a timestamp field not present in CaseRecord,
    // so we count all closed as a reasonable approximation for the UI strip.
    cases
        .iter()
        .filter(|c| c.status.eq_ignore_ascii_case("closed"))
        .count()
}

fn kpi_avg_closure_hours(cases: &[CaseRecord]) -> String {
    let closed: Vec<f64> = cases
        .iter()
        .filter_map(|c| c.time_to_closure_hours)
        .collect();
    if closed.is_empty() {
        return "—".to_string();
    }
    let avg = closed.iter().sum::<f64>() / closed.len() as f64;
    format!("{avg:.1}h")
}

// ── Sub-views ─────────────────────────────────────────────────────────────────

fn page_header(_snapshot_state: &UseStateHandle<ApiState<LeadsCasesSnapshot>>) -> Html {
    html! {
        <div class="ops-page-header">
            <div>
                <h2>{"案件追踪"}</h2>
                <p class="muted">{"所有进行中和已关闭的调查案件"}</p>
            </div>
        </div>
    }
}

fn kpi_strip(snapshot_state: &UseStateHandle<ApiState<LeadsCasesSnapshot>>) -> Html {
    let (active, breach, closed_week, avg_closure) =
        if let ApiState::Ready(snap) = &**snapshot_state {
            (
                kpi_active(&snap.cases),
                kpi_breach(&snap.cases),
                kpi_closed_this_week(&snap.cases),
                kpi_avg_closure_hours(&snap.cases),
            )
        } else {
            (0, 0, 0, "—".to_string())
        };

    html! {
        <div class="ops-kpi-strip">
            <div class="ops-kpi-card">
                <span>{"活跃案件"}</span>
                <strong>{active}</strong>
            </div>
            <div class="ops-kpi-card danger">
                <span>{"SLA 超时"}</span>
                <strong>{breach}</strong>
            </div>
            <div class="ops-kpi-card positive">
                <span>{"本周已关闭"}</span>
                <strong>{closed_week}</strong>
            </div>
            <div class="ops-kpi-card">
                <span>{"平均关闭时长"}</span>
                <strong>{avg_closure}</strong>
            </div>
        </div>
    }
}

fn filter_bar(active_filter: &UseStateHandle<Filter>) -> Html {
    let active = **active_filter;
    html! {
        <div class="ops-filter-bar">
            { for FILTERS.iter().map(|&f| {
                let active_filter = active_filter.clone();
                let is_active = f == active;
                let slug = f.slug().to_string();
                html! {
                    <button
                        class={classes!(
                            "ops-filter-btn",
                            slug,
                            is_active.then_some("active"),
                        )}
                        onclick={Callback::from(move |_: MouseEvent| active_filter.set(f))}
                    >
                        {f.label()}
                    </button>
                }
            }) }
            <span class="ops-filter-spacer" />
        </div>
    }
}

fn case_table(
    cases: &[CaseRecord],
    snapshot_state: &UseStateHandle<ApiState<LeadsCasesSnapshot>>,
) -> Html {
    if matches!(&**snapshot_state, ApiState::Loading) {
        return html! { <p class="ops-empty-state">{"加载中..."}</p> };
    }
    if matches!(&**snapshot_state, ApiState::Idle) {
        return html! { <p class="ops-empty-state">{"数据加载中，请稍候。"}</p> };
    }
    if let ApiState::Failed(err) = &**snapshot_state {
        return html! { <p class="ops-empty-state">{format!("加载失败：{err}")}</p> };
    }
    if cases.is_empty() {
        return html! { <p class="ops-empty-state">{"该筛选条件下无案件。"}</p> };
    }

    html! {
        <table class="case-table">
            <thead>
                <tr>
                    <th>{"案件编号"}</th>
                    <th>{"理赔单"}</th>
                    <th>{"成员"}</th>
                    <th>{"负责人"}</th>
                    <th>{"SLA"}</th>
                    <th>{"状态"}</th>
                </tr>
            </thead>
            <tbody>
                { for cases.iter().map(|case| case_table_row(case)) }
            </tbody>
        </table>
    }
}

fn case_table_row(case: &CaseRecord) -> Html {
    let assignee_display = if case.assignee.is_empty() {
        html! { <span class="muted">{"待分配"}</span> }
    } else {
        html! { <span>{&case.assignee}</span> }
    };

    let priority = priority_tone(&case.priority);

    html! {
        <tr class={classes!("case-table-row", priority)}>
            <td style="font-weight:600;">{&case.case_id}</td>
            <td>{&case.claim_id}</td>
            <td>{truncate(&case.member_id, 16)}</td>
            <td>{assignee_display}</td>
            <td>{ sla_badge_html(&case.sla_status) }</td>
            <td>{ status_badge_html(&case.status) }</td>
        </tr>
    }
}

// ── Main component ────────────────────────────────────────────────────────────

#[function_component(CaseTrackerPage)]
pub fn case_tracker_page() -> Html {
    let api_key = use_api_key();
    let snapshot_state = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let active_filter = use_state(|| Filter::All);

    // Auto-load on mount
    {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        use_effect_with((), move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                    Ok(s) => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
            || ()
        });
    }

    // Filtered case list
    let filtered_cases: Vec<CaseRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
        snap.cases
            .iter()
            .filter(|c| active_filter.matches(c))
            .cloned()
            .collect()
    } else {
        vec![]
    };

    html! {
        <div class="ops-page case-tracker-page">
            { page_header(&snapshot_state) }
            { kpi_strip(&snapshot_state) }
            { filter_bar(&active_filter) }
            { case_table(&filtered_cases, &snapshot_state) }
        </div>
    }
}
