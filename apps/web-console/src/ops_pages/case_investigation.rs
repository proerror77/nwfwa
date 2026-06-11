use crate::api::*;
use crate::types::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::ops_pages::investigation_layers::{
    layer_document_completeness,
    layer_risk_signals,
    layer_member_behavior,
    layer_provider_analysis,
    layer_association_network,
    layer_similar_cases,
    layer_ai_summary,
};
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use serde_json::json;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};

// ── Conclusion kind ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Conclusion {
    ConfirmedFwa,
    FalsePositive,
    InsufficientEvidence,
    ImproperPayment,
    DocumentationIssue,
}

impl Conclusion {
    fn label(self) -> &'static str {
        match self {
            Conclusion::ConfirmedFwa          => "确认 FWA — 拒赔",
            Conclusion::FalsePositive         => "误报 — 放行",
            Conclusion::InsufficientEvidence  => "需补充材料",
            Conclusion::ImproperPayment       => "不当支付 (非诈骗)",
            Conclusion::DocumentationIssue    => "文件问题",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            Conclusion::ConfirmedFwa         => "fwa",
            Conclusion::FalsePositive        => "clear",
            Conclusion::InsufficientEvidence => "more",
            Conclusion::ImproperPayment      => "improper",
            Conclusion::DocumentationIssue   => "doc",
        }
    }

    fn outcome(self) -> &'static str {
        match self {
            Conclusion::ConfirmedFwa         => "confirmed_fwa_prevented_payment",
            Conclusion::FalsePositive        => "false_positive",
            Conclusion::InsufficientEvidence => "insufficient_evidence",
            Conclusion::ImproperPayment      => "improper_payment",
            Conclusion::DocumentationIssue   => "documentation_issue",
        }
    }

    fn confirmed_fwa(self) -> bool {
        matches!(self, Conclusion::ConfirmedFwa)
    }
}

const CONCLUSIONS: &[Conclusion] = &[
    Conclusion::ConfirmedFwa,
    Conclusion::FalsePositive,
    Conclusion::InsufficientEvidence,
    Conclusion::ImproperPayment,
    Conclusion::DocumentationIssue,
];

// ── Supplement doc types ───────────────────────────────────────────────────────

const SUPPLEMENT_DOCS: &[(&str, &str)] = &[
    ("surgery_record",    "手术记录"),
    ("discharge_summary", "出院小结"),
    ("diagnosis_cert",    "诊断证明"),
    ("bill_detail",       "账单明细"),
    ("informed_consent",  "知情同意书"),
    ("other",             "其他"),
];

// ── Priority sort key ──────────────────────────────────────────────────────────

fn priority_sort_key(priority: &str) -> u8 {
    match priority.to_ascii_lowercase().as_str() {
        "critical" | "high" => 0,
        "medium"             => 1,
        _                    => 2,
    }
}

fn sla_sort_key(sla_status: &str) -> u8 {
    match sla_status.to_ascii_lowercase().as_str() {
        "breached" => 0,
        "at_risk"  => 1,
        _          => 2,
    }
}

// ── Small UI helpers ───────────────────────────────────────────────────────────

fn rag_badge(rag: &str) -> Html {
    let (bg, fg) = match rag.to_ascii_lowercase().as_str() {
        "red"    => ("#3d1a1a", "#f85149"),
        "amber"  => ("#3d2000", "#d29922"),
        "green"  => ("#1b3a1f", "#3fb950"),
        _        => ("#21262d", "#8b949e"),
    };
    let score_label = match rag.to_ascii_lowercase().as_str() {
        "red"   => "高",
        "amber" => "中",
        "green" => "低",
        _       => rag,
    };
    html! {
        <span style={format!("background:{bg};color:{fg};border:1px solid {fg};border-radius:10px;padding:1px 8px;font-size:0.72rem;font-weight:600;")}>
            {score_label}
        </span>
    }
}

fn scheme_chip(scheme: &str) -> Html {
    let (bg, fg) = match scheme.to_ascii_lowercase().as_str() {
        s if s.contains("dental")   => ("#0d2d3a", "#58a6ff"),
        s if s.contains("vision")   => ("#1a1a3a", "#a5a6ff"),
        s if s.contains("pharmacy") => ("#1a2d0a", "#56d364"),
        s if s.contains("life")     => ("#2d1a0a", "#f0883e"),
        _                           => ("#21262d", "#8b949e"),
    };
    html! {
        <span style={format!("background:{bg};color:{fg};border:1px solid {fg};border-radius:10px;padding:1px 7px;font-size:0.7rem;font-weight:600;")}>
            {scheme}
        </span>
    }
}

