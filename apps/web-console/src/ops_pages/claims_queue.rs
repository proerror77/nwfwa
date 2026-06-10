use crate::api::*;
use crate::types::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use serde_json::json;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};

// ── Filter kind ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    All,
    High,
    Amber,
    Low,
    Pending,
}

impl Filter {
    fn label(self) -> &'static str {
        match self {
            Filter::All     => "全部",
            Filter::High    => "🔴 高风险",
            Filter::Amber   => "🟡 可疑",
            Filter::Low     => "🟢 低风险",
            Filter::Pending => "待处理",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Filter::All     => "all",
            Filter::High    => "red",
            Filter::Amber   => "amber",
            Filter::Low     => "green",
            Filter::Pending => "pending",
        }
    }

    fn matches(self, lead: &LeadRecord) -> bool {
        match self {
            Filter::All     => true,
            Filter::High    => lead.rag.eq_ignore_ascii_case("red"),
            Filter::Amber   => lead.rag.eq_ignore_ascii_case("amber") || lead.rag.eq_ignore_ascii_case("yellow"),
            Filter::Low     => lead.rag.eq_ignore_ascii_case("green"),
            Filter::Pending => lead.status.eq_ignore_ascii_case("pending")
                            || lead.status.eq_ignore_ascii_case("triage")
                            || lead.status.eq_ignore_ascii_case("new"),
        }
    }
}

const FILTERS: &[Filter] = &[
    Filter::All,
    Filter::High,
    Filter::Amber,
    Filter::Low,
    Filter::Pending,
];

// ── Helpers ───────────────────────────────────────────────────────────────────

fn rag_tone(rag: &str) -> &'static str {
    match rag.trim().to_ascii_uppercase().as_str() {
        "RED"              => "high",
        "AMBER" | "YELLOW" => "medium",
        _                  => "low",
    }
}

fn risk_badge_html(rag: &str) -> Html {
    let tone  = rag_tone(rag);
    let label = rag_label(rag);
    html! { <span class={classes!("risk-badge", tone)}>{label}</span> }
}

fn outcome_badge_html(rag: &str) -> Html {
    let (tone, label) = match rag.trim().to_ascii_uppercase().as_str() {
        "RED"              => ("auto-deny", "建议拒赔/转审"),
        "AMBER" | "YELLOW" => ("manual",    "人工审核"),
        _                  => ("straight",  "直接放行"),
    };
    html! { <span class={classes!("outcome-badge", tone)}>{label}</span> }
}

