/// Action Queue — three tabs of items that need human action.
///
/// Tab 1: Investigate   — high-risk leads + investigating cases (manual_review, pending_evidence)
/// Tab 2: Pending docs  — leads/cases waiting for document submission
/// Tab 3: Medical       — claims routed to medical or fraud review
///
/// Clicking a case/lead fires `on_open_case` which navigates to the Investigate workbench.
use crate::api::*;
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
            (QueueTab::Investigate, Language::En) => "Investigate",
            (QueueTab::Investigate, Language::Zh) => "待调查",
            (QueueTab::PendingDocs, Language::En) => "Pending Docs",
            (QueueTab::PendingDocs, Language::Zh) => "补件中",
            (QueueTab::Medical, Language::En) => "Medical / Fraud Review",
            (QueueTab::Medical, Language::Zh) => "医疗 / 欺诈复核",
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

// ── Tab 1: Investigate ────────────────────────────────────────────────────────

fn investigate_tab(
    snapshot: &LeadsCasesSnapshot,
    on_open: &Callback<String>,
    language: Language,
) -> Html {
    // Merge high-risk leads (not yet triaged) and investigating/pending_evidence cases.
    let mut rows: Vec<(String, String, u8, String, String, String, String)> = Vec::new(); // (id, claim, score, rag, scheme, status, sla)

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
        ));
    }

    if rows.is_empty() {
        return html! {
            <p class="empty">{tr(language, "No items needing investigation.", "暂无待调查案件。")}</p>
        };
    }

    html! {
        <div class="queue-list">
            { for rows.iter().map(|(id, claim, score, rag, scheme, status, sla)| {
                let id_val = id.clone();
                let on_open = on_open.clone();
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
                                <span class="queue-scheme">{scheme}</span>
                            </div>
                            <div class="queue-row-chips">
                                <span class="status-chip">{status}</span>
                                { if !sla.is_empty() { sla_badge(sla, language) } else { html!{} } }
                            </div>
                        </div>
                        <span class="queue-row-arrow">{"›"}</span>
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
                                <span class="queue-scheme">{&c.scheme_family}</span>
                            </div>
                            { if !missing.is_empty() {
                                html! {
                                    <div class="queue-missing-docs">
                                        <span class="missing-label">{tr(language, "Missing:", "缺少：")}</span>
                                        { for missing.iter().map(|d| html! {
                                            <span class="doc-chip warning">{d}</span>
                                        }) }
                                    </div>
                                }
                            } else { html!{} } }
                            <div class="queue-row-chips">
                                { sla_badge(&c.sla_status, language) }
                                <span class="status-chip">{&c.assignee}</span>
                            </div>
                        </div>
                        <span class="queue-row-arrow">{"›"}</span>
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
                    ("fraud_investigation_review", Language::En) => "Fraud investigation",
                    ("fraud_investigation_review", Language::Zh) => "欺诈调查",
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
                                <span class="status-chip">{&item.review_status}</span>
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
                    <h2>{tr(language, "Action Queue", "需要处理")}</h2>
                    <p class="muted">{tr(language, "Only items that need a human decision are shown here.", "只显示需要人工处理的案件。")}</p>
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