fn sla_badge(sla_status: &str) -> Html {
    let (bg, fg, label) = match sla_status.to_ascii_lowercase().as_str() {
        "breached" => ("#3d1a1a", "#f85149", "SLA 超时"),
        "at_risk"  => ("#3d2000", "#d29922", "SLA 预警"),
        _          => ("#1b3a1f", "#3fb950", "SLA 正常"),
    };
    html! {
        <span style={format!("background:{bg};color:{fg};border:1px solid {fg};border-radius:4px;padding:1px 6px;font-size:0.7rem;font-weight:600;")}>
            {label}
        </span>
    }
}

fn status_badge(status: &str) -> Html {
    let (bg, fg) = match status.to_ascii_lowercase().as_str() {
        "investigating"    => ("#0d2d3a", "#58a6ff"),
        "triage"           => ("#3d2000", "#d29922"),
        "pending_evidence" => ("#2d1a0a", "#f0883e"),
        _                  => ("#21262d", "#8b949e"),
    };
    html! {
        <span style={format!("background:{bg};color:{fg};border:1px solid {fg};border-radius:4px;padding:1px 6px;font-size:0.7rem;")}>
            {status}
        </span>
    }
}

// ── Layer metadata (for collapsed summaries) ───────────────────────────────────

struct LayerMeta {
    number: &'static str,
    title: &'static str,
}

const LAYER_META: &[LayerMeta] = &[
    LayerMeta { number: "①", title: "资料完整性 & 金额合理性" },
    LayerMeta { number: "②", title: "风险信号" },
    LayerMeta { number: "③", title: "成员行为模式" },
    LayerMeta { number: "④", title: "Provider 风险分析" },
    LayerMeta { number: "⑤", title: "关联网络" },
    LayerMeta { number: "⑥", title: "相似已确认案例" },
    LayerMeta { number: "⑦", title: "AI 调查摘要" },
];

// ── Left column: case queue ────────────────────────────────────────────────────

fn queue_panel(
    cases: &[CaseRecord],
    selected_case_id: &UseStateHandle<String>,
    loading: bool,
) -> Html {
    let count = cases.len();
    html! {
        <div style="width:260px;flex-shrink:0;background:#161b22;border-right:1px solid #30363d;display:flex;flex-direction:column;position:sticky;top:0;height:100vh;overflow:hidden;">
            <div style="padding:16px;border-bottom:1px solid #30363d;flex-shrink:0;">
                <h3 style="margin:0;font-size:0.95rem;color:#e6edf3;">
                    {format!("调查队列 ({})", count)}
                </h3>
                <p style="margin:4px 0 0;font-size:0.75rem;color:#8b949e;">{"调查中 · 分诊 · 待证据"}</p>
            </div>
            <div style="flex:1;overflow-y:auto;padding:8px 0;">
                { if loading {
                    html! { <p style="padding:16px;color:#8b949e;font-size:0.85rem;">{"加载中..."}</p> }
                } else if cases.is_empty() {
                    html! { <p style="padding:16px;color:#8b949e;font-size:0.85rem;">{"暂无待调查案件。"}</p> }
                } else {
                    html! {
                        <>
                        { for cases.iter().map(|c| {
                            let is_active   = **selected_case_id == c.case_id;
                            let case_id_val = c.case_id.clone();
                            let selected    = selected_case_id.clone();
                            let short_id    = if c.case_id.len() > 14 {
                                format!("{}…", &c.case_id[..14])
                            } else {
                                c.case_id.clone()
                            };
                            let border_style = if is_active {
                                "border-left:3px solid #1f6feb;background:#1c2942;"
                            } else {
                                "border-left:3px solid transparent;background:transparent;"
                            };
                            let rag = c.evidence_package
                                .get("rag")
                                .and_then(|v| v.as_str())
                                .unwrap_or("amber");
                            html! {
                                <div
                                    style={format!("padding:10px 14px;cursor:pointer;border-bottom:1px solid #21262d;{border_style}")}
                                    onclick={Callback::from(move |_: MouseEvent| selected.set(case_id_val.clone()))}
                                >
                                    <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:4px;">
                                        <span style="font-size:0.8rem;color:#e6edf3;font-family:monospace;font-weight:600;">{short_id}</span>
                                        { rag_badge(rag) }
                                    </div>
                                    <div style="font-size:0.75rem;color:#8b949e;margin-bottom:6px;">
                                        {format!("理赔 {}", &c.claim_id)}
                                    </div>
                                    <div style="display:flex;flex-wrap:wrap;gap:4px;align-items:center;">
                                        { scheme_chip(&c.scheme_family) }
                                        { sla_badge(&c.sla_status) }
                                        { status_badge(&c.status) }
                                    </div>
                                </div>
                            }
                        }) }
                        </>
                    }
                } }
            </div>
        </div>
    }
}

