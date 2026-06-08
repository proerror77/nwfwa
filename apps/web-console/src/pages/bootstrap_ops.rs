use crate::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

#[function_component(BootstrapOpsPage)]
pub fn bootstrap_ops_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
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

#[derive(Properties, PartialEq)]
struct BootstrapOpsProps {
    state: ApiState<BootstrapOpsSnapshot>,
}

#[function_component(BootstrapOpsView)]
fn bootstrap_ops_view(props: &BootstrapOpsProps) -> Html {
    html! {
        {match &props.state {
            ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load label handoff queues to inspect replay, evidence, and reviewed-label readiness."}</p></section> },
            ApiState::Loading => html! { <section class="panel"><p>{"Loading label handoff queues..."}</p></section> },
            ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
            ApiState::Ready(snapshot) => html! {
                <>
                    <section class="summary-grid">
                        <div>
                            <span>{"Backfills"}</span>
                            <strong>{snapshot.backfills.len()}</strong>
                        </div>
                        <div>
                            <span>{"Evidence requests"}</span>
                            <strong>{snapshot.evidence_requests.len()}</strong>
                        </div>
                        <div>
                            <span>{"Open labels"}</span>
                            <strong>{snapshot.label_items.iter().filter(|item| item.review_status != "reviewed").count()}</strong>
                        </div>
                    </section>
                    <section class="workflow-card-grid">
                        {bootstrap_backfill_panel(&snapshot.backfills)}
                        {bootstrap_evidence_panel(&snapshot.evidence_requests)}
                        {bootstrap_label_panel(&snapshot.label_items)}
                    </section>
                </>
            },
        }}
    }
}

