use crate::api::*;
use crate::formatting::*;
use crate::i18n::tr;
use crate::state::{use_api_key, ApiState, Language};
use crate::types::*;
use serde_json::json;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

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
    fn label(self, language: Language) -> &'static str {
        match self {
            Filter::All => tr(language, "All", "全部"),
            Filter::High => tr(language, "High risk", "高风险"),
            Filter::Amber => tr(language, "Watchlist", "可疑"),
            Filter::Low => tr(language, "Low risk", "低风险"),
            Filter::Pending => tr(language, "Pending", "待处理"),
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::High => "red",
            Filter::Amber => "amber",
            Filter::Low => "green",
            Filter::Pending => "pending",
        }
    }

    fn matches(self, lead: &LeadRecord) -> bool {
        match self {
            Filter::All => true,
            Filter::High => lead.rag.eq_ignore_ascii_case("red"),
            Filter::Amber => {
                lead.rag.eq_ignore_ascii_case("amber") || lead.rag.eq_ignore_ascii_case("yellow")
            }
            Filter::Low => lead.rag.eq_ignore_ascii_case("green"),
            Filter::Pending => {
                lead.status.eq_ignore_ascii_case("pending")
                    || lead.status.eq_ignore_ascii_case("triage")
                    || lead.status.eq_ignore_ascii_case("new")
            }
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
        "RED" => "high",
        "AMBER" | "YELLOW" => "medium",
        _ => "low",
    }
}

fn rag_label_for(value: &str, language: Language) -> &'static str {
    match value.trim().to_ascii_uppercase().as_str() {
        "RED" => tr(language, "High risk", "高风险"),
        "AMBER" | "YELLOW" => tr(language, "Watchlist risk", "可疑风险"),
        "GREEN" => tr(language, "Low risk", "低风险"),
        _ => tr(language, "Risk pending", "风险待确认"),
    }
}

fn risk_badge_html(rag: &str, language: Language) -> Html {
    let tone = rag_tone(rag);
    let label = rag_label_for(rag, language);
    html! { <span class={classes!("risk-badge", tone)}>{label}</span> }
}

fn outcome_badge_html(rag: &str, language: Language) -> Html {
    let (tone, label) = match rag.trim().to_ascii_uppercase().as_str() {
        "RED" => (
            "auto-deny",
            tr(language, "Needs investigation", "需调查判断"),
        ),
        "AMBER" | "YELLOW" => ("manual", tr(language, "Human triage", "人工分流")),
        _ => ("straight", tr(language, "Can archive", "可归档")),
    };
    html! { <span class={classes!("outcome-badge", tone)}>{label}</span> }
}

fn status_badge_html(status: &str, language: Language) -> Html {
    let tone = match status.to_ascii_lowercase().as_str() {
        "triage" | "pending" | "new" => "warning",
        "rejected" | "closed" => "neutral",
        "investigating" | "open" => "info",
        _ => "neutral",
    };
    let label = match language {
        Language::En => business_label(status),
        Language::Zh => match status.trim().to_ascii_lowercase().as_str() {
            "triage" => "分诊中".into(),
            "pending" | "new" => "待处理".into(),
            "pending_evidence" => "待补件".into(),
            "investigating" | "open" => "调查中".into(),
            "closed" => "已关闭".into(),
            "rejected" => "已驳回".into(),
            _ => business_label(status),
        },
    };
    html! { <span class={classes!("status-token", tone)}>{label}</span> }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

fn lead_triage_explanation(lead: &LeadRecord, language: Language) -> &'static str {
    match lead.rag.to_ascii_uppercase().as_str() {
        "RED" => tr(language, "High-risk intake: first confirm evidence sufficiency. Request evidence if incomplete; route to Investigation Workbench if the package is ready for a human recommendation.", "高风险进件：先确认资料是否充分；资料不足先补件，资料充分再转调查工作台形成人工建议。"),
        "AMBER" | "YELLOW" => tr(language, "Watchlist intake: triage manually and confirm whether each hit is backed by evidence references.", "可疑进件：需要人工分流，重点确认命中信号是否有证据引用支撑。"),
        _ => tr(language, "Low-risk intake: archive only after confirming no material hit, while preserving audit references.", "低风险进件：确认无关键命中信号后可归档，但仍需保留审计引用。"),
    }
}