// ── Right column: conclusion panel ────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn conclusion_panel(
    case: Option<&CaseRecord>,
    selected_conclusion: &UseStateHandle<Option<Conclusion>>,
    supplement_docs: &UseStateHandle<Vec<String>>,
    supplement_sent: &UseStateHandle<bool>,
    notes: &UseStateHandle<String>,
    evidence_refs: &UseStateHandle<String>,
    write_state: &UseStateHandle<ApiState<PilotWritebackResponse>>,
    confirm_msg: &UseStateHandle<Option<String>>,
    on_submit: Callback<MouseEvent>,
    on_send_supplement: Callback<MouseEvent>,
) -> Html {
    let loading = matches!(&**write_state, ApiState::Loading);
    let shows_supplement = **selected_conclusion == Some(Conclusion::InsufficientEvidence);

    html! {
        <div style="width:340px;flex-shrink:0;background:#161b22;border-left:1px solid #30363d;display:flex;flex-direction:column;position:sticky;top:0;height:100vh;overflow:hidden;">
            <div style="padding:16px;border-bottom:1px solid #30363d;flex-shrink:0;">
                <h3 style="margin:0;font-size:0.95rem;color:#e6edf3;">{"调查结论"}</h3>
            </div>
            <div style="flex:1;overflow-y:auto;padding:16px;display:flex;flex-direction:column;gap:12px;">

                { if let Some(msg) = &**confirm_msg {
                    html! {
                        <div style="padding:8px 12px;background:#1b3a1f;border:1px solid #3fb950;border-radius:6px;font-size:0.82rem;color:#3fb950;">
                            <strong style="display:block;margin-bottom:2px;">{"提交成功"}</strong>
                            {msg}
                        </div>
                    }
                } else { html! {} } }

                { if let ApiState::Failed(err) = &**write_state {
                    html! {
                        <div style="padding:8px 12px;background:#3d1a1a;border:1px solid #f85149;border-radius:6px;font-size:0.82rem;color:#f85149;">
                            <strong style="display:block;margin-bottom:2px;">{"提交失败"}</strong>
                            {err}
                        </div>
                    }
                } else { html! {} } }

                { if let Some(case) = case {
                    html! {
                        <>
                        // Case basic info
                        <div style="background:#0d1117;border:1px solid #30363d;border-radius:6px;padding:10px;">
                            <div style="display:grid;grid-template-columns:5rem 1fr;gap:4px 8px;font-size:0.8rem;">
                                <span style="color:#8b949e;">{"理赔 ID"}</span>
                                <span style="color:#e6edf3;font-family:monospace;">{&case.claim_id}</span>
                                <span style="color:#8b949e;">{"成员 ID"}</span>
                                <span style="color:#e6edf3;font-family:monospace;">{&case.member_id}</span>
                                <span style="color:#8b949e;">{"供应商 ID"}</span>
                                <span style="color:#e6edf3;font-family:monospace;">{&case.provider_id}</span>
                            </div>
                        </div>

                        // Conclusion radio buttons
                        <div style="display:flex;flex-direction:column;gap:6px;">
                            { for CONCLUSIONS.iter().map(|&c| {
                                let is_active  = **selected_conclusion == Some(c);
                                let sel        = selected_conclusion.clone();
                                let (border_color, bg) = if is_active {
                                    match c {
                                        Conclusion::ConfirmedFwa         => ("#f85149", "#3d1a1a"),
                                        Conclusion::FalsePositive        => ("#3fb950", "#1b3a1f"),
                                        Conclusion::InsufficientEvidence => ("#d29922", "#3d2000"),
                                        Conclusion::ImproperPayment      => ("#f0883e", "#2d1a0a"),
                                        Conclusion::DocumentationIssue   => ("#8b949e", "#21262d"),
                                    }
                                } else {
                                    ("#30363d", "#0d1117")
                                };
                                html! {
                                    <button
                                        style={format!(
                                            "background:{bg};border:1px solid {border_color};border-radius:6px;padding:8px 12px;font-size:0.83rem;color:#e6edf3;cursor:pointer;text-align:left;transition:background 0.15s;"
                                        )}
                                        onclick={Callback::from(move |_: MouseEvent| sel.set(Some(c)))}
                                        disabled={loading}
                                    >
                                        { c.label() }
                                    </button>
                                }
                            }) }
                        </div>

                        // Supplement docs sub-panel (only when InsufficientEvidence selected)
                        { if shows_supplement {
                            let supplement_docs_clone = supplement_docs.clone();
                            html! {
                                <div style="background:#0d1117;border:1px solid #30363d;border-radius:6px;padding:12px;">
                                    <p style="margin:0 0 10px;font-size:0.78rem;color:#8b949e;text-transform:uppercase;letter-spacing:.05em;">{"需补充资料类型"}</p>
                                    <div style="display:flex;flex-direction:column;gap:6px;margin-bottom:10px;">
                                        { for SUPPLEMENT_DOCS.iter().map(|(key, label)| {
                                            let key_str = key.to_string();
                                            let is_checked = supplement_docs.contains(&key_str);
                                            let docs_handle = supplement_docs_clone.clone();
                                            let key_owned = key_str.clone();
                                            html! {
                                                <label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:0.83rem;color:#e6edf3;">
                                                    <input
                                                        type="checkbox"
                                                        checked={is_checked}
                                                        onchange={Callback::from(move |_: Event| {
                                                            let mut current = (*docs_handle).clone();
                                                            if current.contains(&key_owned) {
                                                                current.retain(|k| k != &key_owned);
                                                            } else {
                                                                current.push(key_owned.clone());
                                                            }
                                                            docs_handle.set(current);
                                                        })}
                                                    />
                                                    {*label}
                                                </label>
                                            }
                                        }) }
                                    </div>
                                    { if **supplement_sent {
                                        html! {
                                            <div style="padding:8px 12px;background:#1b3a1f;border:1px solid #3fb950;border-radius:4px;font-size:0.82rem;color:#3fb950;">
                                                {"补件通知已发送（Mock）"}
                                            </div>
                                        }
                                    } else {
                                        html! {
                                            <button
                                                style="background:#1f6feb;color:#e6edf3;border:none;border-radius:6px;padding:8px 16px;font-size:0.83rem;cursor:pointer;font-weight:600;width:100%;"
                                                onclick={on_send_supplement}
                                            >
                                                {"[发送补件通知]"}
                                            </button>
                                        }
                                    } }
                                </div>
                            }
                        } else { html! {} } }

                        // Evidence refs
                        <label style="display:flex;flex-direction:column;gap:4px;">
                            <span style="font-size:0.78rem;color:#8b949e;">{"证据引用（逗号分隔）"}</span>
                            <textarea
                                style="background:#0d1117;border:1px solid #30363d;border-radius:6px;color:#e6edf3;padding:8px;font-size:0.82rem;resize:vertical;min-height:60px;font-family:monospace;"
                                placeholder="例：claims:abc123, rule_runs:xyz"
                                value={(*evidence_refs).to_string()}
                                oninput={{
                                    let evidence_refs = evidence_refs.clone();
                                    Callback::from(move |e: InputEvent| {
                                        evidence_refs.set(e.target_unchecked_into::<HtmlTextAreaElement>().value())
                                    })
                                }}
                            />
                        </label>

                        // Notes
                        <label style="display:flex;flex-direction:column;gap:4px;">
                            <span style="font-size:0.78rem;color:#8b949e;">{"调查备注 *（必填）"}</span>
                            <textarea
                                style="background:#0d1117;border:1px solid #30363d;border-radius:6px;color:#e6edf3;padding:8px;font-size:0.82rem;resize:vertical;min-height:80px;"
                                placeholder="请填写调查备注"
                                value={(*notes).to_string()}
                                oninput={{
                                    let notes = notes.clone();
                                    Callback::from(move |e: InputEvent| {
                                        notes.set(e.target_unchecked_into::<HtmlTextAreaElement>().value())
                                    })
                                }}
                            />
                        </label>

                        // Submit button
                        <button
                            style={format!(
                                "background:{};color:#e6edf3;border:none;border-radius:6px;padding:10px 16px;font-size:0.88rem;font-weight:600;cursor:pointer;",
                                if loading || selected_conclusion.is_none() { "#21262d" } else { "#238636" }
                            )}
                            onclick={on_submit}
                            disabled={loading || selected_conclusion.is_none()}
                        >
                            { if loading { "提交中..." } else { "提交结论" } }
                        </button>
                        </>
                    }
                } else {
                    html! { <p style="color:#8b949e;font-size:0.85rem;">{"请选择案件。"}</p> }
                } }
            </div>
        </div>
    }
}