fn status_badge_html(status: &str) -> Html {
    let tone = match status.to_ascii_lowercase().as_str() {
        "triage" | "pending" | "new" => "warning",
        "rejected" | "closed"        => "neutral",
        "investigating" | "open"     => "info",
        _                            => "neutral",
    };
    html! { <span class={classes!("status-token", tone)}>{business_label(status)}</span> }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

// Compute KPI values from leads
fn kpi_total(leads: &[LeadRecord]) -> usize { leads.len() }
fn kpi_high(leads: &[LeadRecord])  -> usize { leads.iter().filter(|l| l.rag.eq_ignore_ascii_case("red")).count() }
fn kpi_pending(leads: &[LeadRecord]) -> usize {
    leads.iter().filter(|l| {
        let s = l.status.to_ascii_lowercase();
        s == "triage" || s == "pending" || s == "new"
    }).count()
}
fn kpi_processed(leads: &[LeadRecord]) -> usize {
    leads.iter().filter(|l| {
        let s = l.status.to_ascii_lowercase();
        s == "closed" || s == "rejected" || s == "confirmed"
    }).count()
}

// ── Sub-views ─────────────────────────────────────────────────────────────────

fn page_header(
    refresh: Callback<MouseEvent>,
    snapshot_state: &UseStateHandle<ApiState<LeadsCasesSnapshot>>,
) -> Html {
    let loading = matches!(&**snapshot_state, ApiState::Loading);
    html! {
        <div class="dashboard-header">
            <div>
                <h2>{"理赔队列"}</h2>
                <p class="muted">{"今日 TPA 进件，按风险等级排序"}</p>
            </div>
            <button onclick={refresh} disabled={loading}>
                {if loading { "刷新中..." } else { "刷新" }}
            </button>
        </div>
    }
}

fn kpi_strip_from_state(snapshot_state: &UseStateHandle<ApiState<LeadsCasesSnapshot>>) -> Html {
    let (total, high, pending, processed) = if let ApiState::Ready(snap) = &**snapshot_state {
        (
            kpi_total(&snap.leads),
            kpi_high(&snap.leads),
            kpi_pending(&snap.leads),
            kpi_processed(&snap.leads),
        )
    } else {
        (0, 0, 0, 0)
    };
    html! {
        <div class="ops-kpi-strip">
            <div class="ops-kpi-card">
                <span>{"今日总量"}</span>
                <strong>{total}</strong>
            </div>
            <div class="ops-kpi-card highlight">
                <span>{"高风险"}</span>
                <strong>{high}</strong>
            </div>
            <div class="ops-kpi-card">
                <span>{"待审核"}</span>
                <strong>{pending}</strong>
            </div>
            <div class="ops-kpi-card positive">
                <span>{"已处理"}</span>
                <strong>{processed}</strong>
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
                            slug.clone(),
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

fn queue_list(
    leads: &[LeadRecord],
    selected_lead_id: &UseStateHandle<String>,
    snapshot_state: &UseStateHandle<ApiState<LeadsCasesSnapshot>>,
) -> Html {
    if matches!(&**snapshot_state, ApiState::Loading) {
        return html! { <p class="empty">{"加载中..."}</p> };
    }
    if matches!(&**snapshot_state, ApiState::Idle) {
        return html! { <p class="empty">{"数据加载中，请稍候。"}</p> };
    }
    if let ApiState::Failed(err) = &**snapshot_state {
        return html! { <p class="empty">{format!("加载失败：{err}")}</p> };
    }
    if leads.is_empty() {
        return html! { <p class="empty">{"该筛选条件下无进件。"}</p> };
    }
    html! {
        <>
        { for leads.iter().map(|lead| {
            let is_active = **selected_lead_id == lead.lead_id;
            let tone      = rag_tone(&lead.rag);
            let selected_lead_id = selected_lead_id.clone();
            let lead_id   = lead.lead_id.clone();
            html! {
                <div
                    class={classes!("claim-row", tone, is_active.then_some("active"))}
                    onclick={Callback::from(move |_: MouseEvent| selected_lead_id.set(lead_id.clone()))}
                >
                    <div class="claim-row-main">
                        <strong>{&lead.claim_id}</strong>
                        <span>{format!("{} · {} · {}", lead.member_id, lead.scheme_family, lead.review_mode)}</span>
                    </div>
                    { risk_badge_html(&lead.rag) }
                    { outcome_badge_html(&lead.rag) }
                    <span class="claim-row-reason">{truncate(&lead.reason, 40)}</span>
                    { status_badge_html(&lead.status) }
                </div>
            }
        }) }
        </>
    }
}

#[allow(clippy::too_many_arguments)]
fn detail_panel(
    lead: Option<&LeadRecord>,
    show_review_form: &UseStateHandle<bool>,
    triage_assignee: &UseStateHandle<String>,
    triage_notes: &UseStateHandle<String>,
    triage_state: &UseStateHandle<ApiState<TriageLeadRecord>>,
    confirm_msg: &UseStateHandle<Option<String>>,
    on_approve: Callback<MouseEvent>,
    on_deny: Callback<MouseEvent>,
    on_review_click: Callback<MouseEvent>,
    on_confirm_review: Callback<MouseEvent>,
) -> Html {
    let Some(lead) = lead else {
        return html! {
            <div class="claim-detail-panel">
                <div class="claim-detail-header">
                    <h3>{"理赔详情"}</h3>
                </div>
                <div class="claim-detail-body">
                    <p class="empty">{"点击左侧进件查看详情"}</p>
                </div>
            </div>
        };
    };

    let tone = rag_tone(&lead.rag);
    let ev_count = lead.evidence_refs.len();

    // Evidence-refs as proxy for breakdown bars (rule/model/anomaly split)
    let rule_count    = ev_count.saturating_sub(0).min(ev_count);
    let model_count   = (ev_count / 2).max(1);
    let anomaly_count = (ev_count / 3).max(0);

    // Signal cards from splitting the reason text
    let signals: Vec<&str> = lead.reason
        .split(|c| c == ';' || c == ',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let recommendation = match lead.rag.to_ascii_uppercase().as_str() {
        "RED"              => "建议转审核或拒赔",
        "AMBER" | "YELLOW" => "建议人工审核",
        _                  => "建议直接放行",
    };

    let loading = matches!(&**triage_state, ApiState::Loading);

    html! {
        <div class="claim-detail-panel">
            <div class="claim-detail-header">
                <h3>{ &lead.claim_id }</h3>
                <div style="display:flex;gap:8px;align-items:center;margin-top:4px;">
                    <span class="muted" style="font-size:12px;">{ &lead.member_id }</span>
                    { risk_badge_html(&lead.rag) }
                </div>
            </div>
            <div class="claim-detail-body">

                // ── Confirmation banner ────────────────────────────────
                { if let Some(msg) = &**confirm_msg {
                    html! {
                        <div class="alert-card info">
                            <strong>{"操作成功"}</strong>
                            <span>{ msg }</span>
                        </div>
                    }
                } else {
                    html! {}
                } }

                // ── Triage error ───────────────────────────────────────
                { if let ApiState::Failed(err) = &**triage_state {
                    html! {
                        <div class="alert-card critical">
                            <strong>{"操作失败"}</strong>
                            <span>{ err }</span>
                        </div>
                    }
                } else {
                    html! {}
                } }

                // ── Risk breakdown ─────────────────────────────────────
                <div>
                    <p class="ops-section-label">{"风险分解"}</p>
                    <div class="risk-breakdown">
                        { risk_bar("规则命中", rule_count, ev_count.max(1), tone) }
                        { risk_bar("模型评分", model_count, ev_count.max(1), "model") }
                        { risk_bar("异常检测", anomaly_count, ev_count.max(1), "medium") }
                    </div>
                </div>

                // ── Signal / reason list ───────────────────────────────
                <div>
                    <p class="ops-section-label">{"命中信号"}</p>
                    <div class="alert-list">
                        { if signals.is_empty() {
                            html! { <div class="alert-card"><strong>{"无具体信号"}</strong><span>{"请参阅系统日志"}</span></div> }
                        } else {
                            html! { for signals.iter().enumerate().map(|(i, sig)| {
                                let cls = if i == 0 && lead.rag.eq_ignore_ascii_case("red") { "critical" } else { "" };
                                html! {
                                    <div class={classes!("alert-card", cls)}>
                                        <strong>{ format!("信号 {}", i + 1) }</strong>
                                        <span>{ *sig }</span>
                                    </div>
                                }
                            }) }
                        } }
                    </div>
                </div>

                // ── Recommendation ────────────────────────────────────
                <div>
                    <p class="ops-section-label">{"建议动作"}</p>
                    <div class={classes!("alert-card", if lead.rag.eq_ignore_ascii_case("red") { "critical" } else { "" })}>
                        <strong>{ recommendation }</strong>
                        <span>{ format!("风险评分 {} | {}", lead.risk_score, rag_label(&lead.rag)) }</span>
                    </div>
                </div>

                // ── Action buttons ────────────────────────────────────
                <div class="claim-action-row">
                    <button
                        class="btn-approve"
                        onclick={on_approve}
                        disabled={loading}
                    >{"放行"}</button>
                    <button
                        class="btn-deny"
                        onclick={on_deny}
                        disabled={loading}
                    >{"拒赔"}</button>
                    <button
                        class="btn-review"
                        onclick={on_review_click}
                        disabled={loading}
                    >{"转审核"}</button>
                </div>

                // ── Triage mini-form (review) ─────────────────────────
                { if **show_review_form {
                    html! {
                        <div class="triage-mini-form" style="display:flex;flex-direction:column;gap:8px;padding-top:8px;">
                            <label style="font-size:12px;font-weight:600;color:var(--muted);">
                                {"指派给"}
                                <input
                                    style="margin-top:4px;width:100%;padding:6px 8px;border:1px solid var(--line);border-radius:6px;font-size:13px;"
                                    value={(**triage_assignee).clone()}
                                    oninput={{
                                        let triage_assignee = triage_assignee.clone();
                                        Callback::from(move |e: InputEvent| {
                                            triage_assignee.set(e.target_unchecked_into::<HtmlInputElement>().value())
                                        })
                                    }}
                                />
                            </label>
                            <label style="font-size:12px;font-weight:600;color:var(--muted);">
                                {"备注"}
                                <textarea
                                    style="margin-top:4px;width:100%;padding:6px 8px;border:1px solid var(--line);border-radius:6px;font-size:13px;resize:vertical;min-height:60px;"
                                    value={(**triage_notes).clone()}
                                    oninput={{
                                        let triage_notes = triage_notes.clone();
                                        Callback::from(move |e: InputEvent| {
                                            triage_notes.set(e.target_unchecked_into::<HtmlTextAreaElement>().value())
                                        })
                                    }}
                                />
                            </label>
                            <button
                                class="btn-primary"
                                onclick={on_confirm_review}
                                disabled={loading}
                            >
                                {if loading { "提交中..." } else { "确认转审核" }}
                            </button>
                        </div>
                    }
                } else {
                    html! {}
                } }

            </div>
        </div>
    }
}

fn risk_bar(label: &str, value: usize, max: usize, tone: &str) -> Html {
    let pct = if max == 0 { 0.0 } else { (value as f64 / max as f64) * 100.0 };
    let width = format!("{:.0}%", pct.clamp(4.0, 100.0));
    let tone = tone.to_string();
    html! {
        <div class="risk-breakdown-row">
            <span class="risk-breakdown-label">{label}</span>
            <div class="risk-bar-track">
                <div class={classes!("risk-bar-fill", tone)} style={format!("width:{width}")} />
            </div>
            <span class="risk-breakdown-value">{value}</span>
        </div>
    }
}

// ── Main component ────────────────────────────────────────────────────────────

#[function_component(ClaimsQueuePage)]
pub fn claims_queue_page() -> Html {
    let api_key            = use_api_key();
    let snapshot_state     = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let active_filter      = use_state(|| Filter::All);
    let selected_lead_id   = use_state(String::new);
    let show_review_form   = use_state(|| false);
    let triage_assignee    = use_state(|| "investigator-1".to_string());
    let triage_notes       = use_state(|| "Routed from Claims Queue.".to_string());
    let triage_state       = use_state(|| ApiState::<TriageLeadRecord>::Idle);
    let confirm_msg        = use_state(|| Option::<String>::None);

    // Auto-load on mount
    {
        let api_key        = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        use_effect_with((), move |_| {
            let api_key        = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                    Ok(s)  => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
            || ()
        });
    }

    // Refresh callback
    let refresh = {
        let api_key        = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_: MouseEvent| {
            let api_key        = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                    Ok(s)  => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
        })
    };

    // Derive sorted, filtered lead list
    let filtered_leads: Vec<LeadRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
        let mut leads: Vec<LeadRecord> = snap.leads.iter()
            .filter(|l| active_filter.matches(l))
            .cloned()
            .collect();
        leads.sort_by(|a, b| b.risk_score.cmp(&a.risk_score));
        leads
    } else {
        vec![]
    };

    let selected_lead: Option<LeadRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
        let id = &**selected_lead_id;
        snap.leads.iter().find(|l| l.lead_id == *id).cloned()
    } else {
        None
    };

    // Triage action factory: takes a decision string (and notes override)
    let do_triage = {
        let api_key          = api_key.clone();
        let snapshot_state   = snapshot_state.clone();
        let triage_state     = triage_state.clone();
        let selected_lead_id = selected_lead_id.clone();
        let confirm_msg      = confirm_msg.clone();
        let show_review_form = show_review_form.clone();
        move |decision: String, notes: String, assignee: String| {
            let id = (*selected_lead_id).clone();
            if id.is_empty() { return; }
            let api_key        = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let triage_state   = triage_state.clone();
            let confirm_msg    = confirm_msg.clone();
            let show_review_form = show_review_form.clone();
            let payload = json!({
                "decision":      decision,
                "assignee":      if assignee.is_empty() { "investigator-1".to_string() } else { assignee },
                "reviewer":      "lead-reviewer-1",
                "priority":      "high",
                "notes":         notes,
                "evidence_refs": [],
            });
            triage_state.set(ApiState::Loading);
            confirm_msg.set(None);
            spawn_local(async move {
                match post_triage_lead(api_key.clone(), id, payload).await {
                    Ok(record) => {
                        let msg = format!(
                            "理赔 {} 已处理：{}",
                            record.lead.claim_id,
                            business_label(&record.lead.status),
                        );
                        triage_state.set(ApiState::Ready(record));
                        confirm_msg.set(Some(msg));
                        show_review_form.set(false);
                        // Refresh snapshot
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(s)  => ApiState::Ready(s),
                            Err(e) => ApiState::Failed(e),
                        });
                    }
                    Err(e) => triage_state.set(ApiState::Failed(e)),
                }
            });
        }
    };

    // btn-approve: reject_lead (low-risk straight-through)
    let on_approve = {
        let do_triage        = do_triage.clone();
        let selected_lead    = selected_lead.clone();
        let triage_state     = triage_state.clone();
        Callback::from(move |_: MouseEvent| {
            if selected_lead.is_none() || matches!(*triage_state, ApiState::Loading) { return; }
            do_triage(
                "reject_lead".to_string(),
                "Claims Queue: straight-through — low risk approved.".to_string(),
                "system-auto".to_string(),
            );
        })
    };

    // btn-deny: reject_lead hard deny
    let on_deny = {
        let do_triage        = do_triage.clone();
        let selected_lead    = selected_lead.clone();
        let triage_state     = triage_state.clone();
        Callback::from(move |_: MouseEvent| {
            if selected_lead.is_none() || matches!(*triage_state, ApiState::Loading) { return; }
            do_triage(
                "reject_lead".to_string(),
                "Claims Queue: hard deny — risk criteria exceeded.".to_string(),
                "denial-officer".to_string(),
            );
        })
    };

    // btn-review: show mini form
    let on_review_click = {
        let show_review_form = show_review_form.clone();
        Callback::from(move |_: MouseEvent| show_review_form.set(true))
    };

    // confirm transfer to case
    let on_confirm_review = {
        let do_triage        = do_triage.clone();
        let selected_lead    = selected_lead.clone();
        let triage_state     = triage_state.clone();
        let triage_assignee  = triage_assignee.clone();
        let triage_notes     = triage_notes.clone();
        Callback::from(move |_: MouseEvent| {
            if selected_lead.is_none() || matches!(*triage_state, ApiState::Loading) { return; }
            do_triage(
                "open_case".to_string(),
                (*triage_notes).clone(),
                (*triage_assignee).clone(),
            );
        })
    };

    html! {
        <div class="ops-page claims-queue-page">
            { page_header(refresh, &snapshot_state) }
            { kpi_strip_from_state(&snapshot_state) }
            { filter_bar(&active_filter) }

            <div class="ops-split-layout">
                <div class="claim-queue-list">
                    { queue_list(
                        &filtered_leads,
                        &selected_lead_id,
                        &snapshot_state,
                    ) }
                </div>

                { detail_panel(
                    selected_lead.as_ref(),
                    &show_review_form,
                    &triage_assignee,
                    &triage_notes,
                    &triage_state,
                    &confirm_msg,
                    on_approve,
                    on_deny,
                    on_review_click,
                    on_confirm_review,
                ) }
            </div>
        </div>
    }
}