fn bootstrap_action_state(state: &UseStateHandle<ApiState<String>>) -> Html {
    match &**state {
        ApiState::Idle => {
            html! { <p class="empty">{"Actions write audit events; suspicious leads and missing evidence stay out of the training handoff until reviewed."}</p> }
        }
        ApiState::Loading => html! { <p>{"Submitting label handoff action..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(message) => html! { <p class="success-note">{message}</p> },
    }
}

fn bootstrap_backfill_panel(backfills: &[HistoricalBackfillJob]) -> Html {
    html! {
        <section class="panel result-stack">
            <div class="panel-heading-row">
                <h3>{"Historical Replay"}</h3>
                <span class="status-pill">{backfills.first().map(|job| job.status.as_str()).unwrap_or("empty")}</span>
            </div>
            if backfills.is_empty() {
                <p class="empty">{"No backfill jobs yet."}</p>
            } else {
                <div class="finding-list">
                    {for backfills.iter().take(5).map(|job| html! {
                        <div class="finding-row">
                            <strong>{&job.job_id}</strong>
                            <span>{format!("{} candidates / {} datasets", job.candidate_count, job.dataset_refs.len())}</span>
                            <small>{format!("rules {} / evidence {}", refs_count_label(&job.rule_refs), refs_count_label(&job.evidence_refs))}</small>
                            <details class="data-source-detail governance-detail">
                                <summary>{"Backfill evidence detail"}</summary>
                                <small>{format!("datasets: {}", refs_label(&job.dataset_refs))}</small>
                                <small>{format!("rules: {}", refs_label(&job.rule_refs))}</small>
                                <small>{format!("evidence: {}", refs_label(&job.evidence_refs))}</small>
                            </details>
                        </div>
                    })}
                </div>
            }
        </section>
    }
}

fn bootstrap_evidence_panel(requests: &[EvidenceRequestRecord]) -> Html {
    html! {
        <section class="panel result-stack">
            <div class="panel-heading-row">
                <h3>{"Evidence Requests"}</h3>
                <span class="status-pill">{requests.iter().filter(|request| request.status == "open").count()}</span>
            </div>
            if requests.is_empty() {
                <p class="empty">{"No generated evidence requests yet."}</p>
            } else {
                <div class="finding-list">
                    {for requests.iter().take(8).map(|request| html! {
                        <div class="finding-row">
                            <strong>{&request.claim_id}</strong>
                            <span>{format!("{} / {}", request.status, request.request_reason)}</span>
                            <div class="summary-grid">
                                <div><span>{"Missing"}</span><strong>{refs_count_label(&request.missing_evidence)}</strong></div>
                                <div><span>{"Items"}</span><strong>{request.items.len()}</strong></div>
                                <div><span>{"Queue"}</span><strong>{&request.reviewer_queue}</strong></div>
                                <div><span>{"Evidence"}</span><strong>{refs_count_label(&request.evidence_refs)}</strong></div>
                            </div>
                            <details class="data-source-detail governance-detail">
                                <summary>{"Bootstrap evidence detail"}</summary>
                                <small>{format!("request: {}", request.request_id)}</small>
                                <small>{format!("audit: {}", request.scoring_audit_id)}</small>
                                <small>{format!("missing: {}", refs_label(&request.missing_evidence))}</small>
                                <small>{format!("items: {}", evidence_request_items_label(&request.items))}</small>
                                <small>{format!("evidence: {}", refs_label(&request.evidence_refs))}</small>
                            </details>
                        </div>
                    })}
                </div>
            }
        </section>
    }
}

fn bootstrap_label_panel(items: &[LabelBootstrapItem]) -> Html {
    html! {
        <section class="panel result-stack">
            <div class="panel-heading-row">
                <h3>{"Label Evidence Handoff"}</h3>
                <span class="status-pill">{items.iter().filter(|item| item.training_eligible).count()}</span>
            </div>
            if items.is_empty() {
                <p class="empty">{"No reviewed-label handoff candidates yet."}</p>
            } else {
                <div class="finding-list">
                    {for items.iter().take(8).map(|item| html! {
                        <div class="finding-row">
                            <strong>{&item.suggested_label_name}</strong>
                            <span>{format!("{} / {}", item.review_status, item.governance_status)}</span>
                            <small>{format!("claim {} / training {} / evidence {}", item.claim_id, item.training_eligible, refs_count_label(&item.evidence_refs))}</small>
                            <details class="data-source-detail governance-detail">
                                <summary>{"Bootstrap label detail"}</summary>
                                <small>{format!("item: {}", item.item_id)}</small>
                                <small>{format!("source: {} / {}", item.source_type, item.source_id)}</small>
                                <small>{format!("feedback target: {}", item.feedback_target)}</small>
                                <small>{format!("evidence: {}", refs_label(&item.evidence_refs))}</small>
                            </details>
                        </div>
                    })}
                </div>
            }
        </section>
    }
}

fn bootstrap_evidence_request_select(
    snapshot_state: &UseStateHandle<ApiState<BootstrapOpsSnapshot>>,
    selected_id: &UseStateHandle<String>,
    on_change: Callback<Event>,
) -> Html {
    html! {
        <select value={(**selected_id).clone()} onchange={on_change}>
            <option value="">{"Select request"}</option>
            if let ApiState::Ready(snapshot) = &**snapshot_state {
                {for snapshot.evidence_requests.iter()
                    .filter(|request| request.status == "open" || request.status == "requested")
                    .map(|request| html! {
                        <option value={request.request_id.clone()}>
                            {format!("{} / {} / missing {}", request.claim_id, request.status, refs_label(&request.missing_evidence))}
                        </option>
                    })}
            }
        </select>
    }
}

fn bootstrap_label_item_select(
    snapshot_state: &UseStateHandle<ApiState<BootstrapOpsSnapshot>>,
    selected_id: &UseStateHandle<String>,
    on_change: Callback<Event>,
) -> Html {
    html! {
        <select value={(**selected_id).clone()} onchange={on_change}>
            <option value="">{"Select label item"}</option>
            if let ApiState::Ready(snapshot) = &**snapshot_state {
                {for snapshot.label_items.iter()
                    .filter(|item| item.review_status != "reviewed")
                    .map(|item| html! {
                        <option value={item.item_id.clone()}>
                            {format!("{} / {} / {}", item.claim_id, item.suggested_label_name, item.governance_status)}
                        </option>
                    })}
            }
        </select>
    }
}

fn bootstrap_selected_evidence_request(
    snapshot_state: &UseStateHandle<ApiState<BootstrapOpsSnapshot>>,
    selected_id: &UseStateHandle<String>,
) -> Html {
    let selected_id = (**selected_id).trim().to_string();
    if selected_id.is_empty() {
        return html! { <p class="empty">{"Select the request before recording received evidence."}</p> };
    }
    match evidence_request_by_id(snapshot_state, &selected_id) {
        Some(request) => html! {
            <div class="selected-work-item">
                <span>{"Selected evidence request"}</span>
                <strong>{format!("{} / {}", request.claim_id, request.request_id)}</strong>
                <small>{format!("status {} / missing {}", request.status, refs_label(&request.missing_evidence))}</small>
                <small>{format!("reason {} / items {}", request.request_reason, evidence_request_items_label(&request.items))}</small>
                <small>{format!("current evidence {}", refs_label(&request.evidence_refs))}</small>
            </div>
        },
        None => {
            html! { <p class="error">{"Selected evidence request is no longer in the queue."}</p> }
        }
    }
}

fn bootstrap_selected_label_item(
    snapshot_state: &UseStateHandle<ApiState<BootstrapOpsSnapshot>>,
    selected_id: &UseStateHandle<String>,
) -> Html {
    let selected_id = (**selected_id).trim().to_string();
    if selected_id.is_empty() {
        return html! { <p class="empty">{"Select the item before writing a governed label handoff review."}</p> };
    }
    match label_item_by_id(snapshot_state, &selected_id) {
        Some(item) => html! {
            <div class="selected-work-item">
                <span>{"Selected label item"}</span>
                <strong>{format!("{} / {}", item.claim_id, item.suggested_label_name)}</strong>
                <small>{format!("review {} / governance {} / training {}", item.review_status, item.governance_status, item.training_eligible)}</small>
                <small>{format!("evidence {}", refs_label(&item.evidence_refs))}</small>
            </div>
        },
        None => html! { <p class="error">{"Selected label item is no longer in the queue."}</p> },
    }
}

fn evidence_request_by_id(
    snapshot_state: &UseStateHandle<ApiState<BootstrapOpsSnapshot>>,
    request_id: &str,
) -> Option<EvidenceRequestRecord> {
    let ApiState::Ready(snapshot) = &**snapshot_state else {
        return None;
    };
    snapshot
        .evidence_requests
        .iter()
        .find(|request| request.request_id == request_id)
        .cloned()
}

fn evidence_request_items_label(items: &[EvidenceRequestItem]) -> String {
    if items.is_empty() {
        return "none".into();
    }
    items
        .iter()
        .map(|item| {
            let mut label = format!("{}: {}", item.document_type, item.reason);
            if item.blocking {
                label.push_str(" / blocking");
            }
            if let Some(policy_authority_ref) = item.policy_authority_ref.as_deref() {
                label = format!("{label} / {policy_authority_ref}");
            }
            if let Some(exception_check) = item.exception_check.as_deref() {
                label = format!("{label} / {exception_check}");
            }
            label
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn label_item_by_id(
    snapshot_state: &UseStateHandle<ApiState<BootstrapOpsSnapshot>>,
    item_id: &str,
) -> Option<LabelBootstrapItem> {
    let ApiState::Ready(snapshot) = &**snapshot_state else {
        return None;
    };
    snapshot
        .label_items
        .iter()
        .find(|item| item.item_id == item_id)
        .cloned()
}

fn selected_label_is_insufficient_evidence(
    snapshot_state: &UseStateHandle<ApiState<BootstrapOpsSnapshot>>,
    item_id: &str,
) -> bool {
    label_item_by_id(snapshot_state, item_id)
        .map(|item| item.suggested_label_name == "insufficient_evidence")
        .unwrap_or(false)
}

fn document_refs_text(refs: &[String]) -> String {
    refs.iter()
        .filter(|reference| reference.starts_with("evidence_documents:"))
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

fn has_document_evidence_ref(refs: &[String]) -> bool {
    refs.iter()
        .any(|reference| reference.starts_with("evidence_documents:"))
}