// ── Collapsible layer panel ────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct CollapsibleLayerProps {
    index: usize,
    expanded: bool,
    on_toggle: Callback<usize>,
    children: Children,
}

#[function_component(CollapsibleLayer)]
fn collapsible_layer(props: &CollapsibleLayerProps) -> Html {
    let meta   = &LAYER_META[props.index];
    let idx    = props.index;
    let on_tog = props.on_toggle.clone();
    let arrow  = if props.expanded { "▲" } else { "▼" };

    html! {
        <div style="background:#161b22;border:1px solid #30363d;border-radius:8px;margin-bottom:12px;overflow:hidden;">
            <div
                style="display:flex;align-items:center;gap:8px;padding:12px 16px;cursor:pointer;background:#1c2128;border-bottom:1px solid #30363d;"
                onclick={Callback::from(move |_: MouseEvent| on_tog.emit(idx))}
            >
                <span style="font-size:1rem;color:#e6edf3;font-weight:600;">{meta.number}</span>
                <span style="font-size:0.9rem;color:#e6edf3;flex:1;">{meta.title}</span>
                <span style="font-size:0.75rem;color:#8b949e;">{arrow}</span>
            </div>
            { if props.expanded {
                html! {
                    <div style="padding:16px;">
                        { for props.children.iter() }
                    </div>
                }
            } else { html! {} } }
        </div>
    }
}

