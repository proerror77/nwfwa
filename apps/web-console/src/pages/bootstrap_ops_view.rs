use crate::api::*;
use crate::types::*;
use crate::constants::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::ui_helpers::*;
use crate::visual_helpers::*;
use crate::case_helpers::*;
use crate::rule_helpers::*;
use crate::rule_ui_helpers::*;
use crate::inbox_helpers::*;
use crate::payload_helpers::*;
use crate::data_helpers::*;
use crate::data_lineage_helpers::*;
use crate::medical_review_helpers::*;
use crate::model_ui_helpers::*;
use crate::runtime_helpers::*;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(super) struct BootstrapOpsProps {
    pub(super) state: ApiState<BootstrapOpsSnapshot>,
}

#[function_component(BootstrapOpsView)]
pub(super) fn bootstrap_ops_view(props: &BootstrapOpsProps) -> Html {
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

pub(super) fn bootstrap_action_state(state: &UseStateHandle<ApiState<String>>) -> Html {
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
                    {for backfills.iter().map(|job| html! {
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
                    {for requests.iter().map(|request| html! {
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
                    {for items.iter().map(|item| html! {
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

pub(super) fn bootstrap_evidence_request_select(
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

pub(super) fn bootstrap_label_item_select(
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

pub(super) fn bootstrap_selected_evidence_request(
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

pub(super) fn bootstrap_selected_label_item(
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

pub(super) fn evidence_request_by_id(
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

pub(super) fn label_item_by_id(
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

pub(super) fn selected_label_is_insufficient_evidence(
    snapshot_state: &UseStateHandle<ApiState<BootstrapOpsSnapshot>>,
    item_id: &str,
) -> bool {
    label_item_by_id(snapshot_state, item_id)
        .map(|item| item.suggested_label_name == "insufficient_evidence")
        .unwrap_or(false)
}

pub(super) fn document_refs_text(refs: &[String]) -> String {
    refs.iter()
        .filter(|reference| reference.starts_with("evidence_documents:"))
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn has_document_evidence_ref(refs: &[String]) -> bool {
    refs.iter()
        .any(|reference| reference.starts_with("evidence_documents:"))
}
