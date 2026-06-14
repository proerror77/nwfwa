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
                    "load the QA review before writeback".into(),
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
                    <h2>{"QA Review"}</h2>
                    <p>{"Inspect sampled QA cases, unresolved feedback, evidence coverage, and closure signals before routing changes or model promotion."}</p>
                </div>
                <span class="status-pill">{"QA Feedback Loop"}</span>
            </div>

            <section class="panel">
                <h3>{"QA Source"}</h3>
                <p class="empty">{"Using the configured QA feedback workspace for sampled reviews, unresolved feedback, and closure signals."}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh QA review" }}
                    </button>
                </div>
            </section>

            <QaReviewView state={(*snapshot_state).clone()} />

            <section class="panel result-stack">
                <h3>{"QA Writeback"}</h3>
                <p class="empty">{"Submit a QA conclusion for a sampled case. All fields are required."}</p>
                <div class="form-grid action-form-grid">
                    <label>
                        {"QA Case"}
                        <select
                            onchange={{
                                let selected_qa_case_id = selected_qa_case_id.clone();
                                Callback::from(move |event: Event| {
                                    selected_qa_case_id.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                })
                            }}
                        >
                            {if queue_items.is_empty() {
                                html! { <option value="">{"— load QA review first —"}</option> }
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
                        {"Conclusion"}
                        <select
                            onchange={{
                                let qa_conclusion = qa_conclusion.clone();
                                Callback::from(move |event: Event| {
                                    qa_conclusion.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                })
                            }}
                        >
                            <option value="pass" selected={(*qa_conclusion).as_str() == "pass"}>{"Pass"}</option>
                            <option value="issue_found_return" selected={(*qa_conclusion).as_str() == "issue_found_return"}>{"Issue found — return"}</option>
                            <option value="issue_found_escalate" selected={(*qa_conclusion).as_str() == "issue_found_escalate"}>{"Issue found — escalate"}</option>
                        </select>
                    </label>
                    <label>
                        {"Issue Type"}
                        <select
                            onchange={{
                                let issue_type = issue_type.clone();
                                Callback::from(move |event: Event| {
                                    issue_type.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                })
                            }}
                        >
                            <option value="none" selected={(*issue_type).as_str() == "none"}>{"None"}</option>
                            <option value="confirmed_fwa" selected={(*issue_type).as_str() == "confirmed_fwa"}>{"Confirmed FWA"}</option>
                            <option value="false_positive" selected={(*issue_type).as_str() == "false_positive"}>{"False Positive"}</option>
                            <option value="improper_payment" selected={(*issue_type).as_str() == "improper_payment"}>{"Improper Payment"}</option>
                            <option value="insufficient_evidence" selected={(*issue_type).as_str() == "insufficient_evidence"}>{"Insufficient Evidence"}</option>
                            <option value="abuse_not_fraud" selected={(*issue_type).as_str() == "abuse_not_fraud"}>{"Abuse Not Fraud"}</option>
                            <option value="documentation_issue" selected={(*issue_type).as_str() == "documentation_issue"}>{"Documentation Issue"}</option>
                            <option value="medical_necessity_issue" selected={(*issue_type).as_str() == "medical_necessity_issue"}>{"Medical Necessity Issue"}</option>
                            <option value="policy_exclusion" selected={(*issue_type).as_str() == "policy_exclusion"}>{"Policy Exclusion"}</option>
                            <option value="qa_review_completed" selected={(*issue_type).as_str() == "qa_review_completed"}>{"QA Review Completed"}</option>
                            <option value="alert_handling_incomplete" selected={(*issue_type).as_str() == "alert_handling_incomplete"}>{"Alert Handling Incomplete"}</option>
                            <option value="medical_reasonableness" selected={(*issue_type).as_str() == "medical_reasonableness"}>{"Medical Reasonableness"}</option>
                            <option value="provider_pattern" selected={(*issue_type).as_str() == "provider_pattern"}>{"Provider Pattern"}</option>
                            <option value="model_under_scored_confirmed_issue" selected={(*issue_type).as_str() == "model_under_scored_confirmed_issue"}>{"Model Under-scored Confirmed Issue"}</option>
                            <option value="workflow_missing_evidence" selected={(*issue_type).as_str() == "workflow_missing_evidence"}>{"Workflow Missing Evidence"}</option>
                        </select>
                    </label>
                    <label>
                        {"Feedback Target"}
                        <select
                            onchange={{
                                let feedback_target = feedback_target.clone();
                                Callback::from(move |event: Event| {
                                    feedback_target.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                })
                            }}
                        >
                            <option value="rules" selected={(*feedback_target).as_str() == "rules"}>{"Rules"}</option>
                            <option value="model" selected={(*feedback_target).as_str() == "model"}>{"Model"}</option>
                            <option value="features" selected={(*feedback_target).as_str() == "features"}>{"Features"}</option>
                            <option value="provider_profile" selected={(*feedback_target).as_str() == "provider_profile"}>{"Provider Profile"}</option>
                            <option value="workflow" selected={(*feedback_target).as_str() == "workflow"}>{"Workflow"}</option>
                            <option value="tpa" selected={(*feedback_target).as_str() == "tpa"}>{"TPA"}</option>
                        </select>
                    </label>
                    <label>
                        {"Evidence Refs (comma-separated)"}
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
                    {"Notes"}
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
                        {if matches!(&*qa_write_state, ApiState::Loading) { "Submitting..." } else { "Submit QA Review" }}
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
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load QA review to inspect queue and feedback closure."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading QA review..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {qa_feedback_loop_cockpit(snapshot)}

                        <section class="panel result-stack">
                            <h3>{"QA Queue Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Open"}</span><strong>{snapshot.summary.open_count}</strong></div>
                                <div><span>{"Unresolved"}</span><strong>{snapshot.summary.unresolved_count}</strong></div>
                                <div><span>{"Highest Priority"}</span><strong>{&snapshot.summary.highest_priority}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"In Progress"}</span><strong>{snapshot.summary.in_progress_count}</strong></div>
                                <div><span>{"Resolved"}</span><strong>{snapshot.summary.resolved_count}</strong></div>
                                <div><span>{"Dismissed"}</span><strong>{snapshot.summary.dismissed_count}</strong></div>
                                <div><span>{"High Priority"}</span><strong>{snapshot.summary.high_priority_count}</strong></div>
                                <div><span>{"Evidence Backed"}</span><strong>{snapshot.summary.evidence_backed_count}</strong></div>
                                <div><span>{"Queue Items"}</span><strong>{snapshot.queue.len()}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Rules Feedback"}</span><strong>{snapshot.summary.rules_feedback_count}</strong></div>
                                <div><span>{"Models Feedback"}</span><strong>{snapshot.summary.models_feedback_count}</strong></div>
                                <div><span>{"Features Feedback"}</span><strong>{snapshot.summary.features_feedback_count}</strong></div>
                                <div><span>{"Provider Feedback"}</span><strong>{snapshot.summary.provider_profile_feedback_count}</strong></div>
                                <div><span>{"Workflow Feedback"}</span><strong>{snapshot.summary.workflow_feedback_count}</strong></div>
                                <div><span>{"TPA Feedback"}</span><strong>{snapshot.summary.tpa_feedback_count}</strong></div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Review Findings"}</h3>
                            if snapshot.queue.is_empty() {
                                <p class="empty">{"No sampled QA cases in the queue."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.queue.iter().map(qa_queue_card)}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Feedback Closure"}</h3>
                            if snapshot.feedback_items.is_empty() {
                                <p class="empty">{"No QA feedback items returned."}</p>
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
        .unwrap_or("no qa case");
    let claim_id = selected_queue
        .map(|item| item.claim_id.as_str())
        .or_else(|| selected_feedback.map(|item| item.claim_id.as_str()))
        .unwrap_or("no claim");
    let conclusion = selected_queue
        .and_then(|item| item.qa_conclusion.as_deref())
        .or_else(|| selected_feedback.map(|item| item.qa_conclusion.as_str()))
        .unwrap_or("pending");
    let issue_type = selected_queue
        .and_then(|item| item.issue_type.as_deref())
        .or_else(|| selected_feedback.map(|item| item.issue_type.as_str()))
        .unwrap_or("issue pending");
    let feedback_target = selected_queue
        .and_then(|item| item.feedback_target.as_deref())
        .or_else(|| selected_feedback.map(|item| item.feedback_target.as_str()))
        .unwrap_or("target pending");
    let feedback_status = selected_feedback
        .map(|item| item.status.as_str())
        .or_else(|| selected_queue.map(|item| item.status.as_str()))
        .unwrap_or("status pending");
    let status_audit = selected_feedback
        .and_then(|item| item.status_audit_id.as_deref())
        .unwrap_or("audit pending");
    let evidence_count = selected_queue
        .map(|item| {
            item.evidence_refs.len()
                + item.canonical_source_refs.len()
                + item.canonical_evidence_refs.len()
        })
        .or_else(|| selected_feedback.map(|item| item.evidence_refs.len()))
        .unwrap_or(0);
    let evidence_label = if evidence_count == 0 {
        "evidence pending".into()
    } else {
        format!("{evidence_count} evidence refs")
    };
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"QA feedback loop cockpit"}</h3>
                    <p>{"Sampled review findings move into governed feedback targets for rule, model, feature, provider, workflow, and TPA remediation."}</p>
                </div>
                <span class={classes!("status-token", status_tone(feedback_status))}>{feedback_status}</span>
            </div>
            <div class="qa-cockpit">
                <aside class="case-brief qa-brief">
                    <span>{"Selected QA case"}</span>
                    <strong>{qa_case_id}</strong>
                    <dl>
                        <div><dt>{"Claim"}</dt><dd>{claim_id}</dd></div>
                        <div><dt>{"Conclusion"}</dt><dd>{conclusion}</dd></div>
                        <div><dt>{"Issue"}</dt><dd>{issue_type}</dd></div>
                        <div><dt>{"Target"}</dt><dd>{feedback_target}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("open {}", snapshot.summary.open_count)}</span>
                        <span>{format!("unresolved {}", snapshot.summary.unresolved_count)}</span>
                        <span>{format!("evidence backed {}", snapshot.summary.evidence_backed_count)}</span>
                    </div>
                </aside>

                <div class="qa-loop-map">
                    <div class="qa-map-title">
                        <span>{"QA closed-loop routing"}</span>
                        <strong>{format!("{} -> {}", issue_type, feedback_target)}</strong>
                    </div>
                    <div class="qa-link horizontal"></div>
                    <div class="qa-link diagonal-a"></div>
                    <div class="qa-link diagonal-b"></div>
                    <div class="qa-core">
                        <span>{"QA"}</span>
                        <strong>{"feedback gate"}</strong>
                    </div>
                    <div class="qa-node sample">
                        <span>{"Sampled case"}</span>
                        <strong>{claim_id}</strong>
                    </div>
                    <div class="qa-node reviewer">
                        <span>{"Reviewer finding"}</span>
                        <strong>{conclusion}</strong>
                    </div>
                    <div class="qa-node target">
                        <span>{"Feedback target"}</span>
                        <strong>{feedback_target}</strong>
                    </div>
                    <div class="qa-node evidence">
                        <span>{"Canonical evidence"}</span>
                        <strong>{evidence_label}</strong>
                    </div>
                    <div class="qa-node audit">
                        <span>{"Audit status"}</span>
                        <strong>{status_audit}</strong>
                    </div>
                </div>

                <aside class="case-timeline qa-trace">
                    <h4>{"Feedback closure path"}</h4>
                    {timeline_item("Sample", qa_case_id, "done")}
                    {timeline_item("Review", conclusion, "review")}
                    {timeline_item("Route", feedback_target, "ready")}
                    {timeline_item("Closure", feedback_status, feedback_status)}
                </aside>
            </div>
        </section>
    }
}