// ── Evidence refs extractor ────────────────────────────────────────────────────

fn evidence_refs_from_package(package: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = package.get("evidence_refs").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    } else {
        vec![]
    }
}

// ── Main component ─────────────────────────────────────────────────────────────

#[function_component(CaseInvestigationPage)]
pub fn case_investigation_page() -> Html {
    let api_key          = use_api_key();
    let snapshot_state   = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let selected_case_id = use_state(String::new);
    let ctx_state        = use_state(|| ApiState::<InvestigationContext>::Idle);

    // Layers 0–6, default: layers 0 and 1 expanded
    let expanded_layers = use_state(|| vec![true, true, false, false, false, false, false]);

    // Conclusion form
    let selected_conclusion = use_state(|| Option::<Conclusion>::None);
    let supplement_docs     = use_state(|| Vec::<String>::new());
    let supplement_sent     = use_state(|| false);
    let notes               = use_state(String::new);
    let evidence_refs       = use_state(String::new);
    let write_state         = use_state(|| ApiState::<PilotWritebackResponse>::Idle);
    let confirm_msg         = use_state(|| Option::<String>::None);

    // Auto-load snapshot on mount
    {
        let api_key        = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let selected_case_id = selected_case_id.clone();
        use_effect_with((), move |_| {
            let api_key        = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let selected_case_id = selected_case_id.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                match get_leads_cases_snapshot(api_key).await {
                    Ok(snap) => {
                        let first_id = snap.cases.iter()
                            .filter(|c| {
                                let s = c.status.to_ascii_lowercase();
                                s == "investigating" || s == "triage" || s == "pending_evidence"
                            })
                            .min_by_key(|c| (sla_sort_key(&c.sla_status), priority_sort_key(&c.priority)))
                            .map(|c| c.case_id.clone())
                            .unwrap_or_default();
                        if !first_id.is_empty() {
                            selected_case_id.set(first_id);
                        }
                        snapshot_state.set(ApiState::Ready(snap));
                    }
                    Err(e) => snapshot_state.set(ApiState::Failed(e)),
                }
            });
            || ()
        });
    }

    // Filtered + sorted case list for the queue
    let queue_cases: Vec<CaseRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
        let mut cases: Vec<CaseRecord> = snap.cases.iter()
            .filter(|c| {
                let s = c.status.to_ascii_lowercase();
                s == "investigating" || s == "triage" || s == "pending_evidence"
            })
            .cloned()
            .collect();
        cases.sort_by_key(|c| (sla_sort_key(&c.sla_status), priority_sort_key(&c.priority)));
        cases
    } else {
        vec![]
    };

    let selected_case: Option<CaseRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
        snap.cases.iter().find(|c| c.case_id == **selected_case_id).cloned()
    } else {
        None
    };

    // Load InvestigationContext when selected case changes
    {
        let api_key             = api_key.clone();
        let ctx_state           = ctx_state.clone();
        let selected_conclusion = selected_conclusion.clone();
        let supplement_docs     = supplement_docs.clone();
        let supplement_sent     = supplement_sent.clone();
        let notes               = notes.clone();
        let evidence_refs       = evidence_refs.clone();
        let confirm_msg         = confirm_msg.clone();
        let write_state         = write_state.clone();
        let selected_case       = selected_case.clone();
        let snapshot_leads: Vec<LeadRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
            snap.leads.clone()
        } else {
            vec![]
        };

        use_effect_with((*selected_case_id).clone(), move |_| {
            // Reset form state
            selected_conclusion.set(None);
            supplement_docs.set(vec![]);
            supplement_sent.set(false);
            notes.set(String::new());
            confirm_msg.set(None);
            write_state.set(ApiState::Idle);

            if let Some(case) = selected_case {
                // Pre-populate evidence refs
                let refs = evidence_refs_from_package(&case.evidence_package);
                evidence_refs.set(refs.join(", "));

                let api_key   = (*api_key).clone();
                let ctx_state = ctx_state.clone();
                ctx_state.set(ApiState::Loading);
                spawn_local(async move {
                    let ctx = load_investigation_context(api_key, case, &snapshot_leads).await;
                    ctx_state.set(ApiState::Ready(ctx));
                });
            } else {
                evidence_refs.set(String::new());
                ctx_state.set(ApiState::Idle);
            }
            || ()
        });
    }

    // Layer toggle handler
    let on_layer_toggle = {
        let expanded_layers = expanded_layers.clone();
        Callback::from(move |idx: usize| {
            let mut new = (*expanded_layers).clone();
            if let Some(v) = new.get_mut(idx) {
                *v = !*v;
            }
            expanded_layers.set(new);
        })
    };

    // Send supplement notification
    let on_send_supplement = {
        let supplement_sent = supplement_sent.clone();
        Callback::from(move |_: MouseEvent| {
            supplement_sent.set(true);
        })
    };

    // Submit conclusion
    let on_submit = {
        let api_key             = api_key.clone();
        let selected_case       = selected_case.clone();
        let selected_conclusion = selected_conclusion.clone();
        let notes               = notes.clone();
        let evidence_refs       = evidence_refs.clone();
        let write_state         = write_state.clone();
        let confirm_msg         = confirm_msg.clone();
        let selected_case_id    = selected_case_id.clone();
        let snapshot_state      = snapshot_state.clone();

        Callback::from(move |_: MouseEvent| {
            let Some(case) = &selected_case else { return; };
            let Some(conclusion) = *selected_conclusion else { return; };
            if matches!(*write_state, ApiState::Loading) { return; }

            let notes_val = (*notes).trim().to_string();
            if notes_val.is_empty() { return; }

            let refs: Vec<String> = (*evidence_refs)
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let payload = json!({
                "claim_id":              case.claim_id,
                "investigation_id":      case.case_id,
                "case_id":               case.case_id,
                "outcome":               conclusion.outcome(),
                "confirmed_fwa":         conclusion.confirmed_fwa(),
                "financial_impact_type": if conclusion.confirmed_fwa() { "prevented_payment" } else { "none" },
                "saving_amount":         serde_json::Value::Null,
                "notes":                 notes_val,
                "evidence_refs":         refs,
            });

            let api_key          = (*api_key).clone();
            let write_state      = write_state.clone();
            let confirm_msg      = confirm_msg.clone();
            let selected_case_id = selected_case_id.clone();
            let snapshot_state   = snapshot_state.clone();
            let claim_id         = case.claim_id.clone();

            write_state.set(ApiState::Loading);
            confirm_msg.set(None);

            spawn_local(async move {
                match post_investigation_result(api_key.clone(), payload).await {
                    Ok(resp) => {
                        let msg = format!("理赔 {} 已提交：{}", resp.claim_id, business_label(&resp.event_status));
                        write_state.set(ApiState::Ready(resp));
                        confirm_msg.set(Some(msg));
                        // Refresh snapshot and advance to next case
                        if let Ok(snap) = get_leads_cases_snapshot(api_key).await {
                            let next_id = snap.cases.iter()
                                .filter(|c| {
                                    let s = c.status.to_ascii_lowercase();
                                    (s == "investigating" || s == "triage" || s == "pending_evidence")
                                        && c.claim_id != claim_id
                                })
                                .min_by_key(|c| (sla_sort_key(&c.sla_status), priority_sort_key(&c.priority)))
                                .map(|c| c.case_id.clone())
                                .unwrap_or_default();
                            selected_case_id.set(next_id);
                            snapshot_state.set(ApiState::Ready(snap));
                        }
                    }
                    Err(e) => write_state.set(ApiState::Failed(e)),
                }
            });
        })
    };

    let snap_loading = matches!(&*snapshot_state, ApiState::Loading);
    let expanded     = (*expanded_layers).clone();

    html! {
        <div style="display:flex;flex-direction:column;height:100vh;overflow:hidden;background:#0d1117;">

            // Top bar
            <div style="flex-shrink:0;padding:12px 20px;border-bottom:1px solid #30363d;background:#161b22;display:flex;align-items:center;gap:16px;">
                <div>
                    <h2 style="margin:0;font-size:1.1rem;color:#e6edf3;font-weight:600;">{"调查工作台"}</h2>
                    <p style="margin:2px 0 0;font-size:0.78rem;color:#8b949e;">{"7层调查分析 · 选择案件开始"}</p>
                </div>
            </div>

            // Three-column layout
            <div style="flex:1;display:flex;overflow:hidden;">

                // Left: Case queue (260px sticky)
                { queue_panel(&queue_cases, &selected_case_id, snap_loading) }

                // Center: Investigation panels (scrollable, flex 1)
                <div style="flex:1;overflow-y:auto;padding:20px;">
                    { if selected_case_id.is_empty() || selected_case.is_none() {
                        html! {
                            <div style="display:flex;align-items:center;justify-content:center;height:100%;min-height:200px;">
                                <p style="color:#8b949e;font-size:1rem;">{"← 从队列选择一个案件开始调查"}</p>
                            </div>
                        }
                    } else {
                        match &*ctx_state {
                            ApiState::Idle | ApiState::Loading => html! {
                                <div style="display:flex;align-items:center;justify-content:center;height:100%;min-height:200px;">
                                    <p style="color:#8b949e;font-size:0.9rem;">{"加载调查数据..."}</p>
                                </div>
                            },
                            ApiState::Failed(e) => html! {
                                <div style="padding:20px;color:#f85149;font-size:0.85rem;">
                                    {format!("加载失败：{e}")}
                                </div>
                            },
                            ApiState::Ready(ctx) => {
                                let ctx = ctx.clone();
                                let layers: [Html; 7] = [
                                    layer_document_completeness(&ctx),
                                    layer_risk_signals(&ctx),
                                    layer_member_behavior(&ctx),
                                    layer_provider_analysis(&ctx),
                                    layer_association_network(&ctx),
                                    layer_similar_cases(&ctx),
                                    layer_ai_summary(&ctx),
                                ];
                                html! {
                                    <>
                                    { for layers.into_iter().enumerate().map(|(i, layer_html)| {
                                        let exp = expanded.get(i).copied().unwrap_or(false);
                                        let tog = on_layer_toggle.clone();
                                        html! {
                                            <CollapsibleLayer
                                                index={i}
                                                expanded={exp}
                                                on_toggle={tog}
                                            >
                                                { layer_html }
                                            </CollapsibleLayer>
                                        }
                                    }) }
                                    </>
                                }
                            }
                        }
                    } }
                </div>

                // Right: Conclusion panel (340px sticky)
                { conclusion_panel(
                    selected_case.as_ref(),
                    &selected_conclusion,
                    &supplement_docs,
                    &supplement_sent,
                    &notes,
                    &evidence_refs,
                    &write_state,
                    &confirm_msg,
                    on_submit,
                    on_send_supplement,
                ) }
            </div>
        </div>
    }
}
