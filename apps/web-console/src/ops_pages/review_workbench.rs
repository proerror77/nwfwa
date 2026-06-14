use crate::api::*;
use crate::formatting::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use serde_json::json;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

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
            Conclusion::ConfirmedFwa => "确认 FWA — 拒赔",
            Conclusion::FalsePositive => "误报 — 放行",
            Conclusion::InsufficientEvidence => "需补充材料",
            Conclusion::ImproperPayment => "不当支付 (非诈骗)",
            Conclusion::DocumentationIssue => "文件问题",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            Conclusion::ConfirmedFwa => "fwa",
            Conclusion::FalsePositive => "clear",
            Conclusion::InsufficientEvidence => "more",
            Conclusion::ImproperPayment => "improper",
            Conclusion::DocumentationIssue => "doc",
        }
    }

    fn outcome(self) -> &'static str {
        match self {
            Conclusion::ConfirmedFwa => "confirmed_fwa_prevented_payment",
            Conclusion::FalsePositive => "false_positive",
            Conclusion::InsufficientEvidence => "insufficient_evidence",
            Conclusion::ImproperPayment => "improper_payment",
            Conclusion::DocumentationIssue => "documentation_issue",
        }
    }

    fn confirmed_fwa(self) -> bool {
        matches!(self, Conclusion::ConfirmedFwa)
    }

    fn show_saving_amount(self) -> bool {
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

// ── Helpers ────────────────────────────────────────────────────────────────────

fn priority_badge(priority: &str) -> Html {
    let tone = match priority.to_ascii_lowercase().as_str() {
        "critical" | "high" => "high",
        "medium" => "medium",
        _ => "low",
    };
    html! { <span class={classes!("risk-badge", tone)}>{priority}</span> }
}

fn sla_badge(sla_status: &str) -> Html {
    let (tone, label) = match sla_status.to_ascii_lowercase().as_str() {
        "breached" => ("critical", "SLA 超时"),
        "at_risk" => ("warning", "SLA 预警"),
        _ => ("ok", "SLA 正常"),
    };
    html! { <span class={classes!("status-token", tone)}>{label}</span> }
}

fn evidence_refs_from_package(package: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = package.get("evidence_refs").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    } else {
        vec![]
    }
}

// ── Queue panel ────────────────────────────────────────────────────────────────