fn qa_queue_card(item: &QaQueueItem) -> Html {
    let conclusion = item.qa_conclusion.as_deref().unwrap_or("pending");
    let issue = item.issue_type.as_deref().unwrap_or("pending");
    let feedback = item.feedback_target.as_deref().unwrap_or("not routed");
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
                <div><span>{"Risk Score"}</span><strong>{item.risk_score}</strong></div>
                <div><span>{"Status"}</span><strong>{&item.status}</strong></div>
                <div><span>{"Reviewer"}</span><strong>{&item.reviewer}</strong></div>
                <div><span>{"Conclusion"}</span><strong>{conclusion}</strong></div>
                <div><span>{"Issue"}</span><strong>{issue}</strong></div>
                <div><span>{"Feedback target"}</span><strong>{feedback}</strong></div>
                <div><span>{"Evidence package"}</span><strong>{format!("{} refs", evidence_count)}</strong></div>
                <div><span>{"Sample"}</span><strong>{&item.sample_id}</strong></div>
            </div>
            <small>{format!("lead: {}", item.lead_id)}</small>
            {qa_evidence_details(
                "QA evidence detail",
                &[
                    ("Operational refs", &item.evidence_refs),
                    ("Source trace", &item.canonical_source_refs),
                    ("Canonical evidence", &item.canonical_evidence_refs),
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
                <div><span>{"Claim"}</span><strong>{&item.claim_id}</strong></div>
                <div><span>{"QA Case"}</span><strong>{&item.qa_case_id}</strong></div>
                <div><span>{"Issue"}</span><strong>{&item.issue_type}</strong></div>
                <div><span>{"Conclusion"}</span><strong>{&item.qa_conclusion}</strong></div>
                <div><span>{"Notes"}</span><strong>{yes_no(item.note_present)}</strong></div>
                <div><span>{"Updated By"}</span><strong>{item.status_updated_by.as_deref().unwrap_or("pending")}</strong></div>
                <div><span>{"Status audit"}</span><strong>{item.status_audit_id.as_deref().unwrap_or("pending")}</strong></div>
            </div>
            <small>{format!("created: {} / updated: {}", item.created_at.as_deref().unwrap_or("unknown"), item.status_updated_at.as_deref().unwrap_or("pending"))}</small>
            {qa_evidence_details(
                "Closure evidence detail",
                &[
                    ("Feedback evidence", &item.evidence_refs),
                    ("Status evidence", &item.status_evidence_refs),
                ],
            )}
        </div>
    }
}

fn qa_evidence_details(title: &str, groups: &[(&str, &Vec<String>)]) -> Html {
    let total_refs: usize = groups.iter().map(|(_, refs)| refs.len()).sum();
    html! {
        <details class="qa-evidence-details">
            <summary>{format!("{title}: {total_refs} refs")}</summary>
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
            html! { <p class="empty">{"Select a QA case and submit a controlled conclusion."}</p> }
        }
        ApiState::Loading => html! { <p>{"Submitting QA review..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="summary-grid">
                <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                <div><span>{"Event"}</span><strong>{&response.event_type}</strong></div>
                <div><span>{"Status"}</span><strong>{&response.event_status}</strong></div>
                <div><span>{"Audit"}</span><strong>{&response.audit_id}</strong></div>
                <div><span>{"Run"}</span><strong>{&response.run_id}</strong></div>
                <div><span>{"Evidence"}</span><strong>{refs_label(&response.evidence_refs)}</strong></div>
            </div>
        },
    }
}
