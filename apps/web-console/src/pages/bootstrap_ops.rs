use crate::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

#[path = "bootstrap_ops_view.rs"]
mod bootstrap_ops_view;
use bootstrap_ops_view::{
    bootstrap_action_state, bootstrap_evidence_request_select, bootstrap_label_item_select,
    bootstrap_selected_evidence_request, bootstrap_selected_label_item, document_refs_text,
    evidence_request_by_id, has_document_evidence_ref, label_item_by_id,
    selected_label_is_insufficient_evidence, BootstrapOpsView,
};

#[function_component(BootstrapOpsPage)]
pub fn bootstrap_ops_page() -> Html {
    let api_key = use_api_key();
    let snapshot_state = use_state(|| ApiState::<BootstrapOpsSnapshot>::Idle);
    let action_state = use_state(|| ApiState::<String>::Idle);
    let selected_evidence_request_id = use_state(String::new);
    let evidence_refs_input = use_state(String::new);
    let evidence_notes =
        use_state(|| "Evidence packet received and linked for label handoff review.".to_string());
    let selected_label_item_id = use_state(String::new);
    let label_name = use_state(String::new);
    let label_value = use_state(|| "true".to_string());
    let label_governance_status = use_state(|| "approved_for_training".to_string());
    let label_feedback_target = use_state(|| "model".to_string());
    let label_evidence_refs_input = use_state(String::new);
    let label_notes = use_state(|| {
        "Label reviewed against linked evidence for training-platform handoff.".to_string()
    });

    let refresh = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_bootstrap_ops_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let create_backfill = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                match create_bootstrap_backfill(api_key.clone()).await {
                    Ok(response) => {
                        action_state.set(ApiState::Ready(format!(
                            "Backfill {} captured {} candidate leads.",
                            response.job.job_id, response.job.candidate_count
                        )));
                        snapshot_state.set(match get_bootstrap_ops_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let generate_requests = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                match generate_bootstrap_evidence_requests(api_key.clone()).await {
                    Ok(response) => {
                        action_state.set(ApiState::Ready(format!(
                            "Generated {} evidence requests from scoring audits.",
                            response.requests.len()
                        )));
                        snapshot_state.set(match get_bootstrap_ops_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let mark_received = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let action_state = action_state.clone();
        let selected_evidence_request_id = selected_evidence_request_id.clone();
        let evidence_refs_input = evidence_refs_input.clone();
        let evidence_notes = evidence_notes.clone();
        Callback::from(move |_| {
            let request_id = (*selected_evidence_request_id).trim().to_string();
            if request_id.is_empty() {
                action_state.set(ApiState::Failed("select one evidence request first".into()));
                return;
            }
            let evidence_refs = parse_tags(&evidence_refs_input);
            if !has_document_evidence_ref(&evidence_refs) {
                action_state.set(ApiState::Failed(
                    "received evidence must include at least one evidence_documents:* ref".into(),
                ));
                return;
            }
            let notes = (*evidence_notes).trim().to_string();
            if notes.is_empty() {
                action_state.set(ApiState::Failed("evidence notes are required".into()));
                return;
            }
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                match mark_bootstrap_evidence_received(
                    api_key.clone(),
                    request_id,
                    evidence_refs,
                    notes,
                )
                .await
                {
                    Ok(request) => {
                        action_state.set(ApiState::Ready(format!(
                            "Evidence request {} is now {}.",
                            request.request_id, request.status
                        )));
                        snapshot_state.set(match get_bootstrap_ops_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let approve_label = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        let action_state = action_state.clone();
        let selected_label_item_id = selected_label_item_id.clone();
        let label_name = label_name.clone();
        let label_value = label_value.clone();
        let label_governance_status = label_governance_status.clone();
        let label_feedback_target = label_feedback_target.clone();
        let label_evidence_refs_input = label_evidence_refs_input.clone();
        let label_notes = label_notes.clone();
        Callback::from(move |_| {
            let item_id = (*selected_label_item_id).trim().to_string();
            if item_id.is_empty() {
                action_state.set(ApiState::Failed(
                    "select one label handoff item first".into(),
                ));
                return;
            }
            let label_name_value = (*label_name).trim().to_string();
            let label_value_value = (*label_value).trim().to_string();
            let governance_status = (*label_governance_status).trim().to_string();
            let feedback_target = (*label_feedback_target).trim().to_string();
            let notes = (*label_notes).trim().to_string();
            if label_name_value.is_empty()
                || label_value_value.is_empty()
                || governance_status.is_empty()
                || feedback_target.is_empty()
                || notes.is_empty()
            {
                action_state.set(ApiState::Failed(
                    "label, governance, feedback target, and notes are required".into(),
                ));
                return;
            }
            if selected_label_is_insufficient_evidence(&snapshot_state, &item_id)
                && governance_status == "approved_for_training"
            {
                action_state.set(ApiState::Failed(
                    "receive document evidence before approving this item for training handoff"
                        .into(),
                ));
                return;
            }
            let evidence_refs = parse_tags(&label_evidence_refs_input);
            if evidence_refs.is_empty() {
                action_state.set(ApiState::Failed(
                    "label review evidence refs are required".into(),
                ));
                return;
            }
            if governance_status == "approved_for_training"
                && !has_document_evidence_ref(&evidence_refs)
            {
                action_state.set(ApiState::Failed(
                    "training handoff labels require at least one evidence_documents:* ref".into(),
                ));
                return;
            }
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                match review_bootstrap_label(
                    api_key.clone(),
                    item_id,
                    label_name_value,
                    label_value_value,
                    governance_status,
                    feedback_target,
                    notes,
                    evidence_refs,
                )
                .await
                {
                    Ok(response) => {
                        action_state.set(ApiState::Ready(format!(
                            "Label {} reviewed with audit {}.",
                            response.item.item_id, response.audit_id
                        )));
                        snapshot_state.set(match get_bootstrap_ops_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    {
        let refresh = refresh.clone();
        use_effect_with((), move |_| {
            refresh.emit(());
            || ()
        });
    }

    let on_evidence_select = {
        let selected_evidence_request_id = selected_evidence_request_id.clone();
        let evidence_refs_input = evidence_refs_input.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |event: Event| {
            let request_id = event.target_unchecked_into::<HtmlSelectElement>().value();
            selected_evidence_request_id.set(request_id.clone());
            if let Some(request) = evidence_request_by_id(&snapshot_state, &request_id) {
                evidence_refs_input.set(document_refs_text(&request.evidence_refs));
            }
        })
    };

    let on_label_select = {
        let selected_label_item_id = selected_label_item_id.clone();
        let label_name = label_name.clone();
        let label_value = label_value.clone();
        let label_governance_status = label_governance_status.clone();
        let label_feedback_target = label_feedback_target.clone();
        let label_evidence_refs_input = label_evidence_refs_input.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |event: Event| {
            let item_id = event.target_unchecked_into::<HtmlSelectElement>().value();
            selected_label_item_id.set(item_id.clone());
            if let Some(item) = label_item_by_id(&snapshot_state, &item_id) {
                label_name.set(item.suggested_label_name.clone());
                label_value.set(item.suggested_label_value.clone());
                label_governance_status.set(
                    if item.suggested_label_name == "insufficient_evidence" {
                        "rejected_for_training".into()
                    } else {
                        "approved_for_training".into()
                    },
                );
                label_feedback_target.set(item.feedback_target.clone());
                let document_refs = document_refs_text(&item.evidence_refs);
                label_evidence_refs_input.set(if document_refs.is_empty() {
                    refs_label(&item.evidence_refs)
                } else {
                    document_refs
                });
            }
        })
    };

    let refresh_click = {
        let refresh = refresh.clone();
        Callback::from(move |_| refresh.emit(()))
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Training Label Handoff"}</h2>
                    <p>{"Prepare replay findings, missing-evidence requests, and reviewed labels as an audited handoff for the independent training platform."}</p>
                </div>
                <span class="status-pill">{"Label Evidence Handoff"}</span>
            </div>

            <section class="panel">
                <h3>{"Label Evidence Source"}</h3>
                <p class="empty">{"Using the configured pilot operations principal for historical replay, evidence requests, and label handoff governance."}</p>
                <div class="action-bar">
                    <button onclick={refresh_click} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh queues" }}
                    </button>
                    <button onclick={create_backfill} disabled={matches!(&*action_state, ApiState::Loading)}>
                        {"Create backfill"}
                    </button>
                    <button onclick={generate_requests} disabled={matches!(&*action_state, ApiState::Loading)}>
                        {"Generate evidence requests"}
                    </button>
                </div>
                {bootstrap_action_state(&action_state)}
            </section>

            <section class="bootstrap-action-grid">
                <section class="panel result-stack">
                    <div class="section-header compact">
                        <div>
                            <h3>{"Evidence Intake"}</h3>
                            <p>{"Choose a specific request and link actual document evidence before changing its status."}</p>
                        </div>
                    </div>
                    <label>
                        {"Evidence request"}
                        {bootstrap_evidence_request_select(&snapshot_state, &selected_evidence_request_id, on_evidence_select)}
                    </label>
                    {bootstrap_selected_evidence_request(&snapshot_state, &selected_evidence_request_id)}
                    <label>
                        {"Evidence document refs"}
                        <input
                            placeholder="evidence_documents:doc_123, evidence_documents:doc_456"
                            value={(*evidence_refs_input).clone()}
                            oninput={{
                                let evidence_refs_input = evidence_refs_input.clone();
                                Callback::from(move |event: InputEvent| {
                                    evidence_refs_input.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label class="compact-note">
                        {"Evidence notes"}
                        <textarea
                            value={(*evidence_notes).clone()}
                            oninput={{
                                let evidence_notes = evidence_notes.clone();
                                Callback::from(move |event: InputEvent| {
                                    evidence_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={mark_received} disabled={matches!(&*action_state, ApiState::Loading)}>
                            {"Mark selected request received"}
                        </button>
                    </div>
                </section>

                <section class="panel result-stack">
                    <div class="section-header compact">
                        <div>
                            <h3>{"Label Review"}</h3>
                            <p>{"Review one label candidate explicitly; only approved document-backed labels enter the training-platform handoff."}</p>
                        </div>
                    </div>
                    <label>
                        {"Label item"}
                        {bootstrap_label_item_select(&snapshot_state, &selected_label_item_id, on_label_select)}
                    </label>
                    {bootstrap_selected_label_item(&snapshot_state, &selected_label_item_id)}
                    <div class="form-grid action-form-grid">
                        <label>
                            {"Label name"}
                            <input
                                value={(*label_name).clone()}
                                oninput={{
                                    let label_name = label_name.clone();
                                    Callback::from(move |event: InputEvent| {
                                        label_name.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label>
                            {"Label value"}
                            <input
                                value={(*label_value).clone()}
                                oninput={{
                                    let label_value = label_value.clone();
                                    Callback::from(move |event: InputEvent| {
                                        label_value.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label>
                            {"Governance"}
                            <select
                                value={(*label_governance_status).clone()}
                                onchange={{
                                    let label_governance_status = label_governance_status.clone();
                                    Callback::from(move |event: Event| {
                                        label_governance_status.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="approved_for_training">{"approved_for_training"}</option>
                                <option value="rejected_for_training">{"rejected_for_training"}</option>
                                <option value="needs_review">{"needs_review"}</option>
                            </select>
                        </label>
                        <label>
                            {"Feedback target"}
                            <select
                                value={(*label_feedback_target).clone()}
                                onchange={{
                                    let label_feedback_target = label_feedback_target.clone();
                                    Callback::from(move |event: Event| {
                                        label_feedback_target.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="model">{"model"}</option>
                                <option value="workflow">{"workflow"}</option>
                                <option value="rule">{"rule"}</option>
                            </select>
                        </label>
                    </div>
                    <label>
                        {"Review evidence refs"}
                        <input
                            placeholder="evidence_documents:doc_123"
                            value={(*label_evidence_refs_input).clone()}
                            oninput={{
                                let label_evidence_refs_input = label_evidence_refs_input.clone();
                                Callback::from(move |event: InputEvent| {
                                    label_evidence_refs_input.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label class="compact-note">
                        {"Review notes"}
                        <textarea
                            value={(*label_notes).clone()}
                            oninput={{
                                let label_notes = label_notes.clone();
                                Callback::from(move |event: InputEvent| {
                                    label_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={approve_label} disabled={matches!(&*action_state, ApiState::Loading)}>
                            {"Review selected label"}
                        </button>
                    </div>
                </section>
            </section>

            <BootstrapOpsView state={(*snapshot_state).clone()} />
        </section>
    }
}