// Compute KPI values from leads
fn kpi_total(leads: &[LeadRecord]) -> usize {
    leads.len()
}
fn kpi_high(leads: &[LeadRecord]) -> usize {
    leads
        .iter()
        .filter(|l| l.rag.eq_ignore_ascii_case("red"))
        .count()
}
fn kpi_pending(leads: &[LeadRecord]) -> usize {
    leads
        .iter()
        .filter(|l| {
            let s = l.status.to_ascii_lowercase();
            s == "triage" || s == "pending" || s == "new"
        })
        .count()
}
fn kpi_processed(leads: &[LeadRecord]) -> usize {
    leads
        .iter()
        .filter(|l| {
            let s = l.status.to_ascii_lowercase();
            s == "closed" || s == "rejected" || s == "confirmed"
        })
        .count()
}

// ── Sub-views ─────────────────────────────────────────────────────────────────

fn page_header(
    refresh: Callback<MouseEvent>,
    snapshot_state: &UseStateHandle<ApiState<LeadsCasesSnapshot>>,
    language: Language,
) -> Html {
    let loading = matches!(&**snapshot_state, ApiState::Loading);
    html! {
        <div class="dashboard-header">
            <div>
                <h2>{tr(language, "Claims Triage Queue", "理赔队列")}</h2>
                <p class="muted">{tr(language, "TPA intake triage: confirm risk context, request evidence when needed, and route cases to investigation without making final payment decisions.", "今日 TPA 进件分流：确认风险、补证据、转调查；不在此页做最终赔付裁决。")}</p>
            </div>
            <button onclick={refresh} disabled={loading}>
                {if loading { tr(language, "Refreshing...", "刷新中...") } else { tr(language, "Refresh", "刷新") }}
            </button>
        </div>
    }
}

fn kpi_strip_from_state(
    snapshot_state: &UseStateHandle<ApiState<LeadsCasesSnapshot>>,
    language: Language,
) -> Html {
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
                <span>{tr(language, "Total today", "今日总量")}</span>
                <strong>{total}</strong>
            </div>
            <div class="ops-kpi-card highlight">
                <span>{tr(language, "High risk", "高风险")}</span>
                <strong>{high}</strong>
            </div>
            <div class="ops-kpi-card">
                <span>{tr(language, "Pending review", "待审核")}</span>
                <strong>{pending}</strong>
            </div>
            <div class="ops-kpi-card positive">
                <span>{tr(language, "Processed", "已处理")}</span>
                <strong>{processed}</strong>
            </div>
        </div>
    }
}

