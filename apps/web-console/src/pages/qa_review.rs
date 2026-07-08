use crate::api::*;
use crate::data_helpers::*;
use crate::state::{use_api_key, ApiState};
use crate::types::*;
use crate::ui_helpers::*;
use serde_json::json;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

#[function_component(QaReviewPage)]
pub fn qa_review_page() -> Html {
    let api_key = use_api_key();
    let snapshot_state = use_state(|| ApiState::<QaReviewSnapshot>::Idle);

    // Writeback form state
    let selected_qa_case_id = use_state(String::new);
    let qa_conclusion = use_state(|| "pass".to_string());
    let issue_type = use_state(|| "none".to_string());
    let feedback_target = use_state(|| "rules".to_string());
    let qa_notes = use_state(String::new);
    let qa_evidence_refs = use_state(String::new);
    let qa_write_state = use_state(|| ApiState::<PilotWritebackResponse>::Idle);

    let load_qa_review = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let selected_qa_case_id = selected_qa_case_id.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let selected_qa_case_id = selected_qa_case_id.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                match get_qa_review_snapshot(api_key).await {
                    Ok(snapshot) => {
                        if selected_qa_case_id.is_empty() {
                            if let Some(first) = snapshot.queue.first() {
                                selected_qa_case_id.set(first.qa_case_id.clone());
                            }
                        }
                        snapshot_state.set(ApiState::Ready(snapshot));
                    }
                    Err(error) => snapshot_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let refresh = {
        let load_qa_review = load_qa_review.clone();
        Callback::from(move |_| load_qa_review.emit(()))
    };

    let submit_qa_review = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let selected_qa_case_id = selected_qa_case_id.clone();
        let qa_conclusion = qa_conclusion.clone();
        let issue_type = issue_type.clone();
        let feedback_target = feedback_target.clone();
        let qa_notes = qa_notes.clone();
        let qa_evidence_refs = qa_evidence_refs.clone();
        let qa_write_state = qa_write_state.clone();
        let load_qa_review = load_qa_review.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                qa_write_state.set(ApiState::Failed(
                    "请先加载 QA 审核数据再回写".into(),
                ));
                return;
            };
            let qa_case_id = (*selected_qa_case_id).clone();
            let claim_id = snapshot
                .queue
                .iter()
                .find(|item| item.qa_case_id == qa_case_id)
                .map(|item| item.claim_id.clone())
                .unwrap_or_default();
            let evidence_refs = parse_tags(&qa_evidence_refs);
            let payload = json!({
                "qa_case_id": qa_case_id,
                "claim_id": claim_id,
                "qa_conclusion": (*qa_conclusion).clone(),
                "issue_type": (*issue_type).clone(),
                "feedback_target": (*feedback_target).clone(),
                "notes": (*qa_notes).clone(),
                "evidence_refs": evidence_refs,
            });
            let api_key = (*api_key).clone();
            let qa_write_state = qa_write_state.clone();
            let load_qa_review = load_qa_review.clone();
            qa_write_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_qa_review(api_key, payload).await {
                    Ok(response) => {
                        qa_write_state.set(ApiState::Ready(response));
                        load_qa_review.emit(());
                    }
                    Err(error) => qa_write_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    {
        let load_qa_review = load_qa_review.clone();
        use_effect_with((), move |_| {
            load_qa_review.emit(());
            || ()
        });
    }

    let queue_items = match &*snapshot_state {
        ApiState::Ready(snapshot) => snapshot.queue.clone(),
        _ => vec![],
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"QA 审核"}</h2>
                    <p>{"在路由变更或模型发布前，检查抽样 QA 案例、未关闭反馈、证据覆盖和闭环信号。"}</p>
                </div>
                <span class="status-pill">{"QA 反馈闭环"}</span>
            </div>

            <section class="panel">
                <h3>{"QA 来源"}</h3>
                <p class="empty">{"使用已配置的 QA 反馈工作区读取抽样审核、未关闭反馈和闭环信号。"}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "刷新中..." } else { "刷新 QA 审核" }}
                    </button>
                </div>
            </section>

            <QaReviewView state={(*snapshot_state).clone()} />

            <section class="panel result-stack">
                <h3>{"QA 回写"}</h3>
                <p class="empty">{"为抽样案例提交 QA 结论。所有字段都需要填写。"}</p>
                <div class="form-grid action-form-grid">
                    <label>
                        {"QA 案例"}
                        <select
                            onchange={{
                                let selected_qa_case_id = selected_qa_case_id.clone();
                                Callback::from(move |event: Event| {
                                    selected_qa_case_id.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                })
                            }}
                        >
                            {if queue_items.is_empty() {
                                html! { <option value="">{"- 请先加载 QA 审核 -"}</option> }
                            } else {
                                html! {
                                    {for queue_items.iter().map(|item| {
                                        let val = item.qa_case_id.clone();
                                        let label = format!("{} / {}", item.qa_case_id, item.claim_id);
                                        let selected = *selected_qa_case_id == val;
                                        html! { <option value={val} selected={selected}>{label}</option> }
                                    })}
                                }
                            }}
                        </select>
                    </label>
                    <label>
                        {"结论"}
                        <select
                            onchange={{
                                let qa_conclusion = qa_conclusion.clone();
                                Callback::from(move |event: Event| {
                                    qa_conclusion.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                })
                            }}
                        >
                            <option value="pass" selected={(*qa_conclusion).as_str() == "pass"}>{"通过"}</option>
                            <option value="issue_found_return" selected={(*qa_conclusion).as_str() == "issue_found_return"}>{"发现问题 - 退回"}</option>
                            <option value="issue_found_escalate" selected={(*qa_conclusion).as_str() == "issue_found_escalate"}>{"发现问题 - 升级"}</option>
                        </select>
                    </label>
                    <label>
                        {"问题类型"}
                        <select
                            onchange={{
                                let issue_type = issue_type.clone();
                                Callback::from(move |event: Event| {
                                    issue_type.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                })
                            }}
                        >
                            <option value="none" selected={(*issue_type).as_str() == "none"}>{"无"}</option>
                            <option value="confirmed_fwa" selected={(*issue_type).as_str() == "confirmed_fwa"}>{"确认 FWA"}</option>
                            <option value="false_positive" selected={(*issue_type).as_str() == "false_positive"}>{"误报"}</option>
                            <option value="improper_payment" selected={(*issue_type).as_str() == "improper_payment"}>{"不当支付"}</option>
                            <option value="insufficient_evidence" selected={(*issue_type).as_str() == "insufficient_evidence"}>{"证据不足"}</option>
                            <option value="abuse_not_fraud" selected={(*issue_type).as_str() == "abuse_not_fraud"}>{"滥用但非欺诈"}</option>
                            <option value="documentation_issue" selected={(*issue_type).as_str() == "documentation_issue"}>{"文档问题"}</option>
                            <option value="medical_necessity_issue" selected={(*issue_type).as_str() == "medical_necessity_issue"}>{"医疗必要性问题"}</option>
                            <option value="policy_exclusion" selected={(*issue_type).as_str() == "policy_exclusion"}>{"保单除外责任"}</option>
                            <option value="qa_review_completed" selected={(*issue_type).as_str() == "qa_review_completed"}>{"QA 审核完成"}</option>
                            <option value="alert_handling_incomplete" selected={(*issue_type).as_str() == "alert_handling_incomplete"}>{"告警处理不完整"}</option>
                            <option value="medical_reasonableness" selected={(*issue_type).as_str() == "medical_reasonableness"}>{"医疗合理性"}</option>
                            <option value="provider_pattern" selected={(*issue_type).as_str() == "provider_pattern"}>{"机构模式"}</option>
                            <option value="model_under_scored_confirmed_issue" selected={(*issue_type).as_str() == "model_under_scored_confirmed_issue"}>{"模型低估的已确认问题"}</option>
                            <option value="workflow_missing_evidence" selected={(*issue_type).as_str() == "workflow_missing_evidence"}>{"流程缺少证据"}</option>
                        </select>
                    </label>
                    <label>
                        {"反馈目标"}
                        <select
                            onchange={{
                                let feedback_target = feedback_target.clone();
                                Callback::from(move |event: Event| {
                                    feedback_target.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                })
                            }}
                        >
                            <option value="rules" selected={(*feedback_target).as_str() == "rules"}>{"规则"}</option>
                            <option value="model" selected={(*feedback_target).as_str() == "model"}>{"模型"}</option>
                            <option value="features" selected={(*feedback_target).as_str() == "features"}>{"特征"}</option>
                            <option value="provider_profile" selected={(*feedback_target).as_str() == "provider_profile"}>{"机构画像"}</option>
                            <option value="workflow" selected={(*feedback_target).as_str() == "workflow"}>{"流程"}</option>
                            <option value="tpa" selected={(*feedback_target).as_str() == "tpa"}>{"TPA"}</option>
                        </select>
                    </label>
                    <label>
                        {"证据引用（逗号分隔）"}
                        <input
                            value={(*qa_evidence_refs).clone()}
                            placeholder={"ref-001, ref-002"}
                            oninput={{
                                let qa_evidence_refs = qa_evidence_refs.clone();
                                Callback::from(move |event: InputEvent| {
                                    qa_evidence_refs.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <label class="compact-note">
                    {"备注"}
                    <textarea
                        value={(*qa_notes).clone()}
                        oninput={{
                            let qa_notes = qa_notes.clone();
                            Callback::from(move |event: InputEvent| {
                                qa_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <div class="button-row">
                    <button
                        onclick={submit_qa_review}
                        disabled={matches!(&*qa_write_state, ApiState::Loading)}
                    >
                        {if matches!(&*qa_write_state, ApiState::Loading) { "提交中..." } else { "提交 QA 审核" }}
                    </button>
                </div>
                <QaWritebackResultView state={(*qa_write_state).clone()} />
            </section>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct QaReviewProps {
    state: ApiState<QaReviewSnapshot>,
}

#[function_component(QaReviewView)]
fn qa_review_view(props: &QaReviewProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"加载 QA 审核后可检查队列和反馈闭环。"}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"正在加载 QA 审核..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {qa_feedback_loop_cockpit(snapshot)}

                        <section class="panel result-stack">
                            <h3>{"QA 队列摘要"}</h3>
                            <div class="score-hero">
                                <div><span>{"开放"}</span><strong>{snapshot.summary.open_count}</strong></div>
                                <div><span>{"未解决"}</span><strong>{snapshot.summary.unresolved_count}</strong></div>
                                <div><span>{"最高优先级"}</span><strong>{priority_label(&snapshot.summary.highest_priority)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"处理中"}</span><strong>{snapshot.summary.in_progress_count}</strong></div>
                                <div><span>{"已解决"}</span><strong>{snapshot.summary.resolved_count}</strong></div>
                                <div><span>{"已驳回"}</span><strong>{snapshot.summary.dismissed_count}</strong></div>
                                <div><span>{"高优先级"}</span><strong>{snapshot.summary.high_priority_count}</strong></div>
                                <div><span>{"有证据支撑"}</span><strong>{snapshot.summary.evidence_backed_count}</strong></div>
                                <div><span>{"队列项"}</span><strong>{snapshot.queue.len()}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"规则反馈"}</span><strong>{snapshot.summary.rules_feedback_count}</strong></div>
                                <div><span>{"模型反馈"}</span><strong>{snapshot.summary.models_feedback_count}</strong></div>
                                <div><span>{"特征反馈"}</span><strong>{snapshot.summary.features_feedback_count}</strong></div>
                                <div><span>{"机构反馈"}</span><strong>{snapshot.summary.provider_profile_feedback_count}</strong></div>
                                <div><span>{"流程反馈"}</span><strong>{snapshot.summary.workflow_feedback_count}</strong></div>
                                <div><span>{"TPA 反馈"}</span><strong>{snapshot.summary.tpa_feedback_count}</strong></div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"审核发现"}</h3>
                            if snapshot.queue.is_empty() {
                                <p class="empty">{"队列中没有抽样 QA 案例。"}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.queue.iter().map(qa_queue_card)}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"反馈闭环"}</h3>
                            if snapshot.feedback_items.is_empty() {
                                <p class="empty">{"没有返回 QA 反馈项。"}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.feedback_items.iter().map(qa_feedback_card)}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn qa_conclusion_label(value: &str) -> &str {
    match value {
        "pass" => "通过",
        "issue_found_return" => "发现问题 - 退回",
        "issue_found_escalate" => "发现问题 - 升级",
        "pending" => "待定",
        _ => value,
    }
}

fn issue_type_label(value: &str) -> &str {
    match value {
        "none" => "无",
        "confirmed_fwa" => "确认 FWA",
        "false_positive" => "误报",
        "improper_payment" => "不当支付",
        "insufficient_evidence" => "证据不足",
        "abuse_not_fraud" => "滥用但非欺诈",
        "documentation_issue" => "文档问题",
        "medical_necessity_issue" => "医疗必要性问题",
        "policy_exclusion" => "保单除外责任",
        "qa_review_completed" => "QA 审核完成",
        "alert_handling_incomplete" => "告警处理不完整",
        "medical_reasonableness" => "医疗合理性",
        "provider_pattern" => "机构模式",
        "model_under_scored_confirmed_issue" => "模型低估的已确认问题",
        "workflow_missing_evidence" => "流程缺少证据",
        "pending" => "待定",
        _ => value,
    }
}

fn feedback_target_label(value: &str) -> &str {
    match value {
        "rules" => "规则",
        "model" => "模型",
        "features" => "特征",
        "provider_profile" => "机构画像",
        "workflow" => "流程",
        "tpa" => "TPA",
        "pending" => "待定",
        _ => value,
    }
}

fn status_label(value: &str) -> &str {
    match value {
        "open" => "开放",
        "in_progress" => "处理中",
        "resolved" => "已解决",
        "dismissed" => "已驳回",
        "done" => "已完成",
        "review" => "复核中",
        "ready" => "就绪",
        "pending" => "待定",
        _ => value,
    }
}

fn priority_label(value: &str) -> &str {
    match value {
        "critical" => "严重",
        "high" => "高",
        "medium" => "中",
        "low" => "低",
        "none" => "无",
        _ => value,
    }
}

fn qa_feedback_loop_cockpit(snapshot: &QaReviewSnapshot) -> Html {
    let selected_queue = snapshot.queue.first();
    let selected_feedback = selected_queue
        .and_then(|queue| {
            snapshot
                .feedback_items
                .iter()
                .find(|feedback| feedback.qa_case_id == queue.qa_case_id)
        })
        .or_else(|| snapshot.feedback_items.first());
    let qa_case_id = selected_queue
        .map(|item| item.qa_case_id.as_str())
        .or_else(|| selected_feedback.map(|item| item.qa_case_id.as_str()))
        .unwrap_or("无 QA 案例");
    let claim_id = selected_queue
        .map(|item| item.claim_id.as_str())
        .or_else(|| selected_feedback.map(|item| item.claim_id.as_str()))
        .unwrap_or("无理赔");
    let conclusion = selected_queue
        .and_then(|item| item.qa_conclusion.as_deref())
        .or_else(|| selected_feedback.map(|item| item.qa_conclusion.as_str()))
        .unwrap_or("pending");
    let issue_type = selected_queue
        .and_then(|item| item.issue_type.as_deref())
        .or_else(|| selected_feedback.map(|item| item.issue_type.as_str()))
        .unwrap_or("pending");
    let feedback_target = selected_queue
        .and_then(|item| item.feedback_target.as_deref())
        .or_else(|| selected_feedback.map(|item| item.feedback_target.as_str()))
        .unwrap_or("pending");
    let feedback_status = selected_feedback
        .map(|item| item.status.as_str())
        .or_else(|| selected_queue.map(|item| item.status.as_str()))
        .unwrap_or("pending");
    let status_audit = selected_feedback
        .and_then(|item| item.status_audit_id.as_deref())
        .unwrap_or("审计待定");
    let evidence_count = selected_queue
        .map(|item| {
            item.evidence_refs.len()
                + item.canonical_source_refs.len()
                + item.canonical_evidence_refs.len()
        })
        .or_else(|| selected_feedback.map(|item| item.evidence_refs.len()))
        .unwrap_or(0);
    let evidence_label = if evidence_count == 0 {
        "证据待定".into()
    } else {
        format!("{evidence_count} 条证据引用")
    };
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"QA 反馈闭环驾驶舱"}</h3>
                    <p>{"抽样审核发现会进入受治理的反馈目标，用于规则、模型、特征、机构、流程和 TPA 修复。"}</p>
                </div>
                <span class={classes!("status-token", status_tone(feedback_status))}>{status_label(feedback_status)}</span>
            </div>
            <div class="qa-cockpit">
                <aside class="case-brief qa-brief">
                    <span>{"所选 QA 案例"}</span>
                    <strong>{qa_case_id}</strong>
                    <dl>
                        <div><dt>{"理赔"}</dt><dd>{claim_id}</dd></div>
                        <div><dt>{"结论"}</dt><dd>{qa_conclusion_label(conclusion)}</dd></div>
                        <div><dt>{"问题"}</dt><dd>{issue_type_label(issue_type)}</dd></div>
                        <div><dt>{"目标"}</dt><dd>{feedback_target_label(feedback_target)}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("开放 {}", snapshot.summary.open_count)}</span>
                        <span>{format!("未解决 {}", snapshot.summary.unresolved_count)}</span>
                        <span>{format!("有证据 {}", snapshot.summary.evidence_backed_count)}</span>
                    </div>
                </aside>

                <div class="qa-loop-map">
                    <div class="qa-map-title">
                        <span>{"QA 闭环路由"}</span>
                        <strong>{format!("{} -> {}", issue_type_label(issue_type), feedback_target_label(feedback_target))}</strong>
                    </div>
                    <div class="qa-link horizontal"></div>
                    <div class="qa-link diagonal-a"></div>
                    <div class="qa-link diagonal-b"></div>
                    <div class="qa-core">
                        <span>{"QA"}</span>
                        <strong>{"反馈门禁"}</strong>
                    </div>
                    <div class="qa-node sample">
                        <span>{"抽样案例"}</span>
                        <strong>{claim_id}</strong>
                    </div>
                    <div class="qa-node reviewer">
                        <span>{"审核结论"}</span>
                        <strong>{qa_conclusion_label(conclusion)}</strong>
                    </div>
                    <div class="qa-node target">
                        <span>{"反馈目标"}</span>
                        <strong>{feedback_target_label(feedback_target)}</strong>
                    </div>
                    <div class="qa-node evidence">
                        <span>{"规范证据"}</span>
                        <strong>{evidence_label}</strong>
                    </div>
                    <div class="qa-node audit">
                        <span>{"审计状态"}</span>
                        <strong>{status_audit}</strong>
                    </div>
                </div>

                <aside class="case-timeline qa-trace">
                    <h4>{"反馈闭环路径"}</h4>
                    {timeline_item("抽样", qa_case_id, "done")}
                    {timeline_item("审核", qa_conclusion_label(conclusion), "review")}
                    {timeline_item("路由", feedback_target_label(feedback_target), "ready")}
                    {timeline_item("闭环", status_label(feedback_status), feedback_status)}
                </aside>
            </div>
        </section>
    }
}

fn qa_queue_card(item: &QaQueueItem) -> Html {
    let conclusion = item.qa_conclusion.as_deref().unwrap_or("pending");
    let issue = item.issue_type.as_deref().unwrap_or("pending");
    let feedback = item.feedback_target.as_deref().unwrap_or("pending");
    let evidence_count = item.evidence_refs.len()
        + item.canonical_source_refs.len()
        + item.canonical_evidence_refs.len();

    html! {
        <div class="factor-card qa-review-card">
            <div>
                <strong>{format!("{} / {}", item.qa_case_id, item.claim_id)}</strong>
                <span>{format!("{} / {} / {}", item.scheme_family, item.rag, item.assignment_queue)}</span>
            </div>
            <div class="summary-grid">
                <div><span>{"风险分"}</span><strong>{item.risk_score}</strong></div>
                <div><span>{"状态"}</span><strong>{status_label(&item.status)}</strong></div>
                <div><span>{"审核人"}</span><strong>{&item.reviewer}</strong></div>
                <div><span>{"结论"}</span><strong>{qa_conclusion_label(conclusion)}</strong></div>
                <div><span>{"问题"}</span><strong>{issue_type_label(issue)}</strong></div>
                <div><span>{"反馈目标"}</span><strong>{feedback_target_label(feedback)}</strong></div>
                <div><span>{"证据包"}</span><strong>{format!("{} 条引用", evidence_count)}</strong></div>
                <div><span>{"样本"}</span><strong>{&item.sample_id}</strong></div>
            </div>
            <small>{format!("线索：{}", item.lead_id)}</small>
            {qa_evidence_details(
                "QA 证据详情",
                &[
                    ("运营引用", &item.evidence_refs),
                    ("来源追踪", &item.canonical_source_refs),
                    ("规范证据", &item.canonical_evidence_refs),
                ],
            )}
        </div>
    }
}

fn qa_feedback_card(item: &QaFeedbackItem) -> Html {
    html! {
        <div class="factor-card qa-review-card">
            <div>
                <strong>{format!("{} / {}", item.feedback_id, item.feedback_target)}</strong>
                <span>{format!("{} / {} / {}", item.priority, item.status, item.source)}</span>
            </div>
            <p>{&item.summary}</p>
            <div class="summary-grid">
                <div><span>{"理赔"}</span><strong>{&item.claim_id}</strong></div>
                <div><span>{"QA 案例"}</span><strong>{&item.qa_case_id}</strong></div>
                <div><span>{"问题"}</span><strong>{issue_type_label(&item.issue_type)}</strong></div>
                <div><span>{"结论"}</span><strong>{qa_conclusion_label(&item.qa_conclusion)}</strong></div>
                <div><span>{"备注"}</span><strong>{if item.note_present { "有" } else { "无" }}</strong></div>
                <div><span>{"更新人"}</span><strong>{item.status_updated_by.as_deref().unwrap_or("待定")}</strong></div>
                <div><span>{"状态审计"}</span><strong>{item.status_audit_id.as_deref().unwrap_or("待定")}</strong></div>
            </div>
            <small>{format!("创建：{} / 更新：{}", item.created_at.as_deref().unwrap_or("未知"), item.status_updated_at.as_deref().unwrap_or("待定"))}</small>
            {qa_evidence_details(
                "闭环证据详情",
                &[
                    ("反馈证据", &item.evidence_refs),
                    ("状态证据", &item.status_evidence_refs),
                ],
            )}
        </div>
    }
}

fn qa_evidence_details(title: &str, groups: &[(&str, &Vec<String>)]) -> Html {
    let total_refs: usize = groups.iter().map(|(_, refs)| refs.len()).sum();
    html! {
        <details class="qa-evidence-details">
            <summary>{format!("{title}：{total_refs} 条引用")}</summary>
            <div class="qa-evidence-detail-grid">
                {for groups.iter().map(|(label, refs)| html! {
                    <div>
                        <span>{format!("{} ({})", label, refs.len())}</span>
                        <small>{refs_label(refs)}</small>
                    </div>
                })}
            </div>
        </details>
    }
}

#[derive(Properties, PartialEq)]
struct QaWritebackResultProps {
    state: ApiState<PilotWritebackResponse>,
}

#[function_component(QaWritebackResultView)]
fn qa_writeback_result_view(props: &QaWritebackResultProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"选择 QA 案例并提交受控结论。"}</p> }
        }
        ApiState::Loading => html! { <p>{"正在提交 QA 审核..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="summary-grid">
                <div><span>{"理赔"}</span><strong>{&response.claim_id}</strong></div>
                <div><span>{"事件"}</span><strong>{&response.event_type}</strong></div>
                <div><span>{"状态"}</span><strong>{status_label(&response.event_status)}</strong></div>
                <div><span>{"审计"}</span><strong>{&response.audit_id}</strong></div>
                <div><span>{"运行"}</span><strong>{&response.run_id}</strong></div>
                <div><span>{"证据"}</span><strong>{refs_label(&response.evidence_refs)}</strong></div>
            </div>
        },
    }
}
