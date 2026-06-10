use crate::api::*;
use crate::types::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::ui_helpers::*;
use crate::payload_helpers::*;
use crate::data_helpers::*;
use crate::medical_review_helpers::*;
use yew::prelude::*;
use serde_json::json;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

#[function_component(MedicalReviewPage)]
pub fn medical_review_page() -> Html {
    let api_key = use_api_key();
    let limit = use_state(|| "100".to_string());
    let selected_audit_id = use_state(String::new);
    let reviewer = use_state(|| "medical-reviewer-1".to_string());
    let decision = use_state(|| "request_more_evidence".to_string());
    let clinical_outcomes = use_state(|| "insufficient_evidence".to_string());
    let notes = use_state(|| "Medical review recorded from Operations Studio.".to_string());
    let evidence_refs = use_state(String::new);
    let queue_state = use_state(|| ApiState::<Vec<MedicalReviewQueueItem>>::Idle);
    let result_state = use_state(|| ApiState::<MedicalReviewResultResponse>::Idle);

    let selected_review_summary = match &*queue_state {
        ApiState::Ready(items) => selected_medical_item(items, &selected_audit_id).map(|item| {
            (
                item.claim_id.clone(),
                item.audit_id.clone(),
                item.first_issue_type
                    .clone()
                    .unwrap_or_else(|| "issue pending".into()),
                format!(
                    "{} / {}",
                    business_label(&item.review_route),
                    business_label(&item.evidence_status)
                ),
            )
        }),
        _ => None,
    };
    let has_selected_review = selected_review_summary.is_some();

    let load_queue = {
        let api_key = api_key.clone();
        let limit = limit.clone();
        let queue_state = queue_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let limit = (*limit).clone();
            let queue_state = queue_state.clone();
            queue_state.set(ApiState::Loading);
            spawn_local(async move {
                queue_state.set(match get_medical_review_queue(api_key, limit).await {
                    Ok(items) => ApiState::Ready(items),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_queue = load_queue.clone();
        Callback::from(move |_| load_queue.emit(()))
    };

    let submit_review = {
        let api_key = api_key.clone();
        let selected_audit_id = selected_audit_id.clone();
        let reviewer = reviewer.clone();
        let decision = decision.clone();
        let clinical_outcomes = clinical_outcomes.clone();
        let notes = notes.clone();
        let evidence_refs = evidence_refs.clone();
        let queue_state = queue_state.clone();
        let result_state = result_state.clone();
        let limit = limit.clone();
        Callback::from(move |_| {
            let ApiState::Ready(items) = &*queue_state else {
                result_state.set(ApiState::Failed(
                    "load the medical review queue before writeback".into(),
                ));
                return;
            };
            let item = selected_medical_item(items, &selected_audit_id);
            let Some(item) = item else {
                result_state.set(ApiState::Failed("select a medical review item".into()));
                return;
            };
            let fallback_refs = medical_review_fallback_refs(item);
            let payload = json!({
                "claim_id": item.claim_id,
                "scoring_audit_id": item.audit_id,
                "reviewer": (*reviewer).clone(),
                "decision": (*decision).clone(),
                "clinical_outcomes": parse_tags(&clinical_outcomes),
                "notes": (*notes).clone(),
                "evidence_refs": refs_or_fallback(&evidence_refs, fallback_refs),
            });
            let api_key = (*api_key).clone();
            let result_state = result_state.clone();
            let queue_state = queue_state.clone();
            let limit = (*limit).clone();
            result_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_medical_review_result(api_key.clone(), payload).await {
                    Ok(response) => {
                        result_state.set(ApiState::Ready(response));
                        queue_state.set(match get_medical_review_queue(api_key, limit).await {
                            Ok(items) => ApiState::Ready(items),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => result_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    {
        let load_queue = load_queue.clone();
        use_effect_with((), move |_| {
            load_queue.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Medical Review"}</h2>
                    <p>{"Review clinical evidence gaps, medical necessity signals, source trace coverage, and reviewer writeback before case or model governance consumes labels."}</p>
                </div>
                <span class="status-pill">{"Clinical Signals"}</span>
            </div>

            <section class="panel">
                <h3>{"Review Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"Limit"}
                        <input
                            value={(*limit).clone()}
                            oninput={{
                                let limit = limit.clone();
                                Callback::from(move |event: InputEvent| {
                                    limit.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Scoring audit ID"}
                        <input
                            value={(*selected_audit_id).clone()}
                            placeholder={"blank uses first queue item"}
                            oninput={{
                                let selected_audit_id = selected_audit_id.clone();
                                Callback::from(move |event: InputEvent| {
                                    selected_audit_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*queue_state, ApiState::Loading)}>
                        {if matches!(&*queue_state, ApiState::Loading) { "Refreshing..." } else { "Refresh queue" }}
                    </button>
                </div>
            </section>

            <div class="leads-cases-workflow medical-review-workflow">
                <div class="queue-column">
                    <MedicalReviewQueueView state={(*queue_state).clone()} />
                </div>

                <aside class="panel result-stack case-action-panel">
                    <h3>{"Human Clinical Decision"}</h3>
                    <div class="selected-work-item">
                        <span>{"Selected review"}</span>
                        <strong>{selected_review_summary.as_ref().map(|item| item.0.as_str()).unwrap_or("none")}</strong>
                        <small>
                            {selected_review_summary.as_ref().map(|item| {
                                format!("{} / {} / {}", item.1, item.2, item.3)
                            }).unwrap_or_else(|| "Load the queue, then enter a scoring audit ID or use the first queue item.".into())}
                        </small>
                    </div>
                    <div class="form-grid action-form-grid">
                        {text_input("Reviewer", &reviewer)}
                        <label>
                            {"Decision"}
                            <select
                                onchange={{
                                    let decision = decision.clone();
                                    Callback::from(move |event: Event| {
                                        decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="request_more_evidence" selected={(*decision).as_str() == "request_more_evidence"}>{"Request more evidence"}</option>
                                <option value="evidence_sufficient" selected={(*decision).as_str() == "evidence_sufficient"}>{"Evidence sufficient"}</option>
                                <option value="medical_necessity_issue" selected={(*decision).as_str() == "medical_necessity_issue"}>{"Medical necessity issue"}</option>
                                <option value="no_medical_issue" selected={(*decision).as_str() == "no_medical_issue"}>{"No medical issue"}</option>
                            </select>
                        </label>
                        <label>
                            {"Controlled outcome"}
                            <select
                                onchange={{
                                    let clinical_outcomes = clinical_outcomes.clone();
                                    Callback::from(move |event: Event| {
                                        clinical_outcomes.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="insufficient_evidence" selected={(*clinical_outcomes).as_str() == "insufficient_evidence"}>{"Insufficient evidence"}</option>
                                <option value="documentation_issue" selected={(*clinical_outcomes).as_str() == "documentation_issue"}>{"Documentation issue"}</option>
                                <option value="medical_necessity_review_required" selected={(*clinical_outcomes).as_str() == "medical_necessity_review_required"}>{"Medical necessity review required"}</option>
                                <option value="clinical_evidence_sufficient" selected={(*clinical_outcomes).as_str() == "clinical_evidence_sufficient"}>{"Clinical evidence sufficient"}</option>
                                <option value="no_medical_issue" selected={(*clinical_outcomes).as_str() == "no_medical_issue"}>{"No medical issue"}</option>
                            </select>
                        </label>
                        {text_input("Evidence refs", &evidence_refs)}
                    </div>
                    <label class="compact-note">
                        {"Notes"}
                        <textarea
                            value={(*notes).clone()}
                            oninput={{
                                let notes = notes.clone();
                                Callback::from(move |event: InputEvent| {
                                    notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={submit_review} disabled={!has_selected_review || matches!(&*result_state, ApiState::Loading)}>
                            {if matches!(&*result_state, ApiState::Loading) { "Submitting..." } else { "Confirm clinical review" }}
                        </button>
                    </div>
                    <MedicalReviewResultView state={(*result_state).clone()} />
                </aside>
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct MedicalReviewQueueProps {
    state: ApiState<Vec<MedicalReviewQueueItem>>,
}

#[function_component(MedicalReviewQueueView)]
fn medical_review_queue_view(props: &MedicalReviewQueueProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load the queue to inspect medical review candidates."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading medical review queue..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(items) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Clinical Queue Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Queue Items"}</span><strong>{items.len()}</strong></div>
                                <div><span>{"Open"}</span><strong>{items.iter().filter(|item| item.review_status == "open").count()}</strong></div>
                                <div><span>{"Evidence Missing"}</span><strong>{items.iter().filter(|item| !item.missing_evidence.is_empty()).count()}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Completed"}</span><strong>{items.iter().filter(|item| item.review_status.starts_with("completed")).count()}</strong></div>
                                <div><span>{"Pending Evidence"}</span><strong>{items.iter().filter(|item| item.review_status == "pending_evidence").count()}</strong></div>
                                <div><span>{"Avg Medical Score"}</span><strong>{format!("{:.1}", average_medical_score(items))}</strong></div>
                            </div>
                        </section>

                        {medical_review_cockpit(items)}

                        <section class="panel result-stack">
                            <h3>{"Medical Review Queue"}</h3>
                            if items.is_empty() {
                                <p class="empty">{"No medical review queue items returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for items.iter().map(|item| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", item.claim_id, item.audit_id)}</strong>
                                                <span>{format!("{} / {} / {}", business_label(&item.review_route), business_label(&item.evidence_status), business_label(&item.review_status))}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Medical Score"}</span><strong>{item.medical_reasonableness_score}</strong></div>
                                                <div><span>{"Findings"}</span><strong>{item.item_finding_count}</strong></div>
                                                <div><span>{"First Item"}</span><strong>{item.first_item_code.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"First Issue"}</span><strong>{item.first_issue_type.as_deref().map(business_label).unwrap_or_else(|| "None".into())}</strong></div>
                                                <div><span>{"Reviewer"}</span><strong>{item.reviewer.as_deref().unwrap_or("pending")}</strong></div>
                                                <div><span>{"Decision"}</span><strong>{item.review_decision.as_deref().map(business_label).unwrap_or_else(|| "Pending".into())}</strong></div>
                                            </div>
                                            <small>{format!("missing evidence: {}", refs_label(&item.missing_evidence))}</small>
                                            <small>{format!("canonical: {} / {}", refs_label(&item.canonical_source_refs), refs_label(&item.canonical_evidence_refs))}</small>
                                            <small>{format!("review audit: {} / reviewed at: {}", item.review_audit_id.as_deref().unwrap_or("pending"), item.reviewed_at.as_deref().unwrap_or("pending"))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[derive(Properties, PartialEq)]
struct MedicalReviewResultProps {
    state: ApiState<MedicalReviewResultResponse>,
}

#[function_component(MedicalReviewResultView)]
fn medical_review_result_view(props: &MedicalReviewResultProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Select a queue item and confirm a controlled clinical outcome."}</p> }
        }
        ApiState::Loading => html! { <p>{"Submitting medical review result..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="summary-grid">
                <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                <div><span>{"Status"}</span><strong>{business_label(&response.review_status)}</strong></div>
                <div><span>{"Audit"}</span><strong>{&response.audit_id}</strong></div>
                <div><span>{"Run"}</span><strong>{&response.run_id}</strong></div>
                <div><span>{"Clinical Outcomes"}</span><strong>{refs_label(&response.clinical_outcomes)}</strong></div>
                <div><span>{"Evidence"}</span><strong>{refs_label(&response.evidence_refs)}</strong></div>
            </div>
        },
    }
}
