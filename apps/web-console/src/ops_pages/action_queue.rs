/// Action Queue — three tabs of items that need human action.
///
/// Tab 1: Investigate   — high-risk leads + investigating cases (manual_review, pending_evidence)
/// Tab 2: Pending docs  — leads/cases waiting for document submission
/// Tab 3: Medical       — claims routed to medical or fraud review
///
/// Clicking a case/lead fires `on_open_case` which navigates to the Investigate workbench.
use crate::api::*;
use crate::formatting::{business_label, localized_business_text};
use crate::i18n::tr;
use crate::state::{use_api_key, ApiState, Language};
use crate::types::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

// ── Tab enum ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QueueTab {
    Investigate,
    PendingDocs,
    Medical,
}

impl QueueTab {
    fn label(self, language: Language) -> &'static str {
        match (self, language) {
            (QueueTab::Investigate, Language::En) => "Decide risk leads",
            (QueueTab::Investigate, Language::Zh) => "判断风险线索",
            (QueueTab::PendingDocs, Language::En) => "Request evidence",
            (QueueTab::PendingDocs, Language::Zh) => "追补证据",
            (QueueTab::Medical, Language::En) => "Medical review",
            (QueueTab::Medical, Language::Zh) => "医疗复核",
        }
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn rag_badge(rag: &str, language: Language) -> Html {
    let (tone, label) = match rag.to_ascii_uppercase().as_str() {
        "RED" => ("danger", tr(language, "High", "高")),
        "AMBER" | "YELLOW" => ("warning", tr(language, "Medium", "中")),
        _ => ("success", tr(language, "Low", "低")),
    };
    html! {
        <span class={classes!("rag-badge-inline", tone)}>{label}</span>
    }
}

fn priority_sort_key(p: &str) -> u8 {
    match p.to_ascii_lowercase().as_str() {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        _ => 3,
    }
}

fn sla_badge(sla: &str, language: Language) -> Html {
    let (tone, label) = match sla {
        "breached" => ("danger", tr(language, "SLA breached", "超时")),
        "closed_breached" => ("warning", tr(language, "Closed late", "超时关闭")),
        _ => ("neutral", tr(language, "On time", "按时")),
    };
    html! { <span class={classes!("sla-chip", tone)}>{label}</span> }
}

fn status_label(status: &str, language: Language) -> String {
    match (status, language) {
        ("new", Language::Zh) => "新线索".to_string(),
        ("triage" | "pending_triage", Language::Zh) => "待分诊".to_string(),
        ("investigating", Language::Zh) => "调查中".to_string(),
        ("pending_evidence", Language::Zh) => "待补证据".to_string(),
        ("open", Language::Zh) => "待处理".to_string(),
        _ => business_label(status),
    }
}

fn scheme_label(scheme: &str, language: Language) -> String {
    match (scheme, language) {
        ("diagnosis_procedure_mismatch", Language::Zh) => "诊疗不匹配".to_string(),
        ("high_amount", Language::Zh) => "金额异常".to_string(),
        ("duplicate_claim", Language::Zh) => "重复理赔".to_string(),
        ("provider_pattern", Language::Zh) => "机构模式异常".to_string(),
        _ => localized_business_text(scheme, language),
    }
}

fn next_action_label(status: &str, rag: &str, language: Language) -> &'static str {
    match status {
        "pending_evidence" => tr(language, "Ask for documents", "发起补件"),
        "triage" | "new" | "pending_triage" => tr(language, "Open workbench", "进入调查"),
        "investigating" => tr(language, "Finish recommendation", "形成建议"),
        _ if rag.eq_ignore_ascii_case("red") => tr(language, "Review now", "立即复核"),
        _ => tr(language, "Review", "复核"),
    }
}

fn shorten_reason(reason: &str, language: Language) -> String {
    let text = if reason.trim().is_empty() {
        tr(
            language,
            "No reason text. Open the case and verify evidence refs.",
            "暂无原因说明，进入案件核验证据引用。",
        )
        .to_string()
    } else {
        localized_business_text(reason, language)
    };
    if text.chars().count() > 120 {
        format!("{}…", text.chars().take(120).collect::<String>())
    } else {
        text
    }
}

// ── Tab 1: Investigate ────────────────────────────────────────────────────────

