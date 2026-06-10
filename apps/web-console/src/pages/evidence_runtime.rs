use crate::api::*;
use crate::types::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::data_helpers::*;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

#[function_component(EvidenceRuntimePage)]
pub fn evidence_runtime_page() -> Html {
    let api_key = use_api_key();
    let selected_document_id = use_state(String::new);
    let snapshot_state = use_state(|| ApiState::<EvidenceRuntimeSnapshot>::Idle);
    let action_state = use_state(|| ApiState::<String>::Idle);

    let load_runtime = {
        let api_key = api_key.clone();
        let selected_document_id = selected_document_id.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let selected_document_id = (*selected_document_id).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_evidence_runtime_snapshot(api_key, selected_document_id).await {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    {
        let load_runtime = load_runtime.clone();
        use_effect_with((), move |_| {
            load_runtime.emit(());
            || ()
        });
    }

    let run_demo_lifecycle = {
        let api_key = api_key.clone();
        let selected_document_id = selected_document_id.clone();
        let snapshot_state = snapshot_state.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let next_index = match &*snapshot_state {
                ApiState::Ready(snapshot) => snapshot.documents.len() + 1,
                _ => 1,
            };
            let selected_document_id = selected_document_id.clone();
            let snapshot_state = snapshot_state.clone();
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_evidence_demo_lifecycle(api_key.clone(), next_index).await {
                    Ok(document_id) => {
                        selected_document_id.set(document_id.clone());
                        action_state.set(ApiState::Ready(format!(
                            "registered evidence lifecycle for {document_id}"
                        )));
                        snapshot_state.set(ApiState::Loading);
                        snapshot_state.set(
                            match get_evidence_runtime_snapshot(api_key, document_id).await {
                                Ok(snapshot) => ApiState::Ready(snapshot),
                                Err(error) => ApiState::Failed(error),
                            },
                        );
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let refresh = {
        let load_runtime = load_runtime.clone();
        Callback::from(move |_| load_runtime.emit(()))
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Evidence Runtime"}</h2>
                    <p>{"Operate the AI evidence metadata lifecycle without exposing raw document text or embedding vectors to the browser."}</p>
                </div>
                <span class="status-pill">{"AI Evidence Foundation"}</span>
            </div>

            <section class="panel">
                <h3>{"Runtime Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"Selected document id"}
                        <input
                            value={(*selected_document_id).clone()}
                            placeholder="leave blank to use first document"
                            oninput={{
                                let selected_document_id = selected_document_id.clone();
                                Callback::from(move |event: InputEvent| {
                                    selected_document_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh.clone()} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh evidence" }}
                    </button>
                    <button onclick={run_demo_lifecycle} disabled={matches!(&*action_state, ApiState::Loading)}>
                        {if matches!(&*action_state, ApiState::Loading) { "Registering..." } else { "Run demo evidence lifecycle" }}
                    </button>
                </div>
                {match &*action_state {
                    ApiState::Idle => html! { <p class="empty">{"Demo lifecycle writes document, chunk, OCR, embedding job, retrieval audit, and governance audit events."}</p> },
                    ApiState::Loading => html! { <p>{"Registering governed evidence metadata..."}</p> },
                    ApiState::Ready(message) => html! { <p class="success-note">{message}</p> },
                    ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                }}
            </section>

            <EvidenceRuntimeView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct EvidenceRuntimeProps {
    state: ApiState<EvidenceRuntimeSnapshot>,
}

#[function_component(EvidenceRuntimeView)]
fn evidence_runtime_view(props: &EvidenceRuntimeProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load evidence runtime metadata to inspect the current governed packet state."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading evidence runtime metadata..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {evidence_runtime_cockpit(snapshot)}
                        <section class="panel result-stack">
                            <h3>{"Document Packets"}</h3>
                            if snapshot.documents.is_empty() {
                                <p class="empty">{"No evidence documents registered for this customer scope."}</p>
                            } else {
                                <div class="evidence-runtime-grid">
                                    {for snapshot.documents.iter().map(evidence_document_card)}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Selected Document Outputs"}</h3>
                            <div class="summary-grid">
                                <div><span>{"Selected Document"}</span><strong>{snapshot.selected_document_id.as_deref().unwrap_or("none")}</strong></div>
                                <div><span>{"Chunks"}</span><strong>{snapshot.chunks.len()}</strong></div>
                                <div><span>{"OCR Outputs"}</span><strong>{snapshot.ocr_outputs.len()}</strong></div>
                            </div>
                            <div class="evidence-runtime-grid two-column">
                                <div>
                                    <h4>{"Chunks"}</h4>
                                    if snapshot.chunks.is_empty() {
                                        <p class="empty">{"No chunk metadata returned."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.chunks.iter().map(evidence_chunk_row)}
                                        </div>
                                    }
                                </div>
                                <div>
                                    <h4>{"OCR Outputs"}</h4>
                                    if snapshot.ocr_outputs.is_empty() {
                                        <p class="empty">{"No OCR metadata returned."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.ocr_outputs.iter().map(evidence_ocr_row)}
                                        </div>
                                    }
                                </div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Embedding And Retrieval Audit"}</h3>
                            <div class="evidence-runtime-grid two-column">
                                <div>
                                    <h4>{"Embedding Jobs"}</h4>
                                    if snapshot.embedding_jobs.is_empty() {
                                        <p class="empty">{"No embedding jobs registered."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.embedding_jobs.iter().map(evidence_embedding_row)}
                                        </div>
                                    }
                                </div>
                                <div>
                                    <h4>{"Retrieval Audit Events"}</h4>
                                    if snapshot.retrieval_audit_events.is_empty() {
                                        <p class="empty">{"No retrieval audit events recorded."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.retrieval_audit_events.iter().map(evidence_retrieval_row)}
                                        </div>
                                    }
                                </div>
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn evidence_runtime_cockpit(snapshot: &EvidenceRuntimeSnapshot) -> Html {
    html! {
        <section class="panel evidence-runtime-cockpit">
            <div class="evidence-runtime-map">
                {evidence_runtime_stage("Document", &snapshot.documents.len().to_string(), "source URI + checksum", "source")}
                {evidence_runtime_stage("Chunk", &snapshot.chunks.len().to_string(), "offsets + token count", "chunk")}
                {evidence_runtime_stage("OCR", &snapshot.ocr_outputs.len().to_string(), "engine + quality", "ocr")}
                {evidence_runtime_stage("Embedding", &snapshot.embedding_jobs.len().to_string(), "vector store refs", "embedding")}
                {evidence_runtime_stage("Retrieval", &snapshot.retrieval_audit_events.len().to_string(), "query checksum only", "retrieval")}
                <div class="evidence-runtime-core">
                    <span>{"Boundary"}</span>
                    <strong>{"no raw text in UI"}</strong>
                </div>
            </div>
        </section>
    }
}

fn evidence_runtime_stage(label: &str, value: &str, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("evidence-runtime-stage", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}

fn evidence_document_card(document: &EvidenceDocumentRecord) -> Html {
    html! {
        <div class="factor-card evidence-document-card">
            <div>
                <strong>{&document.document_id}</strong>
                <span>{format!("{} / {} / {}", document.document_type, document.ingestion_status, document.redaction_status)}</span>
            </div>
            <div class="summary-grid">
                <div><span>{"Claim"}</span><strong>{document.claim_id.as_deref().unwrap_or("none")}</strong></div>
                <div><span>{"Scope"}</span><strong>{&document.customer_scope_id}</strong></div>
                <div><span>{"Retention"}</span><strong>{&document.retention_policy_id}</strong></div>
            </div>
            <small>{format!("storage: {}", document.storage_uri)}</small>
            <small>{format!("checksum: {}", document.content_checksum)}</small>
            <small>{format!("evidence: {}", refs_label(&document.evidence_refs))}</small>
        </div>
    }
}

fn evidence_chunk_row(chunk: &EvidenceDocumentChunkRecord) -> Html {
    html! {
        <div class="metric-row compact-metric-row">
            <span>{format!("{} / index {}", chunk.chunk_id, chunk.chunk_index)}</span>
            <strong>{format!("{} tokens", chunk.token_count)}</strong>
            <small>{format!("{} / {}", chunk.chunking_version, chunk.redaction_status)}</small>
            <small>{format!("evidence: {}", refs_label(&chunk.evidence_refs))}</small>
        </div>
    }
}

fn evidence_ocr_row(output: &EvidenceOcrOutputRecord) -> Html {
    html! {
        <div class="metric-row compact-metric-row">
            <span>{format!("{} / {}", output.ocr_output_id, output.ocr_engine)}</span>
            <strong>{&output.quality_status}</strong>
            <small>{format!("version {} / confidence {}", output.ocr_engine_version, output.confidence_score.as_ref().map(display_value).unwrap_or_else(|| "none".into()))}</small>
            <small>{format!("evidence: {}", refs_label(&output.evidence_refs))}</small>
        </div>
    }
}

fn evidence_embedding_row(job: &EvidenceEmbeddingJobRecord) -> Html {
    html! {
        <div class="metric-row compact-metric-row">
            <span>{format!("{} / {}", job.embedding_job_id, job.target_ref)}</span>
            <strong>{&job.status}</strong>
            <small>{format!("{} {} -> {}", job.embedding_model, job.embedding_model_version, job.vector_store_kind)}</small>
            <small>{format!("evidence: {}", refs_label(&job.evidence_refs))}</small>
        </div>
    }
}

fn evidence_retrieval_row(event: &EvidenceRetrievalAuditEventRecord) -> Html {
    html! {
        <div class="metric-row compact-metric-row">
            <span>{format!("{} / {}", event.retrieval_id, event.query_kind)}</span>
            <strong>{format!("top {}", event.top_k)}</strong>
            <small>{format!("{} / actor {}", event.retrieval_method, event.actor_role)}</small>
            <small>{format!("sources: {} / results: {}", refs_label(&event.source_refs), refs_label(&event.result_refs))}</small>
        </div>
    }
}