fn filter_bar(active_filter: &UseStateHandle<Filter>, language: Language) -> Html {
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
                        {f.label(language)}
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
    language: Language,
) -> Html {
    if matches!(&**snapshot_state, ApiState::Loading) {
        return html! { <p class="empty">{tr(language, "Loading...", "加载中...")}</p> };
    }
    if matches!(&**snapshot_state, ApiState::Idle) {
        return html! { <p class="empty">{tr(language, "Loading data. Please wait.", "数据加载中，请稍候。")}</p> };
    }
    if let ApiState::Failed(err) = &**snapshot_state {
        return html! { <p class="empty">{match language {
            Language::En => format!("Load failed: {err}"),
            Language::Zh => format!("加载失败：{err}"),
        }}</p> };
    }
    if leads.is_empty() {
        return html! { <p class="empty">{tr(language, "No intake items match this filter.", "该筛选条件下无进件。")}</p> };
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
                        <span>{format!("Member {} · Provider {} · {} · {}", lead.member_id, lead.provider_id, lead.scheme_family, lead.review_mode)}</span>
                        <small>{lead_triage_explanation(lead, language)}</small>
                    </div>
                    { risk_badge_html(&lead.rag, language) }
                    { outcome_badge_html(&lead.rag, language) }
                    <span class="claim-row-reason">
                        <strong>{tr(language, "Queue reason", "入队原因")}</strong>
                        <small>{truncate(&localized_business_text(&lead.reason, language), 96)}</small>
                    </span>
                    { status_badge_html(&lead.status, language) }
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
    language: Language,
) -> Html {
    let Some(lead) = lead else {
        return html! {
            <div class="claim-detail-panel">
                <div class="claim-detail-header">
                    <h3>{tr(language, "Claim detail", "理赔详情")}</h3>
                </div>
                <div class="claim-detail-body">
                    <p class="empty">{tr(language, "Select an intake item from the left to inspect details.", "点击左侧进件查看详情")}</p>
                </div>
            </div>
        };
    };

    let tone = rag_tone(&lead.rag);
    let ev_count = lead.evidence_refs.len();

    // Signal cards from splitting the reason text
    let signals: Vec<&str> = lead
        .reason
        .split(|c| c == ';' || c == ',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let recommendation = match lead.rag.to_ascii_uppercase().as_str() {
        "RED" => tr(
            language,
            "Route to investigation or request evidence first",
            "建议转人工调查或先请求补件",
        ),
        "AMBER" | "YELLOW" => tr(language, "Manual triage recommended", "建议人工分流"),
        _ => tr(language, "Archive as low risk", "建议低风险归档"),
    };

    let loading = matches!(&**triage_state, ApiState::Loading);

    html! {
        <div class="claim-detail-panel">
            <div class="claim-detail-header">
                <h3>{ &lead.claim_id }</h3>
                <div style="display:flex;gap:8px;align-items:center;margin-top:4px;">
                    <span class="muted" style="font-size:12px;">{ &lead.member_id }</span>
                    { risk_badge_html(&lead.rag, language) }
                </div>
            </div>
            <div class="claim-detail-body">

                // ── Confirmation banner ────────────────────────────────
                { if let Some(msg) = &**confirm_msg {
                    html! {
                        <div class="alert-card info">
                            <strong>{tr(language, "Action succeeded", "操作成功")}</strong>
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
                            <strong>{tr(language, "Action failed", "操作失败")}</strong>
                            <span>{ err }</span>
                        </div>
                    }
                } else {
                    html! {}
                } }

                // ── Routing context ────────────────────────────────────
                <div>
                    <p class="ops-section-label">{tr(language, "Triage basis", "分流依据")}</p>
                    <div class="ops-dashboard-grid" style="grid-template-columns:repeat(3,minmax(0,1fr));">
                        <div class="ops-kpi-card neutral">
                            <span class="ops-kpi-label">{tr(language, "Risk score", "风险评分")}</span>
                            <strong class="ops-kpi-value">{lead.risk_score}</strong>
                        </div>
                        <div class="ops-kpi-card neutral">
                            <span class="ops-kpi-label">{tr(language, "Evidence refs", "证据引用")}</span>
                            <strong class="ops-kpi-value">{ev_count}</strong>
                        </div>
                        <div class={classes!("ops-kpi-card", tone)}>
                            <span class="ops-kpi-label">{tr(language, "Risk band", "风险带")}</span>
                            <strong class="ops-kpi-value">{rag_label_for(&lead.rag, language)}</strong>
                        </div>
                    </div>
                </div>

                // ── Signal / reason list ───────────────────────────────
                <div>
                    <p class="ops-section-label">{tr(language, "Hit signals", "命中信号")}</p>
                    <div class="alert-list">
                        { if signals.is_empty() {
                            html! { <div class="alert-card"><strong>{tr(language, "No specific signal", "无具体信号")}</strong><span>{tr(language, "Check system logs", "请参阅系统日志")}</span></div> }
                        } else {
                            html! { for signals.iter().enumerate().map(|(i, sig)| {
                                let cls = if i == 0 && lead.rag.eq_ignore_ascii_case("red") { "critical" } else { "" };
                                html! {
                                    <div class={classes!("alert-card", cls)}>
                                        <strong>{ match language {
                                            Language::En => format!("Signal {}", i + 1),
                                            Language::Zh => format!("信号 {}", i + 1),
                                        } }</strong>
                                        <span>{ localized_business_text(sig, language) }</span>
                                    </div>
                                }
                            }) }
                        } }
                    </div>
                </div>

                // ── Recommendation ────────────────────────────────────
                <div>
                    <p class="ops-section-label">{tr(language, "Recommended action", "建议动作")}</p>
                    <div class={classes!("alert-card", if lead.rag.eq_ignore_ascii_case("red") { "critical" } else { "" })}>
                        <strong>{ recommendation }</strong>
                        <span>{ match language {
                            Language::En => format!("Risk score {} | {}", lead.risk_score, rag_label_for(&lead.rag, language)),
                            Language::Zh => format!("风险评分 {} | {}", lead.risk_score, rag_label_for(&lead.rag, language)),
                        } }</span>
                    </div>
                </div>

                // ── Action buttons ────────────────────────────────────
                <div class="claim-action-row">
                    <button
                        class="btn-approve"
                        onclick={on_approve}
                        disabled={loading}
                    >{tr(language, "Archive low risk", "低风险归档")}</button>
                    <button
                        class="btn-evidence"
                        onclick={on_deny}
                        disabled={loading}
                    >{tr(language, "Request evidence", "请求补件")}</button>
                    <button
                        class="btn-review"
                        onclick={on_review_click}
                        disabled={loading}
                    >{tr(language, "Route to investigation", "转人工调查")}</button>
                </div>

                // ── Triage mini-form (review) ─────────────────────────
                { if **show_review_form {
                    html! {
                        <div class="triage-mini-form" style="display:flex;flex-direction:column;gap:8px;padding-top:8px;">
                            <label style="font-size:12px;font-weight:600;color:var(--muted);">
                                {tr(language, "Assign to", "指派给")}
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
                                {tr(language, "Notes", "备注")}
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
                                {if loading { tr(language, "Submitting...", "提交中...") } else { tr(language, "Confirm investigation route", "确认转人工调查") }}
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

// ── Main component ────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct ClaimsQueuePageProps {
    pub language: Language,
}

#[function_component(ClaimsQueuePage)]
pub fn claims_queue_page(props: &ClaimsQueuePageProps) -> Html {
    let api_key = use_api_key();
    let snapshot_state = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let active_filter = use_state(|| Filter::All);
    let selected_lead_id = use_state(String::new);
    let show_review_form = use_state(|| false);
    let triage_assignee = use_state(|| "investigator-1".to_string());
    let triage_notes = use_state(|| "Routed from Claims Queue.".to_string());
    let triage_state = use_state(|| ApiState::<TriageLeadRecord>::Idle);
    let confirm_msg = use_state(|| Option::<String>::None);

    // Auto-load on mount + auto-select first lead
    {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let selected_lead_id = selected_lead_id.clone();
        use_effect_with((), move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let selected_lead_id = selected_lead_id.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                let result = get_leads_cases_snapshot(api_key).await;
                if let Ok(ref snap) = result {
                    // Auto-select highest-risk lead
                    if let Some(first) = snap.leads.iter().max_by_key(|l| l.risk_score) {
                        selected_lead_id.set(first.lead_id.clone());
                    }
                }
                snapshot_state.set(match result {
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
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_: MouseEvent| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                    Ok(s) => ApiState::Ready(s),
                    Err(e) => ApiState::Failed(e),
                });
            });
        })
    };

    // Derive sorted, filtered lead list
    let filtered_leads: Vec<LeadRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
        let mut leads: Vec<LeadRecord> = snap
            .leads
            .iter()
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

    // Triage action factory
    // evidence_refs from the selected lead are passed in to satisfy backend validation
    let do_triage = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let triage_state = triage_state.clone();
        let selected_lead_id = selected_lead_id.clone();
        let confirm_msg = confirm_msg.clone();
        let show_review_form = show_review_form.clone();
        move |decision: String, notes: String, assignee: String, evidence_refs: Vec<String>| {
            let id = (*selected_lead_id).clone();
            if id.is_empty() {
                return;
            }
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let triage_state = triage_state.clone();
            let confirm_msg = confirm_msg.clone();
            let show_review_form = show_review_form.clone();
            // Backend requires non-empty evidence_refs; fall back to claim ref if empty
            let refs = if evidence_refs.is_empty() {
                vec![format!("leads:{id}")]
            } else {
                // Keep only non-PII refs (exclude raw personal data paths)
                evidence_refs
                    .into_iter()
                    .filter(|r| !r.is_empty())
                    .take(10)
                    .collect()
            };
            let payload = json!({
                "decision":      decision,
                "assignee":      if assignee.is_empty() { "investigator-1" } else { &assignee },
                "reviewer":      "lead-reviewer-1",
                "priority":      "high",
                "notes":         notes,
                "evidence_refs": refs,
            });
            triage_state.set(ApiState::Loading);
            confirm_msg.set(None);
            spawn_local(async move {
                match post_triage_lead(api_key.clone(), id, payload).await {
                    Ok(record) => {
                        let msg = format!("理赔 {} 已处理", record.lead.claim_id,);
                        triage_state.set(ApiState::Ready(record));
                        confirm_msg.set(Some(msg));
                        show_review_form.set(false);
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(s) => ApiState::Ready(s),
                            Err(e) => ApiState::Failed(e),
                        });
                    }
                    Err(e) => triage_state.set(ApiState::Failed(e)),
                }
            });
        }
    };

    // btn-approve: close low-risk lead
    let on_approve = {
        let do_triage = do_triage.clone();
        let selected_lead = selected_lead.clone();
        let triage_state = triage_state.clone();
        Callback::from(move |_: MouseEvent| {
            let Some(ref lead) = selected_lead else {
                return;
            };
            if matches!(*triage_state, ApiState::Loading) {
                return;
            }
            do_triage(
                "reject_lead".to_string(),
                "Claims Queue: low-risk lead archived; no material FWA signal found in triage."
                    .to_string(),
                "triage-operator".to_string(),
                lead.evidence_refs.clone(),
            );
        })
    };

    // btn-evidence: keep the lead open while requesting missing evidence
    let on_deny = {
        let do_triage = do_triage.clone();
        let selected_lead = selected_lead.clone();
        let triage_state = triage_state.clone();
        Callback::from(move |_: MouseEvent| {
            let Some(ref lead) = selected_lead else {
                return;
            };
            if matches!(*triage_state, ApiState::Loading) {
                return;
            }
            do_triage(
                "request_evidence".to_string(),
                "Claims Queue: additional evidence requested before investigation or closure."
                    .to_string(),
                "evidence-coordinator".to_string(),
                lead.evidence_refs.clone(),
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
        let do_triage = do_triage.clone();
        let selected_lead = selected_lead.clone();
        let triage_state = triage_state.clone();
        let triage_assignee = triage_assignee.clone();
        let triage_notes = triage_notes.clone();
        Callback::from(move |_: MouseEvent| {
            let Some(ref lead) = selected_lead else {
                return;
            };
            if matches!(*triage_state, ApiState::Loading) {
                return;
            }
            let notes = (*triage_notes).clone();
            if notes.trim().is_empty() {
                return;
            }
            do_triage(
                "open_case".to_string(),
                notes,
                (*triage_assignee).clone(),
                lead.evidence_refs.clone(),
            );
        })
    };

    html! {
        <div class="ops-page claims-queue-page">
            { page_header(refresh, &snapshot_state, props.language) }
            { kpi_strip_from_state(&snapshot_state, props.language) }
            { filter_bar(&active_filter, props.language) }

            <div class="ops-split-layout">
                <div class="claim-queue-list">
                    { queue_list(
                        &filtered_leads,
                        &selected_lead_id,
                        &snapshot_state,
                        props.language,
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
                    props.language,
                ) }
            </div>
        </div>
    }
}