fn investigate_tab(
    snapshot: &LeadsCasesSnapshot,
    on_open: &Callback<String>,
    language: Language,
) -> Html {
    // Merge high-risk leads (not yet triaged) and investigating/pending_evidence cases.
    let mut rows: Vec<(
        String,
        String,
        u8,
        String,
        String,
        String,
        String,
        String,
        usize,
    )> = Vec::new(); // id, claim, score, rag, scheme, status, sla, reason, evidence count

    // High-risk leads awaiting triage
    for lead in &snapshot.leads {
        if lead.rag.to_ascii_uppercase() == "RED"
            && matches!(lead.status.as_str(), "new" | "triage" | "pending_triage")
        {
            rows.push((
                lead.lead_id.clone(),
                lead.claim_id.clone(),
                lead.risk_score,
                lead.rag.clone(),
                lead.scheme_family.clone(),
                tr(language, "Awaiting triage", "待分诊").to_string(),
                String::new(),
                shorten_reason(&lead.reason, language),
                lead.evidence_refs.len(),
            ));
        }
    }

    // Open cases
    let mut cases: Vec<&CaseRecord> = snapshot
        .cases
        .iter()
        .filter(|c| {
            matches!(
                c.status.as_str(),
                "triage" | "investigating" | "pending_evidence"
            )
        })
        .collect();
    cases.sort_by_key(|c| (priority_sort_key(&c.priority), c.sla_status == "breached"));

    for c in cases {
        let score = c
            .evidence_package
            .get("risk_score")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8;
        let rag = c
            .evidence_package
            .get("rag")
            .and_then(|v| v.as_str())
            .unwrap_or("amber")
            .to_string();
        rows.push((
            c.case_id.clone(),
            c.claim_id.clone(),
            score,
            rag,
            c.scheme_family.clone(),
            c.status.clone(),
            c.sla_status.clone(),
            shorten_reason(&c.routing_reason, language),
            c.evidence_package
                .get("evidence_refs")
                .and_then(|v| v.as_array())
                .map(|refs| refs.len())
                .unwrap_or(0),
        ));
    }

    if rows.is_empty() {
        return html! {
            <p class="empty">{tr(language, "No items needing investigation.", "暂无待调查案件。")}</p>
        };
    }

    html! {
        <div class="queue-list">
            { for rows.iter().map(|(id, claim, score, rag, scheme, status, sla, reason, evidence_count)| {
                let id_val = id.clone();
                let on_open = on_open.clone();
                let next_action = next_action_label(status, rag, language);
                html! {
                    <button
                        class="queue-row"
                        onclick={Callback::from(move |_: MouseEvent| on_open.emit(id_val.clone()))}
                    >
                        <div class="queue-row-left">
                            <span class="queue-score">{score}</span>
                            { rag_badge(rag, language) }
                        </div>
                        <div class="queue-row-body">
                            <div class="queue-row-top">
                                <span class="queue-claim-id">{claim}</span>
                                <span class="queue-scheme">{scheme_label(scheme, language)}</span>
                            </div>
                            <p class="queue-reason">
                                <strong>{tr(language, "Why this is here", "进入队列原因")}</strong>
                                {reason}
                            </p>
                            <div class="queue-row-chips">
                                <span class="status-chip">{status_label(status, language)}</span>
                                <span class="status-chip">
                                    {match language {
                                        Language::En => format!("{evidence_count} evidence refs"),
                                        Language::Zh => format!("{evidence_count} 条证据引用"),
                                    }}
                                </span>
                                { if !sla.is_empty() { sla_badge(sla, language) } else { html!{} } }
                            </div>
                        </div>
                        <span class="queue-next-action">{next_action}</span>
                    </button>
                }
            }) }
        </div>
    }
}

// ── Tab 2: Pending docs ───────────────────────────────────────────────────────