fn queue_panel(
    cases: &[CaseRecord],
    selected_case_id: &UseStateHandle<String>,
    loading: bool,
) -> Html {
    let count = cases.len();
    html! {
        <div class="review-queue-panel">
            <div class="review-queue-header">
                <h3>{ format!("审核队列 ({})", count) }</h3>
            </div>
            <div class="review-queue-list">
                { if loading {
                    html! { <p class="empty">{"加载中..."}</p> }
                } else if cases.is_empty() {
                    html! { <p class="empty">{"暂无待审核案件。"}</p> }
                } else {
                    html! {
                        <>
                        { for cases.iter().map(|c| {
                            let is_active   = **selected_case_id == c.case_id;
                            let case_id_val = c.case_id.clone();
                            let selected    = selected_case_id.clone();
                            html! {
                                <div
                                    class={classes!("review-queue-item", is_active.then_some("active"))}
                                    onclick={Callback::from(move |_: MouseEvent| selected.set(case_id_val.clone()))}
                                >
                                    <div class="review-queue-item-main">
                                        <strong>{ &c.case_id }</strong>
                                        <span class="muted">{ format!("{} · {}", c.scheme_family, c.claim_id) }</span>
                                    </div>
                                    <div class="review-queue-item-badges">
                                        { priority_badge(&c.priority) }
                                        { sla_badge(&c.sla_status) }
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

// ── Evidence cards ─────────────────────────────────────────────────────────────

fn card_claim_info(case: &CaseRecord) -> Html {
    html! {
        <div class="evidence-card">
            <div class="evidence-card-header">
                <h4>{"理赔信息"}</h4>
            </div>
            <div class="evidence-card-body">
                <div class="info-grid">
                    <span class="info-label">{"理赔 ID"}</span>
                    <span class="info-value">{ &case.claim_id }</span>
                    <span class="info-label">{"成员 ID"}</span>
                    <span class="info-value">{ &case.member_id }</span>
                    <span class="info-label">{"供应商 ID"}</span>
                    <span class="info-value">{ &case.provider_id }</span>
                    <span class="info-label">{"险种"}</span>
                    <span class="info-value">{ &case.scheme_family }</span>
                    <span class="info-label">{"审核模式"}</span>
                    <span class="info-value">{ &case.review_mode }</span>
                </div>
                <div class="routing-reason-row">
                    <span class="info-label">{"标记原因"}</span>
                    <p class="routing-reason-text">{ &case.routing_reason }</p>
                </div>
            </div>
        </div>
    }
}

fn card_risk_signals(case: &CaseRecord) -> Html {
    let signals: Vec<&str> = case
        .routing_reason
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let risk_tone = match case.priority.to_ascii_lowercase().as_str() {
        "critical" | "high" => "High",
        "medium" => "Medium",
        _ => "Low",
    };
    let badge_tone = match risk_tone {
        "High" => "high",
        "Medium" => "medium",
        _ => "low",
    };

    html! {
        <div class="evidence-card">
            <div class="evidence-card-header">
                <h4>{"风险信号"}</h4>
                <span class={classes!("risk-badge", badge_tone)}>{ risk_tone }</span>
            </div>
            <div class="evidence-card-body">
                <div class="alert-list">
                    { if signals.is_empty() {
                        html! {
                            <div class="alert-card">
                                <strong>{"无具体信号"}</strong>
                                <span>{"请参阅路由原因"}</span>
                            </div>
                        }
                    } else {
                        html! {
                            <>
                            { for signals.iter().enumerate().map(|(i, sig)| {
                                let cls = if i == 0 { "critical" } else { "" };
                                html! {
                                    <div class={classes!("alert-card", cls)}>
                                        <strong>{ format!("信号 {}", i + 1) }</strong>
                                        <span>{ *sig }</span>
                                    </div>
                                }
                            }) }
                            </>
                        }
                    } }
                </div>
            </div>
        </div>
    }
}

fn card_member_profile(member_state: &UseStateHandle<ApiState<MemberProfileSummary>>) -> Html {
    let body = match &**member_state {
        ApiState::Idle => html! { <p class="empty muted">{"选择案件以加载成员画像。"}</p> },
        ApiState::Loading => html! { <p class="empty muted">{"加载成员画像..."}</p> },
        ApiState::Failed(e) => html! { <p class="empty muted">{ format!("加载失败：{e}") }</p> },
        ApiState::Ready(m) => html! {
            <div class="info-grid">
                <span class="info-label">{"成员 ID"}</span>
                <span class="info-value">{ &m.member_id }</span>
                <span class="info-label">{"历史理赔数"}</span>
                <span class="info-value">{ m.claim_count }</span>
                <span class="info-label">{"高风险理赔"}</span>
                <span class="info-value">{ m.high_risk_claim_count }</span>
                <span class="info-label">{"风险等级"}</span>
                <span class="info-value">{ &m.risk_level_summary }</span>
                <span class="info-label">{"险种历史"}</span>
                <span class="info-value">{ &m.profile_summary }</span>
            </div>
        },
    };
    html! {
        <div class="evidence-card">
            <div class="evidence-card-header">
                <h4>{"成员画像"}</h4>
            </div>
            <div class="evidence-card-body">
                { body }
            </div>
        </div>
    }
}

fn card_evidence_package(evidence_refs: &[String]) -> Html {
    html! {
        <div class="evidence-card">
            <div class="evidence-card-header">
                <h4>{"证据包"}</h4>
            </div>
            <div class="evidence-card-body">
                { if evidence_refs.is_empty() {
                    html! { <p class="empty muted">{"无证据引用。"}</p> }
                } else {
                    html! {
                        <div class="evidence-list">
                            { for evidence_refs.iter().map(|r| {
                                html! {
                                    <div class="evidence-row">
                                        <span class="evidence-ref-icon">{"📎"}</span>
                                        <span class="evidence-ref-text">{ r }</span>
                                    </div>
                                }
                            }) }
                        </div>
                    }
                } }
            </div>
        </div>
    }
}

// ── Conclusion panel ───────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn conclusion_panel(
    case: Option<&CaseRecord>,
    selected_conclusion: &UseStateHandle<Option<Conclusion>>,
    saving_amount: &UseStateHandle<String>,
    notes: &UseStateHandle<String>,
    evidence_refs_input: &UseStateHandle<String>,
    submit_state: &UseStateHandle<ApiState<PilotWritebackResponse>>,
    confirm_msg: &UseStateHandle<Option<String>>,
    on_submit: Callback<MouseEvent>,
) -> Html {
    let loading = matches!(&**submit_state, ApiState::Loading);

    html! {
        <div class="conclusion-panel">
            <div class="conclusion-panel-header">
                <h3>{"调查结论"}</h3>
            </div>

            { if let Some(msg) = &**confirm_msg {
                html! {
                    <div class="alert-card info" style="margin:0 0 12px;">
                        <strong>{"提交成功"}</strong>
                        <span>{ msg }</span>
                    </div>
                }
            } else { html! {} } }

            { if let ApiState::Failed(err) = &**submit_state {
                html! {
                    <div class="alert-card critical" style="margin:0 0 12px;">
                        <strong>{"提交失败"}</strong>
                        <span>{ err }</span>
                    </div>
                }
            } else { html! {} } }

            { if case.is_none() {
                html! { <p class="empty muted">{"请选择案件。"}</p> }
            } else {
                html! {
                    <>
                    <div class="conclusion-options">
                        { for CONCLUSIONS.iter().map(|&c| {
                            let is_active  = **selected_conclusion == Some(c);
                            let sel        = selected_conclusion.clone();
                            html! {
                                <button
                                    class={classes!("conclusion-option", c.css_class(), is_active.then_some("active"))}
                                    onclick={Callback::from(move |_: MouseEvent| sel.set(Some(c)))}
                                    disabled={loading}
                                >
                                    { c.label() }
                                </button>
                            }
                        }) }
                    </div>

                    { if selected_conclusion.as_ref().map(|c| c.show_saving_amount()).unwrap_or(false) {
                        let saving_amount = saving_amount.clone();
                        html! {
                            <label class="form-field">
                                <span class="form-label">{"节省金额"}</span>
                                <input
                                    class="form-input"
                                    type="text"
                                    placeholder="例：12500.00"
                                    value={(*saving_amount).to_string()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        saving_amount.set(e.target_unchecked_into::<HtmlInputElement>().value())
                                    })}
                                />
                            </label>
                        }
                    } else { html! {} } }

                    <label class="form-field">
                        <span class="form-label">{"调查备注 *"}</span>
                        <textarea
                            class="form-textarea"
                            placeholder="请填写调查备注（必填）"
                            value={(*notes).to_string()}
                            oninput={{
                                let notes = notes.clone();
                                Callback::from(move |e: InputEvent| {
                                    notes.set(e.target_unchecked_into::<HtmlTextAreaElement>().value())
                                })
                            }}
                        />
                    </label>

                    <label class="form-field">
                        <span class="form-label">{"证据引用"}</span>
                        <input
                            class="form-input"
                            type="text"
                            placeholder="逗号分隔"
                            value={(*evidence_refs_input).to_string()}
                            oninput={{
                                let evidence_refs_input = evidence_refs_input.clone();
                                Callback::from(move |e: InputEvent| {
                                    evidence_refs_input.set(e.target_unchecked_into::<HtmlInputElement>().value())
                                })
                            }}
                        />
                    </label>

                    <button
                        class="btn-primary"
                        onclick={on_submit}
                        disabled={loading || selected_conclusion.is_none()}
                    >
                        { if loading { "提交中..." } else { "提交结论" } }
                    </button>
                    </>
                }
            } }
        </div>
    }
}

// ── Main component ─────────────────────────────────────────────────────────────

#[function_component(ReviewWorkbenchPage)]
pub fn review_workbench_page() -> Html {
    let api_key = use_api_key();
    let snapshot_state = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let selected_case_id = use_state(String::new);
    let member_state = use_state(|| ApiState::<MemberProfileSummary>::Idle);

    // Conclusion form state
    let selected_conclusion = use_state(|| Option::<Conclusion>::None);
    let saving_amount = use_state(String::new);
    let notes = use_state(String::new);
    let evidence_refs_input = use_state(String::new);
    let submit_state = use_state(|| ApiState::<PilotWritebackResponse>::Idle);
    let confirm_msg = use_state(|| Option::<String>::None);

    // Auto-load on mount
    {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let selected_case_id = selected_case_id.clone();
        use_effect_with((), move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let selected_case_id = selected_case_id.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                match get_leads_cases_snapshot(api_key).await {
                    Ok(snap) => {
                        // Auto-select first investigating case
                        let first_id = snap
                            .cases
                            .iter()
                            .find(|c| {
                                let s = c.status.to_ascii_lowercase();
                                s == "investigating" || s == "pending"
                            })
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

    // Derive filtered case list (investigating / pending)
    let reviewing_cases: Vec<CaseRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
        snap.cases
            .iter()
            .filter(|c| {
                let s = c.status.to_ascii_lowercase();
                s == "investigating" || s == "pending"
            })
            .cloned()
            .collect()
    } else {
        vec![]
    };

    let selected_case: Option<CaseRecord> = if let ApiState::Ready(snap) = &*snapshot_state {
        let id = &**selected_case_id;
        snap.cases.iter().find(|c| c.case_id == *id).cloned()
    } else {
        None
    };

    // Load member profile when selected case changes
    {
        let api_key = api_key.clone();
        let member_state = member_state.clone();
        let notes = notes.clone();
        let evidence_refs_input = evidence_refs_input.clone();
        let selected_conclusion = selected_conclusion.clone();
        let confirm_msg = confirm_msg.clone();
        let submit_state = submit_state.clone();
        let saving_amount = saving_amount.clone();
        let selected_case = selected_case.clone();

        use_effect_with((*selected_case_id).clone(), move |_| {
            // Reset form state on case change
            selected_conclusion.set(None);
            saving_amount.set(String::new());
            notes.set(String::new());
            confirm_msg.set(None);
            submit_state.set(ApiState::Idle);

            if let Some(case) = &selected_case {
                let member_id = case.member_id.clone();
                // Pre-populate evidence refs from evidence_package
                let refs = evidence_refs_from_package(&case.evidence_package);
                evidence_refs_input.set(refs.join(", "));

                let api_key = (*api_key).clone();
                let member_state = member_state.clone();
                if !member_id.is_empty() {
                    member_state.set(ApiState::Loading);
                    spawn_local(async move {
                        member_state.set(
                            match get_member_profile_summary(api_key, member_id).await {
                                Ok(m) => ApiState::Ready(m),
                                Err(e) => ApiState::Failed(e),
                            },
                        );
                    });
                }
            } else {
                evidence_refs_input.set(String::new());
                member_state.set(ApiState::Idle);
            }
            || ()
        });
    }

    // Evidence refs list for current case
    let ev_refs: Vec<String> = selected_case
        .as_ref()
        .map(|c| evidence_refs_from_package(&c.evidence_package))
        .unwrap_or_default();

    // Submit conclusion
    let on_submit = {
        let api_key = api_key.clone();
        let selected_case = selected_case.clone();
        let selected_conclusion = selected_conclusion.clone();
        let saving_amount = saving_amount.clone();
        let notes = notes.clone();
        let evidence_refs_input = evidence_refs_input.clone();
        let submit_state = submit_state.clone();
        let confirm_msg = confirm_msg.clone();
        let selected_case_id = selected_case_id.clone();
        let snapshot_state = snapshot_state.clone();

        Callback::from(move |_: MouseEvent| {
            let Some(case) = &selected_case else {
                return;
            };
            let Some(conclusion) = *selected_conclusion else {
                return;
            };
            if matches!(*submit_state, ApiState::Loading) {
                return;
            }

            let notes_val = (*notes).trim().to_string();
            if notes_val.is_empty() {
                return;
            }

            let refs: Vec<String> = (*evidence_refs_input)
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let saving_str = (*saving_amount).trim().to_string();
            let saving_val: serde_json::Value =
                if conclusion.show_saving_amount() && !saving_str.is_empty() {
                    saving_str
                        .parse::<f64>()
                        .map(|v| json!(v))
                        .unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                };

            let payload = json!({
                "claim_id":             case.claim_id,
                "investigation_id":     case.case_id,
                "case_id":              case.case_id,
                "outcome":              conclusion.outcome(),
                "confirmed_fwa":        conclusion.confirmed_fwa(),
                "financial_impact_type": if conclusion.confirmed_fwa() { "prevented_payment" } else { "none" },
                "saving_amount":        saving_val,
                "notes":                notes_val,
                "evidence_refs":        refs,
            });

            let api_key = (*api_key).clone();
            let submit_state = submit_state.clone();
            let confirm_msg = confirm_msg.clone();
            let selected_case_id = selected_case_id.clone();
            let snapshot_state = snapshot_state.clone();
            let claim_id = case.claim_id.clone();

            submit_state.set(ApiState::Loading);
            confirm_msg.set(None);

            spawn_local(async move {
                match post_investigation_result(api_key.clone(), payload).await {
                    Ok(resp) => {
                        let msg = format!(
                            "理赔 {} 已提交：{}",
                            resp.claim_id,
                            business_label(&resp.event_status)
                        );
                        submit_state.set(ApiState::Ready(resp));
                        confirm_msg.set(Some(msg));
                        // Refresh snapshot and advance to next case
                        if let Ok(snap) = get_leads_cases_snapshot(api_key).await {
                            let next_id = snap
                                .cases
                                .iter()
                                .find(|c| {
                                    let s = c.status.to_ascii_lowercase();
                                    (s == "investigating" || s == "pending")
                                        && c.claim_id != claim_id
                                })
                                .map(|c| c.case_id.clone())
                                .unwrap_or_default();
                            selected_case_id.set(next_id);
                            snapshot_state.set(ApiState::Ready(snap));
                        }
                    }
                    Err(e) => submit_state.set(ApiState::Failed(e)),
                }
            });
        })
    };

    let snap_loading = matches!(&*snapshot_state, ApiState::Loading);

    html! {
        <div class="ops-page review-workbench-page">
            <div class="dashboard-header">
                <div>
                    <h2>{"审核工作台"}</h2>
                    <p class="muted">{"调查进行中的案件，记录结论"}</p>
                </div>
            </div>

            <div class="review-layout">
                { queue_panel(&reviewing_cases, &selected_case_id, snap_loading) }

                <div class="review-evidence-panel">
                    { if snap_loading {
                        html! { <p class="empty">{"加载中..."}</p> }
                    } else if let Some(case) = &selected_case {
                        html! {
                            <>
                            { card_claim_info(case) }
                            { card_risk_signals(case) }
                            { card_member_profile(&member_state) }
                            { card_evidence_package(&ev_refs) }
                            </>
                        }
                    } else {
                        html! { <p class="empty">{"点击左侧案件以查看证据。"}</p> }
                    } }
                </div>

                { conclusion_panel(
                    selected_case.as_ref(),
                    &selected_conclusion,
                    &saving_amount,
                    &notes,
                    &evidence_refs_input,
                    &submit_state,
                    &confirm_msg,
                    on_submit,
                ) }
            </div>
        </div>
    }
}
