use crate::*;
use wasm_bindgen_futures::spawn_local;

#[function_component(QaReviewPage)]
pub fn qa_review_page() -> Html {
    let api_key = use_api_key();
    let snapshot_state = use_state(|| ApiState::<QaReviewSnapshot>::Idle);

    let load_qa_review = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_qa_review_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_qa_review = load_qa_review.clone();
        Callback::from(move |_| load_qa_review.emit(()))
    };

    {
        let load_qa_review = load_qa_review.clone();
        use_effect_with((), move |_| {
            load_qa_review.emit(());
            || ()
        });
    }

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