fn pending_docs_tab(
    snapshot: &LeadsCasesSnapshot,
    on_open: &Callback<String>,
    language: Language,
) -> Html {
    let items: Vec<&CaseRecord> = snapshot
        .cases
        .iter()
        .filter(|c| c.status == "pending_evidence")
        .collect();

    if items.is_empty() {
        return html! {
            <p class="empty">{tr(language, "No cases waiting for document submission.", "暂无待补件案件。")}</p>
        };
    }

    html! {
        <div class="queue-list">
            { for items.iter().map(|c| {
                let case_id = c.case_id.clone();
                let on_open = on_open.clone();
                let missing: Vec<String> = c
                    .evidence_package
                    .get("missing_evidence")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|i| i.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                html! {
                    <button
                        class="queue-row"
                        onclick={Callback::from(move |_| on_open.emit(case_id.clone()))}
                    >
                        <div class="queue-row-body" style="flex:1">
                            <div class="queue-row-top">
                                <span class="queue-claim-id">{&c.claim_id}</span>
                                <span class="queue-scheme">{scheme_label(&c.scheme_family, language)}</span>
                            </div>
                            { if !missing.is_empty() {
                                html! {
                                    <div class="queue-missing-docs">
                                        <span class="missing-label">{tr(language, "Missing evidence:", "缺少证据：")}</span>
                                        { for missing.iter().map(|d| html! {
                                            <span class="doc-chip warning">{d}</span>
                                        }) }
                                    </div>
                                }
                            } else { html!{} } }
                            <div class="queue-row-chips">
                                { sla_badge(&c.sla_status, language) }
                                <span class="status-chip">{format!("{}: {}", tr(language, "Owner", "负责人"), &c.assignee)}</span>
                            </div>
                        </div>
                        <span class="queue-next-action">{tr(language, "Open request", "处理补件")}</span>
                    </button>
                }
            }) }
        </div>
    }
}

// ── Tab 3: Medical / fraud review ────────────────────────────────────────────

fn medical_tab(items: &[MedicalReviewQueueItem], language: Language) -> Html {
    if items.is_empty() {
        return html! {
            <p class="empty">{tr(language, "No claims in medical or fraud review queue.", "暂无待医疗/欺诈复核案件。")}</p>
        };
    }

    html! {
        <div class="queue-list">
            { for items.iter().map(|item| {
                let route_tone = match item.review_route.as_str() {
                    "fraud_investigation_review" => "danger",
                    "medical_review" => "warning",
                    _ => "neutral",
                };
                let route_label = match (item.review_route.as_str(), language) {
                    ("fraud_investigation_review", Language::En) => "Fraud review",
                    ("fraud_investigation_review", Language::Zh) => "欺诈复核",
                    ("medical_review", Language::En) => "Medical review",
                    ("medical_review", Language::Zh) => "医疗复核",
                    (_, Language::En) => "Documentation",
                    (_, Language::Zh) => "文件补充",
                };
                let score = item.medical_reasonableness_score;
                html! {
                    <div class="queue-row queue-row-readonly">
                        <div class="queue-row-left">
                            <span class="queue-score">{score}</span>
                            <span class={classes!("rag-badge-inline", route_tone)}>{route_label}</span>
                        </div>
                        <div class="queue-row-body">
                            <div class="queue-row-top">
                                <span class="queue-claim-id">{&item.claim_id}</span>
                                { item.first_item_code.as_deref().map(|c| html!{
                                    <span class="queue-scheme">{c}</span>
                                }).unwrap_or_default() }
                            </div>
                            { if !item.missing_evidence.is_empty() {
                                html! {
                                    <div class="queue-missing-docs">
                                        { for item.missing_evidence.iter().take(3).map(|d| html!{
                                            <span class="doc-chip warning">{d}</span>
                                        }) }
                                    </div>
                                }
                            } else { html!{} } }
                            <div class="queue-row-chips">
                                <span class="status-chip">{status_label(&item.review_status, language)}</span>
                                <span class="status-chip">
                                    {match language {
                                        Language::En => format!("{} evidence refs", item.evidence_refs.len()),
                                        Language::Zh => format!("{} 条证据引用", item.evidence_refs.len()),
                                    }}
                                </span>
                            </div>
                        </div>
                    </div>
                }
            }) }
        </div>
    }
}

// ── Main component ────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct ActionQueuePageProps {
    pub language: Language,
    /// Navigate to Investigation workbench for this case/lead id.
    pub on_open_case: Callback<String>,
}

#[function_component(ActionQueuePage)]
pub fn action_queue_page(props: &ActionQueuePageProps) -> Html {
    let api_key = use_api_key();
    let active_tab = use_state(|| QueueTab::Investigate);
    let snapshot_state = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let medical_state = use_state(|| ApiState::<MedicalReviewQueueResponse>::Idle);

    {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let medical_state = medical_state.clone();
        use_effect_with((), move |_| {
            let api_key = (*api_key).clone();
            {
                let api_key = api_key.clone();
                let snapshot_state = snapshot_state.clone();
                snapshot_state.set(ApiState::Loading);
                spawn_local(async move {
                    snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                        Ok(s) => ApiState::Ready(s),
                        Err(e) => ApiState::Failed(e),
                    });
                });
            }
            {
                let api_key = api_key.clone();
                let medical_state = medical_state.clone();
                medical_state.set(ApiState::Loading);
                spawn_local(async move {
                    medical_state.set(
                        match get_medical_review_queue(api_key, "100".to_string()).await {
                            Ok(items) => ApiState::Ready(MedicalReviewQueueResponse { items }),
                            Err(e) => ApiState::Failed(e),
                        },
                    );
                });
            }
            || ()
        });
    }

    let language = props.language;
    let tab = *active_tab;

    // Count badges
    let investigate_count = match &*snapshot_state {
        ApiState::Ready(s) => {
            let high_leads = s
                .leads
                .iter()
                .filter(|l| {
                    l.rag.to_ascii_uppercase() == "RED"
                        && matches!(l.status.as_str(), "new" | "triage" | "pending_triage")
                })
                .count();
            let inv_cases = s
                .cases
                .iter()
                .filter(|c| matches!(c.status.as_str(), "triage" | "investigating"))
                .count();
            high_leads + inv_cases
        }
        _ => 0,
    };
    let pending_count = match &*snapshot_state {
        ApiState::Ready(s) => s
            .cases
            .iter()
            .filter(|c| c.status == "pending_evidence")
            .count(),
        _ => 0,
    };
    let medical_count = match &*medical_state {
        ApiState::Ready(r) => r.items.iter().filter(|i| i.review_status == "open").count(),
        _ => 0,
    };

    html! {
        <div class="ops-page action-queue-page">
            <div class="ops-page-header">
                <div>
                    <h2>{tr(language, "Human Work Queue", "人工处理队列")}</h2>
                    <p class="muted">{tr(language, "Every row says why it is here, what evidence exists, and the next reviewer action.", "每一行都要说明：为什么进队列、证据是否够、下一步人要做什么。")}</p>
                </div>
            </div>

            // ── Tab bar ─────────────────────────────────────────────────────
            <div class="tab-bar">
                { [QueueTab::Investigate, QueueTab::PendingDocs, QueueTab::Medical]
                    .iter()
                    .map(|&t| {
                        let count = match t {
                            QueueTab::Investigate => investigate_count,
                            QueueTab::PendingDocs => pending_count,
                            QueueTab::Medical => medical_count,
                        };
                        let active_tab = active_tab.clone();
                        let is_active = tab == t;
                        let badge_tone = if count > 0 && t == QueueTab::Investigate { "danger" }
                            else if count > 0 { "warning" }
                            else { "neutral" };
                        html! {
                            <button
                                class={classes!("tab-btn", is_active.then_some("active"))}
                                onclick={Callback::from(move |_: MouseEvent| active_tab.set(t))}
                            >
                                {t.label(language)}
                                { if count > 0 {
                                    html! { <span class={classes!("tab-count", badge_tone)}>{count}</span> }
                                } else { html!{} } }
                            </button>
                        }
                    })
                    .collect::<Html>()
                }
            </div>

            // ── Tab body ────────────────────────────────────────────────────
            <div class="tab-body">
                { match tab {
                    QueueTab::Investigate => match &*snapshot_state {
                        ApiState::Ready(s) => investigate_tab(s, &props.on_open_case, language),
                        ApiState::Loading | ApiState::Idle => html!{ <p class="muted">{tr(language, "Loading...", "加载中...")}</p> },
                        ApiState::Failed(e) => html!{ <p class="empty">{e}</p> },
                    },
                    QueueTab::PendingDocs => match &*snapshot_state {
                        ApiState::Ready(s) => pending_docs_tab(s, &props.on_open_case, language),
                        ApiState::Loading | ApiState::Idle => html!{ <p class="muted">{tr(language, "Loading...", "加载中...")}</p> },
                        ApiState::Failed(e) => html!{ <p class="empty">{e}</p> },
                    },
                    QueueTab::Medical => match &*medical_state {
                        ApiState::Ready(r) => medical_tab(&r.items, language),
                        ApiState::Loading | ApiState::Idle => html!{ <p class="muted">{tr(language, "Loading...", "加载中...")}</p> },
                        ApiState::Failed(e) => html!{ <p class="empty">{e}</p> },
                    },
                } }
            </div>
        </div>
    }
}
