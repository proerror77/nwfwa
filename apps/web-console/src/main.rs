use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;
mod api;
mod constants;
mod i18n;
mod routing;
mod state;
mod types;

use api::{request_get_json, request_json};
use constants::*;
use i18n::{
    apply_document_language, brand_description, module_context, module_description, module_label,
    section_label, tr,
};
use routing::{
    active_module_from_location, is_known_module, module_icon_class, set_module_hash,
    workspace_system_map, CONTRACT_PANELS, NAV_SECTIONS,
};
use state::{ApiState, Language};
use types::*;

#[function_component(App)]
fn app() -> Html {
    let active = use_state(active_module_from_location);
    let language = use_state(|| Language::En);
    let select_module = {
        let active = active.clone();
        Callback::from(move |module: String| {
            if is_known_module(&module) {
                set_module_hash(&module);
                active.set(module);
            }
        })
    };
    let toggle_language = {
        let language = language.clone();
        Callback::from(move |_| language.set((*language).toggle()))
    };

    {
        let active = active.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().and_then(|window| {
                let active = active.clone();
                let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_| {
                    active.set(active_module_from_location());
                }));
                window
                    .add_event_listener_with_callback(
                        "hashchange",
                        callback.as_ref().unchecked_ref(),
                    )
                    .ok()?;
                Some((window, callback))
            });
            move || {
                if let Some((window, callback)) = listener {
                    let _ = window.remove_event_listener_with_callback(
                        "hashchange",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    {
        let language = *language;
        use_effect(move || {
            apply_document_language(language.document_code());
            || ()
        });
    }

    html! {
        <div class="app">
            <aside class="sidebar">
                <div class="brand-block">
                    <span>{"NOVA FWA"}</span>
                    <h1>{"FWA Platform"}</h1>
                    <p>{brand_description(*language)}</p>
                </div>
                <nav class="module-nav" aria-label="FWA operations modules">
                    {for NAV_SECTIONS.iter().map(|(section, modules)| html! {
                        <div class="nav-section">
                            <p class="nav-section-title">{section_label(section, *language)}</p>
                            {for modules.iter().map(|module| {
                                let select_module = select_module.clone();
                                let module_name = (*module).to_string();
                                let is_active = *active == module_name;
                                html! {
                                    <button
                                        class={classes!(is_active.then_some("active"))}
                                        onclick={Callback::from(move |_| select_module.emit(module_name.clone()))}
                                    >
                                        <span class={classes!("nav-icon", module_icon_class(module))}></span>
                                        <span class="nav-copy">
                                            <span class="nav-label">{module_label(module, *language)}</span>
                                            <span class="nav-description">{module_description(module, *language)}</span>
                                        </span>
                                    </button>
                                }
                            })}
                        </div>
                    })}
                </nav>
            </aside>
            <main class="workspace">
                <div class="workspace-topbar">
                    <div class="topbar-context">
                        <span class="eyebrow">{tr(*language, "Real-time operations", "实时运营")}</span>
                        <strong>{module_context(&active, *language)}</strong>
                    </div>
                    <div class="topbar-actions">
                        <span class="api-chip status-live">{"live"}</span>
                        <span class="user-chip">{"Pilot Ops"}</span>
                        <button class="language-toggle" onclick={toggle_language}>
                            {(*language).code()}
                        </button>
                    </div>
                </div>
                {workspace_system_map(active.as_str(), select_module.clone(), *language)}
                <div class="workspace-content">
                    if *active == "Intake Ops" {
                        <ClaimInboxPage />
                    } else if *active == "Dashboard" {
                        <DashboardPage on_navigate={select_module.clone()} />
                    } else if *active == "Runtime Scoring" {
                        <RuntimeScoringPage />
                    } else if *active == "Review Workbench" {
                        {review_workbench_page(select_module.clone())}
                    } else if *active == "Bootstrap Ops" {
                        <BootstrapOpsPage />
                    } else if *active == "Discovery Review" {
                        {discovery_review_page(select_module.clone())}
                    } else if *active == "Evidence Hub" {
                        {evidence_hub_page(select_module.clone())}
                    } else if *active == "Provider Model Intake" {
                        <MlopsWorkspacePage />
                    } else if *active == "Evidence Runtime" {
                        <EvidenceRuntimePage />
                    } else if *active == "Rules" {
                        <RulesPage />
                    } else if *active == "Models" {
                        <ModelsPage />
                    } else if *active == "Routing Policies" {
                        <RoutingPoliciesPage />
                    } else if *active == "Data Sources" {
                        <DataSourcesPage />
                    } else if *active == "Factor Factory" {
                        <FactorFactoryPage />
                    } else if *active == "Leads & Cases" {
                        <LeadsCasesPage />
                    } else if *active == "Member Profile" {
                        <MemberProfilePage />
                    } else if *active == "Provider Risk" {
                        <ProviderRiskPage />
                    } else if *active == "Medical Review" {
                        <MedicalReviewPage />
                    } else if *active == "Audit Sampling" {
                        <AuditSamplingPage />
                    } else if *active == "Knowledge Base" {
                        <KnowledgeBasePage />
                    } else if *active == "Agent Investigator" {
                        <AgentInvestigatorPage />
                    } else if *active == "QA Review" {
                        <QaReviewPage />
                    } else if *active == "Governance" {
                        <GovernancePage />
                    } else {
                        <ModuleStatusPage title={(*active).clone()} />
                    }
                </div>
            </main>
        </div>
    }
}

fn review_workbench_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Review Workbench"}</h2>
                    <p>{"Use this as the single entry point for human review. Clinical necessity and QA feedback stay separate, but operators do not need two top-level menus."}</p>
                </div>
                <span class="status-pill">{"Human review"}</span>
            </div>
            <div class="workflow-card-grid">
                {workflow_action_card("Medical Review", "Resolve clinical reasonableness, necessity, and documentation questions.", "Open clinical queue", "Medical Review", "strong", &on_navigate)}
                {workflow_action_card("QA Review", "Close sampled findings, reviewer disagreement, and feedback calibration.", "Open QA queue", "QA Review", "warning", &on_navigate)}
            </div>
        </section>
    }
}

#[function_component(BootstrapOpsPage)]
fn bootstrap_ops_page() -> Html {
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

fn discovery_review_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Rule & Model Discovery Review"}</h2>
                    <p>{"Use this as the single business entry for ML-discovered rule candidates and provider-trained model versions. Operators compare evidence, run backtests, inspect shadow gates, then accept or reject before anything can affect routing."}</p>
                </div>
                <span class="status-pill">{"ML governance control"}</span>
            </div>

            <section class="panel result-stack">
                <div class="section-header">
                    <div>
                        <h3>{"Candidate Review Path"}</h3>
                        <p>{"Every candidate must show source, backtest or evaluation evidence, shadow comparison, review-capacity impact, human decision, and rollback path before it can affect routing."}</p>
                    </div>
                    <span class="status-token strong">{"human approval required"}</span>
                </div>
                <div class="inbox-pipeline release-decision-flow">
                    {pipeline_step("Candidate", "ML discovery / provider model", "done")}
                    {pipeline_step("Evidence", "backtest + eval refs", "warning")}
                    {pipeline_step("Shadow", "compare against current routing", "pending")}
                    {pipeline_step("Review", "accept / reject", "pending")}
                    {pipeline_step("Release", "limited / active / rollback", "pending")}
                </div>
            </section>

            <section class="panel result-stack">
                <div class="section-header">
                    <div>
                        <h3>{"What Operators Decide Here"}</h3>
                        <p>{"Business users do not tune raw features or train models here. They accept or reject explainable candidates based on backtest, shadow, and governance evidence."}</p>
                    </div>
                    <span class="status-token neutral">{"release governance only"}</span>
                </div>
                <div class="summary-grid">
                    <div><span>{"ML rule intake"}</span><strong>{"Model-discovered rules from explanations, offline mining, or QA feedback"}</strong></div>
                    <div><span>{"Model intake"}</span><strong>{"Provider-trained model versions with dataset, split, metric, drift, and artifact evidence"}</strong></div>
                    <div><span>{"Decision"}</span><strong>{"Reject weak explanations, keep in shadow, accept for review, approve limited rollout, activate, or rollback"}</strong></div>
                    <div><span>{"Not here"}</span><strong>{"No ad hoc model training, no raw feature engineering, no autonomous denial"}</strong></div>
                </div>
            </section>

            <div class="workflow-card-grid">
                {workflow_action_card("ML Rule Review Queue", "Rules discovered from model explanations, offline mining, or case feedback must pass backtest, shadow review, and human accept/reject before entering the governed rule library.", "Review discovered rules", "Rules", "strong", &on_navigate)}
                {workflow_action_card("Provider Model Queue", "Provider training output arrives as candidate versions. Compare holdout, out-of-time, shadow, drift, and review-capacity metrics before activation.", "Review model evidence", "Provider Model Intake", "warning", &on_navigate)}
                {workflow_action_card("Routing Impact", "Check whether an approved release affects pre-payment, post-payment, manual review, pending evidence, QA sample, or straight-through routing.", "Check impact", "Routing Policies", "neutral", &on_navigate)}
                {workflow_action_card("Evidence Package", "Inspect dataset, feature-set, split, schema, and evaluation lineage that supports the release decision.", "Validate evidence", "Data Sources", "success", &on_navigate)}
                {workflow_action_card("Release History", "Audit approvals, activation, rollback, API call records, and agent/routing boundaries after release.", "Open governance", "Governance", "strong", &on_navigate)}
            </div>
        </section>
    }
}

fn evidence_hub_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Evidence Hub"}</h2>
                    <p>{"Look up the evidence an investigator needs before making a case decision. This keeps context lookup separate from scoring and review actions."}</p>
                </div>
                <span class="status-pill">{"Context lookup"}</span>
            </div>
            {evidence_hub_visual()}
            <div class="workflow-card-grid">
                {workflow_action_card("Evidence Runtime", "Register document packets, chunks, OCR outputs, embedding jobs, and retrieval audit metadata.", "Open runtime", "Evidence Runtime", "strong", &on_navigate)}
                {workflow_action_card("Provider Risk", "Open provider graph signals, suspicious patterns, and network flags.", "Review provider", "Provider Risk", "danger", &on_navigate)}
                {workflow_action_card("Member Profile", "Inspect member-level utilization, policy, and claim history context.", "Review member", "Member Profile", "neutral", &on_navigate)}
                {workflow_action_card("Knowledge Base", "Search confirmed evidence without crossing adjudication boundaries.", "Search evidence", "Knowledge Base", "strong", &on_navigate)}
                {workflow_action_card("Data Sources", "Check dataset lineage, schema mapping, and evaluation inputs.", "Review data", "Data Sources", "success", &on_navigate)}
            </div>
        </section>
    }
}

fn evidence_hub_visual() -> Html {
    html! {
        <section class="panel evidence-visual-shell">
            <div class="evidence-visual-board">
                <div class="evidence-specimen">
                    <div class="specimen-top">
                        <span>{"Document packet"}</span>
                        <strong>{"redacted + traceable"}</strong>
                    </div>
                    <div class="specimen-lines">
                        <i class="wide"></i>
                        <i></i>
                        <i class="short"></i>
                        <i class="wide warning"></i>
                    </div>
                    <div class="specimen-tags">
                        <span>{"checksum"}</span>
                        <span>{"URI"}</span>
                        <span>{"evidence_refs"}</span>
                    </div>
                </div>
                <div class="evidence-pipeline-rail">
                    {evidence_pipeline_node("01", "Register", "document metadata")}
                    {evidence_pipeline_node("02", "OCR", "redacted output")}
                    {evidence_pipeline_node("03", "Chunk", "source spans")}
                    {evidence_pipeline_node("04", "Embed", "job state")}
                    {evidence_pipeline_node("05", "Audit", "retrieval trail")}
                </div>
                <div class="evidence-loop-note">
                    <span>{"Evidence boundary"}</span>
                    <strong>{"LLM sees references, not raw claims text"}</strong>
                    <small>{"The runtime stores provenance, redaction state, retrieval purpose, and actor scope before Agent or QA views consume the packet."}</small>
                </div>
            </div>
        </section>
    }
}

fn evidence_pipeline_node(step: &str, label: &str, caption: &str) -> Html {
    html! {
        <div class="evidence-pipeline-node">
            <span>{step}</span>
            <strong>{label}</strong>
            <small>{caption}</small>
        </div>
    }
}

#[function_component(EvidenceRuntimePage)]
fn evidence_runtime_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
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
                                    {for snapshot.documents.iter().take(8).map(evidence_document_card)}
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
                                            {for snapshot.embedding_jobs.iter().take(8).map(evidence_embedding_row)}
                                        </div>
                                    }
                                </div>
                                <div>
                                    <h4>{"Retrieval Audit Events"}</h4>
                                    if snapshot.retrieval_audit_events.is_empty() {
                                        <p class="empty">{"No retrieval audit events recorded."}</p>
                                    } else {
                                        <div class="table-list">
                                            {for snapshot.retrieval_audit_events.iter().take(8).map(evidence_retrieval_row)}
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

fn workflow_action_card(
    title: &str,
    description: &str,
    command: &str,
    target: &str,
    tone: &str,
    on_navigate: &Callback<String>,
) -> Html {
    let target = target.to_string();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("workflow-action-card", tone.to_string())}
            onclick={Callback::from(move |_| on_navigate.emit(target.clone()))}
        >
            <span>{title}</span>
            <strong>{command}</strong>
            <small>{description}</small>
        </button>
    }
}

#[function_component(RuntimeScoringPage)]
fn runtime_scoring_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let request_payload = use_state(|| SAMPLE_RUNTIME_SCORE_REQUEST.to_string());
    let score_state = use_state(|| ApiState::<ScoreResponse>::Idle);

    let use_claim_id_template = {
        let request_payload = request_payload.clone();
        Callback::from(move |_| {
            request_payload.set(SAMPLE_RUNTIME_SCORE_REQUEST.to_string());
        })
    };

    let use_full_payload_template = {
        let request_payload = request_payload.clone();
        Callback::from(move |_| {
            request_payload.set(pretty_json(&runtime_full_payload_template()));
        })
    };

    let score = {
        let api_key = api_key.clone();
        let request_payload = request_payload.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let score_state = score_state.clone();
            match serde_json::from_str::<Value>(&request_payload) {
                Ok(payload) => {
                    score_state.set(ApiState::Loading);
                    spawn_local(async move {
                        score_state.set(match score_canonical_claim(payload, api_key).await {
                            Ok(response) => ApiState::Ready(response),
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => score_state.set(ApiState::Failed(format!(
                    "runtime scoring request JSON is invalid: {error}"
                ))),
            }
        })
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Runtime Scoring"}</h2>
                    <p>{"Validate the claim scoring contract and inspect audit-backed routing output. Business reviewers should work from Dashboard, Leads & Cases, or Review Workbench."}</p>
                </div>
                <span class="status-pill">{"Integration Tool"}</span>
            </div>

            {runtime_scoring_blueprint()}

            <div class="inbox-grid">
                <section class="panel result-stack">
                    <h3>{"Scoring Request"}</h3>
                    <label>
                        {"Dev API key"}
                        <input
                            value={(*api_key).clone()}
                            oninput={{
                                let api_key = api_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    api_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={use_claim_id_template}>{"Stored claim"}</button>
                        <button onclick={use_full_payload_template}>{"Full payload"}</button>
                    </div>
                    <label>
                        {"Request JSON"}
                        <textarea
                            class="payload-editor"
                            value={(*request_payload).clone()}
                            oninput={{
                                let request_payload = request_payload.clone();
                                Callback::from(move |event: InputEvent| {
                                    request_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                })
                            }}
                        />
                    </label>
                    <div class="button-row">
                        <button onclick={score} disabled={matches!(&*score_state, ApiState::Loading)}>
                            {if matches!(&*score_state, ApiState::Loading) { "Validating..." } else { "Validate scoring contract" }}
                        </button>
                    </div>
                </section>

                <RuntimeScoreView state={(*score_state).clone()} />
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RuntimeScoreProps {
    state: ApiState<ScoreResponse>,
}

#[function_component(RuntimeScoreView)]
fn runtime_score_view(props: &RuntimeScoreProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Scoring Response"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Submit a stored claim or full payload to validate response shape, route, audit trace, and evidence references."}</p> },
                ApiState::Loading => html! { <p>{"Validating scoring contract..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                            <div><span>{"Risk Score"}</span><strong>{display_value(&response.risk_score)}</strong></div>
                            <div><span>{"RAG"}</span><strong>{response.rag.as_ref().map(display_value).unwrap_or_else(|| "none".into())}</strong></div>
                        </div>
                        {runtime_decision_visual(response)}
                        {runtime_signal_map(response)}
                        <div class="summary-grid">
                            <div><span>{"Action"}</span><strong>{response.recommended_action.as_deref().unwrap_or("review")}</strong></div>
                            <div><span>{"Decision"}</span><strong>{response.decision_outcome.as_deref().unwrap_or("manual_review")}</strong></div>
                            <div><span>{"Authority"}</span><strong>{response.decision_authority.as_deref().unwrap_or("risk_routing_policy")}</strong></div>
                            <div><span>{"Risk Level"}</span><strong>{response.risk_level.as_deref().unwrap_or("unknown")}</strong></div>
                            <div><span>{"Confidence"}</span><strong>{format!("{} / {}", response.confidence.as_deref().unwrap_or("unknown"), optional_u8(response.confidence_score))}</strong></div>
                            <div><span>{"Decision Confidence"}</span><strong>{response.decision_confidence.as_deref().unwrap_or("low")}</strong></div>
                            <div><span>{"Review Required"}</span><strong>{if response.appeal_or_review_required.unwrap_or(true) { "yes" } else { "no" }}</strong></div>
                            <div><span>{"Review Mode"}</span><strong>{response.review_mode.as_deref().unwrap_or("unknown")}</strong></div>
                            <div><span>{"Reason Code"}</span><strong>{response.reason_code.as_deref().unwrap_or("pending")}</strong></div>
                            <div><span>{"Run"}</span><strong>{response.run_id.as_deref().unwrap_or("pending")}</strong></div>
                            <div><span>{"Audit"}</span><strong>{response.audit_id.as_deref().unwrap_or("pending")}</strong></div>
                        </div>
                        <p class="empty">{response.routing_reason.as_deref().unwrap_or("No routing reason returned.")}</p>

                        <h4>{"Risk Signal Breakdown"}</h4>
                        {runtime_score_breakdown(response)}
                        <div class="factor-card-grid">
                            {for response.layers.iter().map(|layer| html! {
                                <div class="metric-row">
                                    <span>{runtime_layer_business_label(layer)}</span>
                                    <strong>{format!("{} / {}", layer.score, layer.status)}</strong>
                                    <small>{&layer.reason}</small>
                                    <small>{format!("evidence: {}", value_refs_label(&layer.evidence_refs))}</small>
                                </div>
                            })}
                        </div>

                        <h4>{"Alerts And Top Reasons"}</h4>
                        <div class="factor-card-grid">
                            {for response.alerts.iter().map(|alert| html! {
                                <div class="metric-row">
                                    <span>{&alert.alert_code}</span>
                                    <strong>{&alert.severity}</strong>
                                    <small>{&alert.reason}</small>
                                    <small>{format!("rule {} v{}", alert.rule_id, alert.rule_version)}</small>
                                    if !alert.required_evidence.is_empty() {
                                        <small>{format!("required evidence: {}", required_evidence_label(&alert.required_evidence))}</small>
                                    }
                                </div>
                            })}
                        </div>
                        if response.top_reasons.is_empty() {
                            <p class="empty">{"No top reasons returned."}</p>
                        } else {
                            <ul class="result-list">
                                {for response.top_reasons.iter().map(|reason| html! { <li>{reason}</li> })}
                            </ul>
                        }

                        <h4>{"Model Output"}</h4>
                        {runtime_model_output(response.model_score.as_ref())}

                        <h4>{"Evidence And Agent Prefill"}</h4>
                        <div class="summary-grid">
                            <div><span>{"Evidence Refs"}</span><strong>{response.evidence_refs.as_ref().map(|refs| refs.len()).unwrap_or(0)}</strong></div>
                            <div><span>{"Features"}</span><strong>{response.feature_values.len()}</strong></div>
                            <div><span>{"Similar Cases"}</span><strong>{response.similar_cases.len()}</strong></div>
                        </div>
                        <small>{format!("evidence: {}", response.evidence_refs.as_ref().map(|refs| value_refs_label(refs)).unwrap_or_else(|| "none".into()))}</small>
                        if let Some(prefill) = &response.agent_investigation_prefill {
                            <pre>{pretty_json(prefill)}</pre>
                        }
                        <details>
                            <summary>{"Routing and clinical payload"}</summary>
                            <pre>{pretty_json(&json!({
                                "routing_policy": response.routing_policy,
                                "clinical_evidence": response.clinical_evidence,
                                "provider_profile": response.provider_profile,
                                "provider_relationships": response.provider_relationships
                            }))}</pre>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

fn runtime_scoring_blueprint() -> Html {
    html! {
        <section class="panel scoring-blueprint-shell">
            <div class="blueprint-claim-card">
                <span>{"Input Contract"}</span>
                <strong>{"Stored claim ID or canonical claim payload"}</strong>
                <div class="blueprint-document">
                    <i class="wide"></i>
                    <i></i>
                    <i class="short"></i>
                    <b></b>
                </div>
            </div>
            <div class="blueprint-layer-rail contract-flow" aria-label="Scoring contract validation flow">
                {blueprint_layer("Request", "Contract", "required IDs, payload shape, tenant scope", "peer")}
                {blueprint_layer("Signals", "Risk context", "rules, model, provider, clinical evidence", "rules")}
                {blueprint_layer("Policy", "Routing", "manual review, case creation, or watchlist", "ml")}
                {blueprint_layer("Audit", "Trace", "run_id, audit_id, evidence_refs", "medical")}
                {blueprint_layer("Queue", "Human work", "reviewers decide; system never denies alone", "route")}
            </div>
            <div class="blueprint-human-card">
                <span>{"Boundary"}</span>
                <strong>{"This page validates runtime output; it is not the claim adjudication desk."}</strong>
                <small>{"Every response must carry route, reason, run_id, audit_id, and evidence_refs."}</small>
            </div>
        </section>
    }
}

fn blueprint_layer(layer: &str, label: &str, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("blueprint-layer", tone.to_string())}>
            <span>{layer}</span>
            <strong>{label}</strong>
            <small>{caption}</small>
        </div>
    }
}

fn runtime_decision_visual(response: &ScoreResponse) -> Html {
    let risk_score = numeric_value(&response.risk_score).clamp(0.0, 100.0);
    let risk_style = format!(
        "background: conic-gradient(var(--red) 0 {:.0}%, #dbe8f8 {:.0}% 100%);",
        risk_score, risk_score
    );
    let rag = response
        .rag
        .as_ref()
        .map(display_value)
        .unwrap_or_else(|| "none".into());
    let evidence_count = response.evidence_refs.as_ref().map(Vec::len).unwrap_or(0);
    html! {
        <div class="runtime-visual-cockpit">
            <div class="risk-gauge-card">
                <div class="risk-gauge" style={risk_style}>
                    <div>
                        <span>{"risk"}</span>
                        <strong>{format!("{:.0}", risk_score)}</strong>
                    </div>
                </div>
                <div class="risk-gauge-meta">
                    <span>{"Routing outcome"}</span>
                    <strong>{response.decision_outcome.as_deref().or(response.recommended_action.as_deref()).unwrap_or("manual_review")}</strong>
                    <small>{format!("{} / {}", rag, response.confidence.as_deref().unwrap_or("confidence pending"))}</small>
                </div>
            </div>
            <div class="runtime-path-card">
                {runtime_path_node("Request", "claim contract", &response.claim_id)}
                {runtime_path_node("Signals", "risk outputs", &format!("{} signals", response.layers.len()))}
                {runtime_path_node("Explain", "alerts + reasons", &format!("{} alerts", response.alerts.len()))}
                {runtime_path_node("Audit", "trace refs", &format!("{evidence_count} refs"))}
                {runtime_path_node("Queue", "human action", response.review_mode.as_deref().unwrap_or("review"))}
            </div>
        </div>
    }
}

fn runtime_signal_map(response: &ScoreResponse) -> Html {
    let model_label = response
        .model_score
        .as_ref()
        .map(|model| format!("{} {}", model.model_key, model.model_version))
        .unwrap_or_else(|| "model pending".into());
    let provider_signal = response
        .provider_profile
        .as_ref()
        .and_then(|profile| profile.get("provider_id"))
        .map(display_value)
        .unwrap_or_else(|| "provider context".into());
    let clinical_signal = response
        .clinical_evidence
        .as_ref()
        .and_then(|clinical| clinical.get("clinical_signal_count"))
        .map(display_value)
        .unwrap_or_else(|| format!("{} layers", response.layers.len()));
    let evidence_count = response.evidence_refs.as_ref().map(Vec::len).unwrap_or(0);

    html! {
        <div class="runtime-signal-map">
            <div class="signal-map-core">
                <span>{"Signal Contract Map"}</span>
                <strong>{&response.claim_id}</strong>
                <small>{response.routing_reason.as_deref().unwrap_or("policy route pending")}</small>
            </div>
            {runtime_signal_node("Controls", &format!("{} alerts", response.alerts.len()), "controls")}
            {runtime_signal_node("Model", &model_label, "model")}
            {runtime_signal_node("Clinical", &clinical_signal, "clinical")}
            {runtime_signal_node("Provider graph", &provider_signal, "graph")}
            {runtime_signal_node("Knowledge", &format!("{} similar cases", response.similar_cases.len()), "knowledge")}
            {runtime_signal_node("Evidence", &format!("{evidence_count} refs"), "evidence")}
        </div>
    }
}

fn runtime_signal_node(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("signal-map-node", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn runtime_layer_business_label(layer: &RuntimeLayerScore) -> String {
    match layer.layer_id.as_str() {
        "L1" => "Peer benchmark signal".into(),
        "L2" => "Deterministic control signal".into(),
        "L3" => "Anomaly signal".into(),
        "L4" => "Model signal".into(),
        "L5" => "Clinical reasonableness signal".into(),
        "L6" => "Provider network signal".into(),
        "L7" => "Routing policy output".into(),
        _ => layer.name.clone(),
    }
}

fn runtime_path_node(label: &str, caption: &str, value: &str) -> Html {
    html! {
        <div class="runtime-path-node">
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct DashboardPageProps {
    on_navigate: Callback<String>,
}

#[function_component(DashboardPage)]
fn dashboard_page(props: &DashboardPageProps) -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let summary_state = use_state(|| ApiState::<DashboardSummary>::Idle);

    let load_summary = {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_dashboard_summary(api_key).await {
                    Ok(summary) => ApiState::Ready(summary),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_summary = load_summary.clone();
        Callback::from(move |_| load_summary.emit(()))
    };

    {
        let load_summary = load_summary.clone();
        use_effect_with((), move |_| {
            load_summary.emit(());
            || ()
        });
    }

    html! {
        <section class="dashboard">
            <div class="dashboard-header">
                    <div>
                        <h2>{"Dashboard"}</h2>
                    <p>{"Watch the operating queue, risk value, review load, and governance health without exposing low-frequency integration tools."}</p>
                </div>
                <span class="status-pill">{"Pilot Operations"}</span>
            </div>

            <section class="panel">
                <h3>{"Dashboard Source"}</h3>
                <p class="empty">{"Using the configured pilot operations principal for queue, value, review-load, and governance signals."}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*summary_state, ApiState::Loading)}>
                        {if matches!(&*summary_state, ApiState::Loading) { "Refreshing..." } else { "Refresh dashboard" }}
                    </button>
                </div>
            </section>

            <DashboardView state={(*summary_state).clone()} on_navigate={props.on_navigate.clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct DashboardProps {
    state: ApiState<DashboardSummary>,
    on_navigate: Callback<String>,
}

#[function_component(DashboardView)]
fn dashboard_view(props: &DashboardProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load the dashboard to inspect operational value and governance coverage."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading dashboard summary..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(summary) => html! {
                    <>
                        {dashboard_pilot_runway(summary, &props.on_navigate)}
                        <section class="panel result-stack">
                            <h3>{"Executive KPIs"}</h3>
                            <div class="score-hero visual-kpis">
                                {kpi_card("Suspected FWA", &summary.suspected_claims.to_string(), "risk")}
                                {kpi_card("Confirmed FWA", &summary.confirmed_fwa.to_string(), "confirmed")}
                                {kpi_card("Risk Amount", &summary.risk_amount, "amount")}
                            </div>
                            <div class="summary-grid">
                                {kpi_card("Savings", &summary.saving_amount, "saving")}
                                {kpi_card("Rule Hits", &summary.rule_hits.to_string(), "rule")}
                                {kpi_card("Investigations", &summary.investigation_results.to_string(), "case")}
                                {kpi_card("QA Reviews", &summary.qa_reviews.to_string(), "qa")}
                                <div><span>{"Risk mix"}</span><strong>{map_counts_business_label(&summary.rag_distribution)}</strong></div>
                                <div><span>{"Schemes"}</span><strong>{map_counts_business_label(&summary.scheme_distribution)}</strong></div>
                            </div>
                            {dashboard_value_proof(summary)}
                            <div class="visual-board">
                                {distribution_bars("Risk distribution", &summary.rag_distribution)}
                                {distribution_bars("Scheme mix", &summary.scheme_distribution)}
                            </div>
                            {operator_queue_snapshot(summary, &props.on_navigate)}
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn dashboard_value_proof(summary: &DashboardSummary) -> Html {
    let value = &summary.value_measurement;
    html! {
        <div class="visual-panel wide-visual value-proof-panel">
            <div class="panel-heading-row">
                <h4>{"Value proof"}</h4>
                <span class="status-token success">{"confirmed vs estimated"}</span>
            </div>
            <div class="summary-grid">
                <div>
                    <span>{"Confirmed prevented payment"}</span>
                    <strong>{format!("{} {}", value.currency, value.prevented_payment)}</strong>
                </div>
                <div>
                    <span>{"Recovered amount"}</span>
                    <strong>{format!("{} {}", value.currency, value.recovered_amount)}</strong>
                </div>
                <div>
                    <span>{"Estimated impact"}</span>
                    <strong>{format!("{} {}", value.currency, value.estimated_impact)}</strong>
                </div>
                <div>
                    <span>{"Review cost"}</span>
                    <strong>{format!("{} {}", value.currency, value.review_cost)}</strong>
                </div>
                <div>
                    <span>{"Reviewer hours"}</span>
                    <strong>{&value.reviewer_capacity_hours}</strong>
                </div>
            </div>
            <p class="empty">{&value.evidence_caveat}</p>
        </div>
    }
}

fn dashboard_pilot_runway(summary: &DashboardSummary, on_navigate: &Callback<String>) -> Html {
    let signal_label = if summary.layer_scores.is_empty() {
        "no signal evidence".into()
    } else {
        format!("{} risk signals", summary.layer_scores.len())
    };
    let qa_work = summary.qa_queue.open_cases + summary.qa_queue.unresolved_feedback_count;
    let audit_label = percent_label(summary.audit_coverage.canonical_trace_coverage);
    html! {
        <section class="panel pilot-runway-panel">
            <div class="section-header">
                <div>
                    <h3>{"Customer Pilot Proof Runway"}</h3>
                    <p>{"A one-screen path for proving a scoped customer principal can move from intake to scoring, human review, QA feedback, audit trace, cost tracking, and savings confirmation."}</p>
                </div>
                <span class="status-token strong">{"demo chain"}</span>
            </div>
            <div class="pilot-runway-map">
                <div class="runway-line"></div>
                {pilot_runway_step("Principal", "Configured principal", "actor + customer scope", "Intake Ops", "source", on_navigate)}
                {pilot_runway_step("Intake", &summary.suspected_claims.to_string(), "normalized claims", "Intake Ops", "intake", on_navigate)}
                {pilot_runway_step("Risk", &signal_label, &map_counts_business_label(&summary.rag_distribution), "Leads & Cases", "score", on_navigate)}
                {pilot_runway_step("Case", &summary.case_sla.open_cases.to_string(), "open investigations", "Leads & Cases", "case", on_navigate)}
                {pilot_runway_step("QA", &qa_work.to_string(), "open QA + feedback", "Review Workbench", "qa", on_navigate)}
                {pilot_runway_step("Audit", &audit_label, "canonical trace coverage", "Governance", "audit", on_navigate)}
                {pilot_runway_step("Value", &summary.value_measurement.prevented_payment, "confirmed prevented payment", "Dashboard", "roi", on_navigate)}
            </div>
            <div class="pilot-runway-proof">
                <div>
                    <span>{"Human gate"}</span>
                    <strong>{format!("{} cases / {} QA", summary.case_sla.open_cases, summary.qa_queue.open_cases)}</strong>
                    <small>{"High-risk work remains routed to manual review, medical review, QA, or investigation."}</small>
                </div>
                <div>
                    <span>{"Agent boundary"}</span>
                    <strong>{format!("{} evidence-backed / {} runs", summary.agent_governance.evidence_backed_runs, summary.agent_governance.total_runs)}</strong>
                    <small>{"Agent output is shown as investigation assistance, with policy checks and approvals tracked separately."}</small>
                </div>
                <div>
                    <span>{"Savings / review cost"}</span>
                    <strong>{format!("{} / {}", summary.saving_amount, summary.value_measurement.review_cost)}</strong>
                    <small>{"Costs are tracked as pilot investment until reviewed savings are confirmed."}</small>
                </div>
            </div>
        </section>
    }
}

fn pilot_runway_step(
    label: &'static str,
    value: &str,
    detail: &str,
    target: &'static str,
    tone: &'static str,
    on_navigate: &Callback<String>,
) -> Html {
    let target_name = target.to_string();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("pilot-runway-step", tone)}
            onclick={Callback::from(move |_| on_navigate.emit(target_name.clone()))}
        >
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{detail}</small>
        </button>
    }
}

#[function_component(RulesPage)]
fn rules_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let rule_id = use_state(|| "rule_early_claim".to_string());
    let model_key = use_state(|| "baseline_fwa".to_string());
    let model_version = use_state(|| "0.3.0-candidate".to_string());
    let explanation_feature = use_state(|| "claim_amount_to_limit_ratio".to_string());
    let explanation_contribution = use_state(|| "1.40".to_string());
    let feature_importance_uri =
        use_state(|| "data/eval/baseline_fwa/v3/feature_importance.parquet".to_string());
    let discovery_dataset_uri =
        use_state(|| "data/public-mvp/split=train/part-00000.parquet".to_string());
    let discovery_label_column = use_state(|| "confirmed_fwa".to_string());
    let discovery_claim_id_column = use_state(|| "claim_id".to_string());
    let discovery_feature_fields = use_state(String::new);
    let discovery_tree_depth = use_state(|| "2".to_string());
    let evaluation_dataset_json = use_state(|| pretty_json(&Value::Array(rule_demo_samples())));
    let selected_candidate_id = use_state(String::new);
    let rule_reviewer = use_state(|| "rule-review".to_string());
    let rule_review_notes = use_state(|| {
        "Explainable signal reviewed against backtest evidence and shadow gate readiness."
            .to_string()
    });
    let rule_review_evidence_refs =
        use_state(|| "rules:discovery-candidate:v1, backtest:demo, shadow:gate-check".to_string());
    let snapshot_state = use_state(|| ApiState::<RuleOpsSnapshot>::Idle);
    let discovery_state = use_state(|| ApiState::<RuleDiscoveryResponse>::Idle);
    let backtest_state = use_state(|| ApiState::<RuleBacktestResponse>::Idle);
    let save_state = use_state(|| ApiState::<Value>::Idle);
    let review_state = use_state(|| ApiState::<Value>::Idle);
    let shadow_state = use_state(|| ApiState::<Value>::Idle);
    let backtested_candidate_id = use_state(String::new);
    let accepted_candidate_ids = use_state(Vec::<String>::new);
    let shadowed_candidate_ids = use_state(Vec::<String>::new);
    let final_accepted_candidate_ids = use_state(Vec::<String>::new);
    let rejected_candidate_ids = use_state(Vec::<String>::new);

    let load_rules = {
        let api_key = api_key.clone();
        let rule_id = rule_id.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let rule_id = (*rule_id).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_rule_ops_snapshot(api_key, rule_id).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_rules = load_rules.clone();
        Callback::from(move |_| load_rules.emit(()))
    };

    {
        let load_rules = load_rules.clone();
        use_effect_with((), move |_| {
            load_rules.emit(());
            || ()
        });
    }

    let discover_candidates = {
        let api_key = api_key.clone();
        let model_key = model_key.clone();
        let model_version = model_version.clone();
        let explanation_feature = explanation_feature.clone();
        let explanation_contribution = explanation_contribution.clone();
        let feature_importance_uri = feature_importance_uri.clone();
        let discovery_dataset_uri = discovery_dataset_uri.clone();
        let discovery_label_column = discovery_label_column.clone();
        let discovery_claim_id_column = discovery_claim_id_column.clone();
        let discovery_feature_fields = discovery_feature_fields.clone();
        let discovery_tree_depth = discovery_tree_depth.clone();
        let evaluation_dataset_json = evaluation_dataset_json.clone();
        let selected_candidate_id = selected_candidate_id.clone();
        let discovery_state = discovery_state.clone();
        let backtest_state = backtest_state.clone();
        let save_state = save_state.clone();
        let review_state = review_state.clone();
        let shadow_state = shadow_state.clone();
        let backtested_candidate_id = backtested_candidate_id.clone();
        let accepted_candidate_ids = accepted_candidate_ids.clone();
        let shadowed_candidate_ids = shadowed_candidate_ids.clone();
        let final_accepted_candidate_ids = final_accepted_candidate_ids.clone();
        let rejected_candidate_ids = rejected_candidate_ids.clone();
        Callback::from(move |_| {
            let Ok(contribution) = explanation_contribution.trim().parse::<f64>() else {
                discovery_state.set(ApiState::Failed(
                    "model contribution must be numeric".into(),
                ));
                return;
            };
            let samples = if discovery_dataset_uri.trim().is_empty() {
                match parse_json_array(&evaluation_dataset_json, "evaluation dataset") {
                    Ok(samples) => samples,
                    Err(error) => {
                        discovery_state.set(ApiState::Failed(error));
                        return;
                    }
                }
            } else {
                Vec::new()
            };
            let payload = rule_discovery_payload(
                &model_key,
                &model_version,
                &explanation_feature,
                contribution,
                &feature_importance_uri,
                &discovery_dataset_uri,
                &discovery_label_column,
                &discovery_claim_id_column,
                &discovery_feature_fields,
                &discovery_tree_depth,
                samples,
            );
            let api_key = (*api_key).clone();
            let selected_candidate_id = selected_candidate_id.clone();
            let discovery_state = discovery_state.clone();
            let backtest_state = backtest_state.clone();
            let save_state = save_state.clone();
            let review_state = review_state.clone();
            discovery_state.set(ApiState::Loading);
            backtest_state.set(ApiState::Idle);
            save_state.set(ApiState::Idle);
            review_state.set(ApiState::Idle);
            shadow_state.set(ApiState::Idle);
            backtested_candidate_id.set(String::new());
            accepted_candidate_ids.set(Vec::new());
            shadowed_candidate_ids.set(Vec::new());
            final_accepted_candidate_ids.set(Vec::new());
            rejected_candidate_ids.set(Vec::new());
            spawn_local(async move {
                match request_json::<RuleDiscoveryResponse>(
                    "/api/v1/ops/rules/discover",
                    api_key,
                    payload,
                )
                .await
                {
                    Ok(response) => {
                        selected_candidate_id.set(
                            response
                                .candidates
                                .first()
                                .map(rule_candidate_id)
                                .unwrap_or_default(),
                        );
                        discovery_state.set(ApiState::Ready(response));
                    }
                    Err(error) => discovery_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let backtest_candidate = {
        let api_key = api_key.clone();
        let selected_candidate_id = selected_candidate_id.clone();
        let discovery_dataset_uri = discovery_dataset_uri.clone();
        let discovery_label_column = discovery_label_column.clone();
        let discovery_claim_id_column = discovery_claim_id_column.clone();
        let evaluation_dataset_json = evaluation_dataset_json.clone();
        let discovery_state = discovery_state.clone();
        let backtest_state = backtest_state.clone();
        let backtested_candidate_id = backtested_candidate_id.clone();
        Callback::from(move |_| {
            let candidate_rule = match &*discovery_state {
                ApiState::Ready(response) => {
                    selected_rule_candidate(response, &selected_candidate_id)
                        .map(|candidate| candidate.rule.clone())
                }
                _ => None,
            };
            let Some(rule) = candidate_rule else {
                backtest_state.set(ApiState::Failed(
                    "select a discovered candidate first".into(),
                ));
                return;
            };
            let candidate_id = (*selected_candidate_id).clone();
            let samples = if discovery_dataset_uri.trim().is_empty() {
                match parse_json_array(&evaluation_dataset_json, "evaluation dataset") {
                    Ok(samples) => samples,
                    Err(error) => {
                        backtest_state.set(ApiState::Failed(error));
                        return;
                    }
                }
            } else {
                Vec::new()
            };
            let api_key = (*api_key).clone();
            let backtest_state = backtest_state.clone();
            let backtested_candidate_id = backtested_candidate_id.clone();
            let payload = rule_backtest_payload(
                rule,
                &discovery_dataset_uri,
                &discovery_label_column,
                &discovery_claim_id_column,
                samples,
            );
            backtested_candidate_id.set(String::new());
            backtest_state.set(ApiState::Loading);
            spawn_local(async move {
                match request_json::<RuleBacktestResponse>(
                    "/api/v1/ops/rules/backtest",
                    api_key,
                    payload,
                )
                .await
                {
                    Ok(response) => {
                        backtested_candidate_id.set(candidate_id);
                        backtest_state.set(ApiState::Ready(response));
                    }
                    Err(error) => backtest_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let selected_candidate_available = matches!(
        &*discovery_state,
        ApiState::Ready(response) if selected_rule_candidate(response, &selected_candidate_id).is_some()
    );
    let selected_candidate_backtest_ready = matches!(
        &*backtest_state,
        ApiState::Ready(backtest)
            if *backtested_candidate_id == *selected_candidate_id
                && backtest.promotion_recommendation == "eligible_for_review"
                && backtest.blockers.is_empty()
    );
    let selected_candidate_draft_saved = (*accepted_candidate_ids)
        .iter()
        .any(|id| id == selected_candidate_id.as_str());
    let selected_candidate_shadow_ready = (*shadowed_candidate_ids)
        .iter()
        .any(|id| id == selected_candidate_id.as_str());
    let can_submit_shadow_evidence =
        selected_candidate_backtest_ready && selected_candidate_draft_saved;
    let save_candidate_draft = {
        let api_key = api_key.clone();
        let selected_candidate_id = selected_candidate_id.clone();
        let discovery_state = discovery_state.clone();
        let backtest_state = backtest_state.clone();
        let snapshot_state = snapshot_state.clone();
        let save_state = save_state.clone();
        let shadow_state = shadow_state.clone();
        let rule_id = rule_id.clone();
        let accepted_candidate_ids = accepted_candidate_ids.clone();
        let rejected_candidate_ids = rejected_candidate_ids.clone();
        Callback::from(move |_| {
            let candidate = match &*discovery_state {
                ApiState::Ready(response) => {
                    selected_rule_candidate(response, &selected_candidate_id).cloned()
                }
                _ => None,
            };
            let Some(candidate) = candidate else {
                save_state.set(ApiState::Failed(
                    "select a discovered candidate before review".into(),
                ));
                return;
            };
            let candidate_rule_id = rule_candidate_id(&candidate);
            let _backtest = match &*backtest_state {
                ApiState::Ready(backtest)
                    if backtest.promotion_recommendation == "eligible_for_review"
                        && backtest.blockers.is_empty() =>
                {
                    backtest.clone()
                }
                ApiState::Ready(backtest) => {
                    save_state.set(ApiState::Failed(format!(
                        "selected candidate backtest is not eligible: {}",
                        if backtest.blockers.is_empty() {
                            backtest.promotion_recommendation.clone()
                        } else {
                            refs_label(&backtest.blockers)
                        }
                    )));
                    return;
                }
                _ => {
                    save_state.set(ApiState::Failed(
                        "run an eligible backtest before saving this candidate for shadow".into(),
                    ));
                    return;
                }
            };
            let api_key = (*api_key).clone();
            let rule_id = rule_id.clone();
            let snapshot_state = snapshot_state.clone();
            let save_state = save_state.clone();
            let shadow_state = shadow_state.clone();
            let accepted_candidate_ids = accepted_candidate_ids.clone();
            let rejected_candidate_ids = rejected_candidate_ids.clone();
            save_state.set(ApiState::Loading);
            spawn_local(async move {
                match save_rule_candidate_draft(
                    api_key.clone(),
                    candidate.rule,
                    Some("rule-discovery-shadow".into()),
                )
                .await
                {
                    Ok(response) => {
                        let saved_rule_id =
                            response_rule_id(&response).unwrap_or(candidate_rule_id.clone());
                        rule_id.set(saved_rule_id.clone());
                        save_state.set(ApiState::Ready(response.clone()));
                        accepted_candidate_ids.set(push_unique(
                            (*accepted_candidate_ids).clone(),
                            candidate_rule_id.clone(),
                        ));
                        rejected_candidate_ids.set(remove_id(
                            (*rejected_candidate_ids).clone(),
                            &candidate_rule_id,
                        ));
                        shadow_state.set(ApiState::Idle);
                        snapshot_state.set(ApiState::Loading);
                        snapshot_state.set(
                            match get_rule_ops_snapshot(api_key, candidate_rule_id).await {
                                Ok(snapshot) => ApiState::Ready(snapshot),
                                Err(error) => ApiState::Failed(error),
                            },
                        );
                    }
                    Err(error) => {
                        save_state.set(ApiState::Failed(error.clone()));
                    }
                }
            });
        })
    };

    let accept_candidate = {
        let api_key = api_key.clone();
        let selected_candidate_id = selected_candidate_id.clone();
        let discovery_state = discovery_state.clone();
        let backtest_state = backtest_state.clone();
        let review_state = review_state.clone();
        let rule_reviewer = rule_reviewer.clone();
        let rule_review_notes = rule_review_notes.clone();
        let rule_review_evidence_refs = rule_review_evidence_refs.clone();
        let final_accepted_candidate_ids = final_accepted_candidate_ids.clone();
        let rejected_candidate_ids = rejected_candidate_ids.clone();
        let shadowed_candidate_ids = shadowed_candidate_ids.clone();
        Callback::from(move |_| {
            let candidate = match &*discovery_state {
                ApiState::Ready(response) => {
                    selected_rule_candidate(response, &selected_candidate_id).cloned()
                }
                _ => None,
            };
            let Some(candidate) = candidate else {
                review_state.set(ApiState::Failed(
                    "select a discovered candidate before final review".into(),
                ));
                return;
            };
            let candidate_rule_id = rule_candidate_id(&candidate);
            if !(*shadowed_candidate_ids)
                .iter()
                .any(|id| id == &candidate_rule_id)
            {
                review_state.set(ApiState::Failed(
                    "submit shadow evidence before accepting this candidate".into(),
                ));
                return;
            }
            let mut evidence_refs = parse_tags(&rule_review_evidence_refs);
            let backtest = match &*backtest_state {
                ApiState::Ready(backtest)
                    if backtest.promotion_recommendation == "eligible_for_review"
                        && backtest.blockers.is_empty() =>
                {
                    backtest.clone()
                }
                _ => {
                    review_state.set(ApiState::Failed(
                        "run an eligible backtest before final candidate review".into(),
                    ));
                    return;
                }
            };
            for evidence_ref in backtest.evidence_refs {
                evidence_refs = push_unique(evidence_refs, evidence_ref);
            }
            evidence_refs = push_unique(
                evidence_refs,
                format!("rule_shadow_runs:artifacts/rules/{candidate_rule_id}/shadow_report.json"),
            );
            let api_key = (*api_key).clone();
            let reviewer = (*rule_reviewer).clone();
            let notes = (*rule_review_notes).clone();
            let review_state = review_state.clone();
            let final_accepted_candidate_ids = final_accepted_candidate_ids.clone();
            let rejected_candidate_ids = rejected_candidate_ids.clone();
            review_state.set(ApiState::Loading);
            spawn_local(async move {
                match accept_rule_candidate(api_key, candidate.rule, reviewer, notes, evidence_refs)
                    .await
                {
                    Ok(response) => {
                        final_accepted_candidate_ids.set(push_unique(
                            (*final_accepted_candidate_ids).clone(),
                            candidate_rule_id.clone(),
                        ));
                        rejected_candidate_ids.set(remove_id(
                            (*rejected_candidate_ids).clone(),
                            &candidate_rule_id,
                        ));
                        review_state.set(ApiState::Ready(response));
                    }
                    Err(error) => review_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let reject_candidate = {
        let api_key = api_key.clone();
        let selected_candidate_id = selected_candidate_id.clone();
        let discovery_state = discovery_state.clone();
        let rule_reviewer = rule_reviewer.clone();
        let rule_review_notes = rule_review_notes.clone();
        let rule_review_evidence_refs = rule_review_evidence_refs.clone();
        let review_state = review_state.clone();
        let accepted_candidate_ids = accepted_candidate_ids.clone();
        let rejected_candidate_ids = rejected_candidate_ids.clone();
        let final_accepted_candidate_ids = final_accepted_candidate_ids.clone();
        Callback::from(move |_| {
            let candidate = match &*discovery_state {
                ApiState::Ready(response) => {
                    selected_rule_candidate(response, &selected_candidate_id).cloned()
                }
                _ => None,
            };
            let Some(candidate) = candidate else {
                review_state.set(ApiState::Failed(
                    "select a discovered candidate before review".into(),
                ));
                return;
            };
            let candidate_rule_id = rule_candidate_id(&candidate);
            let api_key = (*api_key).clone();
            let reviewer = (*rule_reviewer).clone();
            let notes = (*rule_review_notes).clone();
            let evidence_refs = parse_tags(&rule_review_evidence_refs);
            let review_state = review_state.clone();
            let accepted_candidate_ids = accepted_candidate_ids.clone();
            let rejected_candidate_ids = rejected_candidate_ids.clone();
            let final_accepted_candidate_ids = final_accepted_candidate_ids.clone();
            review_state.set(ApiState::Loading);
            spawn_local(async move {
                match reject_rule_candidate(api_key, candidate.rule, reviewer, notes, evidence_refs)
                    .await
                {
                    Ok(response) => {
                        rejected_candidate_ids.set(push_unique(
                            (*rejected_candidate_ids).clone(),
                            candidate_rule_id.clone(),
                        ));
                        accepted_candidate_ids.set(remove_id(
                            (*accepted_candidate_ids).clone(),
                            &candidate_rule_id,
                        ));
                        final_accepted_candidate_ids.set(remove_id(
                            (*final_accepted_candidate_ids).clone(),
                            &candidate_rule_id,
                        ));
                        review_state.set(ApiState::Ready(response));
                    }
                    Err(error) => review_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let submit_shadow_evidence = {
        let api_key = api_key.clone();
        let rule_id = rule_id.clone();
        let selected_candidate_id = selected_candidate_id.clone();
        let discovery_state = discovery_state.clone();
        let accepted_candidate_ids = accepted_candidate_ids.clone();
        let shadowed_candidate_ids = shadowed_candidate_ids.clone();
        let snapshot_state = snapshot_state.clone();
        let backtest_state = backtest_state.clone();
        let rule_reviewer = rule_reviewer.clone();
        let rule_review_notes = rule_review_notes.clone();
        let rule_review_evidence_refs = rule_review_evidence_refs.clone();
        let shadow_state = shadow_state.clone();
        let load_rules = load_rules.clone();
        Callback::from(move |_| {
            let backtest = match &*backtest_state {
                ApiState::Ready(response) => response.clone(),
                _ => {
                    shadow_state.set(ApiState::Failed(
                        "run backtest before submitting shadow evidence".into(),
                    ));
                    return;
                }
            };
            let selected_candidate = match &*discovery_state {
                ApiState::Ready(response) => {
                    selected_rule_candidate(response, &selected_candidate_id).cloned()
                }
                _ => None,
            };
            let (target_rule_id, target_rule_version) = if let Some(candidate) = selected_candidate
            {
                let candidate_rule_id = rule_candidate_id(&candidate);
                if !(*accepted_candidate_ids)
                    .iter()
                    .any(|id| id == &candidate_rule_id)
                {
                    shadow_state.set(ApiState::Failed(
                        "save the discovered candidate draft before submitting shadow evidence for it".into(),
                    ));
                    return;
                }
                (candidate_rule_id, rule_candidate_version(&candidate))
            } else {
                match &*snapshot_state {
                    ApiState::Ready(snapshot) => {
                        (snapshot.gates.rule_id.clone(), snapshot.gates.rule_version)
                    }
                    _ => ((*rule_id).clone(), 1),
                }
            };
            let report_uri = format!("artifacts/rules/{target_rule_id}/shadow_report.json");
            let rule_ref = format!("rules:{target_rule_id}:v{target_rule_version}");
            let shadow_ref = format!("rule_shadow_runs:{report_uri}");
            let mut evidence_refs = parse_tags(&rule_review_evidence_refs);
            evidence_refs = push_unique(evidence_refs, rule_ref);
            evidence_refs = push_unique(evidence_refs, shadow_ref);
            let api_key = (*api_key).clone();
            let reviewer = (*rule_reviewer).clone();
            let notes = (*rule_review_notes).clone();
            let shadow_state = shadow_state.clone();
            let shadowed_candidate_ids = shadowed_candidate_ids.clone();
            let load_rules = load_rules.clone();
            let shadow_rule_id = target_rule_id.clone();
            shadow_state.set(ApiState::Loading);
            spawn_local(async move {
                match submit_rule_shadow_run(
                    api_key,
                    shadow_rule_id,
                    target_rule_version,
                    backtest,
                    report_uri,
                    reviewer,
                    notes,
                    evidence_refs,
                )
                .await
                {
                    Ok(response) => {
                        shadowed_candidate_ids.set(push_unique(
                            (*shadowed_candidate_ids).clone(),
                            target_rule_id.clone(),
                        ));
                        shadow_state.set(ApiState::Ready(response));
                        load_rules.emit(());
                    }
                    Err(error) => shadow_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"ML Rule Candidate Review"}</h2>
                    <p>{"Review rules discovered from model explanations, offline mining, or QA feedback. Operators run backtests, inspect shadow gates, and accept or reject the candidate before it can enter the governed rule library."}</p>
                </div>
                <span class="status-pill">{"Human review gate"}</span>
            </div>

            <section class="panel result-stack">
                <h3>{"Rule Discovery Workbench"}</h3>
                {rule_backfill_pipeline(&discovery_state, &backtest_state, &save_state, &shadow_state, &review_state)}
                <div class="form-grid">
                    <label>
                        {"Gate Rule ID"}
                        <input
                            value={(*rule_id).clone()}
                            oninput={{
                                let rule_id = rule_id.clone();
                                Callback::from(move |event: InputEvent| {
                                    rule_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Model Key"}
                        <input
                            value={(*model_key).clone()}
                            oninput={{
                                let model_key = model_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    model_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Model Version"}
                        <input
                            value={(*model_version).clone()}
                            oninput={{
                                let model_version = model_version.clone();
                                Callback::from(move |event: InputEvent| {
                                    model_version.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Explained Feature"}
                        <input
                            value={(*explanation_feature).clone()}
                            oninput={{
                                let explanation_feature = explanation_feature.clone();
                                Callback::from(move |event: InputEvent| {
                                    explanation_feature.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Contribution"}
                        <input
                            value={(*explanation_contribution).clone()}
                            oninput={{
                                let explanation_contribution = explanation_contribution.clone();
                                Callback::from(move |event: InputEvent| {
                                    explanation_contribution.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Explanation Artifact"}
                        <input
                            value={(*feature_importance_uri).clone()}
                            oninput={{
                                let feature_importance_uri = feature_importance_uri.clone();
                                Callback::from(move |event: InputEvent| {
                                    feature_importance_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Mining Dataset URI"}
                        <input
                            value={(*discovery_dataset_uri).clone()}
                            oninput={{
                                let discovery_dataset_uri = discovery_dataset_uri.clone();
                                Callback::from(move |event: InputEvent| {
                                    discovery_dataset_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Label Column"}
                        <input
                            value={(*discovery_label_column).clone()}
                            oninput={{
                                let discovery_label_column = discovery_label_column.clone();
                                Callback::from(move |event: InputEvent| {
                                    discovery_label_column.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Claim ID Column"}
                        <input
                            value={(*discovery_claim_id_column).clone()}
                            oninput={{
                                let discovery_claim_id_column = discovery_claim_id_column.clone();
                                Callback::from(move |event: InputEvent| {
                                    discovery_claim_id_column.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Feature Columns"}
                        <input
                            value={(*discovery_feature_fields).clone()}
                            oninput={{
                                let discovery_feature_fields = discovery_feature_fields.clone();
                                Callback::from(move |event: InputEvent| {
                                    discovery_feature_fields.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Tree Depth"}
                        <input
                            value={(*discovery_tree_depth).clone()}
                            oninput={{
                                let discovery_tree_depth = discovery_tree_depth.clone();
                                Callback::from(move |event: InputEvent| {
                                    discovery_tree_depth.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Reviewer"}
                        <input
                            value={(*rule_reviewer).clone()}
                            oninput={{
                                let rule_reviewer = rule_reviewer.clone();
                                Callback::from(move |event: InputEvent| {
                                    rule_reviewer.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Review Evidence Refs"}
                        <input
                            value={(*rule_review_evidence_refs).clone()}
                            oninput={{
                                let rule_review_evidence_refs = rule_review_evidence_refs.clone();
                                Callback::from(move |event: InputEvent| {
                                    rule_review_evidence_refs.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <label class="full-field">
                    {"Inline Labeled Evaluation Dataset"}
                    <textarea
                        class="code-field"
                        value={(*evaluation_dataset_json).clone()}
                        oninput={{
                            let evaluation_dataset_json = evaluation_dataset_json.clone();
                            Callback::from(move |event: InputEvent| {
                                evaluation_dataset_json.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <label class="full-field">
                    {"Human Review Notes"}
                    <textarea
                        value={(*rule_review_notes).clone()}
                        oninput={{
                            let rule_review_notes = rule_review_notes.clone();
                            Callback::from(move |event: InputEvent| {
                                rule_review_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <div class="button-row">
                    <button onclick={discover_candidates} disabled={matches!(&*discovery_state, ApiState::Loading)}>
                        {if matches!(&*discovery_state, ApiState::Loading) { "Discovering..." } else { "Discover candidates" }}
                    </button>
                    <button onclick={backtest_candidate} disabled={!selected_candidate_available || matches!(&*backtest_state, ApiState::Loading)}>
                        {if matches!(&*backtest_state, ApiState::Loading) { "Backtesting..." } else { "Run backtest" }}
                    </button>
                    <button onclick={save_candidate_draft} disabled={!selected_candidate_available || !selected_candidate_backtest_ready || matches!(&*save_state, ApiState::Loading)}>
                        {if matches!(&*save_state, ApiState::Loading) { "Saving draft..." } else { "Save draft for shadow" }}
                    </button>
                    <button onclick={submit_shadow_evidence} disabled={!can_submit_shadow_evidence || matches!(&*shadow_state, ApiState::Loading)}>
                        {if matches!(&*shadow_state, ApiState::Loading) { "Submitting shadow..." } else { "Submit shadow evidence" }}
                    </button>
                    <button onclick={accept_candidate} disabled={!selected_candidate_available || !selected_candidate_backtest_ready || !selected_candidate_shadow_ready || matches!(&*review_state, ApiState::Loading)}>
                        {if matches!(&*review_state, ApiState::Loading) { "Reviewing..." } else { "Accept after shadow evidence" }}
                    </button>
                    <button onclick={reject_candidate} disabled={!selected_candidate_available || matches!(&*review_state, ApiState::Loading)}>
                        {"Reject selected candidate"}
                    </button>
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh gates" }}
                    </button>
                </div>
                {rule_candidate_review_state(&review_state)}
                {rule_candidate_workflow(
                    &discovery_state,
                    &backtest_state,
                    &save_state,
                    &shadow_state,
                    &selected_candidate_id,
                    &accepted_candidate_ids,
                    &shadowed_candidate_ids,
                    &final_accepted_candidate_ids,
                    &rejected_candidate_ids,
                )}
            </section>

            <RulesView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RulesProps {
    state: ApiState<RuleOpsSnapshot>,
}

#[function_component(RulesView)]
fn rules_view(props: &RulesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load rules to inspect deterministic detection controls."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading rule operations..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => {
                    let selected_rule = snapshot.rules.iter().find(|rule| rule.rule_id == snapshot.gates.rule_id);
                    html! {
                        <>
                            {rule_pack_matrix(snapshot)}
                            <section class="panel result-stack">
                                <h3>{"Rule Library"}</h3>
                                if snapshot.rules.is_empty() {
                                    <p class="empty">{"No rules returned."}</p>
                                } else {
                                    <div class="factor-card-grid">
                                        {for snapshot.rules.iter().take(4).map(|rule| {
                                            let performance = rule_performance_for(&snapshot.performance, &rule.rule_id);
                                            html! {
                                                <div class="factor-card">
                                                    <div>
                                                        <strong>{&rule.name}</strong>
                                                        <span>{format!("{} / {} / {}", rule.status, rule.review_mode, rule.scheme_family)}</span>
                                                    </div>
                                                    <div class="summary-grid">
                                                        <div><span>{"Score"}</span><strong>{rule.score}</strong></div>
                                                        <div><span>{"Action"}</span><strong>{&rule.recommended_action}</strong></div>
                                                        <div><span>{"Alert"}</span><strong>{&rule.alert_code}</strong></div>
                                                        <div><span>{"Owner"}</span><strong>{&rule.owner}</strong></div>
                                                        <div><span>{"Triggers"}</span><strong>{performance.map(|item| item.trigger_count).unwrap_or(0)}</strong></div>
                                                        <div><span>{"Evidence"}</span><strong>{refs_count_label(&rule.evidence_refs)}</strong></div>
                                                    </div>
                                                    <details class="data-source-detail governance-detail">
                                                        <summary>{"Rule library detail"}</summary>
                                                        <small>{format!("rule: {}", rule.rule_id)}</small>
                                                        <small>{format!("version: active {} / latest {}", optional_u32(rule.active_version), rule.latest_version)}</small>
                                                        <small>{format!("scope: {} / {} / {}", rule.applicability_scope.review_mode, rule.applicability_scope.scheme_family, rule.applicability_scope.source)}</small>
                                                        <small>{format!("evidence: {}", refs_label(&rule.evidence_refs))}</small>
                                                    </details>
                                                </div>
                                            }
                                        })}
                                    </div>
                                    if snapshot.rules.len() > 4 {
                                        <details class="data-source-detail governance-detail">
                                            <summary>{format!("Additional rule library detail: {} rules", snapshot.rules.len() - 4)}</summary>
                                            <div class="governance-check-list">
                                                {for snapshot.rules.iter().skip(4).map(|rule| html! {
                                                    <div>
                                                        <strong>{&rule.name}</strong>
                                                        <span>{format!("{} / {} / {}", rule.rule_id, rule.status, rule.recommended_action)}</span>
                                                        <small>{format!("evidence: {}", refs_count_label(&rule.evidence_refs))}</small>
                                                    </div>
                                                })}
                                            </div>
                                        </details>
                                    }
                                }
                            </section>

                        <section class="panel result-stack">
                            <h3>{"Rule Performance"}</h3>
                            {rule_performance_visual(&snapshot.performance)}
                            if snapshot.performance.is_empty() {
                                <p class="empty">{"No rule performance records returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.performance.iter().take(4).map(|item| html! {
                                            <div class="metric-row">
                                                <span>{format!("{} / {}", item.rule_id, item.alert_code)}</span>
                                                <strong>{format!("precision {}", percent_label(item.precision))}</strong>
                                                <small>{format!("triggers {} / reviewed {} / confirmed {}", item.trigger_count, item.reviewed_count, item.confirmed_fwa_count)}</small>
                                                <details class="data-source-detail governance-detail">
                                                    <summary>{"Performance detail"}</summary>
                                                    <small>{format!("FP {} / rate {} / saving {} / ROI {:.2} / mark {}", item.false_positive_count, percent_label(item.false_positive_rate), item.saving_amount, item.roi, percent_label(item.mark_rate))}</small>
                                                </details>
                                            </div>
                                        })}
                                    </div>
                                    if snapshot.performance.len() > 4 {
                                        <details class="data-source-detail governance-detail">
                                            <summary>{format!("Additional rule performance detail: {} rules", snapshot.performance.len() - 4)}</summary>
                                            <div class="governance-check-list">
                                                {for snapshot.performance.iter().skip(4).map(|item| html! {
                                                    <div>
                                                        <strong>{&item.rule_id}</strong>
                                                        <span>{format!("precision {} / triggers {}", percent_label(item.precision), item.trigger_count)}</span>
                                                        <small>{format!("false positive rate {} / saving {}", percent_label(item.false_positive_rate), item.saving_amount)}</small>
                                                    </div>
                                                })}
                                            </div>
                                        </details>
                                    }
                                }
                            </section>

                        <section class="panel result-stack">
                            <h3>{"Rule Promotion Readiness"}</h3>
                            {rule_gate_pipeline(&snapshot.gates)}
                            <div class="score-hero">
                                <div><span>{"Rule"}</span><strong>{&snapshot.gates.rule_id}</strong></div>
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                                </div>
                                <div class="summary-grid">
                                    <div><span>{"Status"}</span><strong>{&snapshot.gates.status}</strong></div>
                                    <div><span>{"Version"}</span><strong>{snapshot.gates.rule_version}</strong></div>
                                    <div><span>{"Review Mode"}</span><strong>{&snapshot.gates.review_mode}</strong></div>
                                    <div><span>{"Triggers"}</span><strong>{snapshot.gates.trigger_count}</strong></div>
                                    <div><span>{"Reviewed"}</span><strong>{snapshot.gates.reviewed_count}</strong></div>
                                    <div><span>{"False Positive Rate"}</span><strong>{percent_label(snapshot.gates.false_positive_rate)}</strong></div>
                                    <div><span>{"Saving"}</span><strong>{&snapshot.gates.saving_amount}</strong></div>
                                    <div><span>{"Open Feedback"}</span><strong>{snapshot.gates.open_rule_feedback_count}</strong></div>
                                    <div><span>{"Unresolved Feedback"}</span><strong>{snapshot.gates.unresolved_rule_feedback_count}</strong></div>
                                    <div><span>{"Approved Labels"}</span><strong>{snapshot.gates.approved_label_count}</strong></div>
                                    <div><span>{"Needs Review Labels"}</span><strong>{snapshot.gates.needs_review_label_count}</strong></div>
                                    <div><span>{"Selected Rule"}</span><strong>{selected_rule.map(|rule| rule.name.as_str()).unwrap_or("not listed")}</strong></div>
                                </div>
                                <h4>{"Backtest Evidence"}</h4>
                                if let Some(rule) = selected_rule {
                                    <>
                                        <div class="summary-grid">
                                            <div><span>{"Status"}</span><strong>{&rule.backtest_result.status}</strong></div>
                                            <div><span>{"Sample / Matched"}</span><strong>{format!("{} / {}", rule.backtest_result.sample_count, rule.backtest_result.matched_count)}</strong></div>
                                            <div><span>{"Precision / Recall"}</span><strong>{format!("{} / {}", percent_label(rule.backtest_result.precision), percent_label(rule.backtest_result.recall))}</strong></div>
                                            <div><span>{"FP Rate"}</span><strong>{percent_label(rule.backtest_result.false_positive_rate)}</strong></div>
                                            <div><span>{"Saving"}</span><strong>{&rule.backtest_result.estimated_saving}</strong></div>
                                            <div><span>{"Evidence"}</span><strong>{refs_count_label(&rule.backtest_result.evidence_refs)}</strong></div>
                                        </div>
                                        <details class="data-source-detail governance-detail">
                                            <summary>{"Backtest evidence detail"}</summary>
                                            <small>{format!("lift: {:.2}", rule.backtest_result.lift)}</small>
                                            <small>{format!("backtest at: {}", rule.backtest_result.created_at.as_deref().unwrap_or("not_run"))}</small>
                                            <small>{format!("rule estimate: {}", rule.estimated_saving)}</small>
                                            <small>{format!("backtest evidence: {}", refs_label(&rule.backtest_result.evidence_refs))}</small>
                                            <small>{format!("FP history: {} / {} / {}", rule.false_positive_history.status, rule.false_positive_history.false_positive_count, percent_label(rule.false_positive_history.false_positive_rate))}</small>
                                            <small>{format!("FP evidence: {}", refs_label(&rule.false_positive_history.evidence_refs))}</small>
                                        </details>
                                    </>
                                } else {
                                    <p class="empty">{"Selected rule details were not returned in the library list."}</p>
                                }
                                <h4>{"Rule Promotion Gates"}</h4>
                                <details class="data-source-detail governance-detail">
                                    <summary>{format!("Rule promotion gate detail: {} gates", snapshot.gates.gates.len())}</summary>
                                    <div class="governance-check-list">
                                        {for snapshot.gates.gates.iter().map(|gate| html! {
                                            <div>
                                                <strong>{&gate.label}</strong>
                                                <span class={classes!("status-token", if gate.passed { "success" } else { "danger" })}>{if gate.passed { "passed" } else { "blocked" }}</span>
                                                <small>{&gate.evidence_source}</small>
                                                <small>{&gate.blocker}</small>
                                            </div>
                                        })}
                                    </div>
                                </details>
                                if snapshot.gates.blockers.is_empty() {
                                    <p class="empty">{"No rule promotion blockers."}</p>
                                } else {
                                    <ul class="result-list">
                                        {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                    </ul>
                                }
                            </section>
                        </>
                    }
                },
            }}
        </>
    }
}

#[function_component(ModelsPage)]
fn models_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let model_key = use_state(|| "baseline_fwa".to_string());
    let snapshot_state = use_state(|| ApiState::<ModelOpsSnapshot>::Idle);

    let load_models = {
        let api_key = api_key.clone();
        let model_key = model_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let model_key = (*model_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_model_ops_snapshot(api_key, model_key, None).await {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let refresh = {
        let load_models = load_models.clone();
        Callback::from(move |_| load_models.emit(()))
    };

    {
        let load_models = load_models.clone();
        use_effect_with((), move |_| {
            load_models.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Models"}</h2>
                    <p>{"Monitor model versions, scoring drift, promotion gates, QA feedback closure, and retraining readiness."}</p>
                </div>
                <span class="status-pill">{"Model Governance"}</span>
            </div>

            <section class="panel">
                <h3>{"Model Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"Model key"}
                        <input
                            value={(*model_key).clone()}
                            oninput={{
                                let model_key = model_key.clone();
                                Callback::from(move |event: InputEvent| {
                                    model_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh model governance" }}
                    </button>
                </div>
            </section>

            <ModelOpsView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ModelOpsProps {
    state: ApiState<ModelOpsSnapshot>,
}

#[function_component(ModelOpsView)]
fn model_ops_view(props: &ModelOpsProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load model governance to inspect production readiness."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading model governance..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {model_monitoring_cockpit(snapshot)}
                        <section class="panel result-stack">
                            <h3>{"Model Inventory"}</h3>
                            <div class="factor-card-grid">
                                {for snapshot.models.iter().map(|model| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{format!("{} {}", model.model_key, model.version)}</strong>
                                            <span>{format!("{} / {} / {}", model.model_type, model.runtime_kind, model.execution_provider)}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Status"}</span><strong>{&model.status}</strong></div>
                                            <div><span>{"Review Mode"}</span><strong>{&model.review_mode}</strong></div>
                                            <div><span>{"Endpoint"}</span><strong>{model.endpoint_url.as_deref().unwrap_or("none")}</strong></div>
                                        </div>
                                        <small>{format!("artifact: {}", model.artifact_uri.as_deref().unwrap_or("none"))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Model Performance"}</h3>
                            {model_telemetry_visual(&snapshot.performance, &snapshot.gates, &snapshot.retraining)}
                            <div class="score-hero">
                                <div><span>{"Model"}</span><strong>{&snapshot.performance.model_key}</strong></div>
                                <div><span>{"Drift"}</span><strong>{&snapshot.performance.drift_status}</strong></div>
                                <div><span>{"Score PSI"}</span><strong>{optional_number(snapshot.performance.score_psi)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Scored Runs"}</span><strong>{snapshot.performance.scored_runs}</strong></div>
                                <div><span>{"Avg Score"}</span><strong>{format!("{:.1}", snapshot.performance.average_score)}</strong></div>
                                <div><span>{"High Risk"}</span><strong>{snapshot.performance.high_risk_count}</strong></div>
                            </div>
                            <small>{format!("data: {} / latest scored: {}", snapshot.performance.data_status, snapshot.performance.latest_scored_at.as_deref().unwrap_or("none"))}</small>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Promotion Gates"}</h3>
                            <div class="score-hero">
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                                <div><span>{"Evaluation"}</span><strong>{&snapshot.gates.latest_evaluation_id}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Data Quality"}</span><strong>{&snapshot.gates.source_data_quality_status}</strong></div>
                                <div><span>{"Labels"}</span><strong>{snapshot.gates.approved_label_count}</strong></div>
                                <div><span>{"Open Feedback"}</span><strong>{snapshot.gates.unresolved_model_feedback_count}</strong></div>
                            </div>
                            if snapshot.gates.blockers.is_empty() {
                                <p class="empty">{"No promotion blockers."}</p>
                            } else {
                                <ul class="result-list compact-list">
                                    {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                            <div class="factor-card-grid">
                                {for snapshot.gates.gates.iter().map(|gate| html! {
                                    <div class="metric-row">
                                        <span>{&gate.label}</span>
                                        <strong>{if gate.passed { "passed" } else { "blocked" }}</strong>
                                        <small>{&gate.evidence_source}</small>
                                        <small>{&gate.blocker}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Retraining Readiness"}</h3>
                            <div class="score-hero">
                                <div><span>{"Recommendation"}</span><strong>{&snapshot.retraining.recommendation}</strong></div>
                                <div><span>{"Drift"}</span><strong>{&snapshot.retraining.drift_status}</strong></div>
                                <div><span>{"Data Quality"}</span><strong>{&snapshot.retraining.source_data_quality_status}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Open Feedback"}</span><strong>{snapshot.retraining.open_model_feedback_count}</strong></div>
                                <div><span>{"Approved Labels"}</span><strong>{snapshot.retraining.approved_label_count}</strong></div>
                                <div><span>{"Needs Review"}</span><strong>{snapshot.retraining.needs_review_label_count}</strong></div>
                            </div>
                            <h4>{"Triggers"}</h4>
                            if snapshot.retraining.retraining_triggers.is_empty() {
                                <p class="empty">{"No retraining triggers."}</p>
                            } else {
                                <ul class="result-list">
                                    {for snapshot.retraining.retraining_triggers.iter().map(|trigger| html! { <li>{trigger}</li> })}
                                </ul>
                            }
                            <h4>{"Blockers"}</h4>
                            if snapshot.retraining.blockers.is_empty() {
                                <p class="empty">{"No retraining blockers."}</p>
                            } else {
                                <ul class="result-list">
                                    {for snapshot.retraining.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[function_component(MlopsWorkspacePage)]
fn mlops_workspace_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let model_key = use_state(|| "baseline_fwa".to_string());
    let actor = use_state(|| "mlops-operator".to_string());
    let reviewer = use_state(|| "risk-model-owner".to_string());
    let promotion_decision = use_state(|| "approved".to_string());
    let monitoring_task_id = use_state(String::new);
    let monitoring_decision = use_state(|| "acknowledged".to_string());
    let alert_task_id = use_state(String::new);
    let alert_decision = use_state(|| "receipt_confirmed".to_string());
    let retraining_job_id = use_state(String::new);
    let retraining_status = use_state(|| "validation".to_string());
    let candidate_model_version = use_state(|| "0.2.0-candidate".to_string());
    let candidate_artifact_uri = use_state(|| {
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json".to_string()
    });
    let candidate_artifact_sha256 = use_state(|| "sha256:rust-serving-artifact".to_string());
    let training_artifact_uri =
        use_state(|| "s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib".to_string());
    let training_artifact_sha256 = use_state(|| "sha256:training-artifact".to_string());
    let serving_manifest_uri = use_state(|| {
        "s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json".to_string()
    });
    let candidate_endpoint_url =
        use_state(|| "http://127.0.0.1:8001/score/baseline_fwa/0.2.0-candidate".to_string());
    let validation_report_uri =
        use_state(|| "s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json".to_string());
    let candidate_auc = use_state(|| "0.92".to_string());
    let candidate_ks = use_state(|| "0.51".to_string());
    let candidate_precision = use_state(|| "0.78".to_string());
    let candidate_recall = use_state(|| "0.64".to_string());
    let candidate_f1 = use_state(|| "0.70".to_string());
    let candidate_accuracy = use_state(|| "0.89".to_string());
    let candidate_threshold = use_state(|| "0.70".to_string());
    let candidate_confusion_matrix =
        use_state(|| r#"{"tp": 64, "fp": 18, "tn": 820, "fn": 36}"#.to_string());
    let candidate_feature_importance_uri = use_state(|| {
        "data/eval/provider_retraining_candidate/feature_importance.parquet".to_string()
    });
    let candidate_permutation_importance_uri = use_state(|| {
        "data/eval/provider_retraining_candidate/permutation_importance.parquet".to_string()
    });
    let candidate_metrics_json = use_state(|| {
        r#"{"data_quality_status":"passed","split_strategy":"time_group_split","shadow_comparison_status":"passed","review_capacity_threshold_status":"passed"}"#.to_string()
    });
    let mined_rule_candidates_json = use_state(|| {
        r#"[{"rule_id":"candidate_retraining_amount_ratio","version":1,"name":"Retraining mined amount ratio candidate","review_mode":"both","scheme_family":"high_risk_claim","conditions":[{"field":"claim_amount_to_limit_ratio","operator":">=","value":0.82}],"action":{"score":22,"alert_code":"RETRAINING_AMOUNT_RATIO_CANDIDATE","recommended_action":"ManualReview","reason":"External training platform mined this explainable candidate from feature importance and backtest evidence."}}]"#.to_string()
    });
    let training_output_payload_json = use_state(|| {
        r#"{"candidate_model_version":"0.2.0-candidate","artifact_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/rust_serving_artifact.json","artifact_sha256":"sha256:rust-serving-artifact","training_artifact_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/model.joblib","training_artifact_sha256":"sha256:training-artifact","serving_manifest_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/serving_manifest.json","endpoint_url":"http://127.0.0.1:8001/score/baseline_fwa/0.2.0-candidate","validation_report_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/validation.json","feature_importance_uri":"data/eval/provider_retraining_candidate/feature_importance.parquet","permutation_importance_uri":"data/eval/provider_retraining_candidate/permutation_importance.parquet","metrics_json":{"shadow_comparison_status":"passed","review_capacity_threshold_status":"passed","model_artifact_evaluation_status":"passed","model_artifact_evaluation_report_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json","rule_candidate_backtest_status":"passed","rule_candidate_backtest_report_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json","rule_candidate_review_tasks_uri":"s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json","rule_library_writeback_status":"blocked_pending_human_review_and_policy_governance_approval"},"confusion_matrix_json":{"tp":64,"fp":18,"tn":820,"fn":36},"evidence_refs":["model_artifact_evaluations:s3://fwa-models/baseline_fwa/0.2.0-candidate/artifact-evaluation/model_artifact_evaluation_report.json","rule_candidate_backtests:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_report.json","rule_candidate_review_tasks:s3://fwa-models/baseline_fwa/0.2.0-candidate/rule-candidates/backtest/rule_candidate_backtest_review_tasks.json"],"mined_rule_candidates":[]}"#.to_string()
    });
    let anomaly_candidate_kind = use_state(|| "provider_peer_anomaly".to_string());
    let anomaly_candidate_id = use_state(|| "provider_peer:PRV-042:2026-05".to_string());
    let anomaly_source_report_uri = use_state(|| {
        "data/rust-automl-demo/unlabeled_provider_peer_clustering/clusters/provider_peer_clustering_report.json".to_string()
    });
    let anomaly_decision = use_state(|| "accepted_for_review".to_string());
    let anomaly_evidence_refs = use_state(|| {
        "anomaly_clustering_reports:data/rust-automl-demo/unlabeled_provider_peer_clustering/clusters/provider_peer_clustering_report.json, provider_peer_anomaly:PRV-042:2026-05".to_string()
    });
    let anomaly_candidate_payload = use_state(|| {
        r#"{"provider_id":"PRV-042","outlier_score":0.93,"reason":"peer z-score and high-cost rate exceed cohort threshold"}"#.to_string()
    });
    let action_notes = use_state(|| {
        "non-PII governed provider model release review for demo evidence".to_string()
    });
    let evidence_refs = use_state(|| "model_versions:baseline_fwa:v1".to_string());
    let snapshot_state = use_state(|| ApiState::<MlopsWorkspaceSnapshot>::Idle);
    let action_state = use_state(|| ApiState::<Value>::Idle);

    let load_workspace = {
        let api_key = api_key.clone();
        let model_key = model_key.clone();
        let candidate_model_version = candidate_model_version.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let model_key = (*model_key).clone();
            let candidate_model_version = (*candidate_model_version).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_mlops_workspace_snapshot(api_key, model_key, candidate_model_version)
                        .await
                    {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let refresh = {
        let load_workspace = load_workspace.clone();
        Callback::from(move |_| load_workspace.emit(()))
    };

    let governed_action = |action: &'static str| {
        let api_key = api_key.clone();
        let model_key = model_key.clone();
        let actor = actor.clone();
        let reviewer = reviewer.clone();
        let promotion_decision = promotion_decision.clone();
        let monitoring_task_id = monitoring_task_id.clone();
        let monitoring_decision = monitoring_decision.clone();
        let alert_task_id = alert_task_id.clone();
        let alert_decision = alert_decision.clone();
        let retraining_job_id = retraining_job_id.clone();
        let retraining_status = retraining_status.clone();
        let candidate_model_version = candidate_model_version.clone();
        let candidate_artifact_uri = candidate_artifact_uri.clone();
        let candidate_artifact_sha256 = candidate_artifact_sha256.clone();
        let training_artifact_uri = training_artifact_uri.clone();
        let training_artifact_sha256 = training_artifact_sha256.clone();
        let serving_manifest_uri = serving_manifest_uri.clone();
        let candidate_endpoint_url = candidate_endpoint_url.clone();
        let validation_report_uri = validation_report_uri.clone();
        let candidate_auc = candidate_auc.clone();
        let candidate_ks = candidate_ks.clone();
        let candidate_precision = candidate_precision.clone();
        let candidate_recall = candidate_recall.clone();
        let candidate_f1 = candidate_f1.clone();
        let candidate_accuracy = candidate_accuracy.clone();
        let candidate_threshold = candidate_threshold.clone();
        let candidate_confusion_matrix = candidate_confusion_matrix.clone();
        let candidate_feature_importance_uri = candidate_feature_importance_uri.clone();
        let candidate_permutation_importance_uri = candidate_permutation_importance_uri.clone();
        let candidate_metrics_json = candidate_metrics_json.clone();
        let mined_rule_candidates_json = mined_rule_candidates_json.clone();
        let action_notes = action_notes.clone();
        let evidence_refs = evidence_refs.clone();
        let action_state = action_state.clone();
        let load_workspace = load_workspace.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let model_key = (*model_key).clone();
            let actor = (*actor).clone();
            let reviewer = (*reviewer).clone();
            let promotion_decision = (*promotion_decision).clone();
            let monitoring_task_id = (*monitoring_task_id).clone();
            let monitoring_decision = (*monitoring_decision).clone();
            let alert_task_id = (*alert_task_id).clone();
            let alert_decision = (*alert_decision).clone();
            let retraining_job_id_state = retraining_job_id.clone();
            let retraining_job_id = (*retraining_job_id).clone();
            let retraining_status = (*retraining_status).clone();
            let candidate_model_version = (*candidate_model_version).clone();
            let candidate_artifact_uri = (*candidate_artifact_uri).clone();
            let candidate_artifact_sha256 = (*candidate_artifact_sha256).clone();
            let training_artifact_uri = (*training_artifact_uri).clone();
            let training_artifact_sha256 = (*training_artifact_sha256).clone();
            let serving_manifest_uri = (*serving_manifest_uri).clone();
            let candidate_endpoint_url = (*candidate_endpoint_url).clone();
            let validation_report_uri = (*validation_report_uri).clone();
            let candidate_auc = (*candidate_auc).clone();
            let candidate_ks = (*candidate_ks).clone();
            let candidate_precision = (*candidate_precision).clone();
            let candidate_recall = (*candidate_recall).clone();
            let candidate_f1 = (*candidate_f1).clone();
            let candidate_accuracy = (*candidate_accuracy).clone();
            let candidate_threshold = (*candidate_threshold).clone();
            let candidate_confusion_matrix = (*candidate_confusion_matrix).clone();
            let candidate_feature_importance_uri = (*candidate_feature_importance_uri).clone();
            let candidate_permutation_importance_uri =
                (*candidate_permutation_importance_uri).clone();
            let candidate_metrics_json = (*candidate_metrics_json).clone();
            let mined_rule_candidates_json = (*mined_rule_candidates_json).clone();
            let action_notes = (*action_notes).clone();
            let evidence_refs = parse_tags(&evidence_refs);
            let action_state = action_state.clone();
            let load_workspace = load_workspace.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                let result = execute_mlops_governed_action(
                    api_key,
                    model_key,
                    action,
                    actor,
                    reviewer,
                    promotion_decision,
                    monitoring_task_id,
                    monitoring_decision,
                    alert_task_id,
                    alert_decision,
                    retraining_job_id,
                    retraining_status,
                    candidate_model_version,
                    candidate_artifact_uri,
                    candidate_artifact_sha256,
                    training_artifact_uri,
                    training_artifact_sha256,
                    serving_manifest_uri,
                    candidate_endpoint_url,
                    validation_report_uri,
                    candidate_auc,
                    candidate_ks,
                    candidate_precision,
                    candidate_recall,
                    candidate_f1,
                    candidate_accuracy,
                    candidate_threshold,
                    candidate_confusion_matrix,
                    candidate_feature_importance_uri,
                    candidate_permutation_importance_uri,
                    candidate_metrics_json,
                    mined_rule_candidates_json,
                    action_notes,
                    evidence_refs,
                )
                .await;
                match result {
                    Ok(response) => {
                        if let Some(job_id) = response_retraining_job_id(&response) {
                            retraining_job_id_state.set(job_id);
                        }
                        action_state.set(ApiState::Ready(response));
                        load_workspace.emit(());
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let review_anomaly_candidate = {
        let api_key = api_key.clone();
        let reviewer = reviewer.clone();
        let action_notes = action_notes.clone();
        let anomaly_candidate_kind = anomaly_candidate_kind.clone();
        let anomaly_candidate_id = anomaly_candidate_id.clone();
        let anomaly_source_report_uri = anomaly_source_report_uri.clone();
        let anomaly_decision = anomaly_decision.clone();
        let anomaly_evidence_refs = anomaly_evidence_refs.clone();
        let anomaly_candidate_payload = anomaly_candidate_payload.clone();
        let action_state = action_state.clone();
        let load_workspace = load_workspace.clone();
        Callback::from(move |_| {
            let payload =
                match parse_json_object(&anomaly_candidate_payload, "anomaly candidate payload") {
                    Ok(payload) => payload,
                    Err(error) => {
                        action_state.set(ApiState::Failed(error));
                        return;
                    }
                };
            let api_key = (*api_key).clone();
            let reviewer = (*reviewer).clone();
            let notes = (*action_notes).clone();
            let candidate_kind = (*anomaly_candidate_kind).clone();
            let candidate_id = (*anomaly_candidate_id).clone();
            let source_report_uri = (*anomaly_source_report_uri).clone();
            let decision = (*anomaly_decision).clone();
            let evidence_refs = parse_tags(&anomaly_evidence_refs);
            let action_state = action_state.clone();
            let load_workspace = load_workspace.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                match submit_anomaly_candidate_review(
                    api_key,
                    candidate_kind,
                    candidate_id,
                    source_report_uri,
                    decision,
                    reviewer,
                    notes,
                    evidence_refs,
                    payload,
                )
                .await
                {
                    Ok(response) => {
                        action_state.set(ApiState::Ready(response));
                        load_workspace.emit(());
                    }
                    Err(error) => action_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let load_training_output_payload = {
        let training_output_payload_json = training_output_payload_json.clone();
        let candidate_model_version = candidate_model_version.clone();
        let candidate_artifact_uri = candidate_artifact_uri.clone();
        let candidate_artifact_sha256 = candidate_artifact_sha256.clone();
        let training_artifact_uri = training_artifact_uri.clone();
        let training_artifact_sha256 = training_artifact_sha256.clone();
        let serving_manifest_uri = serving_manifest_uri.clone();
        let candidate_endpoint_url = candidate_endpoint_url.clone();
        let validation_report_uri = validation_report_uri.clone();
        let candidate_auc = candidate_auc.clone();
        let candidate_ks = candidate_ks.clone();
        let candidate_precision = candidate_precision.clone();
        let candidate_recall = candidate_recall.clone();
        let candidate_f1 = candidate_f1.clone();
        let candidate_accuracy = candidate_accuracy.clone();
        let candidate_threshold = candidate_threshold.clone();
        let candidate_confusion_matrix = candidate_confusion_matrix.clone();
        let candidate_feature_importance_uri = candidate_feature_importance_uri.clone();
        let candidate_permutation_importance_uri = candidate_permutation_importance_uri.clone();
        let candidate_metrics_json = candidate_metrics_json.clone();
        let mined_rule_candidates_json = mined_rule_candidates_json.clone();
        let evidence_refs = evidence_refs.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let payload =
                match parse_json_object(&training_output_payload_json, "external training payload")
                {
                    Ok(payload) => payload,
                    Err(error) => {
                        action_state.set(ApiState::Failed(error));
                        return;
                    }
                };
            if let Some(value) = json_string_field(&payload, "candidate_model_version") {
                candidate_model_version.set(value);
            }
            if let Some(value) = json_string_field(&payload, "artifact_uri") {
                candidate_artifact_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "artifact_sha256") {
                candidate_artifact_sha256.set(value);
            }
            if let Some(value) = json_string_field(&payload, "training_artifact_uri") {
                training_artifact_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "training_artifact_sha256") {
                training_artifact_sha256.set(value);
            }
            if let Some(value) = json_string_field(&payload, "serving_manifest_uri") {
                serving_manifest_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "endpoint_url") {
                candidate_endpoint_url.set(value);
            }
            if let Some(value) = json_string_field(&payload, "validation_report_uri") {
                validation_report_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "feature_importance_uri") {
                candidate_feature_importance_uri.set(value);
            }
            if let Some(value) = json_string_field(&payload, "permutation_importance_uri") {
                candidate_permutation_importance_uri.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "auc") {
                candidate_auc.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "ks") {
                candidate_ks.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "precision") {
                candidate_precision.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "recall") {
                candidate_recall.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "f1") {
                candidate_f1.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "accuracy") {
                candidate_accuracy.set(value);
            }
            if let Some(value) = json_metric_string(&payload, "threshold") {
                candidate_threshold.set(value);
            }
            if let Some(confusion_matrix) = payload.get("confusion_matrix_json") {
                candidate_confusion_matrix.set(pretty_json(confusion_matrix));
            }
            if let Some(metrics) = payload.get("metrics_json") {
                candidate_metrics_json.set(pretty_json(metrics));
            }
            if let Some(candidates) = payload.get("mined_rule_candidates") {
                mined_rule_candidates_json.set(pretty_json(candidates));
            }
            if let Some(refs) = payload
                .get("evidence_refs")
                .and_then(|value| value.as_array())
            {
                let refs = refs
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect::<Vec<_>>();
                evidence_refs.set(refs.join(", "));
            }
            action_state.set(ApiState::Idle);
        })
    };

    let select_anomaly_review_task = {
        let anomaly_candidate_kind = anomaly_candidate_kind.clone();
        let anomaly_candidate_id = anomaly_candidate_id.clone();
        let anomaly_source_report_uri = anomaly_source_report_uri.clone();
        let anomaly_decision = anomaly_decision.clone();
        let anomaly_evidence_refs = anomaly_evidence_refs.clone();
        let anomaly_candidate_payload = anomaly_candidate_payload.clone();
        Callback::from(move |task: AnomalyReviewQueueTask| {
            anomaly_candidate_kind.set(task.candidate_kind);
            anomaly_candidate_id.set(task.candidate_id);
            anomaly_source_report_uri.set(task.source_report_uri);
            anomaly_decision.set("request_more_evidence".into());
            anomaly_evidence_refs.set(task.evidence_refs.join(", "));
            anomaly_candidate_payload.set(
                serde_json::to_string_pretty(&task.candidate_payload)
                    .unwrap_or_else(|_| "{}".into()),
            );
        })
    };

    let select_monitoring_review_task = {
        let monitoring_task_id = monitoring_task_id.clone();
        let monitoring_decision = monitoring_decision.clone();
        let evidence_refs = evidence_refs.clone();
        Callback::from(move |task: ModelMonitoringReviewTask| {
            let mut refs = vec![
                format!("model_versions:{}:{}", task.model_key, task.model_version),
                format!("model_monitoring_reports:{}", task.report_uri),
                format!("model_monitoring_review_tasks:{}", task.task_id),
            ];
            for evidence_ref in task.evidence_refs {
                refs = push_unique(refs, evidence_ref);
            }
            let decision = if task.retraining_recommendation == "prepare_retraining" {
                "prepare_retraining"
            } else if task.task_kind.contains("shadow") || task.trigger.contains("shadow") {
                "open_shadow_review"
            } else {
                "acknowledged"
            };
            monitoring_task_id.set(task.task_id);
            monitoring_decision.set(decision.into());
            evidence_refs.set(refs.join(", "));
        })
    };

    let select_retraining_job = {
        let retraining_job_id = retraining_job_id.clone();
        let retraining_status = retraining_status.clone();
        let candidate_model_version = candidate_model_version.clone();
        let candidate_artifact_uri = candidate_artifact_uri.clone();
        let candidate_endpoint_url = candidate_endpoint_url.clone();
        let validation_report_uri = validation_report_uri.clone();
        let evidence_refs = evidence_refs.clone();
        Callback::from(move |job: ModelRetrainingJobRecord| {
            retraining_job_id.set(job.job_id.clone());
            retraining_status.set(job.status.clone());
            let evidence_model_version = job
                .candidate_model_version
                .clone()
                .unwrap_or_else(|| job.model_version.clone());
            if let Some(version) = job.candidate_model_version {
                candidate_model_version.set(version);
            }
            if let Some(uri) = job.candidate_artifact_uri {
                candidate_artifact_uri.set(uri);
            }
            if let Some(url) = job.candidate_endpoint_url {
                candidate_endpoint_url.set(url);
            }
            if let Some(uri) = job.validation_report_uri {
                validation_report_uri.set(uri);
            }
            let mut refs = vec![
                format!("model_retraining_jobs:{}", job.job_id),
                format!(
                    "model_versions:{}:{}",
                    job.model_key, evidence_model_version
                ),
            ];
            if let Some(evaluation_id) = job.output_evaluation_id {
                refs = push_unique(refs, format!("model_evaluations:{evaluation_id}"));
            }
            evidence_refs.set(refs.join(", "));
        })
    };

    let (activation_allowed, activation_blockers) = match &*snapshot_state {
        ApiState::Ready(snapshot) => (
            snapshot.model_ops.gates.blockers.is_empty(),
            snapshot.model_ops.gates.blockers.clone(),
        ),
        ApiState::Idle | ApiState::Loading => {
            (false, vec!["load promotion gates before activation".into()])
        }
        ApiState::Failed(error) => (false, vec![error.clone()]),
    };

    {
        let load_workspace = load_workspace.clone();
        use_effect_with((), move |_| {
            load_workspace.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Provider Model Intake"}</h2>
                    <p>{"Review provider-delivered model candidates after offline training. Operators compare evidence and decide shadow, limited rollout, activation, rejection, or rollback."}</p>
                </div>
                <span class="status-pill">{"Provider candidate release"}</span>
            </div>

            <div class="mlops-cockpit">
                <section class="panel mlops-source-panel">
                    <div class="section-header compact">
                        <div>
                            <h3>{"Candidate Source"}</h3>
                            <p>{"Select the provider-trained model candidate to inspect."}</p>
                        </div>
                    </div>
                    <div class="form-grid">
                        <label>
                            {"Model key"}
                            <input
                                value={(*model_key).clone()}
                                oninput={{
                                    let model_key = model_key.clone();
                                    Callback::from(move |event: InputEvent| {
                                        model_key.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                    </div>
                    <div class="button-row">
                        <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                            {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh workspace" }}
                        </button>
                    </div>
                </section>

                <section class="panel result-stack mlops-action-panel">
                    <div class="section-header compact">
                        <div>
                            <h3>{"Governed Actions"}</h3>
                            <p>{"Lifecycle actions require reviewer context and evidence refs before backend gates accept them."}</p>
                        </div>
                        <span class="status-token strong">{"manual evidence required"}</span>
                    </div>
                    <div class="mlops-action-grid">
                        <label class="mlops-field">
                            {"Actor"}
                            <input
                                value={(*actor).clone()}
                                oninput={{
                                    let actor = actor.clone();
                                    Callback::from(move |event: InputEvent| {
                                        actor.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Reviewer"}
                            <input
                                value={(*reviewer).clone()}
                                oninput={{
                                    let reviewer = reviewer.clone();
                                    Callback::from(move |event: InputEvent| {
                                        reviewer.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Promotion decision"}
                            <select
                                value={(*promotion_decision).clone()}
                                onchange={{
                                    let promotion_decision = promotion_decision.clone();
                                    Callback::from(move |event: Event| {
                                        promotion_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="approved">{"approved"}</option>
                                <option value="rejected">{"rejected"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Monitoring task id"}
                            <input
                                value={(*monitoring_task_id).clone()}
                                oninput={{
                                    let monitoring_task_id = monitoring_task_id.clone();
                                    Callback::from(move |event: InputEvent| {
                                        monitoring_task_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Monitoring decision"}
                            <select
                                value={(*monitoring_decision).clone()}
                                onchange={{
                                    let monitoring_decision = monitoring_decision.clone();
                                    Callback::from(move |event: Event| {
                                        monitoring_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="acknowledged">{"acknowledged"}</option>
                                <option value="rejected">{"rejected"}</option>
                                <option value="prepare_retraining">{"prepare_retraining"}</option>
                                <option value="open_shadow_review">{"open_shadow_review"}</option>
                                <option value="open_rollback_review">{"open_rollback_review"}</option>
                                <option value="closed">{"closed"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Alert task id"}
                            <input
                                value={(*alert_task_id).clone()}
                                oninput={{
                                    let alert_task_id = alert_task_id.clone();
                                    Callback::from(move |event: InputEvent| {
                                        alert_task_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Alert decision"}
                            <select
                                value={(*alert_decision).clone()}
                                onchange={{
                                    let alert_decision = alert_decision.clone();
                                    Callback::from(move |event: Event| {
                                        alert_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="receipt_confirmed">{"receipt_confirmed"}</option>
                                <option value="delivery_failed">{"delivery_failed"}</option>
                                <option value="closed_no_action">{"closed_no_action"}</option>
                                <option value="escalated_for_governance_review">{"escalated_for_governance_review"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Training job id"}
                            <input
                                value={(*retraining_job_id).clone()}
                                oninput={{
                                    let retraining_job_id = retraining_job_id.clone();
                                    Callback::from(move |event: InputEvent| {
                                        retraining_job_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Training status"}
                            <select
                                value={(*retraining_status).clone()}
                                onchange={{
                                    let retraining_status = retraining_status.clone();
                                    Callback::from(move |event: Event| {
                                        retraining_status.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="running">{"running"}</option>
                                <option value="validation">{"validation"}</option>
                                <option value="failed">{"failed"}</option>
                                <option value="cancelled">{"cancelled"}</option>
                            </select>
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"External training payload"}
                            <textarea
                                value={(*training_output_payload_json).clone()}
                                oninput={{
                                    let training_output_payload_json = training_output_payload_json.clone();
                                    Callback::from(move |event: InputEvent| {
                                        training_output_payload_json.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <button class="mini-action" onclick={load_training_output_payload.clone()}>
                            {"Load provider output payload"}
                        </button>
                        <label class="mlops-field">
                            {"Candidate version"}
                            <input
                                value={(*candidate_model_version).clone()}
                                oninput={{
                                    let candidate_model_version = candidate_model_version.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_model_version.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate artifact"}
                            <input
                                value={(*candidate_artifact_uri).clone()}
                                oninput={{
                                    let candidate_artifact_uri = candidate_artifact_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_artifact_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate artifact SHA"}
                            <input
                                value={(*candidate_artifact_sha256).clone()}
                                oninput={{
                                    let candidate_artifact_sha256 = candidate_artifact_sha256.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_artifact_sha256.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Training artifact"}
                            <input
                                value={(*training_artifact_uri).clone()}
                                oninput={{
                                    let training_artifact_uri = training_artifact_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        training_artifact_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Training artifact SHA"}
                            <input
                                value={(*training_artifact_sha256).clone()}
                                oninput={{
                                    let training_artifact_sha256 = training_artifact_sha256.clone();
                                    Callback::from(move |event: InputEvent| {
                                        training_artifact_sha256.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Serving manifest"}
                            <input
                                value={(*serving_manifest_uri).clone()}
                                oninput={{
                                    let serving_manifest_uri = serving_manifest_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        serving_manifest_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate endpoint"}
                            <input
                                value={(*candidate_endpoint_url).clone()}
                                oninput={{
                                    let candidate_endpoint_url = candidate_endpoint_url.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_endpoint_url.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Validation report"}
                            <input
                                value={(*validation_report_uri).clone()}
                                oninput={{
                                    let validation_report_uri = validation_report_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        validation_report_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate AUC"}
                            <input
                                value={(*candidate_auc).clone()}
                                oninput={{
                                    let candidate_auc = candidate_auc.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_auc.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate KS"}
                            <input
                                value={(*candidate_ks).clone()}
                                oninput={{
                                    let candidate_ks = candidate_ks.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_ks.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate precision"}
                            <input
                                value={(*candidate_precision).clone()}
                                oninput={{
                                    let candidate_precision = candidate_precision.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_precision.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate recall"}
                            <input
                                value={(*candidate_recall).clone()}
                                oninput={{
                                    let candidate_recall = candidate_recall.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_recall.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate F1"}
                            <input
                                value={(*candidate_f1).clone()}
                                oninput={{
                                    let candidate_f1 = candidate_f1.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_f1.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate accuracy"}
                            <input
                                value={(*candidate_accuracy).clone()}
                                oninput={{
                                    let candidate_accuracy = candidate_accuracy.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_accuracy.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Candidate threshold"}
                            <input
                                value={(*candidate_threshold).clone()}
                                oninput={{
                                    let candidate_threshold = candidate_threshold.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_threshold.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Feature importance URI"}
                            <input
                                value={(*candidate_feature_importance_uri).clone()}
                                oninput={{
                                    let candidate_feature_importance_uri = candidate_feature_importance_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_feature_importance_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Permutation importance URI"}
                            <input
                                value={(*candidate_permutation_importance_uri).clone()}
                                oninput={{
                                    let candidate_permutation_importance_uri = candidate_permutation_importance_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_permutation_importance_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Confusion matrix JSON"}
                            <textarea
                                value={(*candidate_confusion_matrix).clone()}
                                oninput={{
                                    let candidate_confusion_matrix = candidate_confusion_matrix.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_confusion_matrix.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Metrics JSON"}
                            <textarea
                                value={(*candidate_metrics_json).clone()}
                                oninput={{
                                    let candidate_metrics_json = candidate_metrics_json.clone();
                                    Callback::from(move |event: InputEvent| {
                                        candidate_metrics_json.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Draft rule candidate payload"}
                            <textarea
                                value={(*mined_rule_candidates_json).clone()}
                                oninput={{
                                    let mined_rule_candidates_json = mined_rule_candidates_json.clone();
                                    Callback::from(move |event: InputEvent| {
                                        mined_rule_candidates_json.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Anomaly candidate kind"}
                            <select
                                value={(*anomaly_candidate_kind).clone()}
                                onchange={{
                                    let anomaly_candidate_kind = anomaly_candidate_kind.clone();
                                    Callback::from(move |event: Event| {
                                        anomaly_candidate_kind.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="provider_peer_anomaly">{"provider_peer_anomaly"}</option>
                                <option value="provider_graph_anomaly">{"provider_graph_anomaly"}</option>
                                <option value="claim_entity_anomaly">{"claim_entity_anomaly"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Anomaly candidate id"}
                            <input
                                value={(*anomaly_candidate_id).clone()}
                                oninput={{
                                    let anomaly_candidate_id = anomaly_candidate_id.clone();
                                    Callback::from(move |event: InputEvent| {
                                        anomaly_candidate_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Anomaly report URI"}
                            <input
                                value={(*anomaly_source_report_uri).clone()}
                                oninput={{
                                    let anomaly_source_report_uri = anomaly_source_report_uri.clone();
                                    Callback::from(move |event: InputEvent| {
                                        anomaly_source_report_uri.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field">
                            {"Anomaly decision"}
                            <select
                                value={(*anomaly_decision).clone()}
                                onchange={{
                                    let anomaly_decision = anomaly_decision.clone();
                                    Callback::from(move |event: Event| {
                                        anomaly_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                    })
                                }}
                            >
                                <option value="accepted_for_review">{"accepted_for_review"}</option>
                                <option value="rejected">{"rejected"}</option>
                                <option value="open_investigation_review">{"open_investigation_review"}</option>
                                <option value="request_more_evidence">{"request_more_evidence"}</option>
                            </select>
                        </label>
                        <label class="mlops-field">
                            {"Anomaly evidence refs"}
                            <input
                                value={(*anomaly_evidence_refs).clone()}
                                oninput={{
                                    let anomaly_evidence_refs = anomaly_evidence_refs.clone();
                                    Callback::from(move |event: InputEvent| {
                                        anomaly_evidence_refs.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Anomaly candidate payload"}
                            <textarea
                                value={(*anomaly_candidate_payload).clone()}
                                oninput={{
                                    let anomaly_candidate_payload = anomaly_candidate_payload.clone();
                                    Callback::from(move |event: InputEvent| {
                                        anomaly_candidate_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-evidence-field">
                            {"Evidence refs"}
                            <input
                                value={(*evidence_refs).clone()}
                                oninput={{
                                    let evidence_refs = evidence_refs.clone();
                                    Callback::from(move |event: InputEvent| {
                                        evidence_refs.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <label class="mlops-field mlops-notes-field">
                            {"Notes"}
                            <textarea
                                value={(*action_notes).clone()}
                                oninput={{
                                    let action_notes = action_notes.clone();
                                    Callback::from(move |event: InputEvent| {
                                        action_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                        <div class="mlops-boundary-card">
                            <span>{"Boundary"}</span>
                            <strong>{"Evidence before action"}</strong>
                            <small>{"Provider rule candidates are saved as drafts only. Review them one by one in Discovery Review with backtest and shadow evidence before accepting or rejecting."}</small>
                        </div>
                    </div>
                    <div class="button-row mlops-action-buttons">
                        <button onclick={governed_action("queue_retraining")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Request provider retraining"}</button>
                        <button onclick={governed_action("monitoring_review")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit monitoring decision"}</button>
                        <button onclick={governed_action("monitoring_reject")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Reject task"}</button>
                        <button onclick={governed_action("monitoring_prepare")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Prepare retraining from task"}</button>
                        <button onclick={governed_action("monitoring_rollback")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Open rollback review"}</button>
                        <button onclick={governed_action("alert_review")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit alert decision"}</button>
                        <button onclick={governed_action("alert_escalate")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Escalate alert to review"}</button>
                        <button onclick={governed_action("register_retraining_output")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Register completed provider output"}</button>
                        <button onclick={review_anomaly_candidate} disabled={matches!(&*action_state, ApiState::Loading)}>{"Review anomaly candidate"}</button>
                        <button onclick={governed_action("promotion_review")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit release review"}</button>
                        <button onclick={governed_action("activate")} disabled={!activation_allowed || matches!(&*action_state, ApiState::Loading)}>{"Activate approved candidate"}</button>
                        <button onclick={governed_action("rollback")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Rollback active model"}</button>
                    </div>
                    if !activation_allowed {
                        <div class="compact-list">
                            <span>{"Activation blocked by promotion gates"}</span>
                            {for activation_blockers.iter().map(|blocker| html! { <span>{blocker}</span> })}
                        </div>
                    }
                    <MlopsActionView state={(*action_state).clone()} />
                </section>
            </div>

            <MlopsWorkspaceView
                state={(*snapshot_state).clone()}
                on_select_monitoring_task={select_monitoring_review_task}
                on_select_anomaly={select_anomaly_review_task}
                on_select_retraining_job={select_retraining_job}
            />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct MlopsWorkspaceProps {
    state: ApiState<MlopsWorkspaceSnapshot>,
    on_select_monitoring_task: Callback<ModelMonitoringReviewTask>,
    on_select_anomaly: Callback<AnomalyReviewQueueTask>,
    on_select_retraining_job: Callback<ModelRetrainingJobRecord>,
}

#[function_component(MlopsWorkspaceView)]
fn mlops_workspace_view(props: &MlopsWorkspaceProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load provider model intake to inspect candidate release evidence."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading provider model intake..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <div class="mlops-workspace-grid">
                        {provider_model_release_summary(snapshot)}
                        {mlops_model_candidates(snapshot)}
                        {mlops_promotion_gates(snapshot)}
                        {mlops_monitoring_summary(snapshot)}
                        {mlops_monitoring_review_queue(snapshot, &props.on_select_monitoring_task)}
                        {mlops_anomaly_review_queue(snapshot, &props.on_select_anomaly)}
                        {mlops_alert_delivery_queue(snapshot)}
                        {mlops_training_handoff(snapshot)}
                        {mlops_dataset_readiness(snapshot)}
                        {mlops_training_jobs(snapshot, &props.on_select_retraining_job)}
                    </div>
                },
            }}
        </>
    }
}

fn provider_model_release_summary(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    let active_model = active_model_version(&snapshot.model_ops);
    let release_decision = provider_release_decision_label(snapshot);
    html! {
        <section class="panel data-command-center">
            <div class="section-header">
                <div>
                    <h3>{"Provider Candidate Release Control"}</h3>
                    <p>{"This is the business release desk for provider-trained model candidates. Operators do not train or tune models here; they decide shadow, limited rollout, activation, rejection, or rollback from evidence."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.model_ops.gates.decision))}>{&snapshot.model_ops.gates.decision}</span>
            </div>
            <div class="ops-stat-strip">
                <div><span>{"Candidate"}</span><strong>{&snapshot.model_ops.gates.model_version}</strong><small>{&snapshot.model_ops.performance.model_key}</small></div>
                <div><span>{"Evidence"}</span><strong>{format!("{} / {}", snapshot.model_ops.gates.passed_count, snapshot.model_ops.gates.total_count)}</strong><small>{"promotion gates"}</small></div>
                <div><span>{"Current Active"}</span><strong>{active_model.map(|model| model.version.as_str()).unwrap_or("none")}</strong><small>{"serving lock target"}</small></div>
                <div><span>{"Shadow Signal"}</span><strong>{&snapshot.model_ops.performance.drift_status}</strong><small>{optional_number(snapshot.model_ops.performance.score_psi)}</small></div>
                <div><span>{"Next Decision"}</span><strong>{release_decision}</strong><small>{"human gate"}</small></div>
            </div>
        </section>
    }
}

fn mlops_training_handoff(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    let dataset = latest_dataset(&snapshot.data_sources.datasets);
    let active_model = active_model_version(&snapshot.model_ops);
    html! {
        <section class="panel result-stack mlops-handoff-panel">
            <div class="section-header">
                <div>
                    <h3>{"Offline Training Handoff"}</h3>
                    <p>{"The UI exposes the contract that an external training platform must consume and return. Training remains offline; promotion remains human-governed."}</p>
                </div>
                <span class="status-token strong">{"human review required"}</span>
            </div>
            <details class="data-source-detail governance-detail release-evidence-detail">
                <summary>{"Provider training handoff detail"}</summary>
                <div class="summary-grid">
                    <div><span>{"Dataset manifest"}</span><strong>{dataset.map(|item| item.manifest_uri.as_str()).unwrap_or("missing")}</strong></div>
                    <div><span>{"Dataset version"}</span><strong>{dataset.map(dataset_version_label).unwrap_or_else(|| "missing".into())}</strong></div>
                    <div><span>{"Model key"}</span><strong>{&snapshot.model_ops.performance.model_key}</strong></div>
                    <div><span>{"Base version"}</span><strong>{active_model.map(|model| model.version.as_str()).unwrap_or("none")}</strong></div>
                    <div><span>{"Expected output"}</span><strong>{"/api/v1/ops/model-retraining-jobs/{job_id}/output"}</strong></div>
                    <div><span>{"Artifact boundary"}</span><strong>{active_model.and_then(|model| model.artifact_uri.as_deref()).unwrap_or("candidate artifact pending")}</strong></div>
                </div>
                <div class="factor-card-grid mlops-stage-grid">
                    {mlops_handoff_step("1", "Dataset approval", "Use a governed Parquet manifest with time and group split evidence.")}
                    {mlops_handoff_step("2", "Provider training", "External platform writes model, validation, feature, shadow, drift, and fairness artifacts.")}
                    {mlops_handoff_step("3", "Candidate registration", "Training output creates a candidate model and evaluation through the API.")}
                    {mlops_handoff_step("4", "Human release", "Promotion gates and reviewer decision decide shadow, activation, or rejection.")}
                </div>
            </details>
        </section>
    }
}

fn mlops_handoff_step(step: &str, label: &str, detail: &str) -> Html {
    html! {
        <div class="metric-row">
            <span>{format!("Step {step}")}</span>
            <strong>{label}</strong>
            <small>{detail}</small>
        </div>
    }
}

fn mlops_dataset_readiness(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-datasets-panel">
            <div class="section-header">
                <div>
                    <h3>{"Datasets"}</h3>
                    <p>{"Training data must show source scope, label policy, split quality, schema health, and production-evidence boundary before promotion."}</p>
                </div>
            </div>
            if snapshot.data_sources.datasets.is_empty() {
                <p class="empty">{"No datasets registered for provider model review."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail">
                    <summary>{format!("Release dataset evidence detail: {} datasets", snapshot.data_sources.datasets.len())}</summary>
                    <div class="factor-card-grid">
                        {for snapshot.data_sources.datasets.iter().take(6).map(|dataset| {
                            let health = health_for_dataset(&snapshot.data_sources.health, &dataset.dataset_id);
                            html! {
                                <div class="factor-card">
                                    <div>
                                        <strong>{dataset_version_label(dataset)}</strong>
                                        <span>{format!("{} / {} / {}", dataset.business_domain, dataset.sample_grain, dataset.storage_format)}</span>
                                    </div>
                                    <div class="summary-grid">
                                        <div><span>{"Rows"}</span><strong>{dataset.row_count}</strong></div>
                                        <div><span>{"Splits"}</span><strong>{dataset.splits.len()}</strong></div>
                                        <div><span>{"Fields"}</span><strong>{dataset.fields.len()}</strong></div>
                                        <div><span>{"Mappings"}</span><strong>{dataset.mappings.len()}</strong></div>
                                        <div><span>{"Label"}</span><strong>{empty_label(&dataset.label_column)}</strong></div>
                                        <div><span>{"Quality"}</span><strong>{health.map(|item| item.data_quality_status.as_str()).unwrap_or("missing")}</strong></div>
                                    </div>
                                    <small>{format!("manifest: {}", dataset.manifest_uri)}</small>
                                </div>
                            }
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

fn mlops_training_jobs(
    snapshot: &MlopsWorkspaceSnapshot,
    on_select_job: &Callback<ModelRetrainingJobRecord>,
) -> Html {
    html! {
        <section class="panel result-stack mlops-training-panel">
            <div class="section-header">
                <div>
                    <h3>{"Provider Output Handoff"}</h3>
                    <p>{"External training status is read here only as handoff evidence. FWA registers completed candidate outputs; it does not operate the training platform."}</p>
                </div>
                <span class="status-token neutral">{format!("{} jobs", snapshot.retraining_jobs.len())}</span>
            </div>
            if snapshot.retraining_jobs.is_empty() {
                <p class="empty">{"No retraining jobs returned for this model."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail">
                    <summary>{format!("Provider training job detail: {} jobs", snapshot.retraining_jobs.len())}</summary>
                    <div class="ops-table">
                        <div class="ops-table-head">
                            <span>{"Job"}</span>
                            <span>{"Status"}</span>
                            <span>{"Dataset"}</span>
                            <span>{"Candidate"}</span>
                            <span>{"Updated"}</span>
                        </div>
                        {for snapshot.retraining_jobs.iter().take(8).map(|job| html! {
                            <div class="ops-table-row">
                                <div class="primary-cell">
                                    <strong>{&job.job_id}</strong>
                                    <span>{format!("{} {} / requested by {}", job.model_key, job.model_version, job.requested_by)}</span>
                                </div>
                                <span class={classes!("status-token", status_tone(&job.status))}>{&job.status}</span>
                                <span>{format!("{} / {}", job.source_dataset_id, job.source_data_quality_status)}</span>
                                <span>{job.candidate_model_version.as_deref().unwrap_or("pending")}</span>
                                <span>{job.updated_at.as_deref().unwrap_or("missing")}</span>
                                <small class="row-detail">{format!("trigger {} / blocker {} / output {}", refs_label(&job.trigger_summary), refs_label(&job.blocker_summary), job.output_evaluation_id.as_deref().unwrap_or("none"))}</small>
                                <button
                                    class="mini-action"
                                    onclick={{
                                        let on_select_job = on_select_job.clone();
                                        let job = job.clone();
                                        Callback::from(move |_| on_select_job.emit(job.clone()))
                                    }}
                                >
                                    {"Use for output registration"}
                                </button>
                            </div>
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

fn mlops_model_candidates(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-candidates-panel">
            <div class="section-header">
                <div>
                    <h3>{"Model Candidates"}</h3>
                    <p>{"Active and candidate versions are inspected through runtime kind, artifact URI, evaluation lineage, and deployment status."}</p>
                </div>
            </div>
            if snapshot.model_ops.models.is_empty() {
                <p class="empty">{"No model versions returned."}</p>
            } else {
                <div class="factor-card-grid">
                    {for snapshot.model_ops.models.iter().map(|model| html! {
                        <div class="factor-card">
                            <div>
                                <strong>{format!("{} {}", model.model_key, model.version)}</strong>
                                <span>{format!("{} / {} / {}", model.status, model.runtime_kind, model.execution_provider)}</span>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Type"}</span><strong>{&model.model_type}</strong></div>
                                <div><span>{"Review Mode"}</span><strong>{&model.review_mode}</strong></div>
                                <div><span>{"Endpoint"}</span><strong>{model.endpoint_url.as_deref().unwrap_or("none")}</strong></div>
                            </div>
                            <small>{format!("artifact: {}", model.artifact_uri.as_deref().unwrap_or("none"))}</small>
                        </div>
                    })}
                </div>
            }
        </section>
    }
}

fn mlops_promotion_gates(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-promotion-panel">
            <div class="section-header">
                <div>
                    <h3>{"Promotion Gates"}</h3>
                    <p>{"Promotion gates keep data quality, label provenance, shadow evidence, drift, fairness, and approval requirements visible before activation."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.model_ops.gates.decision))}>{&snapshot.model_ops.gates.decision}</span>
            </div>
            <div class="score-hero">
                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.model_ops.gates.passed_count, snapshot.model_ops.gates.total_count)}</strong></div>
                <div><span>{"Evaluation"}</span><strong>{&snapshot.model_ops.gates.latest_evaluation_id}</strong></div>
                <div><span>{"Approved Labels"}</span><strong>{snapshot.model_ops.gates.approved_label_count}</strong></div>
            </div>
            <div class="summary-grid">
                <div><span>{"Serving Manifest"}</span><strong>{snapshot.model_ops.gates.artifact_evidence.serving_manifest_uri.as_deref().unwrap_or("missing")}</strong></div>
                <div><span>{"Artifact Report"}</span><strong>{snapshot.model_ops.gates.artifact_evidence.model_artifact_evaluation_report_uri.as_deref().unwrap_or("missing")}</strong></div>
                <div><span>{"Rust Serving"}</span><strong>{snapshot.model_ops.gates.artifact_evidence.rust_serving_status.as_deref().unwrap_or("missing")}</strong></div>
                <div><span>{"P95 Latency"}</span><strong>{model_latency_label(&snapshot.model_ops.gates.artifact_evidence)}</strong></div>
            </div>
            if snapshot.model_ops.gates.blockers.is_empty() {
                <p class="empty">{"No promotion blockers returned."}</p>
            } else {
                <ul class="result-list compact-list">
                    {for snapshot.model_ops.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                </ul>
            }
            <details class="data-source-detail governance-detail release-evidence-detail">
                <summary>{format!("Promotion gate detail: {} gates", snapshot.model_ops.gates.gates.len())}</summary>
                <div class="governance-check-list">
                    {for snapshot.model_ops.gates.gates.iter().map(|gate| html! {
                        <div>
                            <strong>{&gate.label}</strong>
                            <span class={classes!("status-token", if gate.passed { "success" } else { "danger" })}>{if gate.passed { "passed" } else { "blocked" }}</span>
                            <small>{&gate.evidence_source}</small>
                            <small>{&gate.blocker}</small>
                        </div>
                    })}
                </div>
            </details>
        </section>
    }
}

fn model_latency_label(evidence: &ModelArtifactEvidence) -> String {
    match (
        evidence.rust_serving_latency_status.as_deref(),
        evidence.rust_serving_p95_latency_ms,
    ) {
        (Some(status), Some(ms)) => format!("{status} / {ms}ms"),
        (Some(status), None) => status.to_string(),
        (None, Some(ms)) => format!("{ms}ms"),
        (None, None) => "missing".into(),
    }
}

fn mlops_monitoring_summary(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-monitoring-panel">
            <div class="section-header">
                <div>
                    <h3>{"Monitoring"}</h3>
                    <p>{"Monitoring should trigger retraining readiness, shadow review, or rollback review. It must not automatically promote a model."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.model_ops.retraining.recommendation))}>{&snapshot.model_ops.retraining.recommendation}</span>
            </div>
            <div class="summary-grid">
                <div><span>{"Scored Runs"}</span><strong>{snapshot.model_ops.performance.scored_runs}</strong></div>
                <div><span>{"Average Score"}</span><strong>{format!("{:.1}", snapshot.model_ops.performance.average_score)}</strong></div>
                <div><span>{"High Risk"}</span><strong>{snapshot.model_ops.performance.high_risk_count}</strong></div>
                <div><span>{"Score PSI"}</span><strong>{optional_number(snapshot.model_ops.performance.score_psi)}</strong></div>
                <div><span>{"Drift"}</span><strong>{&snapshot.model_ops.performance.drift_status}</strong></div>
                <div><span>{"Open Feedback"}</span><strong>{snapshot.model_ops.retraining.open_model_feedback_count}</strong></div>
                <div><span>{"Needs Review"}</span><strong>{snapshot.model_ops.retraining.needs_review_label_count}</strong></div>
                <div><span>{"Data Quality"}</span><strong>{&snapshot.model_ops.retraining.source_data_quality_status}</strong></div>
            </div>
            <h4>{"Retraining Triggers"}</h4>
            if snapshot.model_ops.retraining.retraining_triggers.is_empty() {
                <p class="empty">{"No retraining triggers."}</p>
            } else {
                <ul class="result-list compact-list">
                    {for snapshot.model_ops.retraining.retraining_triggers.iter().map(|trigger| html! { <li>{trigger}</li> })}
                </ul>
            }
        </section>
    }
}

fn mlops_monitoring_review_queue(
    snapshot: &MlopsWorkspaceSnapshot,
    on_select_monitoring_task: &Callback<ModelMonitoringReviewTask>,
) -> Html {
    html! {
        <section class="panel result-stack mlops-monitoring-panel">
            <div class="section-header">
                <div>
                    <h3>{"Monitoring Review Queue"}</h3>
                    <p>{"Submitted monitoring reports open human review tasks for drift, shadow, serving, or fairness signals before retraining or rollback can proceed."}</p>
                </div>
                <span class="status-token neutral">{format!("{} tasks", snapshot.monitoring_review_tasks.len())}</span>
            </div>
            if snapshot.monitoring_review_tasks.is_empty() {
                <p class="empty">{"No monitoring review tasks returned for this model."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail" open=true>
                    <summary>{format!("Open monitoring review detail: {} tasks", snapshot.monitoring_review_tasks.len())}</summary>
                    <div class="ops-table">
                        <div class="ops-table-head">
                            <span>{"Task"}</span>
                            <span>{"Trigger"}</span>
                            <span>{"Status"}</span>
                            <span>{"Recommendation"}</span>
                            <span>{"Evidence"}</span>
                        </div>
                        {for snapshot.monitoring_review_tasks.iter().take(8).map(|task| html! {
                            <div class="ops-table-row">
                                <div class="primary-cell">
                                    <strong>{&task.task_kind}</strong>
                                    <span>{format!("{} {} / {}", task.model_key, task.model_version, task.audit_id)}</span>
                                </div>
                                <span>{empty_label(&task.trigger)}</span>
                                <span class={classes!("status-token", status_tone(&task.review_status))}>{&task.review_status}</span>
                                <span>{format!("{} / {}", task.monitoring_status, task.retraining_recommendation)}</span>
                                <span>{refs_label(&task.evidence_refs)}</span>
                                <small class="row-detail">{format!("required refs model_versions:{}:{}; model_monitoring_reports:{}; model_monitoring_review_tasks:{}", task.model_key, task.model_version, task.report_uri, task.task_id)}</small>
                                <button
                                    class="mini-action"
                                    onclick={{
                                        let on_select_monitoring_task = on_select_monitoring_task.clone();
                                        let task = task.clone();
                                        Callback::from(move |_| on_select_monitoring_task.emit(task.clone()))
                                    }}
                                >
                                    {"Use for monitoring review"}
                                </button>
                            </div>
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

fn mlops_anomaly_review_queue(
    snapshot: &MlopsWorkspaceSnapshot,
    on_select_anomaly: &Callback<AnomalyReviewQueueTask>,
) -> Html {
    html! {
        <section class="panel result-stack mlops-monitoring-panel">
            <div class="section-header">
                <div>
                    <h3>{"Anomaly Review Queue"}</h3>
                    <p>{"Unsupervised clustering can only open explainable anomaly candidates for human review. Tasks stay pending_human_review until a reviewer records accepted_for_review, rejected, open_investigation_review, or request_more_evidence; decisions here do not create cases, assign labels, activate models, or write rules."}</p>
                </div>
                <span class="status-token neutral">{format!("{} candidates", snapshot.anomaly_review_tasks.len())}</span>
            </div>
            if snapshot.anomaly_review_tasks.is_empty() {
                <p class="empty">{"No anomaly review tasks returned."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail" open=true>
                    <summary>{format!("Anomaly review detail: {} candidates", snapshot.anomaly_review_tasks.len())}</summary>
                    <div class="ops-table">
                        <div class="ops-table-head">
                            <span>{"Candidate"}</span>
                            <span>{"Kind"}</span>
                            <span>{"Status"}</span>
                            <span>{"Decision"}</span>
                            <span>{"Evidence"}</span>
                        </div>
                        {for snapshot.anomaly_review_tasks.iter().take(8).map(|task| html! {
                            <div class="ops-table-row">
                                <div class="primary-cell">
                                    <strong>{&task.candidate_id}</strong>
                                    <span>{format!("{} / {}", task.dataset_key, task.dataset_version)}</span>
                                </div>
                                <span>{format!("{} / {}", task.candidate_kind, task.report_kind)}</span>
                                <span class={classes!("status-token", status_tone(&task.review_status))}>{&task.review_status}</span>
                                <span>{task.decision.as_deref().unwrap_or("pending decision")}</span>
                                <span>{refs_label(&task.evidence_refs)}</span>
                                <small class="row-detail">{format!("queue {} / required {} / report {}", task.review_queue, task.required_review, task.source_report_uri)}</small>
                                <small class="row-detail">{format!("options: {}", refs_label(&task.decision_options))}</small>
                                <button
                                    class="mini-action"
                                    onclick={{
                                        let on_select_anomaly = on_select_anomaly.clone();
                                        let task = task.clone();
                                        Callback::from(move |_| on_select_anomaly.emit(task.clone()))
                                    }}
                                >
                                    {"Use for review"}
                                </button>
                            </div>
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

fn mlops_alert_delivery_queue(snapshot: &MlopsWorkspaceSnapshot) -> Html {
    html! {
        <section class="panel result-stack mlops-monitoring-panel">
            <div class="section-header">
                <div>
                    <h3>{"Alert Delivery Queue"}</h3>
                    <p>{"Alert delivery tasks track customer alert-router handoff and receipt confirmation before any governance escalation."}</p>
                </div>
                <span class="status-token neutral">{format!("{} alerts", snapshot.alert_delivery_tasks.len())}</span>
            </div>
            if snapshot.alert_delivery_tasks.is_empty() {
                <p class="empty">{"No alert delivery tasks returned for this model."}</p>
            } else {
                <details class="data-source-detail governance-detail release-evidence-detail" open=true>
                    <summary>{format!("Alert delivery detail: {} tasks", snapshot.alert_delivery_tasks.len())}</summary>
                    <div class="ops-table">
                        <div class="ops-table-head">
                            <span>{"Task"}</span>
                            <span>{"Route"}</span>
                            <span>{"Delivery"}</span>
                            <span>{"Receipt"}</span>
                            <span>{"Evidence"}</span>
                        </div>
                        {for snapshot.alert_delivery_tasks.iter().take(8).map(|task| html! {
                            <div class="ops-table-row">
                                <div class="primary-cell">
                                    <strong>{&task.task_kind}</strong>
                                    <span>{format!("{} {} / {}", task.model_key, task.model_version, task.audit_id)}</span>
                                </div>
                                <span>{format!("{} / {}", empty_label(&task.trigger), empty_label(&task.route_key))}</span>
                                <span class={classes!("status-token", status_tone(&task.delivery_status))}>{&task.delivery_status}</span>
                                <span class={classes!("status-token", status_tone(&task.review_status))}>{&task.review_status}</span>
                                <span>{refs_label(&task.evidence_refs)}</span>
                                <small class="row-detail">{format!("required refs model_versions:{}:{}; mlops_scheduler_execution_reports:{}; mlops_alert_delivery_tasks:{}", task.model_key, task.model_version, task.scheduler_execution_report_uri, task.task_id)}</small>
                            </div>
                        })}
                    </div>
                </details>
            }
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct MlopsActionProps {
    state: ApiState<Value>,
}

#[function_component(MlopsActionView)]
fn mlops_action_view(props: &MlopsActionProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Choose an action only after evidence and reviewer context are ready."}</p> }
        }
        ApiState::Loading => {
            html! { <p>{"Submitting governed provider model release action..."}</p> }
        }
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <>
                <p class="empty">{"Action accepted by API. Workspace refresh has been requested."}</p>
                <pre>{pretty_json(response)}</pre>
            </>
        },
    }
}

#[function_component(RoutingPoliciesPage)]
fn routing_policies_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let policy_id = use_state(|| "fwa_risk_fusion_routing".to_string());
    let review_mode = use_state(|| "pre_payment".to_string());
    let version = use_state(|| "1".to_string());
    let evidence_refs =
        use_state(|| "routing_policies:fwa_risk_fusion_routing:v1:pre_payment".to_string());
    let snapshot_state = use_state(|| ApiState::<RoutingPolicySnapshot>::Idle);
    let action_state = use_state(|| ApiState::<RoutingPolicyRecord>::Idle);

    let load_policies = {
        let api_key = api_key.clone();
        let policy_id = policy_id.clone();
        let review_mode = review_mode.clone();
        let version = version.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let policy_id = (*policy_id).clone();
            let review_mode = (*review_mode).clone();
            let version = (*version).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_routing_policy_snapshot(api_key, policy_id, review_mode, version)
                        .await
                    {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let refresh = {
        let load_policies = load_policies.clone();
        Callback::from(move |_| load_policies.emit(()))
    };

    let lifecycle_action = |action: &'static str| {
        let api_key = api_key.clone();
        let policy_id = policy_id.clone();
        let review_mode = review_mode.clone();
        let version = version.clone();
        let evidence_refs = evidence_refs.clone();
        let action_state = action_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let policy_id = (*policy_id).clone();
            let review_mode = (*review_mode).clone();
            let version = (*version).clone();
            let evidence_refs = parse_tags(&evidence_refs);
            let action_state = action_state.clone();
            action_state.set(ApiState::Loading);
            spawn_local(async move {
                action_state.set(
                    match update_routing_policy_lifecycle(
                        api_key,
                        policy_id,
                        review_mode,
                        version,
                        action,
                        evidence_refs,
                    )
                    .await
                    {
                        Ok(record) => ApiState::Ready(record),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    {
        let load_policies = load_policies.clone();
        use_effect_with((), move |_| {
            load_policies.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Routing Policies"}</h2>
                    <p>{"Govern risk bands, confidence gates, provider review thresholds, approvals, activation, and rollback evidence before routing affects claim handling."}</p>
                </div>
                <span class="status-pill">{"Risk Fusion Routing"}</span>
            </div>

            <section class="panel">
                <h3>{"Routing Policy Control"}</h3>
                <div class="form-grid">
                    {text_input("Policy ID", &policy_id)}
                    {text_input("Review mode", &review_mode)}
                    {text_input("Version", &version)}
                    {text_input("Evidence refs", &evidence_refs)}
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh routing policies" }}
                    </button>
                    <button onclick={lifecycle_action("submit")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Submit"}</button>
                    <button onclick={lifecycle_action("approve")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Approve"}</button>
                    <button onclick={lifecycle_action("activate")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Activate"}</button>
                    <button onclick={lifecycle_action("rollback")} disabled={matches!(&*action_state, ApiState::Loading)}>{"Rollback"}</button>
                </div>
                <RoutingPolicyActionView state={(*action_state).clone()} />
            </section>

            <RoutingPoliciesView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RoutingPoliciesProps {
    state: ApiState<RoutingPolicySnapshot>,
}

#[function_component(RoutingPoliciesView)]
fn routing_policies_view(props: &RoutingPoliciesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load routing policies to inspect routing governance."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading routing policies..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {routing_policy_cockpit(snapshot)}
                        <section class="panel result-stack">
                            <h3>{"Routing Policy Inventory"}</h3>
                            <div class="score-hero">
                                <div><span>{"Policies"}</span><strong>{snapshot.policies.len()}</strong></div>
                                <div><span>{"Active"}</span><strong>{snapshot.policies.iter().filter(|policy| policy.status == "active").count()}</strong></div>
                                <div><span>{"Review Modes"}</span><strong>{routing_review_modes(&snapshot.policies)}</strong></div>
                            </div>
                            <div class="factor-card-grid">
                                {for snapshot.policies.iter().map(|policy| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{format!("{} v{} / {}", policy.policy_id, policy.version, policy.review_mode)}</strong>
                                            <span>{format!("{} / owner {}", policy.status, policy.owner)}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Low / Medium"}</span><strong>{format!("{} / {}", policy.risk_thresholds.low_max, policy.risk_thresholds.medium_min)}</strong></div>
                                            <div><span>{"High / Critical"}</span><strong>{format!("{} / {}", policy.risk_thresholds.high_min, policy.risk_thresholds.critical_min)}</strong></div>
                                            <div><span>{"Confidence"}</span><strong>{format!("{} / {}", policy.confidence_thresholds.low_confidence_below, policy.confidence_thresholds.high_confidence_min)}</strong></div>
                                            <div><span>{"Provider Review"}</span><strong>{policy.provider_review_threshold}</strong></div>
                                        </div>
                                        <small>{format!("activated: {} / created: {}", policy.activated_at.as_deref().unwrap_or("none"), policy.created_at.as_deref().unwrap_or("none"))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Routing Promotion Gates"}</h3>
                            <div class="score-hero">
                                <div><span>{"Policy"}</span><strong>{format!("{} v{}", snapshot.gates.policy_id, snapshot.gates.version)}</strong></div>
                                <div><span>{"Decision"}</span><strong>{&snapshot.gates.decision}</strong></div>
                                <div><span>{"Passed"}</span><strong>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Review Mode"}</span><strong>{&snapshot.gates.review_mode}</strong></div>
                                <div><span>{"Status"}</span><strong>{&snapshot.gates.status}</strong></div>
                                <div><span>{"Blockers"}</span><strong>{snapshot.gates.blockers.len()}</strong></div>
                            </div>
                            if snapshot.gates.blockers.is_empty() {
                                <p class="empty">{"No routing policy blockers."}</p>
                            } else {
                                <ul class="result-list compact-list">
                                    {for snapshot.gates.blockers.iter().map(|blocker| html! { <li>{blocker}</li> })}
                                </ul>
                            }
                            <div class="factor-card-grid">
                                {for snapshot.gates.gates.iter().map(|gate| html! {
                                    <div class="metric-row">
                                        <span>{&gate.label}</span>
                                        <strong>{if gate.passed { "passed" } else { "blocked" }}</strong>
                                        <small>{&gate.evidence_source}</small>
                                        <small>{&gate.blocker}</small>
                                    </div>
                                })}
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

fn routing_policy_cockpit(snapshot: &RoutingPolicySnapshot) -> Html {
    let policy = snapshot
        .policies
        .iter()
        .find(|policy| policy.status == "active")
        .or_else(|| snapshot.policies.first());

    if let Some(policy) = policy {
        let blocker_label = snapshot
            .gates
            .blockers
            .first()
            .map(String::as_str)
            .unwrap_or("no blocker");
        html! {
            <section class="panel result-stack">
                <div class="section-header">
                    <div>
                        <h3>{"Routing Decision Map"}</h3>
                        <p>{"How fused risk score, confidence, provider graph pressure, and governance gates route claims without automatic adjudication."}</p>
                    </div>
                    <span class={classes!("status-token", status_tone(&policy.status))}>{&policy.status}</span>
                </div>
                <div class="routing-cockpit">
                    <aside class="routing-brief">
                        <span class="eyebrow">{"Active routing policy"}</span>
                        <strong>{format!("{} v{}", policy.policy_id, policy.version)}</strong>
                        <dl>
                            <div><dt>{"Review mode"}</dt><dd>{&policy.review_mode}</dd></div>
                            <div><dt>{"Owner"}</dt><dd>{&policy.owner}</dd></div>
                            <div><dt>{"Promotion"}</dt><dd>{format!("{} / {}", snapshot.gates.passed_count, snapshot.gates.total_count)}</dd></div>
                            <div><dt>{"Decision"}</dt><dd>{&snapshot.gates.decision}</dd></div>
                        </dl>
                    </aside>

                    <div class="routing-decision-map">
                        <div class="routing-map-title">
                            <span>{"Risk fusion and routing"}</span>
                            <strong>{"risk signals + confidence + policy gate -> human-safe route"}</strong>
                        </div>
                        <div class="routing-link horizontal"></div>
                        <div class="routing-link diagonal-a"></div>
                        <div class="routing-link diagonal-b"></div>
                        <div class="routing-core">
                            <span>{"Routing gate"}</span>
                            <strong>{&policy.review_mode}</strong>
                        </div>
                        {routing_node("Green band", &format!("0-{}", policy.risk_thresholds.low_max), "low")}
                        {routing_node("Amber band", &format!("{}-{}", policy.risk_thresholds.medium_min, policy.risk_thresholds.high_min.saturating_sub(1)), "medium")}
                        {routing_node("Red band", &format!("{}+", policy.risk_thresholds.high_min), "high")}
                        {routing_node("Critical route", &format!("{}+", policy.risk_thresholds.critical_min), "critical")}
                        {routing_node("Confidence gate", &format!("<{} low / {}+ high", policy.confidence_thresholds.low_confidence_below, policy.confidence_thresholds.high_confidence_min), "confidence")}
                        {routing_node("Provider review", &format!("{}+", policy.provider_review_threshold), "provider")}
                    </div>

                    <aside class="routing-trace">
                        <span class="eyebrow">{"Human-safe route"}</span>
                        <div class="provider-signal-stack">
                            {provider_signal_row("Low", "STP or sample QA", "neutral")}
                            {provider_signal_row("Medium", "QA sampling", "warning")}
                            {provider_signal_row("High", "Manual review", "danger")}
                            {provider_signal_row("Rollback gate", blocker_label, "strong")}
                        </div>
                    </aside>
                </div>
            </section>
        }
    } else {
        html! {
            <section class="panel">
                <p class="empty">{"No routing policy available for routing decision map."}</p>
            </section>
        }
    }
}

fn routing_node(label: &str, value: &str, position: &str) -> Html {
    html! {
        <div class={classes!("routing-node", position.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct RoutingPolicyActionProps {
    state: ApiState<RoutingPolicyRecord>,
}

#[function_component(RoutingPolicyActionView)]
fn routing_policy_action_view(props: &RoutingPolicyActionProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Lifecycle actions require evidence refs and enforce current policy status."}</p> }
        }
        ApiState::Loading => html! { <p>{"Updating routing policy..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Policy"}</span><strong>{format!("{} v{}", record.policy_id, record.version)}</strong></div>
                <div><span>{"Review Mode"}</span><strong>{&record.review_mode}</strong></div>
                <div><span>{"Status"}</span><strong>{&record.status}</strong></div>
                <div><span>{"Owner"}</span><strong>{&record.owner}</strong></div>
            </div>
        },
    }
}

#[function_component(FactorFactoryPage)]
fn factor_factory_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let readiness_state = use_state(|| ApiState::<FactorReadinessResponse>::Idle);

    let load_readiness = {
        let api_key = api_key.clone();
        let readiness_state = readiness_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let readiness_state = readiness_state.clone();
            readiness_state.set(ApiState::Loading);
            spawn_local(async move {
                readiness_state.set(match get_factor_readiness(api_key).await {
                    Ok(response) => ApiState::Ready(response),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_readiness = load_readiness.clone();
        Callback::from(move |_| load_readiness.emit(()))
    };

    {
        let load_readiness = load_readiness.clone();
        use_effect_with((), move |_| {
            load_readiness.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Factor Factory"}</h2>
                    <p>{"Review factor readiness by scheme family, online availability, rule convertibility, ownership, and evidence quality."}</p>
                </div>
                <span class="status-pill">{"Factor Readiness"}</span>
            </div>

            <section class="panel">
                <h3>{"Readiness Source"}</h3>
                <p class="empty">{"Using governed dataset and feature metadata from the configured pilot workspace."}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*readiness_state, ApiState::Loading)}>
                        {if matches!(&*readiness_state, ApiState::Loading) { "Refreshing..." } else { "Refresh readiness" }}
                    </button>
                </div>
            </section>

            <FactorReadinessView state={(*readiness_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct FactorReadinessProps {
    state: ApiState<FactorReadinessResponse>,
}

#[function_component(FactorReadinessView)]
fn factor_readiness_view(props: &FactorReadinessProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load readiness to inspect factor governance status."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading factor readiness..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(readiness) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Readiness Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Datasets"}</span><strong>{readiness.dataset_count}</strong></div>
                                <div><span>{"Factors"}</span><strong>{readiness.factor_count}</strong></div>
                                <div><span>{"Data Quality"}</span><strong>{format!("{} / {:.2}", readiness.data_quality_status, readiness.data_quality_score)}</strong></div>
                            </div>
                            <div class="summary-grid">
                                <div><span>{"Online Ready"}</span><strong>{readiness.online_ready_count}</strong></div>
                                <div><span>{"Rule Convertible"}</span><strong>{readiness.rule_convertible_count}</strong></div>
                                <div><span>{"Ready / Review"}</span><strong>{format!("{} / {}", readiness.ready_factor_count, readiness.review_factor_count)}</strong></div>
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Scheme Readiness"}</h3>
                            <div class="factor-card-grid">
                                {for readiness.scheme_readiness.iter().map(|scheme| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{&scheme.scheme_family}</strong>
                                            <span>{format!("ready {} of {} factors", scheme.ready_factor_count, scheme.factor_count)}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Online"}</span><strong>{scheme.online_ready_count}</strong></div>
                                            <div><span>{"Rule convertible"}</span><strong>{scheme.rule_convertible_count}</strong></div>
                                            <div><span>{"Review"}</span><strong>{scheme.review_factor_count}</strong></div>
                                        </div>
                                        <small>{format!("issues: {}", issue_counts_label(&scheme.readiness_issue_counts))}</small>
                                    </div>
                                })}
                            </div>
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Factor Cards"}</h3>
                            <div class="factor-card-grid">
                                {for readiness.factor_cards.iter().take(8).map(|card| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{&card.factor_name}</strong>
                                            <span>{format!("{} / {} / {}", card.chinese_name, card.entity_type, card.scheme_family)}</span>
                                        </div>
                                        <p>{&card.business_meaning}</p>
                                        <div class="summary-grid">
                                            <div><span>{"Status"}</span><strong>{&card.readiness_status}</strong></div>
                                            <div><span>{"Online"}</span><strong>{yes_no(card.online_available)}</strong></div>
                                            <div><span>{"Rule"}</span><strong>{yes_no(card.rule_convertible)}</strong></div>
                                        </div>
                                        <small>{format!("dataset: {} / owner: {}", card.dataset_key, card.owner)}</small>
                                    </div>
                                })}
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[function_component(DataSourcesPage)]
fn data_sources_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let snapshot_state = use_state(|| ApiState::<DataSourcesSnapshot>::Idle);

    let load_sources = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_data_sources_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_sources = load_sources.clone();
        Callback::from(move |_| load_sources.emit(()))
    };

    {
        let load_sources = load_sources.clone();
        use_effect_with((), move |_| {
            load_sources.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Data Sources"}</h2>
                    <p>{"Inspect parquet dataset catalog, data health, schema coverage, field mappings, and model evaluation lineage for governed feature and model operations."}</p>
                </div>
                <span class="status-pill">{"Data & Metric Foundation"}</span>
            </div>

            <section class="panel">
                <h3>{"Data Source Control"}</h3>
                <p class="empty">{"Using the configured data-governance workspace for catalog, schema, and evaluation lineage."}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh data sources" }}
                    </button>
                </div>
            </section>

            <DataSourcesView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct DataSourcesProps {
    state: ApiState<DataSourcesSnapshot>,
}

#[function_component(DataSourcesView)]
fn data_sources_view(props: &DataSourcesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load data sources to inspect catalog and lineage."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading data source catalog..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        <section class="panel data-command-center">
                            <div class="section-header">
                                <div>
                                    <h3>{"Data Foundation Control"}</h3>
                                    <p>{"External claim, policy, provider, and medical datasets must stay traceable before rules, models, and agents can rely on them."}</p>
                                </div>
                            </div>
                            <div class="ops-stat-strip">
                                <div><span>{"Datasets"}</span><strong>{snapshot.datasets.len()}</strong><small>{"registered sources"}</small></div>
                                <div><span>{"Rows"}</span><strong>{total_dataset_rows(&snapshot.datasets)}</strong><small>{"available records"}</small></div>
                                <div><span>{"Fields"}</span><strong>{total_schema_fields(&snapshot.datasets)}</strong><small>{"profiled columns"}</small></div>
                                <div><span>{"Mappings"}</span><strong>{total_field_mappings(&snapshot.datasets)}</strong><small>{"canonical links"}</small></div>
                                <div><span>{"Evaluations"}</span><strong>{snapshot.evaluations.len()}</strong><small>{"model runs"}</small></div>
                            </div>
                        </section>

                        {data_lineage_cockpit(snapshot)}

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Dataset Catalog"}</h3>
                                    <p>{"Which governed datasets can feed scoring, feature creation, and medical review workflows."}</p>
                                </div>
                            </div>
                            if snapshot.datasets.is_empty() {
                                <p class="empty">{"No datasets registered."}</p>
                            } else {
                                <div class="ops-table dataset-catalog-table">
                                    <div class="ops-table-head">
                                        <span>{"Dataset"}</span>
                                        <span>{"Domain"}</span>
                                        <span>{"Rows"}</span>
                                        <span>{"Grain"}</span>
                                        <span>{"Status"}</span>
                                    </div>
                                    {for snapshot.datasets.iter().take(8).map(|dataset| html! {
                                        <div class="ops-table-row">
                                            <div class="primary-cell">
                                                <strong>{&dataset.display_name}</strong>
                                                <span>{format!("{}:{} / {}", dataset.dataset_key, dataset.dataset_version, dataset.storage_format)}</span>
                                            </div>
                                            <span>{&dataset.business_domain}</span>
                                            <strong>{dataset.row_count}</strong>
                                            <span>{&dataset.sample_grain}</span>
                                            <span class={classes!("status-token", status_tone(&dataset.status))}>{&dataset.status}</span>
                                            <small class="row-detail">{format!("source {} / label {} / keys {} / manifest {}", dataset.source_key, empty_label(&dataset.label_column), refs_label(&dataset.entity_keys), dataset.manifest_uri)}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Dataset Health"}</h3>
                                    <p>{"Operational readiness signals used before a dataset is trusted for scoring or training."}</p>
                                </div>
                            </div>
                            if snapshot.health.is_empty() {
                                <p class="empty">{"No dataset health records returned."}</p>
                            } else {
                                <div class="health-grid">
                                    {for snapshot.health.iter().map(|health| html! {
                                        <div class="health-card">
                                            <div>
                                                <strong>{format!("{}:{}", health.dataset_key, health.dataset_version)}</strong>
                                                <span class={classes!("status-token", status_tone(&health.data_quality_status))}>{format!("{} / {:.2}", health.data_quality_status, health.data_quality_score)}</span>
                                            </div>
                                            <dl>
                                                <div><dt>{"Fields"}</dt><dd>{health.field_count}</dd></div>
                                                <div><dt>{"Labels"}</dt><dd>{health.label_count}</dd></div>
                                                <div><dt>{"Keys"}</dt><dd>{health.entity_key_count}</dd></div>
                                                <div><dt>{"Online"}</dt><dd>{health.online_ready_count}</dd></div>
                                                <div><dt>{"Issues"}</dt><dd>{health.issue_count}</dd></div>
                                                <div><dt>{"Missing"}</dt><dd>{health.high_missing_count}</dd></div>
                                            </dl>
                                            <small>{format!("unstable {} / unowned {}", health.unstable_field_count, health.unowned_field_count)}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Split And Schema Coverage"}</h3>
                                    <p>{"Train/validation/test splits and schema fields that determine what can become features, labels, or review evidence."}</p>
                                </div>
                            </div>
                            if snapshot.datasets.is_empty() {
                                <p class="empty">{"No split or schema coverage available."}</p>
                            } else {
                                <div class="dataset-workbench-list">
                                    {for snapshot.datasets.iter().take(6).map(|dataset| html! {
                                        <article class="dataset-workbench">
                                            <div class="workbench-title">
                                                <div>
                                                    <strong>{format!("{}:{}", dataset.dataset_key, dataset.dataset_version)}</strong>
                                                    <span>{format!("schema hash: {}", dataset.schema_hash)}</span>
                                                </div>
                                                <span class="status-token neutral">{format!("{} fields", dataset.fields.len())}</span>
                                            </div>
                                            <div class="workbench-grid">
                                                <div>
                                                    <h4>{"Splits"}</h4>
                                                    if dataset.splits.is_empty() {
                                                        <p class="empty">{"No split records."}</p>
                                                    } else {
                                                        <div class="split-list">
                                                            {for dataset.splits.iter().map(|split| html! {
                                                                <div class="split-row">
                                                                    <div>
                                                                        <strong>{&split.split_name}</strong>
                                                                        <span>{&split.data_uri}</span>
                                                                    </div>
                                                                    <div><span>{"Rows"}</span><strong>{split.row_count}</strong></div>
                                                                    <div><span>{"Labels"}</span><strong>{format!("+{} / -{}", optional_u64(split.positive_count), optional_u64(split.negative_count))}</strong></div>
                                                                    <small>{format!("distribution: {}", payload_keys_label(&split.label_distribution_json))}</small>
                                                                </div>
                                                            })}
                                                        </div>
                                                    }
                                                </div>
                                                <div>
                                                    <h4>{"Schema Fields"}</h4>
                                                    <div class="field-table">
                                                        <div class="field-table-head">
                                                            <span>{"Field"}</span>
                                                            <span>{"Type / Role"}</span>
                                                            <span>{"Nullability"}</span>
                                                            <span>{"Profile"}</span>
                                                        </div>
                                                        {for dataset.fields.iter().take(10).map(|field| html! {
                                                            <div class="field-row">
                                                                <div class="primary-cell">
                                                                    <strong>{&field.field_name}</strong>
                                                                    <span>{empty_label(&field.description)}</span>
                                                                </div>
                                                                <div class="chip-row">
                                                                    <span class="type-chip">{&field.logical_type}</span>
                                                                    <span class="role-chip">{&field.semantic_role}</span>
                                                                </div>
                                                                <span class={classes!("status-token", if field.nullable { "neutral" } else { "strong" })}>
                                                                    {if field.nullable { "nullable" } else { "required" }}
                                                                </span>
                                                                <details class="data-source-detail field-profile-detail">
                                                                    <summary>{payload_signal_count_label(&field.profile_json, "profile signals")}</summary>
                                                                    <small>{payload_keys_label(&field.profile_json)}</small>
                                                                </details>
                                                            </div>
                                                        })}
                                                    </div>
                                                </div>
                                            </div>
                                        </article>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Field Mapping Lineage"}</h3>
                                    <p>{"How external fields become canonical claims, policy, provider, member, or feature entities."}</p>
                                </div>
                            </div>
                            if !snapshot.datasets.iter().any(|dataset| !dataset.mappings.is_empty()) {
                                <p class="empty">{"No field mappings registered."}</p>
                            } else {
                                <div class="lineage-list">
                                    {for snapshot.datasets.iter().flat_map(|dataset| {
                                        dataset.mappings.iter().map(move |mapping| (dataset, mapping))
                                    }).take(12).map(|(dataset, mapping)| html! {
                                        <div class="lineage-row">
                                            <div class="lineage-flow">
                                                <strong>{&mapping.external_field}</strong>
                                                <span>{"->"}</span>
                                                <strong>{&mapping.canonical_target}</strong>
                                            </div>
                                                <span>{mapping.feature_name.as_deref().unwrap_or("no feature")}</span>
                                                <span class={classes!("status-token", status_tone(&mapping.status))}>{&mapping.status}</span>
                                            <details class="data-source-detail">
                                                <summary>{format!("{}:{} / {}", dataset.dataset_key, dataset.dataset_version, mapping.transform_kind)}</summary>
                                                <small>{format!("transform {}", payload_keys_label(&mapping.transform_json))}</small>
                                            </details>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Model Evaluation Lineage"}</h3>
                                    <p>{"Which dataset version and data quality state produced each model evaluation result."}</p>
                                </div>
                            </div>
                            if snapshot.evaluations.is_empty() {
                                <p class="empty">{"No model evaluations registered."}</p>
                            } else {
                                <div class="ops-table evaluation-table">
                                    <div class="ops-table-head">
                                        <span>{"Model"}</span>
                                        <span>{"Run"}</span>
                                        <span>{"AUC"}</span>
                                        <span>{"Precision"}</span>
                                        <span>{"Recall"}</span>
                                        <span>{"Data Quality"}</span>
                                    </div>
                                    {for snapshot.evaluations.iter().take(8).map(|evaluation| {
                                        let lineage = lineage_for(&snapshot.lineage, &evaluation.evaluation_run_id);
                                        html! {
                                            <div class="ops-table-row">
                                                <div class="primary-cell">
                                                    <strong>{format!("{} / {}", evaluation.model_key, evaluation.model_version)}</strong>
                                                    <span>{format!("dataset {}", evaluation.model_dataset_id)}</span>
                                                </div>
                                                <span>{format!("{} / {}", evaluation.evaluation_run_id, evaluation.scheme_family)}</span>
                                                <strong>{optional_metric(&evaluation.auc)}</strong>
                                                <strong>{optional_metric(&evaluation.precision)}</strong>
                                                <strong>{optional_metric(&evaluation.recall)}</strong>
                                                <span>{lineage_data_quality_label(lineage)}</span>
                                                <details class="row-detail data-source-detail evaluation-evidence-detail">
                                                    <summary>{format!("Evaluation evidence detail: f1 {} / threshold {}", optional_metric(&evaluation.f1), optional_metric(&evaluation.threshold))}</summary>
                                                    <div class="data-source-detail-grid">
                                                        <small>{format!("source {}", lineage_source_label(lineage))}</small>
                                                        <small>{format!("metrics {}", payload_signal_count_label(&evaluation.metrics_json, "metric fields"))}</small>
                                                        <small>{format!("confusion {}", payload_signal_count_label(&evaluation.confusion_matrix_json, "confusion fields"))}</small>
                                                        <small>{format!("feature importance {}", evaluation.feature_importance_uri.as_deref().unwrap_or("none"))}</small>
                                                        <small>{format!("permutation importance {}", evaluation.permutation_importance_uri.as_deref().unwrap_or("none"))}</small>
                                                    </div>
                                                </details>
                                            </div>
                                        }
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

#[function_component(LeadsCasesPage)]
fn leads_cases_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let selected_lead_id = use_state(String::new);
    let triage_decision = use_state(|| "open_case".to_string());
    let triage_assignee = use_state(|| "investigator-1".to_string());
    let triage_reviewer = use_state(|| "lead-reviewer-1".to_string());
    let triage_priority = use_state(|| "high".to_string());
    let triage_notes = use_state(|| "Opened from Operations Studio lead triage.".to_string());
    let triage_evidence_refs = use_state(String::new);
    let selected_case_id = use_state(String::new);
    let case_status = use_state(|| "investigating".to_string());
    let case_actor = use_state(|| "case-manager-1".to_string());
    let case_notes =
        use_state(|| "Status updated from Operations Studio case workflow.".to_string());
    let case_evidence_refs = use_state(String::new);
    let investigation_outcome = use_state(|| "confirmed_fwa_prevented_payment".to_string());
    let investigation_confirmed = use_state(|| true);
    let financial_impact_type = use_state(|| "prevented_payment".to_string());
    let saving_amount = use_state(String::new);
    let investigation_notes = use_state(|| {
        "Reviewer confirmed the pre-payment FWA intervention and prevented payment.".to_string()
    });
    let investigation_evidence_refs = use_state(String::new);
    let snapshot_state = use_state(|| ApiState::<LeadsCasesSnapshot>::Idle);
    let triage_state = use_state(|| ApiState::<TriageLeadRecord>::Idle);
    let case_update_state = use_state(|| ApiState::<UpdateCaseStatusRecord>::Idle);
    let investigation_state = use_state(|| ApiState::<PilotWritebackResponse>::Idle);
    let case_agent_state = use_state(|| ApiState::<AgentInvestigationResponse>::Idle);

    let load_cases = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_cases = load_cases.clone();
        Callback::from(move |_| load_cases.emit(()))
    };

    let triage_lead = {
        let api_key = api_key.clone();
        let selected_lead_id = selected_lead_id.clone();
        let triage_decision = triage_decision.clone();
        let triage_assignee = triage_assignee.clone();
        let triage_reviewer = triage_reviewer.clone();
        let triage_priority = triage_priority.clone();
        let triage_notes = triage_notes.clone();
        let triage_evidence_refs = triage_evidence_refs.clone();
        let snapshot_state = snapshot_state.clone();
        let triage_state = triage_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                triage_state.set(ApiState::Failed("load leads before triage".into()));
                return;
            };
            let lead = selected_lead(snapshot, &selected_lead_id);
            let Some(lead) = lead else {
                triage_state.set(ApiState::Failed("select a lead to triage".into()));
                return;
            };
            let api_key = (*api_key).clone();
            let lead_id = lead.lead_id.clone();
            let fallback_refs = lead.evidence_refs.clone();
            let payload = json!({
                "decision": (*triage_decision).clone(),
                "merge_target_lead_id": Value::Null,
                "assignee": (*triage_assignee).clone(),
                "reviewer": (*triage_reviewer).clone(),
                "priority": (*triage_priority).clone(),
                "notes": (*triage_notes).clone(),
                "evidence_refs": refs_or_fallback(&triage_evidence_refs, fallback_refs),
            });
            let triage_state = triage_state.clone();
            let snapshot_state = snapshot_state.clone();
            triage_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_triage_lead(api_key.clone(), lead_id, payload).await {
                    Ok(record) => {
                        triage_state.set(ApiState::Ready(record));
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => triage_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let update_case = {
        let api_key = api_key.clone();
        let selected_case_id = selected_case_id.clone();
        let case_status = case_status.clone();
        let case_actor = case_actor.clone();
        let case_notes = case_notes.clone();
        let case_evidence_refs = case_evidence_refs.clone();
        let snapshot_state = snapshot_state.clone();
        let case_update_state = case_update_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                case_update_state.set(ApiState::Failed("load cases before status update".into()));
                return;
            };
            let case = selected_case(snapshot, &selected_case_id);
            let Some(case) = case else {
                case_update_state.set(ApiState::Failed("select a case to update".into()));
                return;
            };
            let api_key = (*api_key).clone();
            let case_id = case.case_id.clone();
            let payload = json!({
                "status": (*case_status).clone(),
                "actor_id": (*case_actor).clone(),
                "notes": (*case_notes).clone(),
                "evidence_refs": refs_or_fallback(&case_evidence_refs, vec![format!("investigation_cases:{}", case.case_id)]),
            });
            let case_update_state = case_update_state.clone();
            let snapshot_state = snapshot_state.clone();
            case_update_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_case_status(api_key.clone(), case_id, payload).await {
                    Ok(record) => {
                        case_update_state.set(ApiState::Ready(record));
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => case_update_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let write_investigation_result = {
        let api_key = api_key.clone();
        let selected_case_id = selected_case_id.clone();
        let investigation_outcome = investigation_outcome.clone();
        let investigation_confirmed = investigation_confirmed.clone();
        let financial_impact_type = financial_impact_type.clone();
        let saving_amount = saving_amount.clone();
        let investigation_notes = investigation_notes.clone();
        let investigation_evidence_refs = investigation_evidence_refs.clone();
        let snapshot_state = snapshot_state.clone();
        let investigation_state = investigation_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                investigation_state.set(ApiState::Failed(
                    "load cases before investigation writeback".into(),
                ));
                return;
            };
            let case = selected_case(snapshot, &selected_case_id);
            let Some(case) = case else {
                investigation_state.set(ApiState::Failed("select a case to write back".into()));
                return;
            };
            if saving_amount.trim().is_empty() {
                investigation_state.set(ApiState::Failed("confirmed amount is required".into()));
                return;
            }
            let api_key = (*api_key).clone();
            let case_id = case.case_id.clone();
            let claim_id = case.claim_id.clone();
            let fallback_refs = vec![
                format!("investigation_cases:{}", case.case_id),
                format!("leads:{}", case.lead_id),
            ];
            let payload = json!({
                "case_id": case_id,
                "claim_id": claim_id,
                "investigation_id": format!("INV-UI-{}", case.case_id),
                "outcome": (*investigation_outcome).clone(),
                "confirmed_fwa": *investigation_confirmed,
                "financial_impact_type": (*financial_impact_type).clone(),
                "saving_amount": (*saving_amount).clone(),
                "currency": "CNY",
                "notes": (*investigation_notes).clone(),
                "evidence_refs": refs_or_fallback(&investigation_evidence_refs, fallback_refs),
            });
            let investigation_state = investigation_state.clone();
            let snapshot_state = snapshot_state.clone();
            investigation_state.set(ApiState::Loading);
            spawn_local(async move {
                match post_investigation_result(api_key.clone(), payload).await {
                    Ok(record) => {
                        investigation_state.set(ApiState::Ready(record));
                        snapshot_state.set(match get_leads_cases_snapshot(api_key).await {
                            Ok(snapshot) => ApiState::Ready(snapshot),
                            Err(error) => ApiState::Failed(error),
                        });
                    }
                    Err(error) => investigation_state.set(ApiState::Failed(error)),
                }
            });
        })
    };

    let generate_case_investigation_package = {
        let api_key = api_key.clone();
        let selected_lead_id = selected_lead_id.clone();
        let selected_case_id = selected_case_id.clone();
        let snapshot_state = snapshot_state.clone();
        let case_agent_state = case_agent_state.clone();
        Callback::from(move |_| {
            let ApiState::Ready(snapshot) = &*snapshot_state else {
                case_agent_state.set(ApiState::Failed(
                    "load cases before generating the agent package".into(),
                ));
                return;
            };
            let case = selected_case(snapshot, &selected_case_id);
            let Some(case) = case else {
                case_agent_state.set(ApiState::Failed("select a case for investigation".into()));
                return;
            };
            let lead = lead_for_case(snapshot, case)
                .or_else(|| selected_lead(snapshot, &selected_lead_id));
            let top_reasons = lead
                .map(|lead| lead.reason.clone())
                .filter(|reason| !reason.trim().is_empty())
                .unwrap_or_else(|| case.routing_reason.clone());
            let payload = agent_investigation_payload(
                case.claim_id.clone(),
                lead.map(|lead| lead.risk_score.to_string())
                    .unwrap_or_else(|| "70".to_string()),
                lead.map(|lead| lead.rag.to_ascii_uppercase())
                    .unwrap_or_else(|| "RED".to_string()),
                case.scheme_family.clone(),
                top_reasons,
                "case-review".to_string(),
                case.source_system.clone(),
                format!(
                    "{},{},{},{}",
                    case.scheme_family, case.review_mode, case.lead_source, case.provider_id
                ),
            );
            let payload = match payload {
                Ok(payload) => payload,
                Err(error) => {
                    case_agent_state.set(ApiState::Failed(error));
                    return;
                }
            };
            let api_key = (*api_key).clone();
            let case_agent_state = case_agent_state.clone();
            case_agent_state.set(ApiState::Loading);
            spawn_local(async move {
                case_agent_state.set(match post_agent_investigation(api_key, payload).await {
                    Ok(response) => ApiState::Ready(response),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    {
        let load_cases = load_cases.clone();
        use_effect_with((), move |_| {
            load_cases.emit(());
            || ()
        });
    }

    let select_lead = {
        let selected_lead_id = selected_lead_id.clone();
        Callback::from(move |lead_id: String| selected_lead_id.set(lead_id))
    };

    let select_case = {
        let selected_case_id = selected_case_id.clone();
        let case_agent_state = case_agent_state.clone();
        Callback::from(move |case_id: String| {
            selected_case_id.set(case_id);
            case_agent_state.set(ApiState::Idle);
        })
    };

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Leads & Cases"}</h2>
                    <p>{"Triage generated FWA leads into investigation cases and keep case status, SLA, reviewer, and evidence signals current."}</p>
                </div>
                <span class="status-pill">{"Case Workflow"}</span>
            </div>

            <section class="panel queue-source-panel">
                <div class="queue-source-bar">
                    <h3>{"Queue Source"}</h3>
                    <span class="status-token neutral">{"configured queue source"}</span>
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh queue" }}
                    </button>
                </div>
            </section>

            <div class="leads-cases-workflow">
                <div class="queue-column">
                    <LeadsCasesView
                        state={(*snapshot_state).clone()}
                        selected_lead_id={(*selected_lead_id).clone()}
                        selected_case_id={(*selected_case_id).clone()}
                        on_select_lead={select_lead}
                        on_select_case={select_case}
                    />
                </div>

                <aside class="panel result-stack case-action-panel">
                    <h3>{"Case Investigation Workspace"}</h3>
                    {match &*snapshot_state {
                        ApiState::Ready(snapshot) => {
                            let lead = selected_lead(snapshot, &selected_lead_id);
                            let case = selected_case(snapshot, &selected_case_id);
                            html! {
                                <>
                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Selected lead"}</span>
                                            <strong>{lead.map(|lead| lead.lead_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{lead.map(|lead| lead.reason.as_str()).unwrap_or("Lead is only the risk candidate before case work starts.")}</small>
                                        </div>
                                        <h4>{"Lead Triage"}</h4>
                                        <div class="form-grid action-form-grid">
                                            <label>
                                                {"Decision"}
                                                <select
                                                    onchange={{
                                                        let triage_decision = triage_decision.clone();
                                                        Callback::from(move |event: Event| {
                                                            triage_decision.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                                        })
                                                    }}
                                                >
                                                    <option value="open_case" selected={(*triage_decision).as_str() == "open_case"}>{"Open case"}</option>
                                                    <option value="request_evidence" selected={(*triage_decision).as_str() == "request_evidence"}>{"Request evidence"}</option>
                                                    <option value="reject_lead" selected={(*triage_decision).as_str() == "reject_lead"}>{"Reject lead"}</option>
                                                    <option value="merge_lead" selected={(*triage_decision).as_str() == "merge_lead"}>{"Merge lead"}</option>
                                                </select>
                                            </label>
                                            {text_input("Priority", &triage_priority)}
                                            {text_input("Assignee", &triage_assignee)}
                                            {text_input("Reviewer", &triage_reviewer)}
                                            {text_input("Evidence refs", &triage_evidence_refs)}
                                        </div>
                                        <label class="compact-note">
                                            {"Notes"}
                                            <textarea
                                                value={(*triage_notes).clone()}
                                                oninput={{
                                                    let triage_notes = triage_notes.clone();
                                                    Callback::from(move |event: InputEvent| {
                                                        triage_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                                    })
                                                }}
                                            />
                                        </label>
                                        <div class="button-row">
                                            <button onclick={triage_lead} disabled={lead.is_none() || matches!(&*triage_state, ApiState::Loading)}>
                                                {if matches!(&*triage_state, ApiState::Loading) { "Submitting..." } else { "Submit triage" }}
                                            </button>
                                        </div>
                                        <TriageResultView state={(*triage_state).clone()} />
                                    </section>

                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Selected case"}</span>
                                            <strong>{case.map(|case| case.case_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{case.map(|case| case.routing_reason.as_str()).unwrap_or("Case is the human investigation work item.")}</small>
                                        </div>
                                        <h4>{"Case Brief"}</h4>
                                        {case.map(|case| html! {
                                            <>
                                                <div class="score-hero">
                                                    <div><span>{"Claim"}</span><strong>{&case.claim_id}</strong></div>
                                                    <div><span>{"SLA"}</span><strong>{format!("{} / {}h", sla_label(&case.sla_status), case.sla_target_hours)}</strong></div>
                                                    <div><span>{"Reviewer"}</span><strong>{&case.reviewer}</strong></div>
                                                </div>
                                                <div class="summary-grid">
                                                    <div><span>{"Scheme"}</span><strong>{business_label(&case.scheme_family)}</strong></div>
                                                    <div><span>{"Status"}</span><strong>{case_stage_label(&case.status)}</strong></div>
                                                    <div><span>{"Review mode"}</span><strong>{business_label(&case.review_mode)}</strong></div>
                                                    <div><span>{"Outcome"}</span><strong>{case.final_outcome.as_deref().map(business_label).unwrap_or_else(|| "Human review pending".into())}</strong></div>
                                                </div>
                                            </>
                                        }).unwrap_or_else(|| html! { <p class="empty">{"Select a case to open the investigation workspace."}</p> })}
                                        <div class="tag-grid compact-tags">
                                            <span>{"Assistive only"}</span>
                                            <span>{"Human writeback required"}</span>
                                            <span>{"Evidence refs required"}</span>
                                        </div>
                                    </section>

                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Agent-assisted case package"}</span>
                                            <strong>{case.map(|case| case.claim_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{"Draft only: checklist, similar cases, evidence summary, and writeback hints. Reviewer decides."}</small>
                                        </div>
                                        <div class="button-row">
                                            <button onclick={generate_case_investigation_package} disabled={case.is_none() || matches!(&*case_agent_state, ApiState::Loading)}>
                                                {if matches!(&*case_agent_state, ApiState::Loading) { "Generating..." } else { "Generate case package" }}
                                            </button>
                                        </div>
                                    </section>

                                    <AgentInvestigationView state={(*case_agent_state).clone()} />

                                    <section class="action-card">
                                        <div class="selected-work-item">
                                            <span>{"Human Decision / Writeback"}</span>
                                            <strong>{case.map(|case| case.claim_id.as_str()).unwrap_or("none")}</strong>
                                            <small>{case.and_then(|case| case.final_outcome.as_deref()).map(business_label).unwrap_or_else(|| "No investigation result written back yet.".into())}</small>
                                        </div>
                                        <h4>{"Investigation Writeback"}</h4>
                                        <div class="form-grid action-form-grid">
                                            {text_input("Outcome", &investigation_outcome)}
                                            <label>
                                                {"Impact type"}
                                                <select
                                                    onchange={{
                                                        let financial_impact_type = financial_impact_type.clone();
                                                        Callback::from(move |event: Event| {
                                                            financial_impact_type.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                                        })
                                                    }}
                                                >
                                                    <option value="prevented_payment" selected={(*financial_impact_type).as_str() == "prevented_payment"}>{"Prevented payment"}</option>
                                                    <option value="recovered_amount" selected={(*financial_impact_type).as_str() == "recovered_amount"}>{"Recovered amount"}</option>
                                                    <option value="estimated_impact" selected={(*financial_impact_type).as_str() == "estimated_impact"}>{"Estimated impact"}</option>
                                                    <option value="avoided_future_exposure" selected={(*financial_impact_type).as_str() == "avoided_future_exposure"}>{"Avoided exposure"}</option>
                                                    <option value="deterrence_estimate" selected={(*financial_impact_type).as_str() == "deterrence_estimate"}>{"Deterrence estimate"}</option>
                                                </select>
                                            </label>
                                            {text_input("Confirmed amount", &saving_amount)}
                                            {text_input("Evidence refs", &investigation_evidence_refs)}
                                        </div>
                                        <label class="checkbox-row">
                                            <input
                                                type="checkbox"
                                                checked={*investigation_confirmed}
                                                onchange={{
                                                    let investigation_confirmed = investigation_confirmed.clone();
                                                    Callback::from(move |event: Event| {
                                                        investigation_confirmed.set(event.target_unchecked_into::<HtmlInputElement>().checked());
                                                    })
                                                }}
                                            />
                                            {"Confirmed by reviewer"}
                                        </label>
                                        <label class="compact-note">
                                            {"Notes"}
                                            <textarea
                                                value={(*investigation_notes).clone()}
                                                oninput={{
                                                    let investigation_notes = investigation_notes.clone();
                                                    Callback::from(move |event: InputEvent| {
                                                        investigation_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                                    })
                                                }}
                                            />
                                        </label>
                                        <div class="button-row">
                                            <button onclick={write_investigation_result} disabled={case.is_none() || matches!(&*investigation_state, ApiState::Loading)}>
                                                {if matches!(&*investigation_state, ApiState::Loading) { "Writing back..." } else { "Confirm and write back" }}
                                            </button>
                                        </div>
                                        <InvestigationWritebackResultView state={(*investigation_state).clone()} />

                                        <details class="data-source-detail governance-detail">
                                            <summary>{"Case status maintenance"}</summary>
                                            <div class="form-grid action-form-grid">
                                                <label>
                                                    {"Status"}
                                                    <select
                                                        onchange={{
                                                            let case_status = case_status.clone();
                                                            Callback::from(move |event: Event| {
                                                                case_status.set(event.target_unchecked_into::<HtmlSelectElement>().value());
                                                            })
                                                        }}
                                                    >
                                                        <option value="triage" selected={(*case_status).as_str() == "triage"}>{"Triage"}</option>
                                                        <option value="investigating" selected={(*case_status).as_str() == "investigating"}>{"Investigating"}</option>
                                                        <option value="pending_evidence" selected={(*case_status).as_str() == "pending_evidence"}>{"Pending evidence"}</option>
                                                        <option value="confirmed" selected={(*case_status).as_str() == "confirmed"}>{"Confirmed"}</option>
                                                        <option value="rejected" selected={(*case_status).as_str() == "rejected"}>{"Rejected"}</option>
                                                        <option value="closed" selected={(*case_status).as_str() == "closed"}>{"Closed"}</option>
                                                    </select>
                                                </label>
                                                {text_input("Actor", &case_actor)}
                                                {text_input("Evidence refs", &case_evidence_refs)}
                                            </div>
                                            <label class="compact-note">
                                                {"Notes"}
                                                <textarea
                                                    value={(*case_notes).clone()}
                                                    oninput={{
                                                        let case_notes = case_notes.clone();
                                                        Callback::from(move |event: InputEvent| {
                                                            case_notes.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                                        })
                                                    }}
                                                />
                                            </label>
                                            <div class="button-row">
                                                <button onclick={update_case} disabled={case.is_none() || matches!(&*case_update_state, ApiState::Loading)}>
                                                    {if matches!(&*case_update_state, ApiState::Loading) { "Updating..." } else { "Update case status" }}
                                                </button>
                                            </div>
                                            <CaseUpdateResultView state={(*case_update_state).clone()} />
                                        </details>
                                    </section>
                                </>
                            }
                        }
                        ApiState::Loading => html! { <p>{"Loading queue actions..."}</p> },
                        ApiState::Failed(_) => html! { <p class="empty">{"Fix the queue source before taking action."}</p> },
                        ApiState::Idle => html! { <p class="empty">{"Load the queue to select a lead or case."}</p> },
                    }}
                </aside>
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct LeadsCasesProps {
    state: ApiState<LeadsCasesSnapshot>,
    selected_lead_id: String,
    selected_case_id: String,
    on_select_lead: Callback<String>,
    on_select_case: Callback<String>,
}

#[function_component(LeadsCasesView)]
fn leads_cases_view(props: &LeadsCasesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load leads and cases to inspect the investigation queue."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading leads and cases..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Investigation Control"}</h3>
                                    <p>{"Workload, urgency, and queue movement for the human investigation desk."}</p>
                                </div>
                            </div>
                            <div class="case-control-rail">
                                <div><strong>{open_lead_count(&snapshot.leads)}</strong><span>{"open leads"}</span></div>
                                <div><strong>{active_case_count(&snapshot.cases)}</strong><span>{"active cases"}</span></div>
                                <div><strong>{breached_case_count(&snapshot.cases)}</strong><span>{"SLA attention"}</span></div>
                            </div>
                            <div class="case-control-grid">
                                <div class="queue-meter-card">
                                    <span>{"Lead movement"}</span>
                                    {queue_meter("New", lead_status_count(&snapshot.leads, "new"), snapshot.leads.len(), "warning")}
                                    {queue_meter("Needs evidence", lead_status_count(&snapshot.leads, "pending_evidence"), snapshot.leads.len(), "danger")}
                                    {queue_meter("Case opened", lead_status_count(&snapshot.leads, "triaged"), snapshot.leads.len(), "success")}
                                </div>
                                <div class="queue-meter-card">
                                    <span>{"Case movement"}</span>
                                    {queue_meter("Investigating", case_status_count(&snapshot.cases, "investigating"), snapshot.cases.len(), "warning")}
                                    {queue_meter("Confirmed", case_status_count(&snapshot.cases, "confirmed"), snapshot.cases.len(), "success")}
                                    {queue_meter("Closed", case_status_count(&snapshot.cases, "closed"), snapshot.cases.len(), "neutral")}
                                </div>
                                <div class="queue-meter-card case-focus-card">
                                    <span>{"Primary pattern"}</span>
                                    <strong>{top_scheme_label(&snapshot.leads)}</strong>
                                    <small>{"use this to assign medical and SIU review capacity"}</small>
                                </div>
                            </div>
                        </section>

                        <section class="lead-case-queue-grid">
                            <div class="panel result-stack">
                                <h3>{"Generated Leads"}</h3>
                                if snapshot.leads.is_empty() {
                                    <p class="empty">{"No leads returned."}</p>
                                } else {
                                    <div class="queue-list">
                                        {for snapshot.leads.iter().take(12).enumerate().map(|(index, lead)| {
                                            let selected = props.selected_lead_id.trim();
                                            let is_active = if selected.is_empty() {
                                                index == 0
                                            } else {
                                                selected == lead.lead_id
                                            };
                                            let lead_id = lead.lead_id.clone();
                                            let on_select_lead = props.on_select_lead.clone();
                                            html! {
                                                <button
                                                    type="button"
                                                    class={classes!("row-button", "queue-row", is_active.then_some("active"))}
                                                    onclick={Callback::from(move |_| on_select_lead.emit(lead_id.clone()))}
                                                >
                                                    <div class="primary-cell">
                                                        <strong>{format!("{} / {}", lead.lead_id, lead.claim_id)}</strong>
                                                        <span>{&lead.reason}</span>
                                                        <small>{format!("{} / {} / {}", lead.scheme_family, lead.provider_id, lead.member_id)}</small>
                                                    </div>
                                                    <div class="queue-row-meta">
                                                        <span class="status-token strong">{format!("risk {}", lead.risk_score)}</span>
                                                        <span class={classes!("status-token", status_tone(&lead.rag))}>{rag_label(&lead.rag)}</span>
                                                        <span class={classes!("status-token", lead_stage_tone(&lead.status))}>{lead_stage_label(&lead.status)}</span>
                                                    </div>
                                                </button>
                                            }
                                        })}
                                    </div>
                                }
                            </div>

                            <div class="panel result-stack">
                                <h3>{"Investigation Cases"}</h3>
                                if snapshot.cases.is_empty() {
                                    <p class="empty">{"No investigation cases returned."}</p>
                                } else {
                                    <div class="queue-list">
                                        {for snapshot.cases.iter().take(12).enumerate().map(|(index, case)| {
                                            let selected = props.selected_case_id.trim();
                                            let is_active = if selected.is_empty() {
                                                index == 0
                                            } else {
                                                selected == case.case_id
                                            };
                                            let case_id = case.case_id.clone();
                                            let on_select_case = props.on_select_case.clone();
                                            html! {
                                                <button
                                                    type="button"
                                                    class={classes!("row-button", "queue-row", is_active.then_some("active"))}
                                                    onclick={Callback::from(move |_| on_select_case.emit(case_id.clone()))}
                                                >
                                                    <div class="primary-cell">
                                                        <strong>{format!("{} / {}", case.case_id, case.claim_id)}</strong>
                                                        <span>{&case.routing_reason}</span>
                                                        <small>{format!("{} / reviewer {} / lead {}", case.assignee, case.reviewer, case.lead_id)}</small>
                                                        {case.final_outcome.as_ref().map(|outcome| html! {
                                                            <small>{format!("outcome: {} / writeback {}", business_label(outcome), case.investigation_result_id.as_deref().map(business_label).unwrap_or_else(|| "Pending".into()))}</small>
                                                        }).unwrap_or_else(|| html! {})}
                                                    </div>
                                                    <div class="queue-row-meta">
                                                        <span class={classes!("status-token", priority_tone(&case.priority))}>{priority_label(&case.priority)}</span>
                                                        <span class={classes!("status-token", case_stage_tone(&case.status))}>{case_stage_label(&case.status)}</span>
                                                        <span class={classes!("status-token", sla_tone(&case.sla_status))}>{sla_label(&case.sla_status)}</span>
                                                    </div>
                                                </button>
                                            }
                                        })}
                                    </div>
                                }
                            </div>
                        </section>
                    </>
                },
            }}
        </>
    }
}

#[derive(Properties, PartialEq)]
struct TriageResultProps {
    state: ApiState<TriageLeadRecord>,
}

#[function_component(TriageResultView)]
fn triage_result_view(props: &TriageResultProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Submitting lead triage..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Audit"}</span><strong>{&record.audit_id}</strong></div>
                <div><span>{"Lead"}</span><strong>{format!("{} / {}", record.lead.lead_id, lead_stage_label(&record.lead.status))}</strong></div>
                <div><span>{"Case"}</span><strong>{record.case.as_ref().map(|case| case.case_id.as_str()).unwrap_or("none")}</strong></div>
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct CaseUpdateResultProps {
    state: ApiState<UpdateCaseStatusRecord>,
}

#[function_component(CaseUpdateResultView)]
fn case_update_result_view(props: &CaseUpdateResultProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Updating case status..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Audit"}</span><strong>{&record.audit_id}</strong></div>
                <div><span>{"Case"}</span><strong>{&record.case.case_id}</strong></div>
                <div><span>{"Status"}</span><strong>{case_stage_label(&record.case.status)}</strong></div>
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct InvestigationWritebackResultProps {
    state: ApiState<PilotWritebackResponse>,
}

#[function_component(InvestigationWritebackResultView)]
fn investigation_writeback_result_view(props: &InvestigationWritebackResultProps) -> Html {
    match &props.state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Writing investigation result..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(record) => html! {
            <div class="summary-grid">
                <div><span>{"Claim"}</span><strong>{&record.claim_id}</strong></div>
                <div><span>{"Audit"}</span><strong>{&record.audit_id}</strong></div>
                <div><span>{"Writeback"}</span><strong>{business_label(&record.event_status)}</strong></div>
                <div><span>{"Idempotency"}</span><strong>{&record.idempotency_key}</strong></div>
            </div>
        },
    }
}

#[function_component(MemberProfilePage)]
fn member_profile_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let member_id = use_state(|| "MBR-0287".to_string());
    let profile_state = use_state(|| ApiState::<MemberProfileSummary>::Idle);

    let load_profile = {
        let api_key = api_key.clone();
        let member_id = member_id.clone();
        let profile_state = profile_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let member_id = (*member_id).clone();
            let profile_state = profile_state.clone();
            profile_state.set(ApiState::Loading);
            spawn_local(async move {
                profile_state.set(match get_member_profile_summary(api_key, member_id).await {
                    Ok(profile) => ApiState::Ready(profile),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_profile = load_profile.clone();
        Callback::from(move |_| load_profile.emit(()))
    };

    {
        let load_profile = load_profile.clone();
        use_effect_with((), move |_| {
            load_profile.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Member Profile"}</h2>
                    <p>{"Inspect the TPA-facing member profile summary used to explain utilization, policy exposure, high-risk history, and evidence-backed profile context."}</p>
                </div>
                <span class="status-pill">{"Profile Summary API"}</span>
            </div>

            <section class="panel">
                <h3>{"Member Profile Source"}</h3>
                <div class="form-grid">
                    {text_input("Member ID", &member_id)}
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*profile_state, ApiState::Loading)}>
                        {if matches!(&*profile_state, ApiState::Loading) { "Refreshing..." } else { "Refresh member profile" }}
                    </button>
                </div>
            </section>

            <MemberProfileView state={(*profile_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct MemberProfileProps {
    state: ApiState<MemberProfileSummary>,
}

#[function_component(MemberProfileView)]
fn member_profile_view(props: &MemberProfileProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Member Profile Summary"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Load a member profile summary to inspect utilization and evidence."}</p> },
                ApiState::Loading => html! { <p>{"Loading member profile..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(profile) => html! {
                    <>
                        <div class="score-hero">
                            <div><span>{"Member"}</span><strong>{&profile.member_id}</strong></div>
                            <div><span>{"Risk Summary"}</span><strong>{&profile.risk_level_summary}</strong></div>
                            <div><span>{"High-Risk Claims"}</span><strong>{profile.high_risk_claim_count}</strong></div>
                        </div>
                        {member_profile_cockpit(profile)}
                        <div class="summary-grid">
                            <div><span>{"Claims"}</span><strong>{profile.claim_count}</strong></div>
                            <div><span>{"Policies"}</span><strong>{profile.policy_count}</strong></div>
                            <div><span>{"Total Amount"}</span><strong>{format!("{} {}", display_value(&profile.total_claim_amount), profile.currency)}</strong></div>
                            <div><span>{"Latest Claim"}</span><strong>{profile.latest_claim_id.as_deref().unwrap_or("none")}</strong></div>
                            <div><span>{"Evidence Refs"}</span><strong>{profile.evidence_refs.len()}</strong></div>
                        </div>
                        <h4>{"Profile Narrative"}</h4>
                        <p>{&profile.profile_summary}</p>
                        <h4>{"Evidence"}</h4>
                        <small>{refs_label(&profile.evidence_refs)}</small>
                    </>
                },
            }}
        </section>
    }
}

fn member_profile_cockpit(profile: &MemberProfileSummary) -> Html {
    let total_amount = format!(
        "{} {}",
        display_value(&profile.total_claim_amount),
        profile.currency
    );
    html! {
        <div class="member-profile-cockpit">
            <div class="relationship-graph member-relationship-graph">
                <div class="graph-ring"></div>
                <div class="graph-ring inner"></div>
                <div class="graph-center member-profile-center">
                    <span>{"Member Evidence"}</span>
                    <strong>{&profile.member_id}</strong>
                </div>
                {member_graph_entity("Risk summary", &profile.risk_level_summary, "top", "lead")}
                {member_graph_entity("Claims", &profile.claim_count.to_string(), "right", "claim")}
                {member_graph_entity("Policy exposure", &profile.policy_count.to_string(), "bottom", "case")}
                {member_graph_entity("Latest claim", profile.latest_claim_id.as_deref().unwrap_or("none"), "left", "claim")}
                {member_graph_entity("Total amount", &total_amount, "lower-right", "provider")}
                {member_graph_entity("Evidence refs", &profile.evidence_refs.len().to_string(), "lower-left", "reviewer")}
            </div>
            <div class="member-evidence-panel">
                <div>
                    <span>{"Member Evidence Map"}</span>
                    <strong>{format!("{} high-risk / {} total claims", profile.high_risk_claim_count, profile.claim_count)}</strong>
                    <small>{&profile.profile_summary}</small>
                </div>
                <div class="member-signal-stack">
                    {member_signal_row("Utilization Snapshot", &format!("{} claims", profile.claim_count), "strong")}
                    {member_signal_row("Policy exposure", &format!("{} policies", profile.policy_count), "neutral")}
                    {member_signal_row("Risk amount", &total_amount, "warning")}
                    {member_signal_row("Evidence trace", &format!("{} refs", profile.evidence_refs.len()), "success")}
                </div>
                <small>{format!("evidence: {}", refs_label(&profile.evidence_refs))}</small>
            </div>
        </div>
    }
}

fn member_graph_entity(label: &str, value: &str, position: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("graph-entity", position.to_string(), tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn member_signal_row(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("member-signal-row", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[function_component(ProviderRiskPage)]
fn provider_risk_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let summary_state = use_state(|| ApiState::<ProviderRiskSummary>::Idle);

    let load_summary = {
        let api_key = api_key.clone();
        let summary_state = summary_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let summary_state = summary_state.clone();
            summary_state.set(ApiState::Loading);
            spawn_local(async move {
                summary_state.set(match get_provider_risk_summary(api_key).await {
                    Ok(summary) => ApiState::Ready(summary),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_summary = load_summary.clone();
        Callback::from(move |_| load_summary.emit(()))
    };

    {
        let load_summary = load_summary.clone();
        use_effect_with((), move |_| {
            load_summary.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Provider Risk"}</h2>
                    <p>{"Inspect provider network and graph risk profiles, review routing, outlier flags, graph reasons, and evidence refs for provider-focused investigation."}</p>
                </div>
                <span class="status-pill">{"Provider Graph Risk"}</span>
            </div>

            <section class="panel">
                <h3>{"Provider Risk Source"}</h3>
                <p class="empty">{"Using the configured provider-risk workspace for graph and peer-pattern signals."}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*summary_state, ApiState::Loading)}>
                        {if matches!(&*summary_state, ApiState::Loading) { "Refreshing..." } else { "Refresh provider risk" }}
                    </button>
                </div>
            </section>

            <ProviderRiskView state={(*summary_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ProviderRiskProps {
    state: ApiState<ProviderRiskSummary>,
}

#[function_component(ProviderRiskView)]
fn provider_risk_view(props: &ProviderRiskProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load provider risk to inspect provider graph signals."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading provider risk..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(summary) => html! {
                    <>
                        <section class="panel result-stack">
                            <h3>{"Provider Risk Summary"}</h3>
                            <div class="score-hero">
                                <div><span>{"Providers"}</span><strong>{summary.provider_count}</strong></div>
                                <div><span>{"Review Required"}</span><strong>{summary.review_required_count}</strong></div>
                                <div><span>{"High Risk"}</span><strong>{summary.high_risk_count}</strong></div>
                            </div>
                            {provider_graph_cockpit(summary)}
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Provider Risk Profiles"}</h3>
                            if summary.providers.is_empty() {
                                <p class="empty">{"No provider risk profiles returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for summary.providers.iter().take(12).map(|provider| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", provider.provider_id, provider.risk_tier)}</strong>
                                                <span>{format!("score {} / route {}", provider.risk_score, provider.review_route)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Review"}</span><strong>{yes_no(provider.review_required)}</strong></div>
                                                <div><span>{"Claims"}</span><strong>{provider.claim_count}</strong></div>
                                                <div><span>{"Network Risk"}</span><strong>{optional_u8(provider.network_risk_score)}</strong></div>
                                                <div><span>{"Failures"}</span><strong>{provider.review_failure_count}</strong></div>
                                                <div><span>{"Confirmed FWA"}</span><strong>{provider.confirmed_fwa_count}</strong></div>
                                                <div><span>{"False Positives"}</span><strong>{provider.false_positive_count}</strong></div>
                                                <div><span>{"Specialty"}</span><strong>{provider.specialty.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"Network"}</span><strong>{provider.network_status.as_deref().unwrap_or("none")}</strong></div>
                                                <div><span>{"Latest Claim"}</span><strong>{provider.latest_claim_id.as_deref().unwrap_or("none")}</strong></div>
                                            </div>
                                            <small>{format!("outliers: {}", refs_label(&provider.outlier_flags))}</small>
                                            <small>{format!("graph reasons: {}", refs_label(&provider.graph_reasons))}</small>
                                            <small>{format!("evidence: {}", refs_label(&provider.evidence_refs))}</small>
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

fn provider_graph_cockpit(summary: &ProviderRiskSummary) -> Html {
    let primary = summary
        .providers
        .iter()
        .max_by_key(|provider| provider.risk_score);

    if let Some(provider) = primary {
        let network_score = provider
            .network_risk_score
            .map(|score| score.to_string())
            .unwrap_or_else(|| "n/a".into());
        let outlier_label = provider
            .outlier_flags
            .first()
            .cloned()
            .unwrap_or_else(|| "no outlier flag".into());
        let graph_reason = provider
            .graph_reasons
            .first()
            .cloned()
            .unwrap_or_else(|| "graph reason pending".into());
        html! {
            <div class="provider-risk-cockpit">
                <div class="relationship-graph provider-relationship-graph">
                    <div class="graph-ring"></div>
                    <div class="graph-ring inner"></div>
                    <div class="graph-center provider-risk-center">
                        <span>{"Provider Network"}</span>
                        <strong>{&provider.provider_id}</strong>
                    </div>
                    {provider_graph_entity("Risk tier", &provider.risk_tier, "top", "lead")}
                    {provider_graph_entity("Network risk", &network_score, "right", "provider")}
                    {provider_graph_entity("Review route", &provider.review_route, "bottom", "case")}
                    {provider_graph_entity("Latest claim", provider.latest_claim_id.as_deref().unwrap_or("none"), "left", "claim")}
                    {provider_graph_entity("Outlier flag", &outlier_label, "lower-right", "lead")}
                    {provider_graph_entity("Evidence refs", &provider.evidence_refs.len().to_string(), "lower-left", "reviewer")}
                </div>
                <div class="provider-graph-panel">
                    <div>
                        <span>{"Graph Risk Focus"}</span>
                        <strong>{format!("score {} / claims {}", provider.risk_score, provider.claim_count)}</strong>
                        <small>{graph_reason}</small>
                    </div>
                    <div class="provider-signal-stack">
                        {provider_signal_row("Confirmed FWA", &provider.confirmed_fwa_count.to_string(), "danger")}
                        {provider_signal_row("Review failures", &provider.review_failure_count.to_string(), "warning")}
                        {provider_signal_row("False positives", &provider.false_positive_count.to_string(), "neutral")}
                        {provider_signal_row("Network status", provider.network_status.as_deref().unwrap_or("unknown"), "strong")}
                    </div>
                    <small>{format!("evidence: {}", refs_label(&provider.evidence_refs))}</small>
                </div>
            </div>
        }
    } else {
        html! { <p class="empty">{"No provider graph cockpit available until provider risk profiles are returned."}</p> }
    }
}

fn provider_graph_entity(label: &str, value: &str, position: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("graph-entity", position.to_string(), tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn provider_signal_row(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("provider-signal-row", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[function_component(AuditSamplingPage)]
fn audit_sampling_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let sample_mode = use_state(|| "risk_ranked".to_string());
    let population_definition = use_state(|| "Open high-risk leads for QA sampling".to_string());
    let inclusion_criteria = use_state(|| {
        pretty_json(&json!({
            "min_risk_score": 70,
            "rag": "RED",
            "review_mode": "pre_payment"
        }))
    });
    let sample_size = use_state(|| "5".to_string());
    let reviewer = use_state(|| "qa-reviewer-1".to_string());
    let assignment_queue = use_state(|| "qa-high-risk".to_string());
    let deterministic_seed = use_state(|| "demo-seed-2026".to_string());
    let selected_sample_id = use_state(String::new);
    let samples_state = use_state(|| ApiState::<Vec<AuditSampleRecord>>::Idle);
    let create_state = use_state(|| ApiState::<AuditSampleRecord>::Idle);
    let events_state = use_state(|| ApiState::<Vec<AuditEventRecord>>::Idle);

    let load_samples = {
        let api_key = api_key.clone();
        let samples_state = samples_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let samples_state = samples_state.clone();
            samples_state.set(ApiState::Loading);
            spawn_local(async move {
                samples_state.set(match get_audit_samples(api_key).await {
                    Ok(samples) => ApiState::Ready(samples),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let create_sample = {
        let api_key = api_key.clone();
        let sample_mode = sample_mode.clone();
        let population_definition = population_definition.clone();
        let inclusion_criteria = inclusion_criteria.clone();
        let sample_size = sample_size.clone();
        let reviewer = reviewer.clone();
        let assignment_queue = assignment_queue.clone();
        let deterministic_seed = deterministic_seed.clone();
        let create_state = create_state.clone();
        let load_samples = load_samples.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let payload = audit_sample_payload(
                (*sample_mode).clone(),
                (*population_definition).clone(),
                (*inclusion_criteria).clone(),
                (*sample_size).clone(),
                (*reviewer).clone(),
                (*assignment_queue).clone(),
                (*deterministic_seed).clone(),
            );
            let create_state = create_state.clone();
            let load_samples = load_samples.clone();
            match payload {
                Ok(payload) => {
                    create_state.set(ApiState::Loading);
                    spawn_local(async move {
                        create_state.set(match post_audit_sample(api_key, payload).await {
                            Ok(sample) => {
                                load_samples.emit(());
                                ApiState::Ready(sample)
                            }
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => create_state.set(ApiState::Failed(error)),
            }
        })
    };

    let load_events = {
        let api_key = api_key.clone();
        let selected_sample_id = selected_sample_id.clone();
        let events_state = events_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let selected_sample_id = (*selected_sample_id).clone();
            let events_state = events_state.clone();
            events_state.set(ApiState::Loading);
            spawn_local(async move {
                events_state.set(
                    match get_audit_events_for_sample(api_key, selected_sample_id).await {
                        Ok(events) => ApiState::Ready(events),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    {
        let load_samples = load_samples.clone();
        use_effect_with((), move |_| {
            load_samples.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Audit Sampling"}</h2>
                    <p>{"Create governed QA audit samples, inspect selected leads and outcome distribution, and trace audit_sample.created events by sample ID."}</p>
                </div>
                <span class="status-pill">{"QA Sampling Governance"}</span>
            </div>

            <section class="panel result-stack">
                <h3>{"Audit Sample Control"}</h3>
                <div class="form-grid">
                    {text_input("Sample mode", &sample_mode)}
                    {text_input("Population", &population_definition)}
                    {text_input("Sample size", &sample_size)}
                    {text_input("Reviewer", &reviewer)}
                    {text_input("Assignment queue", &assignment_queue)}
                    {text_input("Deterministic seed", &deterministic_seed)}
                    {text_input("Audit sample ID", &selected_sample_id)}
                </div>
                <label>
                    {"Inclusion criteria JSON"}
                    <textarea
                        class="payload-editor"
                        value={(*inclusion_criteria).clone()}
                        oninput={{
                            let inclusion_criteria = inclusion_criteria.clone();
                            Callback::from(move |event: InputEvent| {
                                inclusion_criteria.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <div class="button-row">
                    <button onclick={create_sample} disabled={matches!(&*create_state, ApiState::Loading)}>
                        {if matches!(&*create_state, ApiState::Loading) { "Creating..." } else { "Create audit sample" }}
                    </button>
                    <button onclick={{
                        let load_samples = load_samples.clone();
                        Callback::from(move |_| load_samples.emit(()))
                    }} disabled={matches!(&*samples_state, ApiState::Loading)}>
                        {if matches!(&*samples_state, ApiState::Loading) { "Refreshing..." } else { "Refresh samples" }}
                    </button>
                    <button onclick={load_events} disabled={matches!(&*events_state, ApiState::Loading)}>
                        {if matches!(&*events_state, ApiState::Loading) { "Loading..." } else { "Load sample audit events" }}
                    </button>
                </div>
                <AuditSampleCreateView state={(*create_state).clone()} />
            </section>

            <AuditSamplesView state={(*samples_state).clone()} />
            <AuditSampleEventsView state={(*events_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct AuditSampleCreateProps {
    state: ApiState<AuditSampleRecord>,
}

#[function_component(AuditSampleCreateView)]
fn audit_sample_create_view(props: &AuditSampleCreateProps) -> Html {
    match &props.state {
        ApiState::Idle => {
            html! { <p class="empty">{"Supported sample modes: risk_ranked, random_control, stratified, post_payment_audit, qa_calibration."}</p> }
        }
        ApiState::Loading => html! { <p>{"Creating audit sample..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(sample) => html! {
            <div class="summary-grid">
                <div><span>{"Sample"}</span><strong>{&sample.sample_id}</strong></div>
                <div><span>{"Mode"}</span><strong>{&sample.sample_mode}</strong></div>
                <div><span>{"Selected Leads"}</span><strong>{sample.selected_leads.len()}</strong></div>
                <div><span>{"Selection"}</span><strong>{&sample.selection_method}</strong></div>
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct AuditSamplesProps {
    state: ApiState<Vec<AuditSampleRecord>>,
}

#[function_component(AuditSamplesView)]
fn audit_samples_view(props: &AuditSamplesProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Audit Sample Inventory"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Load audit samples to inspect sampling coverage."}</p> },
                ApiState::Loading => html! { <p>{"Loading audit samples..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(samples) => html! {
                    if samples.is_empty() {
                        <p class="empty">{"No audit samples returned."}</p>
                    } else {
                        <>
                            {audit_sampling_governance_cockpit(samples)}
                            <div class="factor-card-grid">
                                {for samples.iter().take(10).map(|sample| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{format!("{} / {}", sample.sample_id, sample.sample_mode)}</strong>
                                            <span>{format!("{} / {}", sample.selection_method, sample.assignment_queue)}</span>
                                        </div>
                                        <p>{&sample.population_definition}</p>
                                        <div class="summary-grid">
                                            <div><span>{"Requested"}</span><strong>{sample.sample_size}</strong></div>
                                            <div><span>{"Selected"}</span><strong>{sample.selected_leads.len()}</strong></div>
                                            <div><span>{"Reviewer"}</span><strong>{&sample.reviewer}</strong></div>
                                            <div><span>{"Seed"}</span><strong>{sample.deterministic_seed.as_deref().unwrap_or("none")}</strong></div>
                                            <div><span>{"Created"}</span><strong>{sample.created_at.as_deref().unwrap_or("unknown")}</strong></div>
                                            <div><span>{"Criteria"}</span><strong>{payload_signal_count_label(&sample.inclusion_criteria, "criteria fields")}</strong></div>
                                        </div>
                                        <small>{format!("outcome: {}", payload_signal_count_label(&sample.outcome_distribution, "outcome fields"))}</small>
                                        <details class="data-source-detail governance-detail">
                                            <summary>{"Selected lead detail"}</summary>
                                            if sample.selected_leads.is_empty() {
                                                <p class="empty">{"No selected leads in this sample."}</p>
                                            } else {
                                                <div class="factor-card-grid">
                                                    {for sample.selected_leads.iter().take(6).map(|lead| html! {
                                                        <div class="metric-row">
                                                            <span>{format!("{} / {}", lead.lead_id, lead.claim_id)}</span>
                                                            <strong>{format!("{} / {}", lead.risk_score, lead.rag)}</strong>
                                                            <small>{format!("{} / {} / {}", lead.scheme_family, lead.review_mode, lead.risk_band)}</small>
                                                            <small>{format!("provider: {} / {} / {}", lead.provider_id, lead.provider_type, lead.provider_region)}</small>
                                                            <small>{format!("policy: {} / strata: {} / prior reviewer samples: {}", lead.policy_type, lead.strata_key, lead.prior_reviewer_sample_count)}</small>
                                                            <small>{format!("evidence: {}", refs_count_label(&lead.evidence_refs))}</small>
                                                        </div>
                                                    })}
                                                </div>
                                            }
                                        </details>
                                    </div>
                                })}
                            </div>
                        </>
                    }
                },
            }}
        </section>
    }
}

fn audit_sampling_governance_cockpit(samples: &[AuditSampleRecord]) -> Html {
    let sample = &samples[0];
    let primary_lead = sample
        .selected_leads
        .iter()
        .max_by_key(|lead| lead.risk_score);
    let lead_label = primary_lead
        .map(|lead| format!("{} / {}", lead.claim_id, lead.rag))
        .unwrap_or_else(|| "no selected lead".into());
    let risk_label = primary_lead
        .map(|lead| format!("{} / {}", lead.risk_score, lead.risk_band))
        .unwrap_or_else(|| "pending".into());
    let provider_label = primary_lead
        .map(|lead| format!("{} / {}", lead.provider_id, lead.provider_region))
        .unwrap_or_else(|| "pending".into());
    let evidence_label = primary_lead
        .map(|lead| refs_count_label(&lead.evidence_refs))
        .unwrap_or_else(|| "none".into());
    let seed_label = sample.deterministic_seed.as_deref().unwrap_or("none");
    let created_at = sample.created_at.as_deref().unwrap_or("unknown");

    html! {
        <div class="audit-sampling-cockpit">
            <div class="sampling-brief panel-soft">
                <span class="eyebrow">{"Sampling Governance Map"}</span>
                <strong>{format!("{} / {}", sample.sample_id, sample.sample_mode)}</strong>
                <p>{&sample.population_definition}</p>
                <div class="summary-grid">
                    <div><span>{"Requested"}</span><strong>{sample.sample_size}</strong></div>
                    <div><span>{"Selected leads"}</span><strong>{sample.selected_leads.len()}</strong></div>
                    <div><span>{"Reviewer"}</span><strong>{&sample.reviewer}</strong></div>
                    <div><span>{"Queue"}</span><strong>{&sample.assignment_queue}</strong></div>
                </div>
            </div>

            <div class="sampling-governance-map">
                <div class="sampling-map-title">
                    <div>
                        <span>{"QA audit sample"}</span>
                        <strong>{"Population -> Criteria -> Seed -> Leads -> QA -> Audit trace"}</strong>
                    </div>
                    <span>{format!("created {}", created_at)}</span>
                </div>
                <div class="sampling-link"></div>
                <div class="sampling-link diagonal-a"></div>
                <div class="sampling-link diagonal-b"></div>
                <div class="sampling-core">
                    <span>{"Audit Sampling"}</span>
                    <strong>{&sample.selection_method}</strong>
                </div>
                {sampling_node("Population", &sample.sample_mode, "population")}
                {sampling_node("Inclusion Criteria", &payload_signal_count_label(&sample.inclusion_criteria, "criteria fields"), "criteria")}
                {sampling_node("Deterministic seed", seed_label, "seed")}
                {sampling_node("Selected leads", &lead_label, "leads")}
                {sampling_node("QA queue", &sample.assignment_queue, "queue")}
                {sampling_node("Audit trace", "audit_sample.created", "audit")}
            </div>

            <div class="sampling-trace panel-soft">
                <span class="eyebrow">{"Controlled sample output"}</span>
                <div class="provider-signal-stack">
                    {provider_signal_row("Top selected risk", &risk_label, "danger")}
                    {provider_signal_row("Provider focus", &provider_label, "warning")}
                    {provider_signal_row("Outcome distribution", &payload_signal_count_label(&sample.outcome_distribution, "outcome fields"), "neutral")}
                    {provider_signal_row("Evidence refs", &evidence_label, "strong")}
                </div>
                <details class="data-source-detail governance-detail">
                    <summary>{"Sample output detail"}</summary>
                    <small>{format!("inclusion criteria: {}", payload_keys_label(&sample.inclusion_criteria))}</small>
                    <small>{format!("outcome distribution: {}", payload_keys_label(&sample.outcome_distribution))}</small>
                    <small>{format!("top lead evidence: {}", primary_lead.map(|lead| refs_label(&lead.evidence_refs)).unwrap_or_else(|| "none".into()))}</small>
                </details>
            </div>
        </div>
    }
}

fn sampling_node(label: &str, value: &str, position: &str) -> Html {
    html! {
        <div class={classes!("sampling-node", position.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct AuditSampleEventsProps {
    state: ApiState<Vec<AuditEventRecord>>,
}

#[function_component(AuditSampleEventsView)]
fn audit_sample_events_view(props: &AuditSampleEventsProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Audit Sample Event Trace"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Enter an audit sample ID and load sample audit events."}</p> },
                ApiState::Loading => html! { <p>{"Loading sample audit events..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(events) => html! {
                    if events.is_empty() {
                        <p class="empty">{"No audit events returned for this sample."}</p>
                    } else {
                        <ol class="audit-timeline">
                            {for events.iter().map(|event| html! {
                                <li>
                                    <div>
                                        <strong>{&event.event_type}</strong>
                                        <span>{&event.event_status}</span>
                                    </div>
                                    <p>{&event.summary}</p>
                                    <small>{format!("audit: {} / run: {} / at: {}", event.audit_id, event.run_id, event.created_at.as_deref().unwrap_or("unknown"))}</small>
                                    <small>{format!("payload: {} / evidence: {}", payload_signal_count_label(&event.payload, "payload fields"), refs_count_label(&event.evidence_refs))}</small>
                                    <details class="data-source-detail governance-detail">
                                        <summary>{"Sample audit payload detail"}</summary>
                                        <small>{format!("payload: {}", payload_keys_label(&event.payload))}</small>
                                        <small>{format!("evidence: {}", refs_label(&event.evidence_refs))}</small>
                                    </details>
                                </li>
                            })}
                        </ol>
                    }
                },
            }}
        </section>
    }
}

#[function_component(MedicalReviewPage)]
fn medical_review_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
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
                                    {for items.iter().take(10).map(|item| html! {
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

#[function_component(QaReviewPage)]
fn qa_review_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
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
                                    {for snapshot.queue.iter().take(8).map(qa_queue_card)}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Feedback Closure"}</h3>
                            if snapshot.feedback_items.is_empty() {
                                <p class="empty">{"No QA feedback items returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.feedback_items.iter().take(8).map(qa_feedback_card)}
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

#[function_component(KnowledgeBasePage)]
fn knowledge_base_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let claim_id = use_state(|| "CLM-0287".to_string());
    let diagnosis_code = use_state(|| "J10".to_string());
    let provider_region = use_state(|| "Shanghai".to_string());
    let tags_text = use_state(|| "early_claim, high_amount".to_string());
    let snapshot_state = use_state(|| ApiState::<KnowledgeSnapshot>::Idle);

    let load_knowledge = {
        let api_key = api_key.clone();
        let claim_id = claim_id.clone();
        let diagnosis_code = diagnosis_code.clone();
        let provider_region = provider_region.clone();
        let tags_text = tags_text.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let claim_id = (*claim_id).clone();
            let diagnosis_code = (*diagnosis_code).clone();
            let provider_region = (*provider_region).clone();
            let tags_text = (*tags_text).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(
                    match get_knowledge_snapshot(
                        api_key,
                        claim_id,
                        diagnosis_code,
                        provider_region,
                        tags_text,
                    )
                    .await
                    {
                        Ok(snapshot) => ApiState::Ready(snapshot),
                        Err(error) => ApiState::Failed(error),
                    },
                );
            });
        })
    };

    let search = {
        let load_knowledge = load_knowledge.clone();
        Callback::from(move |_| load_knowledge.emit(()))
    };

    {
        let load_knowledge = load_knowledge.clone();
        use_effect_with((), move |_| {
            load_knowledge.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Knowledge Base"}</h2>
                    <p>{"Search confirmed FWA cases with structured signal overlap while preserving evidence provenance and source traceability."}</p>
                </div>
                <span class="status-pill">{"Confirmed Evidence"}</span>
            </div>

            <section class="panel">
                <h3>{"Similar Case Search"}</h3>
                <div class="form-grid">
                    <label>
                        {"Claim ID"}
                        <input
                            value={(*claim_id).clone()}
                            oninput={{
                                let claim_id = claim_id.clone();
                                Callback::from(move |event: InputEvent| {
                                    claim_id.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Diagnosis code"}
                        <input
                            value={(*diagnosis_code).clone()}
                            oninput={{
                                let diagnosis_code = diagnosis_code.clone();
                                Callback::from(move |event: InputEvent| {
                                    diagnosis_code.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Provider region"}
                        <input
                            value={(*provider_region).clone()}
                            oninput={{
                                let provider_region = provider_region.clone();
                                Callback::from(move |event: InputEvent| {
                                    provider_region.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                    <label>
                        {"Tags"}
                        <input
                            value={(*tags_text).clone()}
                            oninput={{
                                let tags_text = tags_text.clone();
                                Callback::from(move |event: InputEvent| {
                                    tags_text.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={search} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Searching..." } else { "Search similar cases" }}
                    </button>
                </div>
            </section>

            <KnowledgeBaseView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct KnowledgeBaseProps {
    state: ApiState<KnowledgeSnapshot>,
}

#[function_component(KnowledgeBaseView)]
fn knowledge_base_view(props: &KnowledgeBaseProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Search the knowledge base to inspect similar confirmed cases."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading knowledge evidence..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {knowledge_evidence_cockpit(snapshot)}

                        <section class="panel result-stack">
                            <h3>{"Confirmed Knowledge Cases"}</h3>
                            if snapshot.cases.is_empty() {
                                <p class="empty">{"No confirmed knowledge cases returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.cases.iter().take(8).map(|case| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", case.case_id, case.title)}</strong>
                                                <span>{format!("{} / {} / {}", case.fwa_type, case.scheme_family, case.provider_region)}</span>
                                            </div>
                                            <p>{&case.summary}</p>
                                            <div class="summary-grid">
                                                <div><span>{"Diagnosis"}</span><strong>{&case.diagnosis_code}</strong></div>
                                                <div><span>{"Provider Type"}</span><strong>{&case.provider_type}</strong></div>
                                                <div><span>{"Tags"}</span><strong>{refs_label(&case.tags)}</strong></div>
                                            </div>
                                            <small>{format!("outcome: {}", case.outcome)}</small>
                                            <small>{format!("Evidence Provenance: {}", refs_label(&case.evidence_refs))}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Similar Results"}</h3>
                            if snapshot.results.is_empty() {
                                <p class="empty">{"No similar cases matched the current query."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.results.iter().take(8).map(|case| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} / {}", case.case_id, case.title)}</strong>
                                                <span>{format!("{} / {:.2} / {}", case.scheme_family, case.similarity_score, case.retrieval_method)}</span>
                                            </div>
                                            <p>{&case.summary}</p>
                                            <div class="summary-grid">
                                                <div><span>{"Matched Signals"}</span><strong>{refs_label(&case.matched_signals)}</strong></div>
                                                <div><span>{"Outcome"}</span><strong>{&case.outcome}</strong></div>
                                                <div><span>{"Evidence"}</span><strong>{refs_label(&case.evidence_refs)}</strong></div>
                                            </div>
                                            <small>{format!("Evidence Provenance: {}", refs_label(&case.provenance_refs))}</small>
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

fn knowledge_evidence_cockpit(snapshot: &KnowledgeSnapshot) -> Html {
    let selected_result = snapshot.results.first();
    let selected_case = selected_result
        .and_then(|result| {
            snapshot
                .cases
                .iter()
                .find(|case| case.case_id == result.case_id)
        })
        .or_else(|| snapshot.cases.first());
    let case_id = selected_result
        .map(|case| case.case_id.as_str())
        .or_else(|| selected_case.map(|case| case.case_id.as_str()))
        .unwrap_or("no case");
    let title = selected_result
        .map(|case| case.title.as_str())
        .or_else(|| selected_case.map(|case| case.title.as_str()))
        .unwrap_or("knowledge case pending");
    let scheme = selected_result
        .map(|case| case.scheme_family.as_str())
        .or_else(|| selected_case.map(|case| case.scheme_family.as_str()))
        .unwrap_or("scheme pending");
    let outcome = selected_result
        .map(|case| case.outcome.as_str())
        .or_else(|| selected_case.map(|case| case.outcome.as_str()))
        .unwrap_or("outcome pending");
    let matched_signal = selected_result
        .and_then(|case| case.matched_signals.first().map(String::as_str))
        .or_else(|| selected_case.and_then(|case| case.tags.first().map(String::as_str)))
        .unwrap_or("signal pending");
    let provenance_ref = selected_result
        .and_then(|case| case.provenance_refs.first().map(String::as_str))
        .or_else(|| selected_case.and_then(|case| case.evidence_refs.first().map(String::as_str)))
        .unwrap_or("provenance pending");
    let evidence_ref = selected_result
        .and_then(|case| case.evidence_refs.first().map(String::as_str))
        .or_else(|| selected_case.and_then(|case| case.evidence_refs.first().map(String::as_str)))
        .unwrap_or("evidence pending");
    let retrieval_method = selected_result
        .map(|case| case.retrieval_method.as_str())
        .unwrap_or("structured catalog");
    let similarity = selected_result
        .map(|case| format!("{:.2}", case.similarity_score))
        .unwrap_or_else(|| "n/a".into());
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Knowledge graph match"}</h3>
                    <p>{"Similar confirmed FWA cases are shown as evidence-backed references for reviewer context, not as automated adjudication."}</p>
                </div>
                <span class="status-token strong">{"Evidence provenance path"}</span>
            </div>
            <div class="knowledge-cockpit">
                <aside class="case-brief knowledge-brief">
                    <span>{"Selected knowledge case"}</span>
                    <strong>{case_id}</strong>
                    <dl>
                        <div><dt>{"Scheme"}</dt><dd>{scheme}</dd></div>
                        <div><dt>{"Similarity"}</dt><dd>{similarity}</dd></div>
                        <div><dt>{"Retrieval"}</dt><dd>{retrieval_method}</dd></div>
                        <div><dt>{"Outcome"}</dt><dd>{outcome}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("confirmed {}", snapshot.cases.len())}</span>
                        <span>{format!("matches {}", snapshot.results.len())}</span>
                        <span>{format!("signals {}", selected_result.map(|case| case.matched_signals.len()).unwrap_or(0))}</span>
                    </div>
                </aside>

                <div class="knowledge-map">
                    <div class="knowledge-map-title">
                        <span>{"Structured + semantic retrieval"}</span>
                        <strong>{title}</strong>
                    </div>
                    <div class="knowledge-link horizontal"></div>
                    <div class="knowledge-link diagonal-a"></div>
                    <div class="knowledge-link diagonal-b"></div>
                    <div class="knowledge-core">
                        <span>{"Confirmed case"}</span>
                        <strong>{case_id}</strong>
                    </div>
                    <div class="knowledge-node signal">
                        <span>{"Matched signal"}</span>
                        <strong>{matched_signal}</strong>
                    </div>
                    <div class="knowledge-node scheme">
                        <span>{"Scheme family"}</span>
                        <strong>{scheme}</strong>
                    </div>
                    <div class="knowledge-node provenance">
                        <span>{"Provenance"}</span>
                        <strong>{provenance_ref}</strong>
                    </div>
                    <div class="knowledge-node evidence">
                        <span>{"Evidence"}</span>
                        <strong>{evidence_ref}</strong>
                    </div>
                </div>

                <aside class="case-timeline knowledge-trace">
                    <h4>{"Source trace"}</h4>
                    {timeline_item("Catalog", &format!("{} confirmed cases", snapshot.cases.len()), "done")}
                    {timeline_item("Search", retrieval_method, "ready")}
                    {timeline_item("Match", matched_signal, "done")}
                    {timeline_item("Review", "human reviewer consumes context", "review")}
                </aside>
            </div>
        </section>
    }
}

#[function_component(AgentInvestigatorPage)]
fn agent_investigator_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let claim_id = use_state(|| "CLM-0287".to_string());
    let risk_score = use_state(|| "87".to_string());
    let rag = use_state(|| "RED".to_string());
    let scheme_family = use_state(|| "provider_peer_outlier".to_string());
    let top_reasons = use_state(|| {
        "金额高于同病种同地区 P99, 保单生效后短期高额理赔, Provider 高价项目比例异常".to_string()
    });
    let diagnosis_code = use_state(|| "J10".to_string());
    let provider_region = use_state(|| "Shanghai".to_string());
    let tags = use_state(|| "provider_pattern, high_amount, peer_deviation".to_string());
    let investigation_state = use_state(|| ApiState::<AgentInvestigationResponse>::Idle);
    let runs_state = use_state(|| ApiState::<Vec<AgentRunRecord>>::Idle);

    let load_runs = {
        let api_key = api_key.clone();
        let runs_state = runs_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let runs_state = runs_state.clone();
            runs_state.set(ApiState::Loading);
            spawn_local(async move {
                runs_state.set(match get_agent_runs(api_key).await {
                    Ok(runs) => ApiState::Ready(runs),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let investigate = {
        let api_key = api_key.clone();
        let claim_id = claim_id.clone();
        let risk_score = risk_score.clone();
        let rag = rag.clone();
        let scheme_family = scheme_family.clone();
        let top_reasons = top_reasons.clone();
        let diagnosis_code = diagnosis_code.clone();
        let provider_region = provider_region.clone();
        let tags = tags.clone();
        let investigation_state = investigation_state.clone();
        let load_runs = load_runs.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let payload = agent_investigation_payload(
                (*claim_id).clone(),
                (*risk_score).clone(),
                (*rag).clone(),
                (*scheme_family).clone(),
                (*top_reasons).clone(),
                (*diagnosis_code).clone(),
                (*provider_region).clone(),
                (*tags).clone(),
            );
            let investigation_state = investigation_state.clone();
            let load_runs = load_runs.clone();
            match payload {
                Ok(payload) => {
                    investigation_state.set(ApiState::Loading);
                    spawn_local(async move {
                        investigation_state.set(
                            match post_agent_investigation(api_key, payload).await {
                                Ok(response) => {
                                    load_runs.emit(());
                                    ApiState::Ready(response)
                                }
                                Err(error) => ApiState::Failed(error),
                            },
                        );
                    });
                }
                Err(error) => investigation_state.set(ApiState::Failed(error)),
            }
        })
    };

    {
        let load_runs = load_runs.clone();
        use_effect_with((), move |_| {
            load_runs.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Agent Investigator"}</h2>
                    <p>{"Generate an assistive-only investigation package from governed risk signals and inspect the Agent run evidence trail."}</p>
                </div>
                <span class="status-pill">{"Assistive Investigation"}</span>
            </div>

            {agent_investigator_blueprint()}

            <section class="panel result-stack">
                <h3>{"Investigation Request"}</h3>
                <div class="form-grid">
                    {text_input("Claim ID", &claim_id)}
                    {text_input("Risk score", &risk_score)}
                    {text_input("RAG", &rag)}
                    {text_input("Scheme family", &scheme_family)}
                    {text_input("Diagnosis code", &diagnosis_code)}
                    {text_input("Provider region", &provider_region)}
                    {text_input("Tags", &tags)}
                </div>
                <label>
                    {"Top reasons"}
                    <textarea
                        value={(*top_reasons).clone()}
                        oninput={{
                            let top_reasons = top_reasons.clone();
                            Callback::from(move |event: InputEvent| {
                                top_reasons.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                            })
                        }}
                    />
                </label>
                <div class="button-row">
                    <button onclick={investigate} disabled={matches!(&*investigation_state, ApiState::Loading)}>
                        {if matches!(&*investigation_state, ApiState::Loading) { "Generating..." } else { "Generate investigation package" }}
                    </button>
                    <button onclick={{
                        let load_runs = load_runs.clone();
                        Callback::from(move |_| load_runs.emit(()))
                    }} disabled={matches!(&*runs_state, ApiState::Loading)}>
                        {if matches!(&*runs_state, ApiState::Loading) { "Refreshing..." } else { "Refresh Agent runs" }}
                    </button>
                </div>
            </section>

            <AgentInvestigationView state={(*investigation_state).clone()} />
            <AgentRunsView state={(*runs_state).clone()} />
        </section>
    }
}

fn agent_investigator_blueprint() -> Html {
    html! {
        <section class="agent-blueprint-cockpit" aria-label="Agent investigation blueprint">
            <aside class="agent-blueprint-brief">
                <span>{"Agent investigation blueprint"}</span>
                <strong>{"assistive, evidence-bound, human-gated"}</strong>
                <dl>
                    <div><dt>{"Input"}</dt><dd>{"risk signals + top reasons"}</dd></div>
                    <div><dt>{"Tools"}</dt><dd>{"claims, rules, models, KB, documents"}</dd></div>
                    <div><dt>{"Output"}</dt><dd>{"risk summary + checklist + QA draft"}</dd></div>
                    <div><dt>{"Boundary"}</dt><dd>{"no auto denial"}</dd></div>
                </dl>
            </aside>
            <div class="agent-blueprint-map">
                <div class="agent-blueprint-rail"></div>
                <div class="agent-blueprint-node risk">
                    <span>{"Risk context"}</span>
                    <strong>{"risk signal findings"}</strong>
                    <small>{"score, RAG, reasons"}</small>
                </div>
                <div class="agent-blueprint-node evidence">
                    <span>{"Evidence collector"}</span>
                    <strong>{"source refs"}</strong>
                    <small>{"claim, rule, model, document"}</small>
                </div>
                <div class="agent-blueprint-core">
                    <span>{"Agent"}</span>
                    <strong>{"case package"}</strong>
                </div>
                <div class="agent-blueprint-node kb">
                    <span>{"Knowledge base"}</span>
                    <strong>{"similar cases"}</strong>
                    <small>{"provenance required"}</small>
                </div>
                <div class="agent-blueprint-node qa">
                    <span>{"QA draft"}</span>
                    <strong>{"review opinion"}</strong>
                    <small>{"human editable"}</small>
                </div>
                <div class="agent-blueprint-node gate">
                    <span>{"Human gate"}</span>
                    <strong>{"review only"}</strong>
                    <small>{"decision stays outside Agent"}</small>
                </div>
            </div>
            <aside class="agent-blueprint-guardrail">
                <span>{"Governance locks"}</span>
                <div class="tag-grid compact-tags">
                    <span>{"Tool allowlist"}</span>
                    <span>{"PII masking"}</span>
                    <span>{"Evidence refs"}</span>
                    <span>{"Audit events"}</span>
                    <span>{"Timeouts"}</span>
                    <span>{"Human approval"}</span>
                </div>
                <p>{"The Agent prepares investigation material. It cannot deny, approve, publish rules, or bypass audit."}</p>
            </aside>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct AgentInvestigationProps {
    state: ApiState<AgentInvestigationResponse>,
}

#[function_component(AgentInvestigationView)]
fn agent_investigation_view(props: &AgentInvestigationProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Investigation Package"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Generate an investigation package to inspect findings, checklist, similar cases, QA draft, and evidence sufficiency."}</p> },
                ApiState::Loading => html! { <p>{"Generating investigation package..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        {agent_investigation_cockpit(response)}
                        <div class="score-hero">
                            <div><span>{"Agent Run"}</span><strong>{&response.agent_run_id}</strong></div>
                            <div><span>{"Boundary"}</span><strong>{business_label(&response.decision_boundary)}</strong></div>
                            <div><span>{"Evidence"}</span><strong>{response.evidence_refs.len()}</strong></div>
                        </div>
                        <p>{&response.risk_summary}</p>
                        <div class="summary-grid">
                            <div><span>{"Evidence Status"}</span><strong>{business_label(&response.evidence_sufficiency.status)}</strong></div>
                            <div><span>{"Scheme"}</span><strong>{business_label(&response.evidence_sufficiency.scheme_family)}</strong></div>
                            <div><span>{"Present"}</span><strong>{response.evidence_sufficiency.present_evidence.len()}</strong></div>
                            <div><span>{"Missing"}</span><strong>{response.evidence_sufficiency.missing_evidence.len()}</strong></div>
                        </div>

                        <h4>{"Findings"}</h4>
                        <div class="factor-card-grid">
                            {for response.findings.iter().map(|finding| html! {
                                <div class="metric-row">
                                    <span>{&finding.finding}</span>
                                    <strong>{refs_count_label(&finding.evidence_refs)}</strong>
                                </div>
                            })}
                        </div>

                        <h4>{"Investigation Checklist"}</h4>
                        <ul class="result-list">
                            {for response.investigation_checklist.iter().map(|item| html! { <li>{item}</li> })}
                        </ul>

                        <h4>{"Similar Cases"}</h4>
                        if response.similar_cases.is_empty() {
                            <p class="empty">{"No similar cases returned."}</p>
                        } else {
                            <div class="factor-card-grid">
                                {for response.similar_cases.iter().map(|case| html! {
                                    <div class="metric-row">
                                        <span>{&case.case_id}</span>
                                        <strong>{format!("{:.2}", case.similarity_score)}</strong>
                                        <small>{format!("signals: {}", refs_count_label(&case.matched_signals))}</small>
                                        <small>{format!("provenance: {}", refs_count_label(&case.provenance_refs))}</small>
                                    </div>
                                })}
                            </div>
                        }

                        <h4>{"QA Opinion Draft"}</h4>
                        <p>{&response.qa_opinion_draft}</p>

                        <h4>{"Evidence Buckets"}</h4>
                        <div class="summary-grid">
                            <div><span>{"Claim"}</span><strong>{response.evidence_refs_by_type.claim.len()}</strong></div>
                            <div><span>{"Rule"}</span><strong>{response.evidence_refs_by_type.rule.len()}</strong></div>
                            <div><span>{"Model"}</span><strong>{response.evidence_refs_by_type.model.len()}</strong></div>
                            <div><span>{"Anomaly"}</span><strong>{response.evidence_refs_by_type.anomaly.len()}</strong></div>
                            <div><span>{"Document"}</span><strong>{response.evidence_refs_by_type.document.len()}</strong></div>
                            <div><span>{"Similar Case"}</span><strong>{response.evidence_refs_by_type.similar_case.len()}</strong></div>
                        </div>
                        <small>{format!("evidence: {}", refs_count_label(&response.evidence_refs))}</small>
                        <details class="data-source-detail governance-detail">
                            <summary>{"Investigation evidence detail"}</summary>
                            <small>{refs_label(&response.evidence_refs)}</small>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

fn agent_investigation_cockpit(response: &AgentInvestigationResponse) -> Html {
    let top_finding = response
        .findings
        .first()
        .map(|finding| finding.finding.as_str())
        .unwrap_or("finding pending");
    let similar_case = response
        .similar_cases
        .first()
        .map(|case| case.case_id.as_str())
        .unwrap_or("no similar case");
    let missing_evidence = response
        .evidence_sufficiency
        .missing_evidence
        .first()
        .map(String::as_str)
        .unwrap_or("none");
    html! {
        <div class="agent-cockpit">
            <aside class="case-brief agent-brief">
                <span>{"Agent investigation command"}</span>
                <strong>{&response.agent_run_id}</strong>
                <dl>
                    <div><dt>{"Boundary"}</dt><dd>{business_label(&response.decision_boundary)}</dd></div>
                    <div><dt>{"Scheme"}</dt><dd>{business_label(&response.evidence_sufficiency.scheme_family)}</dd></div>
                    <div><dt>{"Evidence"}</dt><dd>{response.evidence_refs.len()}</dd></div>
                    <div><dt>{"Status"}</dt><dd>{business_label(&response.evidence_sufficiency.status)}</dd></div>
                </dl>
                <div class="tag-grid compact-tags">
                    <span>{format!("findings {}", response.findings.len())}</span>
                    <span>{format!("checklist {}", response.investigation_checklist.len())}</span>
                    <span>{format!("similar {}", response.similar_cases.len())}</span>
                </div>
            </aside>

            <div class="agent-evidence-map">
                <div class="agent-map-title">
                    <span>{"Agent evidence orchestration"}</span>
                    <strong>{"assistive package only"}</strong>
                </div>
                <div class="agent-map-link horizontal"></div>
                <div class="agent-map-link diagonal-a"></div>
                <div class="agent-map-link diagonal-b"></div>
                <div class="agent-node risk">
                    <span>{"7-layer risk"}</span>
                    <strong>{top_finding}</strong>
                </div>
                <div class="agent-node evidence">
                    <span>{"Evidence buckets"}</span>
                    <strong>{format!(
                        "claim {} / rule {} / model {}",
                        response.evidence_refs_by_type.claim.len(),
                        response.evidence_refs_by_type.rule.len(),
                        response.evidence_refs_by_type.model.len()
                    )}</strong>
                </div>
                <div class="agent-node kb">
                    <span>{"Knowledge memory"}</span>
                    <strong>{similar_case}</strong>
                </div>
                <div class="agent-node qa">
                    <span>{"QA draft"}</span>
                    <strong>{&response.qa_opinion_draft}</strong>
                </div>
                <div class="agent-node human">
                    <span>{"Human gate"}</span>
                    <strong>{missing_evidence}</strong>
                </div>
                <div class="agent-core">
                    <span>{"Agent"}</span>
                    <strong>{"evidence pack"}</strong>
                </div>
            </div>

            <aside class="case-timeline agent-guardrail">
                <h4>{"Guardrail path"}</h4>
                {timeline_item("Input", "risk output + evidence refs", "done")}
                {timeline_item("Tools", "allowlisted retrieval", "done")}
                {timeline_item("Output", "structured summary", "ready")}
                {timeline_item("Action", "human approval required", "review")}
            </aside>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct AgentRunsProps {
    state: ApiState<Vec<AgentRunRecord>>,
}

#[function_component(AgentRunsView)]
fn agent_runs_view(props: &AgentRunsProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Agent Run Evidence Trail"}</h3>
            <p class="empty">{"Assistive Boundary: Agent outputs support investigation and require human approval before high-impact downstream action."}</p>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Refresh Agent runs to inspect evidence trail."}</p> },
                ApiState::Loading => html! { <p>{"Loading Agent runs..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(runs) => html! {
                    if runs.is_empty() {
                        <p class="empty">{"No Agent runs returned."}</p>
                    } else {
                        <>
                            {agent_run_governance_cockpit(&runs[0])}
                            <div class="factor-card-grid">
                                {for runs.iter().take(8).map(|run| html! {
                                    <div class="factor-card">
                                        <div>
                                            <strong>{&run.claim_id}</strong>
                                            <span>{format!("{} / {}", business_label(&run.status), business_label(&run.decision_boundary))}</span>
                                        </div>
                                        <div class="summary-grid">
                                            <div><span>{"Steps"}</span><strong>{run.steps.len()}</strong></div>
                                            <div><span>{"Tool Calls"}</span><strong>{run.tool_calls.len()}</strong></div>
                                            <div><span>{"Policy Checks"}</span><strong>{run.policy_checks.len()}</strong></div>
                                            <div><span>{"Approvals"}</span><strong>{run.approvals.len()}</strong></div>
                                            <div><span>{"Evidence"}</span><strong>{refs_count_label(&run.evidence_refs)}</strong></div>
                                        </div>
                                        <small>{format!("created: {} / completed: {}", run.created_at.as_deref().unwrap_or("unknown"), run.completed_at.as_deref().unwrap_or("pending"))}</small>
                                        <small>{format!("approval: {}", approval_count_label(&run.approvals))}</small>
                                        <details class="data-source-detail governance-detail">
                                            <summary>{"Agent run evidence detail"}</summary>
                                            <small>{format!("agent run: {}", run.agent_run_id)}</small>
                                            <small>{format!("evidence: {}", refs_label(&run.evidence_refs))}</small>
                                            <small>{format!("approval: {}", approval_summary(&run.approvals))}</small>
                                        </details>
                                    </div>
                                })}
                            </div>
                        </>
                    }
                },
            }}
        </section>
    }
}

fn agent_run_governance_cockpit(run: &AgentRunRecord) -> Html {
    let policy_label = run
        .policy_checks
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no policy check".into());
    let tool_label = run
        .tool_calls
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no tool call".into());
    let result_label = run
        .tool_results
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no tool result".into());
    let context_label = run
        .context_snapshots
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no context snapshot".into());
    let step_label = run
        .steps
        .first()
        .map(compact_payload_label)
        .unwrap_or_else(|| "no step".into());
    let approval_label = if run.approvals.is_empty() {
        "no approval record".into()
    } else {
        format!("{} approval records", run.approvals.len())
    };
    let evidence_label = format!("{} evidence refs", run.evidence_refs.len());
    let output_label = compact_payload_label(&run.output_json);

    html! {
        <div class="agent-run-cockpit">
            <aside class="agent-run-brief">
                <span class="eyebrow">{"Agent Run Governance Map"}</span>
                <strong>{&run.agent_run_id}</strong>
                <dl>
                    <div><dt>{"Claim"}</dt><dd>{&run.claim_id}</dd></div>
                    <div><dt>{"Status"}</dt><dd>{business_label(&run.status)}</dd></div>
                    <div><dt>{"Boundary"}</dt><dd>{business_label(&run.decision_boundary)}</dd></div>
                    <div><dt>{"Evidence"}</dt><dd>{run.evidence_refs.len()}</dd></div>
                </dl>
            </aside>

            <div class="agent-run-map">
                <div class="agent-run-map-title">
                    <span>{"Governed agent execution"}</span>
                    <strong>{"context -> policy check -> tool allowlist -> result -> human approval -> audit"}</strong>
                </div>
                <div class="agent-run-link"></div>
                <div class="agent-run-link diagonal-a"></div>
                <div class="agent-run-link diagonal-b"></div>
                <div class="agent-run-core">
                    <span>{"Assistive Only"}</span>
                    <strong>{business_label(&run.status)}</strong>
                </div>
                {agent_run_node("Context snapshot", &context_label, "context")}
                {agent_run_node("Policy check", &policy_label, "policy")}
                {agent_run_node("Tool allowlist", &tool_label, "tool")}
                {agent_run_node("Tool result", &result_label, "result")}
                {agent_run_node("Human approval gate", &approval_label, "approval")}
                {agent_run_node("Evidence audit trail", &evidence_label, "audit")}
            </div>

            <aside class="agent-run-trace">
                <span class="eyebrow">{"Execution counters"}</span>
                <div class="provider-signal-stack">
                    {provider_signal_row("Steps", &format!("{} / {}", run.steps.len(), step_label), "neutral")}
                    {provider_signal_row("Policy checks", &run.policy_checks.len().to_string(), "strong")}
                    {provider_signal_row("Tool calls", &run.tool_calls.len().to_string(), "warning")}
                    {provider_signal_row("Approvals", &run.approvals.len().to_string(), "danger")}
                    {provider_signal_row("Output JSON", &output_label, "neutral")}
                </div>
            </aside>
        </div>
    }
}

fn agent_run_node(label: &str, value: &str, position: &str) -> Html {
    html! {
        <div class={classes!("agent-run-node", position.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

#[function_component(GovernancePage)]
fn governance_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let event_group = use_state(|| "governance".to_string());
    let snapshot_state = use_state(|| ApiState::<GovernanceSnapshot>::Idle);

    let load_governance = {
        let api_key = api_key.clone();
        let event_group = event_group.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let event_group = (*event_group).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_governance_snapshot(api_key, event_group).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_governance = load_governance.clone();
        Callback::from(move |_| load_governance.emit(()))
    };

    {
        let load_governance = load_governance.clone();
        use_effect_with((), move |_| {
            load_governance.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Governance"}</h2>
                    <p>{"Review audit events, API call records, and assistive Agent run logs with evidence references before operational approval."}</p>
                </div>
                <span class="status-pill">{"Audit Coverage"}</span>
            </div>

            <section class="panel">
                <h3>{"Governance Source"}</h3>
                <div class="form-grid">
                    <label>
                        {"Audit event group"}
                        <input
                            value={(*event_group).clone()}
                            oninput={{
                                let event_group = event_group.clone();
                                Callback::from(move |event: InputEvent| {
                                    event_group.set(event.target_unchecked_into::<HtmlInputElement>().value());
                                })
                            }}
                        />
                    </label>
                </div>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh governance" }}
                    </button>
                </div>
            </section>

            <GovernanceView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct GovernanceProps {
    state: ApiState<GovernanceSnapshot>,
}

#[function_component(GovernanceView)]
fn governance_view(props: &GovernanceProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load governance logs to inspect audit and Agent controls."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading governance records..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        {governance_control_tower(snapshot)}

                        <section class="panel result-stack">
                            <h3>{"Pilot Security Readiness"}</h3>
                            {pilot_readiness_cockpit(&snapshot.health)}
                            <div class="score-hero">
                                <div><span>{"Pilot Gate"}</span><strong>{&snapshot.health.pilot_readiness.status}</strong></div>
                                <div><span>{"Customer Pilot"}</span><strong>{if snapshot.health.pilot_readiness.ready_for_customer_pilot { "ready" } else { "blocked" }}</strong></div>
                                <div><span>{"Ready Checks"}</span><strong>{format!("{} / {}", snapshot.health.pilot_readiness.ready_check_count, snapshot.health.pilot_readiness.required_check_count)}</strong></div>
                                <div><span>{"Blocking Checks"}</span><strong>{snapshot.health.pilot_readiness.blocking_check_count}</strong></div>
                                <div><span>{"Health Checks"}</span><strong>{snapshot.health.checks.len()}</strong></div>
                                <div><span>{"Service"}</span><strong>{format!("{} {}", snapshot.health.service, snapshot.health.version)}</strong></div>
                            </div>
                            if snapshot.health.pilot_readiness.blocking_checks.is_empty() {
                                <p class="empty">{"All pilot configuration gates are configured for this environment."}</p>
                            } else {
                                <>
                                    <div class="factor-card-grid">
                                        {for snapshot.health.pilot_readiness.blocking_checks.iter().take(3).map(|check| html! {
                                            <div class="factor-card">
                                                <div>
                                                    <strong>{&check.name}</strong>
                                                    <span>{&check.status}</span>
                                                </div>
                                                <small>{format!("runtime: {}", check.runtime_kind.as_deref().unwrap_or("n/a"))}</small>
                                                if let Some(remediation) = &check.remediation {
                                                    <small>{remediation}</small>
                                                }
                                            </div>
                                        })}
                                    </div>
                                    <details class="data-source-detail governance-detail">
                                        <summary>{format!("All blocking check detail: {} checks", snapshot.health.pilot_readiness.blocking_checks.len())}</summary>
                                        <div class="governance-check-list">
                                            {for snapshot.health.pilot_readiness.blocking_checks.iter().map(|check| html! {
                                                <div>
                                                    <strong>{&check.name}</strong>
                                                    <span class={classes!("status-token", status_tone(&check.status))}>{&check.status}</span>
                                                    <small>{format!("runtime: {}", check.runtime_kind.as_deref().unwrap_or("n/a"))}</small>
                                                    if let Some(remediation) = &check.remediation {
                                                        <small>{remediation}</small>
                                                    }
                                                </div>
                                            })}
                                        </div>
                                    </details>
                                </>
                            }
                            {pilot_configuration_summary(&snapshot.health)}
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Audit Event Log"}</h3>
                            <div class="score-hero">
                                <div><span>{"Audit Events"}</span><strong>{snapshot.audit_events.len()}</strong></div>
                                <div><span>{"API Call Records"}</span><strong>{snapshot.api_calls.len()}</strong></div>
                                <div><span>{"Agent Run Logs"}</span><strong>{snapshot.agent_runs.len()}</strong></div>
                            </div>
                            if snapshot.audit_events.is_empty() {
                                <p class="empty">{"No audit events returned for this filter."}</p>
                            } else {
                                <ol class="audit-timeline">
                                    {for snapshot.audit_events.iter().take(8).map(|event| html! {
                                        <li>
                                            <div>
                                                <strong>{&event.event_type}</strong>
                                                <span>{&event.event_status}</span>
                                            </div>
                                            <p>{&event.summary}</p>
                                            <small>{format!("audit: {} / run: {} / at: {}", event.audit_id, event.run_id, event.created_at.as_deref().unwrap_or("unknown"))}</small>
                                            <small>{format!("evidence: {}", refs_count_label(&event.evidence_refs))}</small>
                                            <details class="inline-detail data-source-detail governance-detail">
                                                <summary>{"Payload trace detail"}</summary>
                                                <small>{payload_keys_label(&event.payload)}</small>
                                            </details>
                                        </li>
                                    })}
                                </ol>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"API Call Records"}</h3>
                            if snapshot.api_calls.is_empty() {
                                <p class="empty">{"No API call records returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.api_calls.iter().take(8).map(|call| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{format!("{} {}", call.method, call.endpoint)}</strong>
                                                <span>{format!("{} / {} / {}", call.status_code, call.result, call.source_system)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Claim"}</span><strong>{empty_label(&call.claim_id)}</strong></div>
                                                <div><span>{"Event"}</span><strong>{&call.event_type}</strong></div>
                                                <div><span>{"Result"}</span><strong>{&call.result}</strong></div>
                                                <div><span>{"Evidence"}</span><strong>{refs_count_label(&call.evidence_refs)}</strong></div>
                                                <div><span>{"Observed"}</span><strong>{call.observed_at.as_deref().unwrap_or("unknown")}</strong></div>
                                            </div>
                                            <details class="data-source-detail governance-detail">
                                                <summary>{"API evidence detail"}</summary>
                                                <small>{format!("call: {}", call.call_id)}</small>
                                                <small>{format!("run: {}", call.run_id)}</small>
                                                <small>{format!("audit: {}", call.audit_id)}</small>
                                                <small>{format!("idempotency: {}", call.idempotency_key.as_deref().unwrap_or("none"))}</small>
                                                <small>{format!("evidence: {}", refs_label(&call.evidence_refs))}</small>
                                            </details>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <h3>{"Agent Run Logs"}</h3>
                            <p class="empty">{"Assistive Boundary: Agent outputs remain investigation support and require human approval for high-impact actions."}</p>
                            if snapshot.agent_runs.is_empty() {
                                <p class="empty">{"No Agent run logs returned."}</p>
                            } else {
                                <div class="factor-card-grid">
                                    {for snapshot.agent_runs.iter().take(8).map(|run| html! {
                                        <div class="factor-card">
                                            <div>
                                                <strong>{&run.claim_id}</strong>
                                                <span>{format!("{} / {}", run.status, run.decision_boundary)}</span>
                                            </div>
                                            <div class="summary-grid">
                                                <div><span>{"Steps"}</span><strong>{run.steps.len()}</strong></div>
                                                <div><span>{"Tool Calls"}</span><strong>{run.tool_calls.len()}</strong></div>
                                                <div><span>{"Tool Results"}</span><strong>{run.tool_results.len()}</strong></div>
                                                <div><span>{"Policy Checks"}</span><strong>{run.policy_checks.len()}</strong></div>
                                                <div><span>{"Context Snapshots"}</span><strong>{run.context_snapshots.len()}</strong></div>
                                                <div><span>{"Approvals"}</span><strong>{run.approvals.len()}</strong></div>
                                            </div>
                                            <small>{format!("created: {} / completed: {}", run.created_at.as_deref().unwrap_or("unknown"), run.completed_at.as_deref().unwrap_or("pending"))}</small>
                                            <small>{format!("evidence: {}", refs_count_label(&run.evidence_refs))}</small>
                                            <small>{format!("approval: {}", approval_summary(&run.approvals))}</small>
                                            <details class="data-source-detail governance-detail">
                                                <summary>{"Agent run detail"}</summary>
                                                <small>{format!("agent run: {}", run.agent_run_id)}</small>
                                                <small>{format!("output: {}", payload_signal_count_label(&run.output_json, "output fields"))}</small>
                                                <small>{format!("evidence: {}", refs_label(&run.evidence_refs))}</small>
                                            </details>
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

fn pilot_readiness_cockpit(health: &HealthResponse) -> Html {
    let readiness = &health.pilot_readiness;
    let ready_count = readiness.ready_check_count;
    let required_count = readiness.required_check_count;
    let blocked_count = readiness.blocking_check_count;
    let ready_pct = if required_count == 0 {
        0
    } else {
        ((ready_count * 100) / required_count).min(100)
    };
    let blocker_label = readiness
        .blocking_checks
        .first()
        .map(|check| check.name.as_str())
        .unwrap_or("no active blocker");
    let ready_label = readiness
        .ready_checks
        .first()
        .map(|check| check.name.as_str())
        .unwrap_or("no ready checks");
    let required_label = readiness
        .required_check_names
        .first()
        .map(String::as_str)
        .unwrap_or("required checks not reported");
    let customer_pilot_label = if readiness.ready_for_customer_pilot {
        "ready for customer pilot"
    } else {
        "blocked for customer pilot"
    };

    html! {
        <div class="pilot-readiness-cockpit">
            <aside class="pilot-readiness-brief">
                <span class="eyebrow">{"Pilot gate status"}</span>
                <strong>{customer_pilot_label}</strong>
                <dl>
                    <div><dt>{"Ready"}</dt><dd>{format!("{ready_count} / {required_count}")}</dd></div>
                    <div><dt>{"Blocked"}</dt><dd>{blocked_count}</dd></div>
                    <div><dt>{"Decision"}</dt><dd>{&readiness.status}</dd></div>
                    <div><dt>{"Health"}</dt><dd>{health.checks.len()}</dd></div>
                    <div><dt>{"Service"}</dt><dd>{format!("{} {}", health.service, health.version)}</dd></div>
                </dl>
            </aside>

            <div class="pilot-readiness-map">
                <div class="readiness-track"></div>
                <div class="readiness-progress" style={format!("width: {ready_pct}%;")}></div>
                {readiness_node("Required", &required_count.to_string(), required_label, "required")}
                {readiness_node("Ready", &format!("{ready_pct}%"), ready_label, "ready")}
                {readiness_node("Blocked", &blocked_count.to_string(), blocker_label, "blocked")}
                {readiness_node("Decision", customer_pilot_label, "worker check-pilot-readiness", "decision")}
            </div>

            <aside class="pilot-readiness-actions">
                <span class="eyebrow">{"Next blocker"}</span>
                <strong>{
                    readiness
                        .blocking_check_names
                        .first()
                        .map(String::as_str)
                        .unwrap_or(blocker_label)
                }</strong>
                if let Some(remediation) = readiness.remediation_summary.first() {
                    <small>{remediation}</small>
                } else if let Some(check) = readiness.blocking_checks.first() {
                    <small>{check.remediation.as_deref().unwrap_or("no remediation returned")}</small>
                } else {
                    <small>{"Pilot readiness has no blocking configuration checks."}</small>
                }
            </aside>
        </div>
    }
}

fn pilot_configuration_summary(health: &HealthResponse) -> Html {
    let configuration_checks = health
        .checks
        .iter()
        .filter(|check| check.name.ends_with("_configuration"))
        .collect::<Vec<_>>();
    let configured_count = configuration_checks
        .iter()
        .filter(|check| status_tone(&check.status) == "success")
        .count();
    let needs_setup_count = configuration_checks.len().saturating_sub(configured_count);

    html! {
        <>
            <div class="summary-grid">
                <div><span>{"Configuration checks"}</span><strong>{configuration_checks.len()}</strong></div>
                <div><span>{"Configured"}</span><strong>{configured_count}</strong></div>
                <div><span>{"Needs setup"}</span><strong>{needs_setup_count}</strong></div>
            </div>
            <details class="data-source-detail governance-detail">
                <summary>{"Configuration check detail"}</summary>
                <div class="governance-check-list">
                    {for configuration_checks.iter().map(|check| html! {
                        <div>
                            <strong>{&check.name}</strong>
                            <span class={classes!("status-token", status_tone(&check.status))}>{&check.status}</span>
                            if let Some(remediation) = &check.remediation {
                                <small>{remediation}</small>
                            }
                        </div>
                    })}
                </div>
            </details>
        </>
    }
}

fn readiness_node(label: &str, value: &str, detail: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("readiness-node", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{detail}</small>
        </div>
    }
}

fn governance_control_tower(snapshot: &GovernanceSnapshot) -> Html {
    let pilot_status = snapshot.health.pilot_readiness.status.as_str();
    let first_blocker = snapshot
        .health
        .pilot_readiness
        .blocking_checks
        .first()
        .map(|check| check.name.as_str())
        .unwrap_or("no blocking checks");
    let first_audit = snapshot
        .audit_events
        .first()
        .map(|event| event.audit_id.as_str())
        .unwrap_or("audit pending");
    let first_api = snapshot
        .api_calls
        .first()
        .map(|call| call.endpoint.as_str())
        .unwrap_or("api call pending");
    let first_agent = snapshot
        .agent_runs
        .first()
        .map(|run| run.agent_run_id.as_str())
        .unwrap_or("agent run pending");
    let config_count = snapshot
        .health
        .checks
        .iter()
        .filter(|check| check.name.ends_with("_configuration"))
        .count();
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Governance control tower"}</h3>
                    <p>{"Audit-by-design map for pilot readiness, API access, Agent boundaries, and evidence trace coverage."}</p>
                </div>
                <span class={classes!("status-token", status_tone(pilot_status))}>{pilot_status}</span>
            </div>
            <div class="governance-cockpit">
                <aside class="case-brief governance-brief">
                    <span>{"Pilot readiness gate"}</span>
                    <strong>{pilot_status}</strong>
                    <dl>
                        <div><dt>{"Service"}</dt><dd>{format!("{} {}", snapshot.health.service, snapshot.health.version)}</dd></div>
                        <div><dt>{"Blockers"}</dt><dd>{snapshot.health.pilot_readiness.blocking_checks.len()}</dd></div>
                        <div><dt>{"Checks"}</dt><dd>{snapshot.health.checks.len()}</dd></div>
                        <div><dt>{"Configs"}</dt><dd>{format!("{} checks", config_count)}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("audit {}", snapshot.audit_events.len())}</span>
                        <span>{format!("api {}", snapshot.api_calls.len())}</span>
                        <span>{format!("agent {}", snapshot.agent_runs.len())}</span>
                    </div>
                </aside>

                <div class="governance-map">
                    <div class="governance-map-title">
                        <span>{"Audit-by-design map"}</span>
                        <strong>{"Evidence Trace Hub"}</strong>
                    </div>
                    <div class="governance-link horizontal"></div>
                    <div class="governance-link diagonal-a"></div>
                    <div class="governance-link diagonal-b"></div>
                    <div class="governance-core">
                        <span>{"Governance"}</span>
                        <strong>{"audit trail"}</strong>
                    </div>
                    <div class="governance-node readiness">
                        <span>{"Pilot gate"}</span>
                        <strong>{first_blocker}</strong>
                    </div>
                    <div class="governance-node api">
                        <span>{"API access"}</span>
                        <strong>{first_api}</strong>
                    </div>
                    <div class="governance-node audit">
                        <span>{"Audit event"}</span>
                        <strong>{first_audit}</strong>
                    </div>
                    <div class="governance-node agent">
                        <span>{"Agent boundary"}</span>
                        <strong>{first_agent}</strong>
                    </div>
                    <div class="governance-node evidence">
                        <span>{"Evidence refs"}</span>
                        <strong>{format!(
                            "{} audit / {} agent",
                            snapshot.audit_events.iter().filter(|event| !event.evidence_refs.is_empty()).count(),
                            snapshot.agent_runs.iter().filter(|run| !run.evidence_refs.is_empty()).count()
                        )}</strong>
                    </div>
                </div>

                <aside class="case-timeline governance-trace">
                    <h4>{"Control path"}</h4>
                    {timeline_item("Readiness", pilot_status, pilot_status)}
                    {timeline_item("API", &format!("{} records", snapshot.api_calls.len()), "done")}
                    {timeline_item("Audit", &format!("{} events", snapshot.audit_events.len()), "done")}
                    {timeline_item("Agent", "human approval boundary", "review")}
                </aside>
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ModuleStatusProps {
    title: String,
}

#[function_component(ModuleStatusPage)]
fn module_status_page(props: &ModuleStatusProps) -> Html {
    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{&props.title}</h2>
                    <p>{"This module remains part of the operations contract while the web console migrates to Yew."}</p>
                </div>
                <span class="status-pill">{"Yew shell"}</span>
            </div>
            <div class="panel">
                <h3>{"Migration Contract"}</h3>
                <p>{"Existing API, audit, QA, model, rule, and governance contracts stay in place while the console prioritizes the active operator workflow."}</p>
                <div class="tag-grid">
                    {for CONTRACT_PANELS.iter().map(|panel| html! { <span>{panel}</span> })}
                </div>
            </div>
        </section>
    }
}

#[function_component(ClaimInboxPage)]
fn claim_inbox_page() -> Html {
    let api_key = use_state(|| API_KEY_DEFAULT.to_string());
    let raw_payload = use_state(|| SAMPLE_INBOX_PAYLOAD.to_string());
    let overlay_payload = use_state(|| "{}".to_string());
    let reviewer_approved = use_state(|| false);
    let normalize_state = use_state(|| ApiState::<InboxNormalizeResponse>::Idle);
    let score_state = use_state(|| ApiState::<ScoreResponse>::Idle);
    let live_demo_state = use_state(|| ApiState::<LiveTpaDemoRun>::Idle);

    let merged_payload = use_memo(
        ((*raw_payload).clone(), (*overlay_payload).clone()),
        |(raw_payload, overlay_payload)| merge_payload_text(raw_payload, overlay_payload),
    );

    let normalize = {
        let api_key = api_key.clone();
        let merged_payload = merged_payload.clone();
        let normalize_state = normalize_state.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let normalize_state = normalize_state.clone();
            let score_state = score_state.clone();
            match &*merged_payload {
                Ok(payload) => {
                    let payload = payload.clone();
                    normalize_state.set(ApiState::Loading);
                    score_state.set(ApiState::Idle);
                    spawn_local(async move {
                        normalize_state.set(match normalize_claim(payload, api_key).await {
                            Ok(response) => ApiState::Ready(response),
                            Err(error) => ApiState::Failed(error),
                        });
                    });
                }
                Err(error) => normalize_state.set(ApiState::Failed(error.clone())),
            }
        })
    };

    let use_template = {
        let overlay_payload = overlay_payload.clone();
        let normalize_state = normalize_state.clone();
        Callback::from(move |_| {
            if let ApiState::Ready(response) = &*normalize_state {
                let template = correction_overlay_template_for(&response.validation_errors);
                overlay_payload.set(pretty_json(&template));
            }
        })
    };

    let score = {
        let api_key = api_key.clone();
        let normalize_state = normalize_state.clone();
        let score_state = score_state.clone();
        Callback::from(move |_| {
            if let ApiState::Ready(response) = &*normalize_state {
                let api_key = (*api_key).clone();
                let score_state = score_state.clone();
                let payload = json!({
                    "source_system": source_system_from_context(&response.canonical_claim_context),
                    "inbox_run_id": response.run_id.clone(),
                });
                score_state.set(ApiState::Loading);
                spawn_local(async move {
                    score_state.set(match score_canonical_claim(payload, api_key).await {
                        Ok(response) => ApiState::Ready(response),
                        Err(error) => ApiState::Failed(error),
                    });
                });
            }
        })
    };

    let run_live_demo = {
        let api_key = api_key.clone();
        let normalize_state = normalize_state.clone();
        let score_state = score_state.clone();
        let live_demo_state = live_demo_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let normalize_state = normalize_state.clone();
            let score_state = score_state.clone();
            let live_demo_state = live_demo_state.clone();
            normalize_state.set(ApiState::Loading);
            score_state.set(ApiState::Idle);
            live_demo_state.set(ApiState::Loading);
            spawn_local(async move {
                let result = async {
                    let before_dashboard = get_dashboard_summary(api_key.clone()).await?;
                    let payload = live_tpa_demo_payload(&before_dashboard)?;
                    let normalize_response = normalize_claim(payload, api_key.clone()).await?;
                    normalize_state.set(ApiState::Ready(normalize_response.clone()));
                    if !normalize_response.scoring_ready {
                        return Err("live demo packet did not pass intake normalization".into());
                    }
                    let score_response = score_canonical_claim(
                        json!({
                            "source_system": source_system_from_context(&normalize_response.canonical_claim_context),
                            "inbox_run_id": normalize_response.run_id.clone(),
                        }),
                        api_key.clone(),
                    )
                    .await?;
                    score_state.set(ApiState::Ready(score_response.clone()));
                    let score_run_id = score_response
                        .run_id
                        .clone()
                        .ok_or_else(|| "score response did not include a run id".to_string())?;
                    let snapshot = get_leads_cases_snapshot(api_key.clone()).await?;
                    let lead = latest_lead_for_score(
                        &snapshot,
                        &score_response.claim_id,
                        &score_run_id,
                    )
                    .ok_or_else(|| {
                        format!(
                            "no generated lead found for {} / {}",
                            score_response.claim_id, score_run_id
                        )
                    })?;
                    let triage = post_triage_lead(
                        api_key.clone(),
                        lead.lead_id.clone(),
                        json!({
                            "decision": "open_case",
                            "merge_target_lead_id": Value::Null,
                            "assignee": "demo-investigator",
                            "reviewer": "demo-reviewer",
                            "priority": "high",
                            "notes": "Live TPA demo opens a governed FWA investigation case.",
                            "evidence_refs": if lead.evidence_refs.is_empty() {
                                vec![format!("leads:{}", lead.lead_id)]
                            } else {
                                lead.evidence_refs.clone()
                            },
                        }),
                    )
                    .await?;
                    let case = triage
                        .case
                        .ok_or_else(|| "triage did not open an investigation case".to_string())?;
                    let case_update = post_case_status(
                        api_key.clone(),
                        case.case_id.clone(),
                        json!({
                            "status": "investigating",
                            "actor_id": "demo-investigator",
                            "notes": "Live TPA demo investigation started from the triaged lead.",
                            "evidence_refs": [
                                format!("investigation_cases:{}", case.case_id),
                                format!("audit:{}", triage.audit_id),
                            ],
                        }),
                    )
                    .await?;
                    let score_audit_id = score_response
                        .audit_id
                        .clone()
                        .unwrap_or_else(|| score_run_id.clone());
                    let investigation = post_investigation_result(
                        api_key.clone(),
                        json!({
                            "case_id": case.case_id,
                            "claim_id": score_response.claim_id,
                            "investigation_id": format!("INV-LIVE-{}", score_run_id),
                            "outcome": "confirmed_fwa_prevented_payment",
                            "confirmed_fwa": true,
                            "financial_impact_type": "prevented_payment",
                            "saving_amount": LIVE_TPA_DEMO_AMOUNT,
                            "currency": "CNY",
                            "notes": "Demo reviewer confirmed the pre-payment FWA intervention and prevented payment.",
                            "evidence_refs": [
                                format!("investigation_cases:{}", case_update.case.case_id),
                                format!("audit:{}", score_audit_id),
                            ],
                        }),
                    )
                    .await?;
                    let after_dashboard = get_dashboard_summary(api_key).await?;
                    Ok(LiveTpaDemoRun {
                        claim_id: score_response.claim_id,
                        claim_amount: LIVE_TPA_DEMO_AMOUNT.to_string(),
                        inbox_run_id: normalize_response.run_id,
                        score_run_id,
                        risk_score: display_value(&score_response.risk_score),
                        rag: score_response
                            .rag
                            .as_ref()
                            .map(display_value)
                            .unwrap_or_else(|| "missing".into()),
                        decision_outcome: score_response
                            .decision_outcome
                            .unwrap_or_else(|| "review".into()),
                        lead_id: lead.lead_id.clone(),
                        case_id: case_update.case.case_id,
                        case_status: case_update.case.status,
                        investigation_audit_id: investigation.audit_id,
                        prevented_before: before_dashboard.value_measurement.prevented_payment,
                        prevented_after: after_dashboard.value_measurement.prevented_payment,
                        dashboard_saving_after: after_dashboard.saving_amount,
                    })
                }
                .await;
                live_demo_state.set(match result {
                    Ok(run) => ApiState::Ready(run),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let hints = match &*normalize_state {
        ApiState::Ready(response) => correction_hints_for(response),
        _ => Vec::new(),
    };
    let can_score = matches!(&*normalize_state, ApiState::Ready(response) if response.scoring_ready || *reviewer_approved);

    html! {
        <section class="claim-inbox">
            <div class="dashboard-header">
                <div>
                    <h2>{"Intake Ops"}</h2>
                    <p>{"Review inbound TPA claim packets, resolve intake blockers, and release accepted claims into the risk and review queue."}</p>
                </div>
                <span class="status-pill">{"Intake Ops"}</span>
            </div>

            <div class="inbox-grid">
                <section class="panel">
                    <h3>{"Inbound Claim Packet"}</h3>
                    <p class="empty">{"Use the configured intake channel to check whether the claim packet is complete enough for downstream review."}</p>
                    <div class="summary-grid">
                        <div><span>{"Source"}</span><strong>{"TPA intake"}</strong></div>
                        <div><span>{"Packet"}</span><strong>{"sample loaded"}</strong></div>
                        <div><span>{"Next step"}</span><strong>{"check intake packet"}</strong></div>
                    </div>
                    <details>
                        <summary>{"Technical payload editor"}</summary>
                        <label>
                            {"Payload JSON"}
                            <textarea
                                class="payload-editor"
                                value={(*raw_payload).clone()}
                                oninput={{
                                    let raw_payload = raw_payload.clone();
                                    Callback::from(move |event: InputEvent| {
                                        raw_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                    </details>
                </section>

                <section class="panel">
                    <h3>{"Correction Worklist"}</h3>
                    <p class="empty">{"After intake checks run, prepare only the missing or reviewer-approved fixes needed for queue release."}</p>
                    <div class="button-row">
                        <button onclick={use_template} disabled={!matches!(&*normalize_state, ApiState::Ready(_))}>
                            {"Prepare correction draft"}
                        </button>
                    </div>
                    <details>
                        <summary>{"Technical correction editor"}</summary>
                        <label>
                            {"Correction JSON"}
                            <textarea
                                class="payload-editor"
                                value={(*overlay_payload).clone()}
                                oninput={{
                                    let overlay_payload = overlay_payload.clone();
                                    Callback::from(move |event: InputEvent| {
                                        overlay_payload.set(event.target_unchecked_into::<HtmlTextAreaElement>().value());
                                    })
                                }}
                            />
                        </label>
                    </details>
                    if let Err(error) = &*merged_payload {
                        <p class="error">{error}</p>
                    }
                </section>
            </div>

            <section class="panel result-stack live-demo-panel">
                <div class="section-header">
                    <div>
                        <h3>{"Live TPA Demo Run"}</h3>
                        <p>{"Show one raw TPA packet becoming a scored lead, investigation case, human writeback, and value proof without switching scripts mid-demo."}</p>
                    </div>
                    <span class="status-token strong">{"TPA packet -> risk queue -> case -> value proof"}</span>
                </div>
                <div class="inbox-pipeline live-demo-flow">
                    {pipeline_step("Receive", "raw TPA packet", "done")}
                    {pipeline_step("Normalize", "canonical claim", if matches!(&*normalize_state, ApiState::Ready(_)) { "done" } else { "pending" })}
                    {pipeline_step("Score", "risk + routing", if matches!(&*score_state, ApiState::Ready(_)) { "done" } else { "pending" })}
                    {pipeline_step("Investigate", "lead + case", if matches!(&*live_demo_state, ApiState::Ready(_)) { "done" } else { "pending" })}
                    {pipeline_step("Prove Value", "prevented payment", if matches!(&*live_demo_state, ApiState::Ready(_)) { "done" } else { "pending" })}
                </div>
                <div class="button-row">
                    <button onclick={run_live_demo} disabled={matches!(&*live_demo_state, ApiState::Loading)}>
                        {if matches!(&*live_demo_state, ApiState::Loading) { "Running live demo..." } else { "Run full TPA demo" }}
                    </button>
                </div>
                <LiveTpaDemoView state={(*live_demo_state).clone()} />
            </section>

            <div class="action-bar">
                <button onclick={normalize.clone()} disabled={matches!(&*normalize_state, ApiState::Loading)}>
                    {if matches!(&*normalize_state, ApiState::Loading) { "Checking..." } else { "Check intake packet" }}
                </button>
                <label class="checkbox-row">
                    <input
                        type="checkbox"
                        checked={*reviewer_approved}
                        onchange={{
                            let reviewer_approved = reviewer_approved.clone();
                            Callback::from(move |event: Event| {
                                reviewer_approved.set(event.target_unchecked_into::<HtmlInputElement>().checked());
                            })
                        }}
                    />
                    {"Reviewer confirms required intake fixes"}
                </label>
                <button onclick={score} disabled={!can_score || matches!(&*score_state, ApiState::Loading)}>
                    {if matches!(&*score_state, ApiState::Loading) { "Releasing..." } else { "Release accepted claim" }}
                </button>
            </div>

            <div class="inbox-grid">
                <NormalizeResultView state={(*normalize_state).clone()} hints={hints} />
                <ScoreResultView state={(*score_state).clone()} />
            </div>
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct NormalizeResultProps {
    state: ApiState<InboxNormalizeResponse>,
    hints: Vec<CorrectionHint>,
}

#[function_component(NormalizeResultView)]
fn normalize_result_view(props: &NormalizeResultProps) -> Html {
    html! {
        <section class="panel result-stack">
            <h3>{"Intake Findings"}</h3>
            {match &props.state {
                ApiState::Idle => html! { <p class="empty">{"Check the intake packet to see blockers, warnings, and required fixes."}</p> },
                ApiState::Loading => html! { <p>{"Checking intake packet..."}</p> },
                ApiState::Failed(error) => html! { <p class="error">{error}</p> },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="score-hero compact-metrics">
                            <div><span>{"Validation"}</span><strong>{business_label(&response.validation_result)}</strong></div>
                            <div><span>{"Queue Ready"}</span><strong>{if response.scoring_ready { "Ready" } else { "Needs review" }}</strong></div>
                            <div><span>{"Mapping"}</span><strong>{&response.mapping_version}</strong></div>
                        </div>
                        {inbox_pipeline_visual(response)}
                        {validation_findings_visual(response, &props.hints)}
                        <details>
                            <summary>{"Audit trace"}</summary>
                            <dl class="result-grid">
                                <div><dt>{"Run ID"}</dt><dd>{&response.run_id}</dd></div>
                                <div><dt>{"Audit ID"}</dt><dd>{&response.audit_id}</dd></div>
                                <div><dt>{"External Message"}</dt><dd>{response.external_message_id.as_deref().unwrap_or("missing")}</dd></div>
                                <div><dt>{"Payload Ref"}</dt><dd>{response.raw_payload_ref.as_deref().unwrap_or("pending")}</dd></div>
                            </dl>
                        </details>
                        <h4>{"Required Fixes"}</h4>
                        if props.hints.is_empty() {
                            <p class="empty">{"No correction hints returned."}</p>
                        } else {
                            <div class="table-list finding-list">
                                {for props.hints.iter().map(|hint| html! {
                                    <div class="finding-row">
                                        <strong>{&hint.field_path}</strong>
                                        <span class={classes!("severity", hint.severity.clone())}>{business_label(&hint.severity)}</span>
                                        <p>{&hint.next_action}</p>
                                        <small>{if hint.blocks_scoring { "blocks queue release" } else { "review signal" }}</small>
                                    </div>
                                })}
                            </div>
                        }
                        <details>
                            <summary>{"Canonical context preview"}</summary>
                            <pre>{pretty_json(&response.canonical_claim_context)}</pre>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ScoreResultProps {
    state: ApiState<ScoreResponse>,
}

#[function_component(ScoreResultView)]
fn score_result_view(props: &ScoreResultProps) -> Html {
    html! {
        <section class="panel result-stack queue-handoff-panel">
            <h3>{"Queue Handoff"}</h3>
            {match &props.state {
                ApiState::Idle => html! {
                    <div class="handoff-status pending">
                        <span>{"Not released"}</span>
                        <strong>{"Waiting for intake check"}</strong>
                        <small>{"Accepted claims enter Leads & Cases or review queues after release."}</small>
                    </div>
                },
                ApiState::Loading => html! {
                    <div class="handoff-status pending">
                        <span>{"Release in progress"}</span>
                        <strong>{"Creating queue handoff"}</strong>
                        <small>{"The claim is being checked by the risk service before downstream routing."}</small>
                    </div>
                },
                ApiState::Failed(error) => html! {
                    <>
                        <div class="handoff-status blocked">
                            <span>{"Not released"}</span>
                            <strong>{release_blocker_title(error)}</strong>
                            <small>{release_blocker_next_step(error)}</small>
                        </div>
                        <details>
                            <summary>{"Diagnostic detail"}</summary>
                            <p class="empty">{error}</p>
                        </details>
                    </>
                },
                ApiState::Ready(response) => html! {
                    <>
                        <div class="handoff-status done">
                            <span>{"Released"}</span>
                            <strong>{"Claim entered downstream queue"}</strong>
                            <small>{"Reviewers continue the case from Leads & Cases or Review Workbench."}</small>
                        </div>
                        <div class="score-hero compact-metrics">
                            <div><span>{"Claim"}</span><strong>{&response.claim_id}</strong></div>
                            <div><span>{"Risk Score"}</span><strong>{display_value(&response.risk_score)}</strong></div>
                            <div><span>{"Queue Route"}</span><strong>{response.recommended_action.as_deref().map(business_label).unwrap_or_else(|| "Manual review".into())}</strong></div>
                        </div>
                        <details>
                            <summary>{"Release trace"}</summary>
                            <dl class="result-grid">
                                <div><dt>{"Audit ID"}</dt><dd>{response.audit_id.as_deref().unwrap_or("pending")}</dd></div>
                                <div><dt>{"Evidence Refs"}</dt><dd>{response.evidence_refs.as_ref().map(|refs| value_refs_label(refs)).unwrap_or_else(|| "none".into())}</dd></div>
                            </dl>
                        </details>
                    </>
                },
            }}
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct LiveTpaDemoProps {
    state: ApiState<LiveTpaDemoRun>,
}

#[function_component(LiveTpaDemoView)]
fn live_tpa_demo_view(props: &LiveTpaDemoProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! {
                    <p class="empty">{"Run the full TPA demo when you want the audience to see the system move from inbound packet to prevented-payment value proof."}</p>
                },
                ApiState::Loading => html! {
                    <div class="handoff-status pending">
                        <span>{"Live demo running"}</span>
                        <strong>{"Normalizing, scoring, opening case, and writing back outcome"}</strong>
                        <small>{"The UI is calling the same APIs that the external TPA demo script calls."}</small>
                    </div>
                },
                ApiState::Failed(error) => html! {
                    <div class="handoff-status blocked">
                        <span>{"Live demo stopped"}</span>
                        <strong>{"Fix the runtime before presenting"}</strong>
                        <small>{error}</small>
                    </div>
                },
                ApiState::Ready(run) => html! {
                    <>
                        <div class="handoff-status done">
                            <span>{"Live demo complete"}</span>
                            <strong>{format!("{} prevented payment recorded", run.claim_amount)}</strong>
                            <small>{"The claim is now visible in Leads & Cases and the value proof dashboard reflects the writeback."}</small>
                        </div>
                        <div class="score-hero compact-metrics">
                            <div><span>{"Claim"}</span><strong>{&run.claim_id}</strong></div>
                            <div><span>{"Risk"}</span><strong>{format!("{} / {}", run.risk_score, rag_label(&run.rag))}</strong></div>
                            <div><span>{"Decision"}</span><strong>{business_label(&run.decision_outcome)}</strong></div>
                        </div>
                        <div class="summary-grid">
                            <div><span>{"Inbox run"}</span><strong>{&run.inbox_run_id}</strong></div>
                            <div><span>{"Score run"}</span><strong>{&run.score_run_id}</strong></div>
                            <div><span>{"Lead"}</span><strong>{&run.lead_id}</strong></div>
                            <div><span>{"Case"}</span><strong>{format!("{} / {}", run.case_id, case_stage_label(&run.case_status))}</strong></div>
                            <div><span>{"Investigation audit"}</span><strong>{&run.investigation_audit_id}</strong></div>
                            <div><span>{"Dashboard value"}</span><strong>{format!("{} -> {}", run.prevented_before, run.prevented_after)}</strong></div>
                        </div>
                        <small>{format!("confirmed dashboard saving amount: {}", run.dashboard_saving_after)}</small>
                    </>
                },
            }}
        </>
    }
}

fn release_blocker_title(error: &str) -> &'static str {
    if error.contains("coverage_limit") || error.contains("coverage limit") {
        "Coverage limit needs correction"
    } else if error.contains("claim_amount") || error.contains("claim amount") {
        "Claim amount needs confirmation"
    } else {
        "Claim packet is not ready"
    }
}

fn release_blocker_next_step(error: &str) -> &'static str {
    if error.contains("coverage_limit") || error.contains("coverage limit") {
        "Update the policy or liability coverage limit, then check the intake packet again."
    } else if error.contains("claim_amount") || error.contains("claim amount") {
        "Confirm the payable claim amount from invoice totals before release."
    } else {
        "Resolve the intake findings on the left before releasing this claim."
    }
}

async fn normalize_claim(
    payload: Value,
    api_key: String,
) -> Result<InboxNormalizeResponse, String> {
    request_json("/api/v1/inbox/claims/normalize", api_key, payload).await
}

async fn score_canonical_claim(payload: Value, api_key: String) -> Result<ScoreResponse, String> {
    request_json("/api/v1/claims/score", api_key, payload).await
}

async fn get_dashboard_summary(api_key: String) -> Result<DashboardSummary, String> {
    request_get_json("/api/v1/ops/dashboard/summary", api_key).await
}

async fn get_rule_ops_snapshot(
    api_key: String,
    rule_id: String,
) -> Result<RuleOpsSnapshot, String> {
    let rules = request_get_json::<RuleListResponse>("/api/v1/ops/rules", api_key.clone())
        .await?
        .rules;
    let selected_rule_id = rules
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .map(|rule| rule.rule_id.clone())
        .or_else(|| rules.first().map(|rule| rule.rule_id.clone()))
        .unwrap_or(rule_id);
    let performance = request_get_json::<RulePerformanceResponse>(
        "/api/v1/ops/rules/performance",
        api_key.clone(),
    )
    .await?
    .rules;
    let gates = request_get_json::<RulePromotionGates>(
        &format!("/api/v1/ops/rules/{selected_rule_id}/promotion-gates"),
        api_key,
    )
    .await?;
    Ok(RuleOpsSnapshot {
        rules,
        performance,
        gates,
    })
}

async fn get_model_ops_snapshot(
    api_key: String,
    model_key: String,
    model_version: Option<String>,
) -> Result<ModelOpsSnapshot, String> {
    let models = request_get_json::<ModelListResponse>("/api/v1/ops/models", api_key.clone())
        .await?
        .models;
    let selected_model_key = models
        .iter()
        .find(|model| model.model_key == model_key)
        .map(|model| model.model_key.clone())
        .or_else(|| models.first().map(|model| model.model_key.clone()))
        .unwrap_or(model_key);
    let performance = request_get_json::<ModelPerformance>(
        &format!("/api/v1/ops/models/{selected_model_key}/performance"),
        api_key.clone(),
    )
    .await?;
    let selected_model_version = model_version
        .as_deref()
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .filter(|version| {
            models
                .iter()
                .any(|model| model.model_key == selected_model_key && model.version == *version)
        });
    let gates_path = if let Some(model_version) = selected_model_version {
        format!("/api/v1/ops/models/{selected_model_key}/versions/{model_version}/promotion-gates")
    } else {
        format!("/api/v1/ops/models/{selected_model_key}/promotion-gates")
    };
    let gates = request_get_json::<ModelPromotionGates>(&gates_path, api_key.clone()).await?;
    let retraining = request_get_json::<ModelRetrainingReadiness>(
        &format!("/api/v1/ops/models/{selected_model_key}/retraining-readiness"),
        api_key,
    )
    .await?;
    Ok(ModelOpsSnapshot {
        models,
        performance,
        gates,
        retraining,
    })
}

async fn get_mlops_workspace_snapshot(
    api_key: String,
    model_key: String,
    candidate_model_version: String,
) -> Result<MlopsWorkspaceSnapshot, String> {
    let data_sources = get_data_sources_snapshot(api_key.clone()).await?;
    let model_ops =
        get_model_ops_snapshot(api_key.clone(), model_key, Some(candidate_model_version)).await?;
    let retraining_jobs = request_get_json::<ModelRetrainingJobListResponse>(
        &format!(
            "/api/v1/ops/models/{}/retraining-jobs",
            model_ops.performance.model_key
        ),
        api_key.clone(),
    )
    .await?
    .jobs;
    let monitoring_review_tasks = request_get_json::<ModelMonitoringReviewQueueResponse>(
        &format!(
            "/api/v1/ops/models/{}/mlops-monitoring-review-queue",
            model_ops.performance.model_key
        ),
        api_key.clone(),
    )
    .await?
    .tasks;
    let alert_delivery_tasks = request_get_json::<MlopsAlertDeliveryQueueResponse>(
        &format!(
            "/api/v1/ops/models/{}/mlops-alert-delivery-queue",
            model_ops.performance.model_key
        ),
        api_key.clone(),
    )
    .await?
    .tasks;
    let anomaly_review_tasks = request_get_json::<AnomalyReviewQueueResponse>(
        "/api/v1/ops/providers/anomaly-review-queue",
        api_key,
    )
    .await?
    .tasks;
    Ok(MlopsWorkspaceSnapshot {
        data_sources,
        model_ops,
        retraining_jobs,
        monitoring_review_tasks,
        alert_delivery_tasks,
        anomaly_review_tasks,
    })
}

async fn execute_mlops_governed_action(
    api_key: String,
    model_key: String,
    action: &str,
    actor: String,
    reviewer: String,
    promotion_decision: String,
    monitoring_task_id: String,
    monitoring_decision: String,
    alert_task_id: String,
    alert_decision: String,
    retraining_job_id: String,
    retraining_status: String,
    candidate_model_version: String,
    candidate_artifact_uri: String,
    candidate_artifact_sha256: String,
    training_artifact_uri: String,
    training_artifact_sha256: String,
    serving_manifest_uri: String,
    candidate_endpoint_url: String,
    validation_report_uri: String,
    candidate_auc: String,
    candidate_ks: String,
    candidate_precision: String,
    candidate_recall: String,
    candidate_f1: String,
    candidate_accuracy: String,
    candidate_threshold: String,
    candidate_confusion_matrix: String,
    candidate_feature_importance_uri: String,
    candidate_permutation_importance_uri: String,
    candidate_metrics_json: String,
    mined_rule_candidates_json: String,
    notes: String,
    mut evidence_refs: Vec<String>,
) -> Result<Value, String> {
    let model_key = model_key.trim();
    match action {
        "queue_retraining" => {
            request_json(
                &format!("/api/v1/ops/models/{model_key}/retraining-jobs"),
                api_key,
                json!({
                    "requested_by": actor.trim(),
                    "notes": notes.trim(),
                }),
            )
            .await
        }
        "monitoring_review"
        | "monitoring_reject"
        | "monitoring_prepare"
        | "monitoring_rollback" => {
            if monitoring_task_id.trim().is_empty() {
                return Err("monitoring review actions require a monitoring task id".into());
            }
            if evidence_refs.is_empty() {
                return Err("monitoring review actions require evidence refs".into());
            }
            let decision = match action {
                "monitoring_reject" => "rejected",
                "monitoring_prepare" => "prepare_retraining",
                "monitoring_rollback" => "open_rollback_review",
                _ => monitoring_decision.trim(),
            };
            request_json(
                &format!(
                    "/api/v1/ops/models/{model_key}/mlops-monitoring-review-tasks/{}/reviews",
                    monitoring_task_id.trim()
                ),
                api_key,
                json!({
                    "decision": decision,
                    "reviewer": reviewer.trim(),
                    "notes": notes.trim(),
                    "evidence_refs": evidence_refs,
                }),
            )
            .await
        }
        "alert_review" | "alert_escalate" => {
            if alert_task_id.trim().is_empty() {
                return Err("alert review actions require an alert task id".into());
            }
            if evidence_refs.is_empty() {
                return Err("alert review actions require evidence refs".into());
            }
            let decision = if action == "alert_escalate" {
                "escalated_for_governance_review"
            } else {
                alert_decision.trim()
            };
            request_json(
                &format!(
                    "/api/v1/ops/models/{model_key}/mlops-alert-delivery-tasks/{}/reviews",
                    alert_task_id.trim()
                ),
                api_key,
                json!({
                    "decision": decision,
                    "reviewer": reviewer.trim(),
                    "notes": notes.trim(),
                    "evidence_refs": evidence_refs,
                }),
            )
            .await
        }
        "claim_retraining_job" => {
            request_json(
                "/api/v1/ops/model-retraining-jobs/claim-next",
                api_key,
                json!({
                    "actor": actor.trim(),
                    "notes": notes.trim(),
                    "model_key": model_key,
                }),
            )
            .await
        }
        "update_retraining_job" => {
            if retraining_job_id.trim().is_empty() {
                return Err("training job status updates require a training job id".into());
            }
            request_json(
                &format!(
                    "/api/v1/ops/model-retraining-jobs/{}/status",
                    retraining_job_id.trim()
                ),
                api_key,
                json!({
                    "status": retraining_status.trim(),
                    "actor": actor.trim(),
                    "notes": notes.trim(),
                }),
            )
            .await
        }
        "register_retraining_output" => {
            if retraining_job_id.trim().is_empty() {
                return Err("provider output registration requires a training job id".into());
            }
            if evidence_refs.is_empty() {
                return Err("provider output registration requires evidence refs".into());
            }
            let confusion_matrix_json =
                parse_json_object(&candidate_confusion_matrix, "confusion matrix")?;
            let metrics_json = parse_json_object(&candidate_metrics_json, "metrics")?;
            let auc = parse_optional_unit_metric(&candidate_auc, "AUC")?;
            let ks = parse_optional_unit_metric(&candidate_ks, "KS")?;
            let precision = parse_optional_unit_metric(&candidate_precision, "precision")?;
            let recall = parse_optional_unit_metric(&candidate_recall, "recall")?;
            let f1 = parse_optional_unit_metric(&candidate_f1, "F1")?;
            let accuracy = parse_optional_unit_metric(&candidate_accuracy, "accuracy")?;
            let threshold = parse_optional_unit_metric(&candidate_threshold, "threshold")?;
            let artifact_sha256 = optional_trimmed_value(&candidate_artifact_sha256);
            let training_artifact_uri = optional_trimmed_value(&training_artifact_uri);
            let training_artifact_sha256 = optional_trimmed_value(&training_artifact_sha256);
            let serving_manifest_uri = optional_trimmed_value(&serving_manifest_uri);
            let endpoint_url = optional_trimmed_value(&candidate_endpoint_url);
            let feature_importance_uri = optional_trimmed_value(&candidate_feature_importance_uri);
            let permutation_importance_uri =
                optional_trimmed_value(&candidate_permutation_importance_uri);
            let mined_rule_candidates =
                parse_optional_json_array(&mined_rule_candidates_json, "mined rule candidates")?;
            let evaluation_run_id = format!(
                "eval_{}_{}",
                model_key,
                candidate_model_version
                    .trim()
                    .replace('.', "_")
                    .replace('-', "_")
            );
            evidence_refs = push_unique(
                evidence_refs,
                format!("model_retraining_jobs:{}", retraining_job_id.trim()),
            );
            evidence_refs = push_unique(
                evidence_refs,
                format!("model_artifacts:{}", candidate_artifact_uri.trim()),
            );
            if let Some(training_artifact_uri) = &training_artifact_uri {
                evidence_refs = push_unique(
                    evidence_refs,
                    format!("model_training_artifacts:{training_artifact_uri}"),
                );
            }
            if let Some(serving_manifest_uri) = &serving_manifest_uri {
                evidence_refs = push_unique(
                    evidence_refs,
                    format!("model_serving_manifests:{serving_manifest_uri}"),
                );
            }
            if let Some(permutation_importance_uri) = &permutation_importance_uri {
                evidence_refs = push_unique(
                    evidence_refs,
                    format!("model_permutation_importance:{permutation_importance_uri}"),
                );
            }
            evidence_refs = push_unique(
                evidence_refs,
                format!("model_validation_reports:{}", validation_report_uri.trim()),
            );
            evidence_refs = push_unique(
                evidence_refs,
                format!("model_evaluations:{evaluation_run_id}"),
            );
            request_json(
                &format!(
                    "/api/v1/ops/model-retraining-jobs/{}/output",
                    retraining_job_id.trim()
                ),
                api_key,
                json!({
                    "actor": actor.trim(),
                    "notes": notes.trim(),
                    "candidate_model_version": candidate_model_version.trim(),
                    "artifact_uri": candidate_artifact_uri.trim(),
                    "artifact_sha256": artifact_sha256,
                    "training_artifact_uri": training_artifact_uri,
                    "training_artifact_sha256": training_artifact_sha256,
                    "serving_manifest_uri": serving_manifest_uri,
                    "endpoint_url": endpoint_url,
                    "validation_report_uri": validation_report_uri.trim(),
                    "evaluation_run_id": evaluation_run_id,
                    "evidence_refs": evidence_refs,
                    "auc": auc,
                    "ks": ks,
                    "precision": precision,
                    "recall": recall,
                    "f1": f1,
                    "accuracy": accuracy,
                    "threshold": threshold,
                    "confusion_matrix_json": confusion_matrix_json,
                    "feature_importance_uri": feature_importance_uri,
                    "permutation_importance_uri": permutation_importance_uri,
                    "metrics_json": metrics_json,
                    "mined_rule_owner": "external-training-platform",
                    "mined_rule_candidates": mined_rule_candidates,
                }),
            )
            .await
        }
        "promotion_review" => {
            if evidence_refs.is_empty() {
                return Err("model promotion review requires evidence refs".into());
            }
            let model_version = candidate_model_version.trim();
            if model_version.is_empty() {
                return Err("model promotion review requires a candidate version".into());
            }
            request_json(
                &format!(
                    "/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-reviews"
                ),
                api_key,
                json!({
                    "decision": promotion_decision.trim(),
                    "reviewer": reviewer.trim(),
                    "notes": notes.trim(),
                    "evidence_refs": evidence_refs,
                }),
            )
            .await
        }
        "activate" => {
            if evidence_refs.is_empty() {
                return Err("model lifecycle actions require evidence refs".into());
            }
            let model_version = candidate_model_version.trim();
            if model_version.is_empty() {
                return Err("model activation requires a candidate version".into());
            }
            request_json(
                &format!("/api/v1/ops/models/{model_key}/versions/{model_version}/activate"),
                api_key,
                json!({ "evidence_refs": evidence_refs }),
            )
            .await
        }
        "rollback" => {
            if evidence_refs.is_empty() {
                return Err("model lifecycle actions require evidence refs".into());
            }
            request_json(
                &format!("/api/v1/ops/models/{model_key}/rollback"),
                api_key,
                json!({ "evidence_refs": evidence_refs }),
            )
            .await
        }
        _ => Err(format!("unknown MLOps action: {action}")),
    }
}

async fn get_factor_readiness(api_key: String) -> Result<FactorReadinessResponse, String> {
    request_get_json("/api/v1/ops/factors/readiness", api_key).await
}

async fn get_routing_policy_snapshot(
    api_key: String,
    policy_id: String,
    review_mode: String,
    version: String,
) -> Result<RoutingPolicySnapshot, String> {
    let policies = request_get_json::<RoutingPolicyListResponse>(
        "/api/v1/ops/routing-policies",
        api_key.clone(),
    )
    .await?
    .policies;
    let version = parse_u32(&version, "routing policy version")?;
    let gates = request_get_json::<RoutingPolicyPromotionGates>(
        &format!(
            "/api/v1/ops/routing-policies/{}/{}/{}/promotion-gates",
            policy_id.trim(),
            review_mode.trim(),
            version
        ),
        api_key,
    )
    .await?;
    Ok(RoutingPolicySnapshot { policies, gates })
}

async fn update_routing_policy_lifecycle(
    api_key: String,
    policy_id: String,
    review_mode: String,
    version: String,
    action: &str,
    evidence_refs: Vec<String>,
) -> Result<RoutingPolicyRecord, String> {
    if evidence_refs.is_empty() {
        return Err("routing policy lifecycle actions require evidence refs".into());
    }
    let version = parse_u32(&version, "routing policy version")?;
    request_json(
        &format!(
            "/api/v1/ops/routing-policies/{}/{}/{}/{}",
            policy_id.trim(),
            review_mode.trim(),
            version,
            action
        ),
        api_key,
        json!({ "evidence_refs": evidence_refs }),
    )
    .await
}

async fn get_data_sources_snapshot(api_key: String) -> Result<DataSourcesSnapshot, String> {
    let datasets =
        request_get_json::<DatasetListResponse>("/api/v1/ops/datasets", api_key.clone()).await?;
    let evaluations =
        request_get_json::<ModelEvaluationListResponse>("/api/v1/ops/model-evaluations", api_key)
            .await?;
    Ok(DataSourcesSnapshot {
        datasets: datasets.datasets,
        health: datasets.health,
        evaluations: evaluations.evaluations,
        lineage: evaluations.lineage,
    })
}

async fn get_leads_cases_snapshot(api_key: String) -> Result<LeadsCasesSnapshot, String> {
    let leads = request_get_json::<LeadListResponse>("/api/v1/ops/leads", api_key.clone())
        .await?
        .leads;
    let cases = request_get_json::<CaseListResponse>("/api/v1/ops/cases", api_key)
        .await?
        .cases;
    Ok(LeadsCasesSnapshot { leads, cases })
}

async fn post_triage_lead(
    api_key: String,
    lead_id: String,
    payload: Value,
) -> Result<TriageLeadRecord, String> {
    request_json(
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        api_key,
        payload,
    )
    .await
}

async fn post_case_status(
    api_key: String,
    case_id: String,
    payload: Value,
) -> Result<UpdateCaseStatusRecord, String> {
    request_json(
        &format!("/api/v1/ops/cases/{case_id}/status"),
        api_key,
        payload,
    )
    .await
}

async fn post_investigation_result(
    api_key: String,
    payload: Value,
) -> Result<PilotWritebackResponse, String> {
    request_json("/api/v1/investigations/results", api_key, payload).await
}

async fn get_member_profile_summary(
    api_key: String,
    member_id: String,
) -> Result<MemberProfileSummary, String> {
    let member_id = member_id.trim();
    if member_id.is_empty() {
        return Err("member id is required".into());
    }
    request_get_json(
        &format!("/api/v1/members/{member_id}/profile-summary"),
        api_key,
    )
    .await
}

async fn get_provider_risk_summary(api_key: String) -> Result<ProviderRiskSummary, String> {
    request_get_json("/api/v1/ops/providers/risk-summary", api_key).await
}

async fn get_audit_samples(api_key: String) -> Result<Vec<AuditSampleRecord>, String> {
    Ok(
        request_get_json::<AuditSampleListResponse>("/api/v1/ops/audit-samples", api_key)
            .await?
            .samples,
    )
}

async fn post_audit_sample(api_key: String, payload: Value) -> Result<AuditSampleRecord, String> {
    request_json("/api/v1/ops/audit-samples", api_key, payload).await
}

async fn get_audit_events_for_sample(
    api_key: String,
    sample_id: String,
) -> Result<Vec<AuditEventRecord>, String> {
    let sample_id = sample_id.trim();
    if sample_id.is_empty() {
        return Err("audit sample id is required".into());
    }
    Ok(request_get_json::<AuditEventListResponse>(
        &format!("/api/v1/ops/audit-events?sample_id={sample_id}&limit=20"),
        api_key,
    )
    .await?
    .events)
}

async fn get_medical_review_queue(
    api_key: String,
    limit: String,
) -> Result<Vec<MedicalReviewQueueItem>, String> {
    let limit = limit
        .trim()
        .parse::<u32>()
        .ok()
        .map(|value| value.clamp(1, 200))
        .unwrap_or(100);
    Ok(request_get_json::<MedicalReviewQueueResponse>(
        &format!("/api/v1/ops/medical-review/queue?limit={limit}"),
        api_key,
    )
    .await?
    .items)
}

async fn post_medical_review_result(
    api_key: String,
    payload: Value,
) -> Result<MedicalReviewResultResponse, String> {
    request_json("/api/v1/ops/medical-review/results", api_key, payload).await
}

async fn get_qa_review_snapshot(api_key: String) -> Result<QaReviewSnapshot, String> {
    let queue = request_get_json::<QaQueueListResponse>("/api/v1/ops/qa/queue", api_key.clone())
        .await?
        .items;
    let summary =
        request_get_json::<QaQueueSummary>("/api/v1/ops/qa/queue-summary", api_key.clone()).await?;
    let feedback_items =
        request_get_json::<QaFeedbackItemListResponse>("/api/v1/ops/qa/feedback-items", api_key)
            .await?
            .items;
    Ok(QaReviewSnapshot {
        queue,
        summary,
        feedback_items,
    })
}

async fn get_bootstrap_ops_snapshot(api_key: String) -> Result<BootstrapOpsSnapshot, String> {
    let backfills = request_get_json::<HistoricalBackfillListResponse>(
        "/api/v1/ops/backfills",
        api_key.clone(),
    )
    .await?
    .jobs;
    let evidence_requests = request_get_json::<EvidenceRequestListResponse>(
        "/api/v1/ops/evidence-requests",
        api_key.clone(),
    )
    .await?
    .requests;
    let label_items = request_get_json::<LabelBootstrapQueueResponse>(
        "/api/v1/ops/label-bootstrap/queue",
        api_key,
    )
    .await?
    .items;
    Ok(BootstrapOpsSnapshot {
        backfills,
        evidence_requests,
        label_items,
    })
}

async fn create_bootstrap_backfill(api_key: String) -> Result<HistoricalBackfillResponse, String> {
    request_json(
        "/api/v1/ops/backfills",
        api_key,
        json!({
            "dataset_refs": ["ops:current_scoring_audit"],
            "rule_refs": ["ops:active_rule_library"],
            "reviewer": "ops-lead",
            "notes": "Create a governed replay snapshot for label handoff.",
            "limit": 25,
        }),
    )
    .await
}

async fn generate_bootstrap_evidence_requests(
    api_key: String,
) -> Result<EvidenceRequestGenerateResponse, String> {
    request_json(
        "/api/v1/ops/evidence-requests/generate",
        api_key,
        json!({
            "requested_by": "clinical-ops",
            "reviewer_queue": "clinical-evidence",
            "notes": "Generate missing-evidence requests from scoring audits.",
            "limit": 50,
        }),
    )
    .await
}

async fn mark_bootstrap_evidence_received(
    api_key: String,
    request_id: String,
    evidence_refs: Vec<String>,
    notes: String,
) -> Result<EvidenceRequestRecord, String> {
    request_json(
        &format!("/api/v1/ops/evidence-requests/{request_id}/status"),
        api_key,
        json!({
            "status": "received",
            "actor_id": "clinical-ops",
            "notes": notes,
            "evidence_refs": evidence_refs,
        }),
    )
    .await
}

async fn review_bootstrap_label(
    api_key: String,
    item_id: String,
    label_name: String,
    label_value: String,
    governance_status: String,
    feedback_target: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<LabelBootstrapReviewResponse, String> {
    request_json(
        &format!("/api/v1/ops/label-bootstrap/items/{item_id}/review"),
        api_key,
        json!({
            "reviewer": "label-governance",
            "label_name": label_name,
            "label_value": label_value,
            "governance_status": governance_status,
            "feedback_target": feedback_target,
            "notes": notes,
            "evidence_refs": evidence_refs,
        }),
    )
    .await
}

async fn get_knowledge_snapshot(
    api_key: String,
    claim_id: String,
    diagnosis_code: String,
    provider_region: String,
    tags_text: String,
) -> Result<KnowledgeSnapshot, String> {
    let tags = parse_tags(&tags_text);
    if diagnosis_code.trim().is_empty() || provider_region.trim().is_empty() || tags.is_empty() {
        return Err("diagnosis code, provider region, and at least one tag are required".into());
    }
    let cases = request_get_json::<KnowledgeCaseListResponse>(
        "/api/v1/ops/knowledge/cases",
        api_key.clone(),
    )
    .await?
    .cases;
    let payload = json!({
        "claim_id": if claim_id.trim().is_empty() { Value::Null } else { Value::String(claim_id.trim().to_string()) },
        "diagnosis_code": diagnosis_code.trim(),
        "provider_region": provider_region.trim(),
        "tags": tags,
    });
    let results = request_json::<SimilarCaseSearchResponse>(
        "/api/v1/knowledge/search-similar",
        api_key,
        payload,
    )
    .await?
    .results;
    Ok(KnowledgeSnapshot { cases, results })
}

async fn get_agent_runs(api_key: String) -> Result<Vec<AgentRunRecord>, String> {
    Ok(
        request_get_json::<AgentRunListResponse>("/api/v1/ops/agent-runs", api_key)
            .await?
            .runs,
    )
}

async fn get_evidence_runtime_snapshot(
    api_key: String,
    selected_document_id: String,
) -> Result<EvidenceRuntimeSnapshot, String> {
    let documents = request_get_json::<EvidenceDocumentListResponse>(
        "/api/v1/ops/evidence/documents",
        api_key.clone(),
    )
    .await?
    .documents;
    let selected_document_id = selected_document_id.trim().to_string();
    let selected_document_id = if selected_document_id.is_empty() {
        documents
            .first()
            .map(|document| document.document_id.clone())
    } else {
        Some(selected_document_id)
    };
    let (chunks, ocr_outputs) = if let Some(document_id) = &selected_document_id {
        let chunks = request_get_json::<EvidenceDocumentChunkListResponse>(
            &format!("/api/v1/ops/evidence/documents/{document_id}/chunks"),
            api_key.clone(),
        )
        .await?
        .chunks;
        let ocr_outputs = request_get_json::<EvidenceOcrOutputListResponse>(
            &format!("/api/v1/ops/evidence/documents/{document_id}/ocr-outputs"),
            api_key.clone(),
        )
        .await?
        .ocr_outputs;
        (chunks, ocr_outputs)
    } else {
        (Vec::new(), Vec::new())
    };
    let embedding_jobs = request_get_json::<EvidenceEmbeddingJobListResponse>(
        "/api/v1/ops/evidence/embedding-jobs",
        api_key.clone(),
    )
    .await?
    .embedding_jobs;
    let retrieval_audit_events = request_get_json::<EvidenceRetrievalAuditEventListResponse>(
        "/api/v1/ops/evidence/retrieval-audit-events",
        api_key,
    )
    .await?
    .retrieval_audit_events;
    Ok(EvidenceRuntimeSnapshot {
        documents,
        selected_document_id,
        chunks,
        ocr_outputs,
        embedding_jobs,
        retrieval_audit_events,
    })
}

async fn post_evidence_demo_lifecycle(
    api_key: String,
    next_index: usize,
) -> Result<String, String> {
    let document_id = format!("web-doc-{next_index:03}");
    let chunk_id = format!("web-chunk-{next_index:03}");
    let ocr_output_id = format!("web-ocr-{next_index:03}");
    let embedding_job_id = format!("web-emb-{next_index:03}");
    let retrieval_id = format!("web-ret-{next_index:03}");
    let claim_id = "CLM-0287";

    let document_payload = json!({
        "document_id": document_id,
        "source_record_ref": format!("claim_documents:{claim_id}"),
        "claim_id": claim_id,
        "external_document_id": format!("TPA-DOC-{next_index:03}"),
        "document_type": "medical_record",
        "storage_uri": format!("s3://customer-approved/evidence/{document_id}.json"),
        "content_checksum": format!("sha256:{document_id}"),
        "ingestion_status": "registered",
        "redaction_status": "redacted",
        "retention_policy_id": "pilot-7y",
        "evidence_refs": [format!("claim_context:{claim_id}")],
        "metadata_json": {
            "demo_source": "web-console",
            "raw_text_present": false,
            "pii_masking": "required"
        }
    });
    let document = request_json::<EvidenceDocumentRecord>(
        "/api/v1/ops/evidence/documents",
        api_key.clone(),
        document_payload,
    )
    .await?;

    let chunk_payload = json!({
        "chunk_id": chunk_id,
        "chunk_index": 0,
        "chunking_version": "medical-record-v1",
        "redaction_status": "redacted",
        "text_checksum": format!("sha256:{chunk_id}"),
        "token_count": 128,
        "storage_uri": format!("s3://customer-approved/evidence/chunks/{chunk_id}.json"),
        "source_offsets_json": {"page": 1, "raw_text_present": false},
        "evidence_refs": [format!("evidence_documents:{}", document.document_id)]
    });
    let chunk = request_json::<EvidenceDocumentChunkRecord>(
        &format!(
            "/api/v1/ops/evidence/documents/{}/chunks",
            document.document_id
        ),
        api_key.clone(),
        chunk_payload,
    )
    .await?;

    let ocr_payload = json!({
        "ocr_output_id": ocr_output_id,
        "ocr_engine": "customer-ocr",
        "ocr_engine_version": "2026.06",
        "output_uri": format!("s3://customer-approved/evidence/ocr/{ocr_output_id}.json"),
        "output_checksum": format!("sha256:{ocr_output_id}"),
        "confidence_score": "0.94",
        "quality_status": "passed",
        "evidence_refs": [format!("evidence_documents:{}", document.document_id)]
    });
    request_json::<EvidenceOcrOutputRecord>(
        &format!(
            "/api/v1/ops/evidence/documents/{}/ocr-outputs",
            document.document_id
        ),
        api_key.clone(),
        ocr_payload,
    )
    .await?;

    let embedding_payload = json!({
        "embedding_job_id": embedding_job_id,
        "target_kind": "document_chunk",
        "target_ref": chunk.chunk_id,
        "embedding_model": "customer-approved-embedder",
        "embedding_model_version": "v1",
        "chunking_version": "medical-record-v1",
        "redaction_status": "redacted",
        "vector_store_kind": "pgvector",
        "vector_store_ref": format!("pgvector:evidence_chunks:{}", chunk.chunk_id),
        "embedding_checksum": format!("sha256:{embedding_job_id}"),
        "status": "queued",
        "evidence_refs": [format!("evidence_chunks:{}", chunk.chunk_id)]
    });
    request_json::<EvidenceEmbeddingJobRecord>(
        "/api/v1/ops/evidence/embedding-jobs",
        api_key.clone(),
        embedding_payload,
    )
    .await?;

    let retrieval_payload = json!({
        "retrieval_id": retrieval_id,
        "query_kind": "masked_claim_context",
        "query_checksum": format!("sha256:masked-query-{next_index:03}"),
        "retrieval_method": "vector_top_k",
        "embedding_model_version": "v1",
        "top_k": 5,
        "source_refs": [format!("claim_context:{claim_id}")],
        "result_refs": [format!("evidence_chunks:{}", chunk.chunk_id)],
        "redaction_status": "redacted",
        "evidence_refs": [format!("retrieval:{retrieval_id}")]
    });
    request_json::<EvidenceRetrievalAuditEventRecord>(
        "/api/v1/ops/evidence/retrieval-audit-events",
        api_key,
        retrieval_payload,
    )
    .await?;

    Ok(document.document_id)
}

async fn post_agent_investigation(
    api_key: String,
    payload: Value,
) -> Result<AgentInvestigationResponse, String> {
    request_json("/api/v1/agent/cases/investigate", api_key, payload).await
}

async fn get_governance_snapshot(
    api_key: String,
    event_group: String,
) -> Result<GovernanceSnapshot, String> {
    let health = request_get_json::<HealthResponse>("/api/v1/health", api_key.clone()).await?;
    let event_group = event_group.trim();
    let audit_path = if event_group.is_empty() {
        "/api/v1/ops/audit-events?limit=20".to_string()
    } else {
        format!("/api/v1/ops/audit-events?event_group={event_group}&limit=20")
    };
    let audit_events = request_get_json::<AuditEventListResponse>(&audit_path, api_key.clone())
        .await?
        .events;
    let api_calls =
        request_get_json::<ApiCallListResponse>("/api/v1/ops/api-calls?limit=20", api_key.clone())
            .await?
            .calls;
    let agent_runs = get_agent_runs(api_key).await?;
    Ok(GovernanceSnapshot {
        health,
        audit_events,
        api_calls,
        agent_runs,
    })
}

fn merge_payload_text(raw_payload: &str, overlay_payload: &str) -> Result<Value, String> {
    let mut payload = serde_json::from_str::<Value>(raw_payload)
        .map_err(|error| format!("raw payload JSON is invalid: {error}"))?;
    let overlay = serde_json::from_str::<Value>(overlay_payload)
        .map_err(|error| format!("correction overlay JSON is invalid: {error}"))?;
    merge_overlay(&mut payload, &overlay);
    Ok(payload)
}

fn merge_overlay(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(key) {
                    Some(base_value) => merge_overlay(base_value, value),
                    None => {
                        base.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (Value::Array(base), Value::Array(overlay)) => {
            for (index, value) in overlay.iter().enumerate() {
                if let Some(base_value) = base.get_mut(index) {
                    merge_overlay(base_value, value);
                } else {
                    base.push(value.clone());
                }
            }
        }
        (base, overlay) => *base = overlay.clone(),
    }
}

fn correction_hints_for(response: &InboxNormalizeResponse) -> Vec<CorrectionHint> {
    if response.scoring_ready {
        return Vec::new();
    }
    response
        .validation_errors
        .iter()
        .map(|error| CorrectionHint {
            field_path: error.field_path.clone(),
            severity: error.severity.clone(),
            blocks_scoring: blocks_direct_scoring(&error.field_path, &error.severity),
            next_action: next_action_for_validation_error(error),
        })
        .collect()
}

fn blocks_direct_scoring(field_path: &str, severity: &str) -> bool {
    if severity != "warning" || !field_path.starts_with("reportCase.policyList[") {
        return false;
    }
    if field_path.contains(".invoiceList[") {
        return false;
    }
    let field = field_path.rsplit('.').next().unwrap_or_default();
    if field_path.contains(".productList[") {
        matches!(field, "validateDate" | "claimValidateDate" | "expireDate")
    } else {
        matches!(field, "coverageLimit" | "validateDate" | "expireDate")
    }
}

fn next_action_for_validation_error(error: &InboxValidationError) -> String {
    if error.field_path == "systemCode" {
        return "use source-system/customer-scope config that matches the payload systemCode"
            .into();
    }
    if error.field_path.ends_with(".coverageLimit") {
        return "map the policy or liability coverage limit before risk queue release".into();
    }
    if error.field_path.ends_with(".validateDate")
        || error.field_path.ends_with(".expireDate")
        || error.field_path.ends_with(".claimValidateDate")
    {
        return "fix or reviewer-resolve the policy/product/liability date window before queue release"
            .into();
    }
    if error.field_path == "reportCase.calculateRisk" {
        return "keep the payload in the FWA audit path unless customer config explicitly allows bypass"
            .into();
    }
    if error.remediation.is_empty() {
        "review this field before queue release".into()
    } else {
        error.remediation.clone()
    }
}

fn correction_overlay_template_for(errors: &[InboxValidationError]) -> Value {
    let mut template = json!({});
    for error in errors {
        apply_overlay_template_field(&mut template, &error.field_path);
    }
    template
}

fn apply_overlay_template_field(template: &mut Value, field_path: &str) {
    let Some(after_policy) = field_path.strip_prefix("reportCase.policyList[") else {
        return;
    };
    let Some((policy_index, rest)) = consume_index(after_policy) else {
        return;
    };

    if matches!(rest, "coverageLimit" | "validateDate" | "expireDate") {
        set_policy_field(
            template,
            policy_index,
            rest,
            placeholder_for("policy", rest),
        );
        return;
    }

    let Some(after_product) = rest.strip_prefix("productList[") else {
        return;
    };
    let Some((product_index, rest)) = consume_index(after_product) else {
        return;
    };
    if matches!(rest, "validateDate" | "expireDate" | "claimValidateDate") {
        set_product_field(
            template,
            policy_index,
            product_index,
            rest,
            placeholder_for("product", rest),
        );
        return;
    }

    let Some(after_liability) = rest.strip_prefix("claimLiabilityList[") else {
        return;
    };
    let Some((liability_index, rest)) = consume_index(after_liability) else {
        return;
    };
    if matches!(rest, "validateDate" | "expireDate" | "claimValidateDate") {
        set_liability_field(
            template,
            policy_index,
            product_index,
            liability_index,
            rest,
            placeholder_for("liability", rest),
        );
    }
}

fn consume_index(value: &str) -> Option<(usize, &str)> {
    let (index, rest) = value.split_once("].")?;
    Some((index.parse().ok()?, rest))
}

fn set_policy_field(template: &mut Value, policy_index: usize, field: &str, value: Value) {
    let policy = policy_template(template, policy_index);
    ensure_object(policy).insert(field.into(), value);
}

fn set_product_field(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    field: &str,
    value: Value,
) {
    let product = product_template(template, policy_index, product_index);
    ensure_object(product).insert(field.into(), value);
}

fn set_liability_field(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    liability_index: usize,
    field: &str,
    value: Value,
) {
    let liability = liability_template(template, policy_index, product_index, liability_index);
    ensure_object(liability).insert(field.into(), value);
}

fn policy_template(template: &mut Value, policy_index: usize) -> &mut Value {
    let report_case = ensure_object(template)
        .entry("reportCase")
        .or_insert_with(|| json!({}));
    let policies = ensure_object(report_case)
        .entry("policyList")
        .or_insert_with(|| json!([]));
    let policies = ensure_array(policies);
    while policies.len() <= policy_index {
        policies.push(json!({}));
    }
    &mut policies[policy_index]
}

fn product_template(template: &mut Value, policy_index: usize, product_index: usize) -> &mut Value {
    let policy = policy_template(template, policy_index);
    let products = ensure_object(policy)
        .entry("productList")
        .or_insert_with(|| json!([]));
    let products = ensure_array(products);
    while products.len() <= product_index {
        products.push(json!({}));
    }
    &mut products[product_index]
}

fn liability_template(
    template: &mut Value,
    policy_index: usize,
    product_index: usize,
    liability_index: usize,
) -> &mut Value {
    let product = product_template(template, policy_index, product_index);
    let liabilities = ensure_object(product)
        .entry("claimLiabilityList")
        .or_insert_with(|| json!([]));
    let liabilities = ensure_array(liabilities);
    while liabilities.len() <= liability_index {
        liabilities.push(json!({}));
    }
    &mut liabilities[liability_index]
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    value
        .as_object_mut()
        .expect("value was converted to object")
}

fn ensure_array(value: &mut Value) -> &mut Vec<Value> {
    if !value.is_array() {
        *value = json!([]);
    }
    value.as_array_mut().expect("value was converted to array")
}

fn placeholder_for(scope: &str, field: &str) -> Value {
    if field == "coverageLimit" {
        return Value::String("<REQUIRED_COVERAGE_LIMIT>".into());
    }
    let mut label = String::new();
    for (index, character) in field.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            label.push('_');
        }
        label.push(character.to_ascii_uppercase());
    }
    Value::String(format!(
        "<REQUIRED_{}_{}_EPOCH_MS>",
        scope.to_ascii_uppercase(),
        label
    ))
}

fn source_system_from_context(context: &Value) -> String {
    context
        .pointer("/claim_header/source_system")
        .and_then(Value::as_str)
        .unwrap_or("AiClaim Core")
        .to_string()
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
}

fn display_value(value: &Value) -> String {
    value
        .as_f64()
        .map(|number| format!("{number:.1}"))
        .or_else(|| value.as_str().map(str::to_string))
        .unwrap_or_else(|| value.to_string())
}

fn numeric_value(value: &Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        .unwrap_or(0.0)
}

fn readable_token(value: &str) -> String {
    value.replace(['_', '-'], " ")
}

fn titleize_token(value: &str) -> String {
    let readable = readable_token(value.trim());
    if readable.is_empty() {
        return "None".into();
    }
    readable
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| {
                    format!(
                        "{}{}",
                        first.to_ascii_uppercase(),
                        characters.as_str().to_ascii_lowercase()
                    )
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn business_label(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => "None".into(),
        "red" => "High risk".into(),
        "amber" | "yellow" => "Watchlist risk".into(),
        "green" => "Low risk".into(),
        "manual_review" | "review" => "Manual review".into(),
        "request_evidence" | "request_more_evidence" => "Request evidence".into(),
        "open_case" => "Open case".into(),
        "reject_lead" => "Reject lead".into(),
        "merge_lead" => "Merge lead".into(),
        "pre_payment" => "Pre-payment review".into(),
        "post_payment" => "Post-payment review".into(),
        "pending_evidence" => "Waiting evidence".into(),
        "evidence_pending" => "Evidence pending".into(),
        "evidence_sufficient" | "clinical_evidence_sufficient" => "Evidence sufficient".into(),
        "insufficient_evidence" => "Insufficient evidence".into(),
        "documentation_issue" => "Documentation issue".into(),
        "medical_necessity_review_required" => "Medical review required".into(),
        "medical_necessity_issue" => "Medical necessity issue".into(),
        "no_medical_issue" => "No medical issue".into(),
        "no_auto_denial" => "No automatic denial".into(),
        "assistive_only" => "Assistive only".into(),
        "approved" => "Approved".into(),
        "approved_for_training" => "Approved for training".into(),
        "blocked" => "Blocked".into(),
        "breached" => "Over SLA".into(),
        "closed" => "Closed".into(),
        "completed" => "Completed".into(),
        "confirmed" => "Confirmed".into(),
        "created" => "Created".into(),
        "done" => "Done".into(),
        "error" => "Error".into(),
        "failed" => "Failed".into(),
        "hold" | "held" => "Held for review".into(),
        "investigating" => "Investigating".into(),
        "new" => "New".into(),
        "ok" | "passed" | "valid" => "Passed".into(),
        "on_track" => "On track".into(),
        "open" => "Open".into(),
        "pending" => "Pending".into(),
        "queued" => "Queued".into(),
        "ready" | "scoring_ready" => "Ready".into(),
        "received" => "Received".into(),
        "rejected" => "Rejected".into(),
        "triage" => "Triage".into(),
        other if other.starts_with("completed") => "Completed".into(),
        _ => titleize_token(value),
    }
}

fn rag_label(value: &str) -> &'static str {
    match value.trim().to_ascii_uppercase().as_str() {
        "RED" => "High risk",
        "AMBER" | "YELLOW" => "Watchlist risk",
        "GREEN" => "Low risk",
        _ => "Risk pending",
    }
}

fn inbox_pipeline_visual(response: &InboxNormalizeResponse) -> Html {
    let has_blockers = response
        .validation_errors
        .iter()
        .any(|error| blocks_direct_scoring(&error.field_path, &error.severity));
    let finding_state = if has_blockers {
        "blocked"
    } else if response.validation_errors.is_empty() {
        "done"
    } else {
        "warning"
    };
    let approval_state = if response.scoring_ready {
        "done"
    } else if has_blockers {
        "blocked"
    } else {
        "warning"
    };
    html! {
        <div class="inbox-pipeline">
            {pipeline_step("Raw", response.external_message_id.as_deref().unwrap_or("message pending"), "done")}
            {pipeline_step("Normalize", &response.mapping_version, "done")}
            {pipeline_step("Findings", &format!("{} findings", response.validation_errors.len()), finding_state)}
            {pipeline_step("Approval", if response.scoring_ready { "queue release" } else { "review gate" }, approval_state)}
            {pipeline_step("Release", if response.scoring_ready { "ready" } else { "held" }, if response.scoring_ready { "done" } else { "pending" })}
        </div>
    }
}

fn validation_findings_visual(response: &InboxNormalizeResponse, hints: &[CorrectionHint]) -> Html {
    let blocking_count = hints.iter().filter(|hint| hint.blocks_scoring).count();
    let warning_count = response
        .validation_errors
        .iter()
        .filter(|error| error.severity == "warning")
        .count();
    let error_count = response
        .validation_errors
        .iter()
        .filter(|error| error.severity == "error")
        .count();
    html! {
        <div class="finding-command-strip">
            <div>
                <span>{"Blocking"}</span>
                <strong>{blocking_count}</strong>
                <small>{"must resolve or reviewer approve"}</small>
            </div>
            <div>
                <span>{"Warnings"}</span>
                <strong>{warning_count}</strong>
                <small>{"allowed with audit trail"}</small>
            </div>
            <div>
                <span>{"Errors"}</span>
                <strong>{error_count}</strong>
                <small>{"block canonical scoring"}</small>
            </div>
            <div>
                <span>{"Data Quality"}</span>
                <strong>{response.data_quality_signals.len()}</strong>
                <small>{refs_label(&response.data_quality_signals)}</small>
            </div>
        </div>
    }
}

fn pipeline_step(label: &str, value: &str, state: &str) -> Html {
    html! {
        <div class={classes!("pipeline-step", state.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn optional_number(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "none".into())
}

fn issue_counts_label(counts: &Map<String, Value>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(key, value)| format!("{key}={}", display_value(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn map_counts_label(counts: &BTreeMap<String, u32>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn map_counts_business_label(counts: &BTreeMap<String, u32>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(key, value)| format!("{}={value}", business_label(key)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn percent_label(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn optional_u32(value: Option<u32>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|error| format!("{label} must be an unsigned integer: {error}"))
}

fn parse_risk_score(value: &str) -> Result<u8, String> {
    let score = value
        .trim()
        .parse::<u8>()
        .map_err(|error| format!("risk score must be an integer from 0 to 100: {error}"))?;
    if score > 100 {
        return Err("risk score must be between 0 and 100".into());
    }
    Ok(score)
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

fn optional_metric(value: &Option<Value>) -> String {
    value
        .as_ref()
        .map(display_value)
        .unwrap_or_else(|| "none".into())
}

fn optional_u8(value: Option<u8>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

fn value_refs_label(refs: &[Value]) -> String {
    if refs.is_empty() {
        return "none".into();
    }
    refs.iter()
        .map(display_value)
        .collect::<Vec<_>>()
        .join(", ")
}

fn required_evidence_label(items: &[RuntimeRequiredEvidence]) -> String {
    items
        .iter()
        .map(|item| {
            let mut label = item.evidence_type.clone();
            if let Some(request_type) = item.evidence_request_type.as_deref() {
                label = format!("{label} / {request_type}");
            }
            if item.blocking {
                label.push_str(" / blocking");
            }
            if let Some(authority_ref) = item.policy_authority_ref.as_deref() {
                label = format!("{label} / {authority_ref}");
            }
            if let Some(exception_check) = item.exception_check.as_deref() {
                label = format!("{label} / {exception_check}");
            }
            label
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn runtime_score_breakdown(response: &ScoreResponse) -> Html {
    if let Some(scores) = &response.scores {
        html! {
            <div class="risk-flow signal-score-grid">
                {risk_node("Peer", "Deviation", &scores.peer_deviation_score.to_string(), "claim amount / stay / frequency")}
                {risk_node("Rules", "Controls", &scores.rule_score.to_string(), "deterministic policy checks")}
                {risk_node("Anomaly", "Pattern", &scores.anomaly_score.to_string(), "rare utilization behavior")}
                {risk_node("Model", "Classifier", &scores.ml_score.to_string(), "trained runtime score")}
                {risk_node("Clinical", "Necessity", &scores.medical_reasonableness_score.to_string(), "medical reasonableness")}
                {risk_node("Provider", "Network", &scores.provider_network_score.to_string(), "relationship and graph risk")}
                {risk_node("Knowledge", "Similar cases", &scores.similar_case_score.to_string(), "confirmed case memory")}
                {risk_node("Route", "Policy score", &scores.final_score.to_string(), "downstream human queue")}
            </div>
        }
    } else {
        html! { <p class="empty">{"No score breakdown returned."}</p> }
    }
}

fn kpi_card(label: &str, value: &str, icon: &str) -> Html {
    html! {
        <div class="visual-kpi">
            <span class={classes!("visual-icon", icon_class(icon))}></span>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn operator_queue_snapshot(summary: &DashboardSummary, on_navigate: &Callback<String>) -> Html {
    html! {
        <div class="visual-panel wide-visual operator-queue-panel">
            <div class="panel-heading-row">
                <h4>{"Next actions"}</h4>
                <span class="status-token strong">{"click to work"}</span>
            </div>
            <div class="operator-queue">
                {operator_queue_card("Triage", &summary.suspected_claims.to_string(), "suspected leads", "Leads & Cases", "danger", on_navigate)}
                {operator_queue_card("Investigate", &summary.case_sla.open_cases.to_string(), "open cases", "Leads & Cases", "warning", on_navigate)}
                {operator_queue_card("Review", &summary.qa_queue.open_cases.to_string(), "open QA samples", "Review Workbench", "strong", on_navigate)}
                {operator_queue_card("Govern", &percent_label(summary.audit_coverage.canonical_trace_coverage), "trace coverage", "Governance", "success", on_navigate)}
            </div>
            {dashboard_operations_map(summary)}
        </div>
    }
}

fn operator_queue_card(
    action: &str,
    value: &str,
    metric: &str,
    target: &str,
    tone: &str,
    on_navigate: &Callback<String>,
) -> Html {
    let target = target.to_string();
    let target_label = target.clone();
    let on_navigate = on_navigate.clone();
    html! {
        <button
            class={classes!("operator-queue-card", tone.to_string())}
            onclick={Callback::from(move |_| on_navigate.emit(target.clone()))}
        >
            <span>{action}</span>
            <strong>{value}</strong>
            <small>{metric}</small>
            <em>{target_label}</em>
        </button>
    }
}

fn dashboard_operations_map(summary: &DashboardSummary) -> Html {
    let review_label = format!(
        "{} cases / {} QA",
        summary.case_sla.open_cases, summary.qa_queue.open_cases
    );
    let engine_label = format!(
        "rules + risk mix: {} / {}",
        summary.rule_hits,
        map_counts_business_label(&summary.rag_distribution)
    );
    html! {
        <div class="ops-system-map-shell">
            <div class="panel-heading-row compact-heading-row">
                <h4>{"FWA operating map"}</h4>
                <span class="status-token strong">{"PRD runtime topology"}</span>
            </div>
            <div class="ops-system-map">
                {ops_map_node("TPA", "claim intake", &summary.suspected_claims.to_string(), "source")}
                <div class="ops-map-core">
                    <span>{"Detect"}</span>
                    <strong>{"Risk scoring service"}</strong>
                    <small>{engine_label}</small>
                </div>
                {ops_map_node("Review", "human queue", &review_label, "qa")}
                {ops_map_node("Evidence", "assistive pack", &format!("{} runs", summary.agent_governance.total_runs), "agent")}
                {ops_map_node("Audit", "trace + approval", &percent_label(summary.audit_coverage.canonical_trace_coverage), "audit")}
                {ops_map_node("Savings", "confirmation gate", &summary.saving_amount, "roi")}
            </div>
        </div>
    }
}

fn ops_map_node(label: &str, caption: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("ops-map-node", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{caption}</small>
        </div>
    }
}

fn risk_node(layer: &str, label: &str, value: &str, caption: &str) -> Html {
    html! {
        <div class="risk-node">
            <span class="risk-node-badge">{layer}</span>
            <strong>{value}</strong>
            <span>{label}</span>
            <small>{caption}</small>
        </div>
    }
}

fn distribution_bars(title: &str, counts: &BTreeMap<String, u32>) -> Html {
    if counts.is_empty() {
        return html! {
            <div class="visual-panel">
                <h4>{title}</h4>
                <p class="empty">{"No distribution records."}</p>
            </div>
        };
    }
    let max_count = counts.values().copied().max().unwrap_or(1);
    html! {
        <div class="visual-panel">
            <h4>{title}</h4>
            <div class="bar-stack">
                {for counts.iter().map(|(label, count)| {
                    let width = scaled_width(*count, max_count);
                    html! {
                        <div class="bar-row">
                            <span>{business_label(label)}</span>
                            <div class="bar-track"><i style={format!("width: {width};")}></i></div>
                            <strong>{count}</strong>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}

fn rule_performance_visual(performance: &[RulePerformance]) -> Html {
    if performance.is_empty() {
        return html! {};
    }
    let max_trigger_count = performance
        .iter()
        .map(|item| item.trigger_count)
        .max()
        .unwrap_or(1);
    html! {
        <div class="visual-panel wide-visual">
            <h4>{"Rule command path"}</h4>
            <div class="rule-bars">
                {for performance.iter().take(6).map(|item| html! {
                    <div class="rule-bar-row">
                        <div>
                            <strong>{&item.rule_id}</strong>
                            <span>{&item.alert_code}</span>
                        </div>
                        <div class="bar-track">
                            <i style={format!("width: {};", scaled_width(item.trigger_count, max_trigger_count))}></i>
                        </div>
                        <div class="dual-meter">
                            <span style={format!("width: {};", percent_width(item.precision))}></span>
                            <em style={format!("width: {};", percent_width(item.false_positive_rate))}></em>
                        </div>
                        <small>{format!("precision {} / FP {}", percent_label(item.precision), percent_label(item.false_positive_rate))}</small>
                    </div>
                })}
            </div>
        </div>
    }
}

fn rule_pack_matrix(snapshot: &RuleOpsSnapshot) -> Html {
    let total_rules = snapshot.rules.len();
    let active_rules = snapshot
        .rules
        .iter()
        .filter(|rule| rule.status == "active")
        .count();
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"FWA Rule Pack Matrix"}</h3>
                    <p>{"Productized rule families for the pilot demo: each family shows current coverage from the live rule library and operational performance."}</p>
                </div>
                <span class="status-token strong">{"rule pack"}</span>
            </div>
            <div class="rule-pack-cockpit">
                <aside class="rule-pack-brief">
                    <span class="eyebrow">{"PRD rule coverage"}</span>
                    <strong>{format!("{} active / {} listed", active_rules, total_rules)}</strong>
                    <small>{"Deterministic rules stay explainable, versioned, backtested, and human-approved before production routing."}</small>
                    <div class="rule-pack-meter">
                        <i style={format!("width: {};", percent_width(rule_pack_coverage_ratio(snapshot))) }></i>
                    </div>
                    <small>{format!("covered families: {} / 5", covered_rule_pack_count(snapshot))}</small>
                </aside>
                <div class="rule-pack-map">
                    <div class="rule-pack-link"></div>
                    <div class="rule-pack-core">
                        <span>{"L2"}</span>
                        <strong>{"Rule engine"}</strong>
                    </div>
                    {rule_pack_family_node(snapshot, "duplicate billing", "same service / amount", "duplicate", "top")}
                    {rule_pack_family_node(snapshot, "early high-value claim", "new policy + high amount", "early", "right")}
                    {rule_pack_family_node(snapshot, "provider peer outlier", "provider cohort deviation", "provider", "bottom")}
                    {rule_pack_family_node(snapshot, "diagnosis-procedure mismatch", "coding consistency", "diagnosis", "left")}
                    {rule_pack_family_node(snapshot, "medical necessity evidence gap", "chart support required", "medical", "lower-right")}
                </div>
                <aside class="rule-pack-legend">
                    <span class="eyebrow">{"Human-safe lifecycle"}</span>
                    {rule_pack_lifecycle_row("Draft", "sandbox / backtest", "neutral")}
                    {rule_pack_lifecycle_row("Review", "QA + false positives", "warning")}
                    {rule_pack_lifecycle_row("Approve", "owner sign-off", "strong")}
                    {rule_pack_lifecycle_row("Route", "recommend review only", "danger")}
                </aside>
            </div>
        </section>
    }
}

fn rule_pack_family_node(
    snapshot: &RuleOpsSnapshot,
    label: &'static str,
    caption: &'static str,
    family_key: &'static str,
    position: &'static str,
) -> Html {
    let rules = snapshot
        .rules
        .iter()
        .filter(|rule| rule_matches_family(rule, family_key))
        .collect::<Vec<_>>();
    let rule_count = rules.len();
    let trigger_count = rules
        .iter()
        .filter_map(|rule| rule_performance_for(&snapshot.performance, &rule.rule_id))
        .map(|performance| performance.trigger_count)
        .sum::<u32>();
    let precision = rules
        .iter()
        .filter_map(|rule| rule_performance_for(&snapshot.performance, &rule.rule_id))
        .map(|performance| performance.precision)
        .next();
    let tone = if rule_count > 0 { "covered" } else { "gap" };
    html! {
        <div class={classes!("rule-pack-node", position, tone)}>
            <span>{label}</span>
            <strong>{if rule_count > 0 { format!("{rule_count} rules") } else { "gap".into() }}</strong>
            <small>{caption}</small>
            <em>{format!("triggers {} / precision {}", trigger_count, precision.map(percent_label).unwrap_or_else(|| "n/a".into()))}</em>
        </div>
    }
}

fn rule_pack_lifecycle_row(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("provider-signal-row", tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn covered_rule_pack_count(snapshot: &RuleOpsSnapshot) -> usize {
    ["duplicate", "early", "provider", "diagnosis", "medical"]
        .iter()
        .filter(|family| {
            snapshot
                .rules
                .iter()
                .any(|rule| rule_matches_family(rule, family))
        })
        .count()
}

fn rule_pack_coverage_ratio(snapshot: &RuleOpsSnapshot) -> f64 {
    covered_rule_pack_count(snapshot) as f64 / 5.0
}

fn rule_matches_family(rule: &RuleSummary, family_key: &str) -> bool {
    let haystack = format!(
        "{} {} {} {} {}",
        rule.rule_id,
        rule.name,
        rule.scheme_family,
        rule.alert_code,
        rule.applicability_scope.scheme_family
    )
    .to_lowercase();
    match family_key {
        "duplicate" => contains_any(&haystack, &["duplicate", "repeat", "same_service"]),
        "early" => contains_any(
            &haystack,
            &["early", "high_amount", "high_value", "short_term"],
        ),
        "provider" => contains_any(&haystack, &["provider", "peer", "outlier", "cohort"]),
        "diagnosis" => contains_any(&haystack, &["diagnosis", "procedure", "mismatch", "coding"]),
        "medical" => contains_any(
            &haystack,
            &["medical", "necessity", "evidence_gap", "documentation"],
        ),
        _ => false,
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn rule_discovery_payload(
    model_key: &UseStateHandle<String>,
    model_version: &UseStateHandle<String>,
    explanation_feature: &UseStateHandle<String>,
    explanation_contribution: f64,
    feature_importance_uri: &UseStateHandle<String>,
    dataset_uri: &UseStateHandle<String>,
    label_column: &UseStateHandle<String>,
    claim_id_column: &UseStateHandle<String>,
    feature_fields: &UseStateHandle<String>,
    tree_depth: &UseStateHandle<String>,
    samples: Vec<Value>,
) -> Value {
    let candidate_feature_fields = comma_separated_values(feature_fields);
    let max_tree_depth = tree_depth.trim().parse::<usize>().unwrap_or(2);
    json!({
        "min_support": 1,
        "max_candidates": 8,
        "max_tree_depth": max_tree_depth,
        "source_model_key": (**model_key).clone(),
        "source_model_version": (**model_version).clone(),
        "feature_importance_uri": (**feature_importance_uri).clone(),
        "dataset_uri": (**dataset_uri).clone(),
        "label_column": (**label_column).clone(),
        "claim_id_column": (**claim_id_column).clone(),
        "candidate_feature_fields": candidate_feature_fields,
        "min_abs_contribution": 0.1,
        "model_explanations": [
            {
                "feature": (**explanation_feature).clone(),
                "direction": "increases_risk",
                "contribution": explanation_contribution,
                "reason": "Operations Studio candidate explanation input"
            }
        ],
        "samples": samples
    })
}

fn rule_backtest_payload(
    rule: Value,
    dataset_uri: &UseStateHandle<String>,
    label_column: &UseStateHandle<String>,
    claim_id_column: &UseStateHandle<String>,
    samples: Vec<Value>,
) -> Value {
    json!({
        "rule": rule,
        "dataset_uri": (**dataset_uri).clone(),
        "label_column": (**label_column).clone(),
        "claim_id_column": (**claim_id_column).clone(),
        "samples": samples,
        "expected_review_capacity": 10
    })
}

fn comma_separated_values(input: &UseStateHandle<String>) -> Vec<String> {
    (**input)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_json_array(input: &str, label: &str) -> Result<Vec<Value>, String> {
    match serde_json::from_str::<Value>(input.trim()) {
        Ok(Value::Array(items)) if !items.is_empty() => Ok(items),
        Ok(Value::Array(_)) => Err(format!("{label} must include at least one sample")),
        Ok(_) => Err(format!("{label} must be a JSON array")),
        Err(error) => Err(format!("{label} JSON is invalid: {error}")),
    }
}

fn parse_optional_json_array(input: &str, label: &str) -> Result<Vec<Value>, String> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    match serde_json::from_str::<Value>(input.trim()) {
        Ok(Value::Array(items)) => Ok(items),
        Ok(_) => Err(format!("{label} must be a JSON array")),
        Err(error) => Err(format!("{label} JSON is invalid: {error}")),
    }
}

fn parse_json_object(input: &str, label: &str) -> Result<Value, String> {
    match serde_json::from_str::<Value>(input.trim()) {
        Ok(value @ Value::Object(_)) => Ok(value),
        Ok(_) => Err(format!("{label} must be a JSON object")),
        Err(error) => Err(format!("{label} JSON is invalid: {error}")),
    }
}

fn json_string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn json_metric_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| {
        value
            .as_str()
            .map(str::to_string)
            .or_else(|| value.as_f64().map(|number| number.to_string()))
    })
}

fn parse_optional_unit_metric(input: &str, label: &str) -> Result<Option<String>, String> {
    let value = input.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<f64>()
        .map_err(|error| format!("{label} must be a decimal between 0 and 1: {error}"))?;
    if !(0.0..=1.0).contains(&parsed) {
        return Err(format!("{label} must be between 0 and 1"));
    }
    Ok(Some(value.to_string()))
}

fn optional_trimmed_value(input: &str) -> Option<String> {
    let value = input.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn response_retraining_job_id(response: &Value) -> Option<String> {
    response
        .get("job_id")
        .and_then(Value::as_str)
        .or_else(|| {
            response
                .get("job")
                .and_then(|job| job.get("job_id"))
                .and_then(Value::as_str)
        })
        .map(str::to_string)
}

async fn accept_rule_candidate(
    api_key: String,
    rule: Value,
    reviewer: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<Value, String> {
    if evidence_refs.is_empty() {
        return Err("rule review actions require evidence refs".into());
    }
    if reviewer.trim().is_empty() {
        return Err("reviewer is required".into());
    }
    if notes.trim().is_empty() {
        return Err("human review notes are required".into());
    }

    request_json::<Value>(
        "/api/v1/ops/rules/candidate-reviews",
        api_key,
        json!({
            "decision": "accepted",
            "reviewer": reviewer.trim(),
            "notes": notes.trim(),
            "evidence_refs": evidence_refs,
            "rule": rule,
        }),
    )
    .await
}

async fn save_rule_candidate_draft(
    api_key: String,
    rule: Value,
    owner: Option<String>,
) -> Result<Value, String> {
    request_json::<Value>(
        "/api/v1/ops/rules/candidates",
        api_key,
        json!({
            "owner": owner,
            "rule": rule,
        }),
    )
    .await
}

async fn reject_rule_candidate(
    api_key: String,
    rule: Value,
    reviewer: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<Value, String> {
    if evidence_refs.is_empty() {
        return Err("rule review actions require evidence refs".into());
    }
    if reviewer.trim().is_empty() {
        return Err("reviewer is required".into());
    }
    if notes.trim().is_empty() {
        return Err("human review notes are required".into());
    }

    request_json::<Value>(
        "/api/v1/ops/rules/candidate-reviews",
        api_key,
        json!({
            "decision": "rejected",
            "reviewer": reviewer.trim(),
            "notes": notes.trim(),
            "evidence_refs": evidence_refs,
            "rule": rule,
        }),
    )
    .await
}

fn response_rule_id(response: &Value) -> Option<String> {
    response
        .get("saved_draft_rule_id")
        .and_then(Value::as_str)
        .or_else(|| {
            response
                .get("summary")
                .and_then(|summary| summary.get("rule_id"))
                .and_then(Value::as_str)
        })
        .map(str::to_string)
}

async fn submit_rule_shadow_run(
    api_key: String,
    rule_id: String,
    rule_version: u32,
    backtest: RuleBacktestResponse,
    report_uri: String,
    reviewer: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<Value, String> {
    if reviewer.trim().is_empty() {
        return Err("reviewer is required".into());
    }
    if notes.trim().is_empty() {
        return Err("shadow review notes are required".into());
    }
    if evidence_refs.is_empty() {
        return Err("shadow evidence requires evidence refs".into());
    }

    request_json::<Value>(
        &format!("/api/v1/ops/rules/{rule_id}/shadow-runs"),
        api_key,
        json!({
            "rule_version": rule_version,
            "reviewed_count": backtest.reviewed_count,
            "matched_count": backtest.matched_count,
            "false_positive_count": backtest.false_positive_count,
            "false_positive_rate": backtest.false_positive_rate,
            "report_uri": report_uri,
            "decision": if backtest.blockers.is_empty() { "shadow_passed" } else { "shadow_blocked" },
            "reviewer": reviewer.trim(),
            "notes": notes.trim(),
            "blockers": backtest.blockers,
            "evidence_refs": evidence_refs,
        }),
    )
    .await
}

async fn submit_anomaly_candidate_review(
    api_key: String,
    candidate_kind: String,
    candidate_id: String,
    source_report_uri: String,
    decision: String,
    reviewer: String,
    notes: String,
    evidence_refs: Vec<String>,
    candidate_payload: Value,
) -> Result<Value, String> {
    request_json::<Value>(
        "/api/v1/ops/providers/anomaly-candidate-reviews",
        api_key,
        json!({
            "candidate_kind": candidate_kind.trim(),
            "candidate_id": candidate_id.trim(),
            "source_report_uri": source_report_uri.trim(),
            "decision": decision.trim(),
            "reviewer": reviewer.trim(),
            "notes": notes.trim(),
            "evidence_refs": evidence_refs,
            "candidate_payload": candidate_payload,
        }),
    )
    .await
}

fn rule_demo_samples() -> Vec<Value> {
    vec![
        json!({
            "external_claim_id": "CLM-RULE-DEMO-TP",
            "claim_amount": "9000",
            "currency": "CNY",
            "service_date": "2026-01-05",
            "confirmed_fwa": true,
            "policy": {
                "external_policy_id": "POL-RULE-DEMO-TP",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
            }
        }),
        json!({
            "external_claim_id": "CLM-RULE-DEMO-TN",
            "claim_amount": "500",
            "currency": "CNY",
            "service_date": "2026-03-01",
            "confirmed_fwa": false,
            "policy": {
                "external_policy_id": "POL-RULE-DEMO-TN",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
            }
        }),
        json!({
            "external_claim_id": "CLM-RULE-DEMO-FN",
            "claim_amount": "6800",
            "currency": "CNY",
            "service_date": "2026-02-04",
            "confirmed_fwa": true,
            "policy": {
                "external_policy_id": "POL-RULE-DEMO-FN",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "12000"
            }
        }),
    ]
}

fn selected_rule_candidate<'a>(
    response: &'a RuleDiscoveryResponse,
    selected_candidate_id: &UseStateHandle<String>,
) -> Option<&'a RuleDiscoveryCandidate> {
    let selected_id = (**selected_candidate_id).as_str();
    response
        .candidates
        .iter()
        .find(|candidate| rule_candidate_id(candidate) == selected_id)
        .or_else(|| response.candidates.first())
}

fn rule_candidate_id(candidate: &RuleDiscoveryCandidate) -> String {
    candidate
        .rule
        .get("rule_id")
        .and_then(Value::as_str)
        .unwrap_or("candidate_rule")
        .to_string()
}

fn rule_candidate_version(candidate: &RuleDiscoveryCandidate) -> u32 {
    candidate
        .rule
        .get("version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .filter(|version| *version > 0)
        .unwrap_or(1)
}

fn push_unique(mut values: Vec<String>, value: String) -> Vec<String> {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
    values
}

fn remove_id(values: Vec<String>, value: &str) -> Vec<String> {
    values
        .into_iter()
        .filter(|existing| existing != value)
        .collect()
}

fn rule_backfill_pipeline(
    discovery_state: &UseStateHandle<ApiState<RuleDiscoveryResponse>>,
    backtest_state: &UseStateHandle<ApiState<RuleBacktestResponse>>,
    save_state: &UseStateHandle<ApiState<Value>>,
    shadow_state: &UseStateHandle<ApiState<Value>>,
    review_state: &UseStateHandle<ApiState<Value>>,
) -> Html {
    let nodes = [
        (
            "Discover",
            matches!(&**discovery_state, ApiState::Ready(_)),
            state_label(discovery_state),
        ),
        (
            "Backtest",
            matches!(&**backtest_state, ApiState::Ready(_)),
            state_label(backtest_state),
        ),
        (
            "Draft",
            matches!(&**save_state, ApiState::Ready(_)),
            state_label(save_state),
        ),
        (
            "Shadow",
            matches!(&**shadow_state, ApiState::Ready(_)),
            state_label(shadow_state),
        ),
        (
            "Review",
            matches!(&**review_state, ApiState::Ready(_)),
            state_label(review_state),
        ),
    ];
    gate_pipeline("Candidate rule workflow", &nodes)
}

fn state_label<T>(state: &UseStateHandle<ApiState<T>>) -> &'static str
where
    T: Clone + PartialEq + 'static,
{
    match &**state {
        ApiState::Idle => "pending",
        ApiState::Loading => "running",
        ApiState::Ready(_) => "ready",
        ApiState::Failed(_) => "blocked",
    }
}

fn rule_candidate_workflow(
    discovery_state: &UseStateHandle<ApiState<RuleDiscoveryResponse>>,
    backtest_state: &UseStateHandle<ApiState<RuleBacktestResponse>>,
    save_state: &UseStateHandle<ApiState<Value>>,
    shadow_state: &UseStateHandle<ApiState<Value>>,
    selected_candidate_id: &UseStateHandle<String>,
    accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    shadowed_candidate_ids: &UseStateHandle<Vec<String>>,
    final_accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    rejected_candidate_ids: &UseStateHandle<Vec<String>>,
) -> Html {
    html! {
        <div class="rule-candidate-workflow">
            {rule_discovery_candidates_view(
                discovery_state,
                selected_candidate_id,
                accepted_candidate_ids,
                shadowed_candidate_ids,
                final_accepted_candidate_ids,
                rejected_candidate_ids,
            )}
            {rule_backtest_view(backtest_state)}
            {rule_save_view(save_state)}
            {rule_shadow_run_state(shadow_state)}
        </div>
    }
}

fn rule_discovery_candidates_view(
    discovery_state: &UseStateHandle<ApiState<RuleDiscoveryResponse>>,
    selected_candidate_id: &UseStateHandle<String>,
    accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    shadowed_candidate_ids: &UseStateHandle<Vec<String>>,
    final_accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    rejected_candidate_ids: &UseStateHandle<Vec<String>>,
) -> Html {
    match &**discovery_state {
        ApiState::Idle => {
            html! { <p class="empty">{"Run discovery to generate governed rule candidates from explainable model signals."}</p> }
        }
        ApiState::Loading => html! { <p>{"Discovering candidate rules..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="result-stack">
                <div class="summary-grid">
                    <div><span>{"Samples"}</span><strong>{response.sample_count}</strong></div>
                    <div><span>{"Positive Labels"}</span><strong>{response.positive_count}</strong></div>
                    <div><span>{"Candidates"}</span><strong>{response.candidates.len()}</strong></div>
                </div>
                <div class="factor-card-grid">
                    {for response.candidates.iter().map(|candidate| {
                        let candidate_id = rule_candidate_id(candidate);
                        let is_selected = candidate_id == **selected_candidate_id;
                        let review_status = candidate_review_label(
                            &candidate_id,
                            accepted_candidate_ids,
                            shadowed_candidate_ids,
                            final_accepted_candidate_ids,
                            rejected_candidate_ids,
                        );
                        let review_tone = candidate_review_tone(
                            &candidate_id,
                            accepted_candidate_ids,
                            shadowed_candidate_ids,
                            final_accepted_candidate_ids,
                            rejected_candidate_ids,
                        );
                        let selected_candidate_id = selected_candidate_id.clone();
                        let candidate_id_for_click = candidate_id.clone();
                        html! {
                            <button
                                class={classes!("rule-candidate-card", review_tone, is_selected.then_some("active"))}
                                onclick={Callback::from(move |_| selected_candidate_id.set(candidate_id_for_click.clone()))}
                            >
                                <span>{candidate_id.clone()}</span>
                                <em>{review_status}</em>
                                <strong>{rule_candidate_name(candidate)}</strong>
                                <small>{&candidate.explanation}</small>
                                <div class="summary-grid compact-summary-grid">
                                    <div><span>{"Support"}</span><strong>{candidate.support}</strong></div>
                                    <div><span>{"Precision"}</span><strong>{percent_label(candidate.precision)}</strong></div>
                                    <div><span>{"Lift"}</span><strong>{format!("{:.2}", candidate.lift)}</strong></div>
                                    <div><span>{"Saving"}</span><strong>{&candidate.estimated_saving}</strong></div>
                                </div>
                                <div class="candidate-evidence-strip">
                                    <small>{format!("matched: {}", refs_label(&candidate.matched_claim_ids))}</small>
                                    <small>{format!("evidence: {}", refs_label(&candidate.evidence_refs))}</small>
                                </div>
                            </button>
                        }
                    })}
                </div>
            </div>
        },
    }
}

fn candidate_review_label(
    candidate_id: &str,
    accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    shadowed_candidate_ids: &UseStateHandle<Vec<String>>,
    final_accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    rejected_candidate_ids: &UseStateHandle<Vec<String>>,
) -> &'static str {
    if rejected_candidate_ids
        .iter()
        .any(|rejected_id| rejected_id == candidate_id)
    {
        "rejected"
    } else if final_accepted_candidate_ids
        .iter()
        .any(|accepted_id| accepted_id == candidate_id)
    {
        "accepted after shadow review"
    } else if shadowed_candidate_ids
        .iter()
        .any(|shadowed_id| shadowed_id == candidate_id)
    {
        "shadow evidence ready"
    } else if accepted_candidate_ids
        .iter()
        .any(|draft_id| draft_id == candidate_id)
    {
        "draft saved for shadow"
    } else {
        "needs backtest"
    }
}

fn candidate_review_tone(
    candidate_id: &str,
    accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    shadowed_candidate_ids: &UseStateHandle<Vec<String>>,
    final_accepted_candidate_ids: &UseStateHandle<Vec<String>>,
    rejected_candidate_ids: &UseStateHandle<Vec<String>>,
) -> &'static str {
    if rejected_candidate_ids
        .iter()
        .any(|rejected_id| rejected_id == candidate_id)
    {
        "rejected"
    } else if final_accepted_candidate_ids
        .iter()
        .any(|accepted_id| accepted_id == candidate_id)
    {
        "accepted"
    } else if shadowed_candidate_ids
        .iter()
        .any(|shadowed_id| shadowed_id == candidate_id)
    {
        "strong"
    } else if accepted_candidate_ids
        .iter()
        .any(|draft_id| draft_id == candidate_id)
    {
        "warning"
    } else {
        "pending-review"
    }
}

fn rule_backtest_view(backtest_state: &UseStateHandle<ApiState<RuleBacktestResponse>>) -> Html {
    match &**backtest_state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Backtesting selected candidate..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(backtest) => html! {
            <section class="visual-panel">
                <h4>{"Backtest Evidence"}</h4>
                <div class="summary-grid">
                    <div><span>{"Matched"}</span><strong>{format!("{} / {}", backtest.matched_count, backtest.sample_count)}</strong></div>
                    <div><span>{"Precision"}</span><strong>{percent_label(backtest.precision)}</strong></div>
                    <div><span>{"Recall"}</span><strong>{percent_label(backtest.recall)}</strong></div>
                    <div><span>{"False Positive"}</span><strong>{percent_label(backtest.false_positive_rate)}</strong></div>
                    <div><span>{"Saving"}</span><strong>{&backtest.estimated_saving}</strong></div>
                    <div><span>{"Recommendation"}</span><strong>{&backtest.promotion_recommendation}</strong></div>
                </div>
                if !backtest.blockers.is_empty() {
                    <div class="compact-list">
                        {for backtest.blockers.iter().map(|blocker| html! { <span>{blocker}</span> })}
                    </div>
                }
            </section>
        },
    }
}

fn rule_save_view(save_state: &UseStateHandle<ApiState<Value>>) -> Html {
    match &**save_state {
        ApiState::Idle => html! {},
        ApiState::Loading => html! { <p>{"Saving draft candidate for shadow..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(saved) => {
            let rule_id = response_rule_id(saved).unwrap_or_else(|| "draft rule".into());
            html! {
                <div class="success-note">
                    {format!("Saved {rule_id} as draft candidate for shadow evidence.")}
                </div>
            }
        }
    }
}

fn rule_shadow_run_state(shadow_state: &UseStateHandle<ApiState<Value>>) -> Html {
    match &**shadow_state {
        ApiState::Idle => html! {
            <p class="empty">{"Run backtest, then submit shadow evidence before promotion review."}</p>
        },
        ApiState::Loading => html! { <p>{"Submitting rule shadow evidence..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="success-note">
                <span>{"Shadow evidence submitted for promotion gates."}</span>
                <pre>{pretty_json(response)}</pre>
            </div>
        },
    }
}

fn rule_candidate_review_state(review_state: &UseStateHandle<ApiState<Value>>) -> Html {
    match &**review_state {
        ApiState::Idle => html! {
            <p class="empty">{"Run backtest, save a draft, submit shadow evidence, then accept or reject the selected candidate."}</p>
        },
        ApiState::Loading => html! { <p>{"Submitting rule candidate review action..."}</p> },
        ApiState::Failed(error) => html! { <p class="error">{error}</p> },
        ApiState::Ready(response) => html! {
            <div class="success-note">
                <span>{"Rule candidate review action accepted."}</span>
                <pre>{pretty_json(response)}</pre>
            </div>
        },
    }
}

fn rule_candidate_name(candidate: &RuleDiscoveryCandidate) -> String {
    if let Some(name) = candidate.rule.get("name").and_then(Value::as_str) {
        name.to_string()
    } else {
        rule_candidate_id(candidate)
    }
}

fn rule_gate_pipeline(gates: &RulePromotionGates) -> Html {
    let nodes = gates
        .gates
        .iter()
        .map(|gate| {
            (
                gate.label.as_str(),
                gate.passed,
                gate.evidence_source.as_str(),
            )
        })
        .collect::<Vec<_>>();
    gate_pipeline("Rule promotion pipeline", &nodes)
}

fn gate_pipeline(title: &str, nodes: &[(&str, bool, &str)]) -> Html {
    if nodes.is_empty() {
        return html! {};
    }
    html! {
        <div class="visual-panel pipeline-panel">
            <h4>{title}</h4>
            <div class="gate-pipeline">
                {for nodes.iter().map(|(label, passed, evidence)| html! {
                    <div class={classes!("gate-node", if *passed { "passed" } else { "blocked" })}>
                        <span>{if *passed { "pass" } else { "block" }}</span>
                        <strong>{label}</strong>
                        <small>{evidence}</small>
                    </div>
                })}
            </div>
        </div>
    }
}

fn model_telemetry_visual(
    performance: &ModelPerformance,
    gates: &ModelPromotionGates,
    retraining: &ModelRetrainingReadiness,
) -> Html {
    let high_risk_density = if performance.scored_runs == 0 {
        0.0
    } else {
        performance.high_risk_count as f64 / performance.scored_runs as f64
    };
    let score_level = (performance.average_score / 100.0).clamp(0.0, 1.0);
    let psi_level = performance.score_psi.unwrap_or(0.0).clamp(0.0, 1.0);
    html! {
        <div class="visual-board model-telemetry">
            <div class="visual-panel">
                <h4>{"Model telemetry map"}</h4>
                <div class="telemetry-orbit">
                    <div class="orbit-core">
                        <strong>{format!("{:.1}", performance.average_score)}</strong>
                        <span>{"avg score"}</span>
                    </div>
                    {telemetry_node("score", score_level, "Score")}
                    {telemetry_node("density", high_risk_density, "High risk")}
                    {telemetry_node("psi", psi_level, "PSI")}
                    {telemetry_node("gates", ratio(gates.passed_count as u32, gates.total_count as u32), "Gates")}
                </div>
            </div>
            <div class="visual-panel">
                <h4>{"Retraining control"}</h4>
                <div class="bar-stack">
                    {meter_row("Approved labels", gates.approved_label_count as u32, 100)}
                    {meter_row("Open feedback", retraining.open_model_feedback_count, 20)}
                    {meter_row("Needs review", retraining.needs_review_label_count, 20)}
                </div>
                <div class="status-ribbon">
                    <span>{format!("drift: {}", retraining.drift_status)}</span>
                    <strong>{&retraining.recommendation}</strong>
                </div>
            </div>
        </div>
    }
}

fn model_monitoring_cockpit(snapshot: &ModelOpsSnapshot) -> Html {
    let active_model = snapshot
        .models
        .iter()
        .find(|model| model.status == "active")
        .or_else(|| snapshot.models.first());
    let model_label = active_model
        .map(|model| format!("{} {}", model.model_key, model.version))
        .unwrap_or_else(|| snapshot.performance.model_key.clone());
    let gate_ratio = ratio(
        snapshot.gates.passed_count as u32,
        snapshot.gates.total_count as u32,
    );
    let label_ratio = ratio(snapshot.gates.approved_label_count, 100);
    let psi_label = optional_number(snapshot.performance.score_psi);
    let first_blocker = snapshot
        .gates
        .blockers
        .first()
        .map(String::as_str)
        .or_else(|| snapshot.retraining.blockers.first().map(String::as_str))
        .unwrap_or("no blocker");

    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Model Monitoring Cockpit"}</h3>
                    <p>{"A pilot-facing view of model version, drift, shadow evidence, promotion gates, QA labels, and retraining readiness before any model affects routing."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&snapshot.gates.decision))}>{&snapshot.gates.decision}</span>
            </div>
            <div class="model-monitoring-cockpit">
                <aside class="model-monitoring-brief">
                    <span class="eyebrow">{"Active candidate"}</span>
                    <strong>{model_label}</strong>
                    <dl>
                        <div><dt>{"Runtime"}</dt><dd>{active_model.map(|model| model.runtime_kind.as_str()).unwrap_or("runtime pending")}</dd></div>
                        <div><dt>{"Provider"}</dt><dd>{active_model.map(|model| model.execution_provider.as_str()).unwrap_or("provider pending")}</dd></div>
                        <div><dt>{"Review mode"}</dt><dd>{active_model.map(|model| model.review_mode.as_str()).unwrap_or("review pending")}</dd></div>
                        <div><dt>{"Latest eval"}</dt><dd>{&snapshot.gates.latest_evaluation_id}</dd></div>
                    </dl>
                </aside>

                <div class="model-monitoring-map">
                    <div class="model-monitoring-link horizontal"></div>
                    <div class="model-monitoring-link diagonal-a"></div>
                    <div class="model-monitoring-link diagonal-b"></div>
                    <div class="model-monitoring-core">
                        <span>{"Release gate"}</span>
                        <strong>{&snapshot.performance.model_key}</strong>
                    </div>
                    {model_monitoring_node("Version lock", active_model.map(|model| model.version.as_str()).unwrap_or("pending"), "top", "version")}
                    {model_monitoring_node("Drift watch", &format!("{} / PSI {}", snapshot.performance.drift_status, psi_label), "right", "drift")}
                    {model_monitoring_node("Shadow evidence", &snapshot.gates.latest_evaluation_id, "bottom", "shadow")}
                    {model_monitoring_node("QA labels", &format!("{} approved", snapshot.gates.approved_label_count), "left", "labels")}
                    {model_monitoring_node("Retraining", &snapshot.retraining.recommendation, "lower-right", "train")}
                </div>

                <aside class="model-monitoring-actions">
                    <span class="eyebrow">{"Promotion readiness"}</span>
                    <div class="model-monitoring-meter">
                        <span>{"Gate pass"}</span>
                        <div><i style={format!("width: {};", percent_width(gate_ratio))}></i></div>
                        <strong>{percent_label(gate_ratio)}</strong>
                    </div>
                    <div class="model-monitoring-meter">
                        <span>{"Label readiness"}</span>
                        <div><i style={format!("width: {};", percent_width(label_ratio))}></i></div>
                        <strong>{snapshot.gates.approved_label_count}</strong>
                    </div>
                    <div class="provider-signal-stack">
                        {provider_signal_row("Data quality", &snapshot.gates.source_data_quality_status, "strong")}
                        {provider_signal_row("Drift", &snapshot.retraining.drift_status, "warning")}
                        {provider_signal_row("Open feedback", &snapshot.retraining.open_model_feedback_count.to_string(), "neutral")}
                        {provider_signal_row("Blocker", first_blocker, "danger")}
                    </div>
                </aside>
            </div>
        </section>
    }
}

fn model_monitoring_node(label: &str, value: &str, position: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("model-monitoring-node", position.to_string(), tone.to_string())}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn telemetry_node(kind: &str, value: f64, label: &str) -> Html {
    html! {
        <div class={classes!("orbit-node", kind.to_string())}>
            <span>{label}</span>
            <strong>{percent_label(value)}</strong>
        </div>
    }
}

fn meter_row(label: &str, value: u32, max_value: u32) -> Html {
    html! {
        <div class="bar-row">
            <span>{label}</span>
            <div class="bar-track"><i style={format!("width: {};", scaled_width(value, max_value.max(1)))}></i></div>
            <strong>{value}</strong>
        </div>
    }
}

fn timeline_item(label: &str, value: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("timeline-item", status_tone(tone))}>
            <span>{label}</span>
            <strong>{value}</strong>
        </div>
    }
}

fn case_action(label: &str, caption: &str, tone: &str) -> Html {
    html! {
        <div class={classes!("case-action", tone.to_string())}>
            <strong>{label}</strong>
            <span>{caption}</span>
        </div>
    }
}

fn scaled_width(value: u32, max_value: u32) -> String {
    let width = if max_value == 0 {
        0.0
    } else {
        value as f64 / max_value as f64 * 100.0
    };
    format!("{:.0}%", width.clamp(4.0, 100.0))
}

fn percent_width(value: f64) -> String {
    format!("{:.0}%", (value * 100.0).clamp(4.0, 100.0))
}

fn ratio(value: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64
    }
}

fn icon_class(icon: &str) -> &'static str {
    match icon {
        "risk" => "icon-risk",
        "confirmed" => "icon-confirmed",
        "amount" => "icon-amount",
        "saving" => "icon-saving",
        "rule" => "icon-rule",
        "case" => "icon-case",
        "qa" => "icon-qa-card",
        "currency" => "icon-currency",
        _ => "icon-default",
    }
}

fn runtime_model_output(model_score: Option<&RuntimeModelScore>) -> Html {
    if let Some(model) = model_score {
        html! {
            <div class="result-stack">
                <div class="summary-grid">
                    <div><span>{"Model"}</span><strong>{format!("{} {}", model.model_key, model.model_version)}</strong></div>
                    <div><span>{"Runtime"}</span><strong>{format!("{} / {}", model.runtime_kind, model.execution_provider)}</strong></div>
                    <div><span>{"Score"}</span><strong>{model.score}</strong></div>
                    <div><span>{"Label"}</span><strong>{&model.label}</strong></div>
                    <div><span>{"Latency"}</span><strong>{format!("{} ms", model.latency_ms)}</strong></div>
                    <div><span>{"Metadata"}</span><strong>{payload_keys_label(&model.metadata)}</strong></div>
                </div>
                if model.explanations.is_empty() {
                    <p class="empty">{"No model explanations returned."}</p>
                } else {
                    <div class="factor-card-grid">
                        {for model.explanations.iter().map(|explanation| html! {
                            <div class="metric-row">
                                <span>{&explanation.feature}</span>
                                <strong>{format!("{} {:.2}", explanation.direction, explanation.contribution)}</strong>
                                <small>{&explanation.reason}</small>
                            </div>
                        })}
                    </div>
                }
            </div>
        }
    } else {
        html! { <p class="empty">{"No model score returned."}</p> }
    }
}

fn runtime_full_payload_template() -> Value {
    json!({
        "source_system": "tpa-demo",
        "review_mode": "pre_payment",
        "claim": {
            "external_claim_id": "CLM-WEB-RUNTIME",
            "claim_amount": "18900",
            "currency": "CNY",
            "service_date": "2026-01-06",
            "diagnosis_code": "J10",
            "items": [
                {
                    "item_code": "IMG-001",
                    "item_type": "procedure",
                    "description": "High cost imaging",
                    "quantity": 1,
                    "unit_amount": "18900",
                    "total_amount": "18900",
                    "currency": "CNY"
                }
            ],
            "member": {
                "external_member_id": "MBR-WEB-RUNTIME",
                "dob": "1985-03-14",
                "gender": "F"
            },
            "policy": {
                "external_policy_id": "POL-WEB-RUNTIME",
                "product_code": "MED",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "20000",
                "currency": "CNY"
            },
            "provider": {
                "external_provider_id": "PRV-WEB-RUNTIME",
                "name": "Northwind Hospital",
                "provider_type": "hospital",
                "region": "Shanghai",
                "risk_tier": "High"
            },
            "documents": [
                {
                    "external_document_id": "DOC-WEB-RUNTIME",
                    "document_type": "medical_record",
                    "linked_item_codes": ["IMG-001"]
                }
            ],
            "provider_profile": {
                "specialty": "general",
                "network_status": "in_network",
                "windows": [
                    {
                        "window_days": 30,
                        "claim_count": 40,
                        "total_claim_amount": "480000",
                        "high_cost_item_ratio": 0.74,
                        "diagnosis_procedure_mismatch_rate": 0.46,
                        "peer_amount_percentile": 96,
                        "peer_frequency_percentile": 93,
                        "review_failure_count": 8,
                        "confirmed_fwa_count": 3,
                        "false_positive_count": 1
                    }
                ]
            },
            "provider_relationships": {
                "high_risk_neighbor_ratio": 0.42,
                "provider_patient_overlap_score": 0.72,
                "referral_concentration_score": 0.66,
                "connected_confirmed_fwa_count": 4,
                "network_component_risk_score": 84,
                "evidence_refs": ["provider_graph:PRV-WEB-RUNTIME"]
            }
        }
    })
}

fn agent_investigation_payload(
    claim_id: String,
    risk_score: String,
    rag: String,
    scheme_family: String,
    top_reasons: String,
    diagnosis_code: String,
    provider_region: String,
    tags: String,
) -> Result<Value, String> {
    let top_reasons = parse_tags(&top_reasons);
    let tags = parse_tags(&tags);
    if claim_id.trim().is_empty() {
        return Err("claim id is required".into());
    }
    if !matches!(rag.trim(), "GREEN" | "AMBER" | "RED") {
        return Err("RAG must be GREEN, AMBER, or RED".into());
    }
    if top_reasons.is_empty() {
        return Err("at least one top reason is required".into());
    }
    if diagnosis_code.trim().is_empty() || provider_region.trim().is_empty() || tags.is_empty() {
        return Err("diagnosis code, provider region, and at least one tag are required".into());
    }
    let scheme_family = scheme_family.trim();
    Ok(json!({
        "claim_id": claim_id.trim(),
        "risk_score": parse_risk_score(&risk_score)?,
        "rag": rag.trim(),
        "scheme_family": if scheme_family.is_empty() {
            Value::Null
        } else {
            Value::String(scheme_family.to_string())
        },
        "top_reasons": top_reasons,
        "similar_case_query": {
            "diagnosis_code": diagnosis_code.trim(),
            "provider_region": provider_region.trim(),
            "tags": tags
        }
    }))
}

fn audit_sample_payload(
    sample_mode: String,
    population_definition: String,
    inclusion_criteria: String,
    sample_size: String,
    reviewer: String,
    assignment_queue: String,
    deterministic_seed: String,
) -> Result<Value, String> {
    let sample_mode = sample_mode.trim();
    if !matches!(
        sample_mode,
        "risk_ranked" | "random_control" | "stratified" | "post_payment_audit" | "qa_calibration"
    ) {
        return Err("sample mode must be risk_ranked, random_control, stratified, post_payment_audit, or qa_calibration".into());
    }
    if population_definition.trim().is_empty() {
        return Err("population definition is required".into());
    }
    if reviewer.trim().is_empty() || assignment_queue.trim().is_empty() {
        return Err("reviewer and assignment queue are required".into());
    }
    let sample_size = sample_size
        .trim()
        .parse::<usize>()
        .map_err(|error| format!("sample size must be a positive integer: {error}"))?;
    if sample_size == 0 {
        return Err("sample size must be greater than zero".into());
    }
    let inclusion_criteria = serde_json::from_str::<Value>(&inclusion_criteria)
        .map_err(|error| format!("inclusion criteria JSON is invalid: {error}"))?;
    if !inclusion_criteria.is_object() {
        return Err("inclusion criteria must be a JSON object".into());
    }
    let deterministic_seed = deterministic_seed.trim();
    Ok(json!({
        "sample_mode": sample_mode,
        "population_definition": population_definition.trim(),
        "inclusion_criteria": inclusion_criteria,
        "sample_size": sample_size,
        "reviewer": reviewer.trim(),
        "assignment_queue": assignment_queue.trim(),
        "deterministic_seed": if deterministic_seed.is_empty() {
            Value::Null
        } else {
            Value::String(deterministic_seed.to_string())
        }
    }))
}

fn total_dataset_rows(datasets: &[DatasetRecord]) -> String {
    datasets
        .iter()
        .map(|dataset| dataset.row_count)
        .sum::<u64>()
        .to_string()
}

fn total_schema_fields(datasets: &[DatasetRecord]) -> usize {
    datasets.iter().map(|dataset| dataset.fields.len()).sum()
}

fn total_field_mappings(datasets: &[DatasetRecord]) -> usize {
    datasets.iter().map(|dataset| dataset.mappings.len()).sum()
}

fn active_model_version(snapshot: &ModelOpsSnapshot) -> Option<&ModelVersion> {
    snapshot
        .models
        .iter()
        .find(|model| model.status == "active")
        .or_else(|| snapshot.models.first())
}

fn provider_release_decision_label(snapshot: &MlopsWorkspaceSnapshot) -> &'static str {
    if !snapshot.model_ops.gates.blockers.is_empty() {
        "resolve blockers"
    } else if snapshot
        .model_ops
        .gates
        .decision
        .to_ascii_lowercase()
        .contains("allowed")
    {
        "approve rollout"
    } else if snapshot
        .model_ops
        .retraining
        .recommendation
        .to_ascii_lowercase()
        .contains("retraining")
    {
        "request retraining"
    } else {
        "keep monitoring"
    }
}

fn latest_dataset(datasets: &[DatasetRecord]) -> Option<&DatasetRecord> {
    datasets
        .iter()
        .max_by_key(|dataset| (&dataset.dataset_key, &dataset.dataset_version))
}

fn dataset_version_label(dataset: &DatasetRecord) -> String {
    format!("{}:{}", dataset.dataset_key, dataset.dataset_version)
}

fn health_for_dataset<'a>(
    health: &'a [DatasetHealthRecord],
    dataset_id: &str,
) -> Option<&'a DatasetHealthRecord> {
    health.iter().find(|item| item.dataset_id == dataset_id)
}

fn status_tone(status: &str) -> &'static str {
    let normalized = status.to_ascii_lowercase();
    if normalized.contains("fail")
        || normalized.contains("error")
        || normalized.contains("breach")
        || normalized.contains("blocked")
        || normalized.contains("high")
    {
        "danger"
    } else if normalized.contains("warn")
        || normalized.contains("pending")
        || normalized.contains("review")
        || normalized.contains("medium")
    {
        "warning"
    } else if normalized.contains("ready")
        || normalized.contains("active")
        || normalized.contains("ok")
        || normalized.contains("pass")
        || normalized.contains("good")
    {
        "success"
    } else {
        "neutral"
    }
}

fn lineage_for<'a>(
    lineage: &'a [ModelEvaluationLineageRecord],
    evaluation_run_id: &str,
) -> Option<&'a ModelEvaluationLineageRecord> {
    lineage
        .iter()
        .find(|record| record.evaluation_run_id == evaluation_run_id)
}

fn lineage_data_quality_label(lineage: Option<&ModelEvaluationLineageRecord>) -> String {
    lineage
        .map(|record| {
            format!(
                "{} / {}",
                record
                    .source_data_quality_status
                    .as_deref()
                    .unwrap_or("missing"),
                optional_number(record.source_data_quality_score)
            )
        })
        .unwrap_or_else(|| "missing".into())
}

fn lineage_source_label(lineage: Option<&ModelEvaluationLineageRecord>) -> String {
    lineage
        .map(|record| {
            format!(
                "{}:{} / {} / {} {}",
                record.source_dataset_key.as_deref().unwrap_or("missing"),
                record
                    .source_dataset_version
                    .as_deref()
                    .unwrap_or("missing"),
                record.source_dataset_id.as_deref().unwrap_or("missing"),
                record.model_key,
                record.model_version
            )
        })
        .unwrap_or_else(|| "missing".into())
}

fn rule_performance_for<'a>(
    performance: &'a [RulePerformance],
    rule_id: &str,
) -> Option<&'a RulePerformance> {
    performance.iter().find(|item| item.rule_id == rule_id)
}

fn selected_lead<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    selected_lead_id: &str,
) -> Option<&'a LeadRecord> {
    let selected_lead_id = selected_lead_id.trim();
    if selected_lead_id.is_empty() {
        snapshot.leads.first()
    } else {
        snapshot
            .leads
            .iter()
            .find(|lead| lead.lead_id == selected_lead_id)
    }
}

fn selected_case<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    selected_case_id: &str,
) -> Option<&'a CaseRecord> {
    let selected_case_id = selected_case_id.trim();
    if selected_case_id.is_empty() {
        snapshot.cases.first()
    } else {
        snapshot
            .cases
            .iter()
            .find(|case| case.case_id == selected_case_id)
    }
}

fn latest_lead_for_score<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    claim_id: &str,
    score_run_id: &str,
) -> Option<&'a LeadRecord> {
    snapshot
        .leads
        .iter()
        .find(|lead| lead.claim_id == claim_id && lead.run_id == score_run_id)
        .or_else(|| snapshot.leads.iter().find(|lead| lead.claim_id == claim_id))
}

fn lead_for_case<'a>(
    snapshot: &'a LeadsCasesSnapshot,
    case: &CaseRecord,
) -> Option<&'a LeadRecord> {
    snapshot
        .leads
        .iter()
        .find(|lead| lead.lead_id == case.lead_id)
}

fn live_tpa_demo_payload(summary: &DashboardSummary) -> Result<Value, String> {
    let suffix = format!(
        "{}-{}-{}",
        summary.suspected_claims, summary.confirmed_fwa, summary.rule_hits
    );
    let mut payload = serde_json::from_str::<Value>(LIVE_TPA_DEMO_PAYLOAD)
        .map_err(|error| format!("live demo payload JSON is invalid: {error}"))?;
    payload["transNo"] = Value::String(format!("TPA-LIVE-DEMO-{suffix}"));
    payload["reportCase"]["reportNo"] = Value::String(format!("CLM-LIVE-DEMO-{suffix}"));
    Ok(payload)
}

fn selected_medical_item<'a>(
    items: &'a [MedicalReviewQueueItem],
    selected_audit_id: &str,
) -> Option<&'a MedicalReviewQueueItem> {
    let selected_audit_id = selected_audit_id.trim();
    if selected_audit_id.is_empty() {
        items.first()
    } else {
        items.iter().find(|item| item.audit_id == selected_audit_id)
    }
}

fn refs_or_fallback(refs_text: &str, fallback: Vec<String>) -> Vec<String> {
    let refs = parse_tags(refs_text);
    if refs.is_empty() {
        fallback
            .into_iter()
            .filter(|reference| !reference.trim().is_empty())
            .collect()
    } else {
        refs
    }
}

fn medical_review_cockpit(items: &[MedicalReviewQueueItem]) -> Html {
    let Some(item) = items.first() else {
        return html! {};
    };
    let missing_evidence = item
        .missing_evidence
        .first()
        .map(String::as_str)
        .unwrap_or("none");
    let canonical_source = item
        .canonical_source_refs
        .first()
        .map(String::as_str)
        .unwrap_or("source pending");
    let canonical_evidence = item
        .canonical_evidence_refs
        .first()
        .map(String::as_str)
        .unwrap_or("evidence pending");
    let first_item = item.first_item_code.as_deref().unwrap_or("item pending");
    let first_issue = item.first_issue_type.as_deref().unwrap_or("issue pending");
    html! {
        <section class="panel result-stack">
            <div class="section-header">
                <div>
                    <h3>{"Clinical evidence cockpit"}</h3>
                    <p>{"Clinical reasonableness workbench linking diagnosis support, bill item evidence, missing records, reviewer outcome, and audit trace."}</p>
                </div>
                <span class={classes!("status-token", status_tone(&item.evidence_status))}>{business_label(&item.evidence_status)}</span>
            </div>
            <div class="clinical-cockpit">
                <aside class="case-brief clinical-brief">
                    <span>{"Selected review"}</span>
                    <strong>{&item.claim_id}</strong>
                    <dl>
                        <div><dt>{"Audit"}</dt><dd>{&item.audit_id}</dd></div>
                        <div><dt>{"Route"}</dt><dd>{business_label(&item.review_route)}</dd></div>
                        <div><dt>{"Status"}</dt><dd>{business_label(&item.review_status)}</dd></div>
                        <div><dt>{"Score"}</dt><dd>{item.medical_reasonableness_score}</dd></div>
                    </dl>
                    <div class="tag-grid compact-tags">
                        <span>{format!("findings {}", item.item_finding_count)}</span>
                        <span>{format!("missing {}", item.missing_evidence.len())}</span>
                        <span>{format!("refs {}", item.evidence_refs.len() + item.canonical_evidence_refs.len())}</span>
                    </div>
                </aside>

                <div class="clinical-evidence-map">
                    <div class="clinical-map-title">
                        <span>{"Medical necessity path"}</span>
                        <strong>{format!("{} -> {}", first_item, first_issue)}</strong>
                    </div>
                    <div class="clinical-path-line"></div>
                    <div class="clinical-node diagnosis">
                        <span>{"Diagnosis"}</span>
                        <strong>{canonical_source}</strong>
                    </div>
                    <div class="clinical-node item">
                        <span>{"Bill item"}</span>
                        <strong>{first_item}</strong>
                    </div>
                    <div class="clinical-node record">
                        <span>{"Medical record"}</span>
                        <strong>{canonical_evidence}</strong>
                    </div>
                    <div class="clinical-node gap">
                        <span>{"Evidence gap"}</span>
                        <strong>{missing_evidence}</strong>
                    </div>
                    <div class="clinical-node reviewer">
                        <span>{"Reviewer"}</span>
                        <strong>{item.reviewer.as_deref().unwrap_or("pending")}</strong>
                    </div>
                </div>

                <aside class="case-timeline clinical-timeline">
                    <h4>{"Clinical trace"}</h4>
                    {timeline_item("Queue created", item.created_at.as_deref().unwrap_or("pending"), "done")}
                    {timeline_item("Evidence status", &business_label(&item.evidence_status), &item.evidence_status)}
                    {timeline_item("Review decision", &item.review_decision.as_deref().map(business_label).unwrap_or_else(|| "Pending".into()), item.review_decision.as_deref().unwrap_or("pending"))}
                    {timeline_item("Review audit", item.review_audit_id.as_deref().unwrap_or("pending"), "pending")}
                </aside>
            </div>
            <div class="clinical-outcome-grid">
                <h4>{"Controlled outcomes"}</h4>
                {case_action("Documentation issue", "clinical evidence incomplete", "warning")}
                {case_action("Medical necessity review required", "human medical gate", "strong")}
                {case_action("Insufficient evidence", "request supplement", "neutral")}
                {case_action("Medical necessity issue", "manual action only", "danger")}
                {case_action("Clinical evidence sufficient", "close clinical gap", "strong")}
                {case_action("False positive", "requires audit note", "neutral")}
            </div>
        </section>
    }
}

fn medical_review_fallback_refs(item: &MedicalReviewQueueItem) -> Vec<String> {
    let mut refs = item.evidence_refs.clone();
    refs.extend(item.canonical_evidence_refs.clone());
    refs.push(format!("audit:{}", item.audit_id));
    refs.into_iter().fold(Vec::new(), |mut values, value| {
        if !values.contains(&value) {
            values.push(value);
        }
        values
    })
}

fn data_lineage_cockpit(snapshot: &DataSourcesSnapshot) -> Html {
    let source_count = unique_dataset_sources(&snapshot.datasets);
    let canonical_count = unique_canonical_targets(&snapshot.datasets);
    let feature_count = feature_mapping_count(&snapshot.datasets);
    let online_ready = snapshot
        .health
        .iter()
        .map(|health| health.online_ready_count)
        .sum::<u32>();
    let issue_count = snapshot
        .health
        .iter()
        .map(|health| health.issue_count)
        .sum::<u32>();
    let quality_label = data_quality_summary(&snapshot.health);
    let quality_tone = status_tone(&quality_label);
    let source_label = snapshot
        .datasets
        .first()
        .map(|dataset| format!("{} / {}", dataset.source_key, dataset.storage_format))
        .unwrap_or_else(|| "no source registered".into());
    let canonical_label = first_canonical_target(&snapshot.datasets);
    let feature_label = first_feature_mapping(&snapshot.datasets);
    let model_label = snapshot
        .evaluations
        .first()
        .map(|evaluation| format!("{} {}", evaluation.model_key, evaluation.model_version))
        .unwrap_or_else(|| "no evaluation".into());
    let runtime_label = if online_ready > 0 {
        format!("{} online fields", online_ready)
    } else {
        "not online ready".into()
    };
    let audit_label = if issue_count == 0 {
        "no open data issues".into()
    } else {
        format!("{} data issues", issue_count)
    };

    html! {
        <section class="panel data-lineage-cockpit">
            <div class="section-header">
                <div>
                    <h3>{"Data Lineage Cockpit"}</h3>
                    <p>{"A visual control map for how external datasets become governed features, model evaluation evidence, scoring inputs, and audit records."}</p>
                </div>
                <span class={classes!("status-token", quality_tone)}>{quality_label.clone()}</span>
            </div>
            <div class="data-lineage-map" aria-label="Data lineage flow">
                <div class="lineage-rail rail-a"></div>
                <div class="lineage-rail rail-b"></div>
                <div class="lineage-rail rail-c"></div>
                {data_lineage_node("source", "Sources", &source_count.to_string(), &source_label)}
                {data_lineage_node("contract", "Schema contract", &total_schema_fields(&snapshot.datasets).to_string(), "field profiles and split manifests")}
                {data_lineage_node("canonical", "Canonical map", &canonical_count.to_string(), &canonical_label)}
                {data_lineage_node("feature", "Feature ready", &feature_count.to_string(), &feature_label)}
                {data_lineage_node("model", "Model lineage", &snapshot.evaluations.len().to_string(), &model_label)}
                {data_lineage_node("runtime", "Runtime inputs", &online_ready.to_string(), &runtime_label)}
                {data_lineage_node("audit", "Audit guard", &issue_count.to_string(), &audit_label)}
            </div>
            <div class="data-lineage-proof-grid">
                <div>
                    <span>{"Governed contract"}</span>
                    <strong>{format!("{} datasets / {} mappings", snapshot.datasets.len(), total_field_mappings(&snapshot.datasets))}</strong>
                    <small>{"schema hash, profile URI, manifest URI, and split records remain visible before scoring."}</small>
                </div>
                <div>
                    <span>{"Evaluation evidence"}</span>
                    <strong>{format!("{} runs / {}", snapshot.evaluations.len(), lineage_source_coverage(&snapshot.lineage))}</strong>
                    <small>{"model metrics stay tied to source dataset version and data-quality state."}</small>
                </div>
                <div>
                    <span>{"Pilot blocker signal"}</span>
                    <strong>{audit_label}</strong>
                    <small>{"data health issues are shown as readiness evidence, not hidden behind model output."}</small>
                </div>
            </div>
        </section>
    }
}

fn data_lineage_node(tone: &'static str, label: &'static str, value: &str, detail: &str) -> Html {
    html! {
        <div class={classes!("data-lineage-node", tone)}>
            <span>{label}</span>
            <strong>{value}</strong>
            <small>{detail}</small>
        </div>
    }
}

fn unique_dataset_sources(datasets: &[DatasetRecord]) -> usize {
    datasets
        .iter()
        .fold(Vec::<&str>::new(), |mut values, dataset| {
            if !values.contains(&dataset.source_key.as_str()) {
                values.push(dataset.source_key.as_str());
            }
            values
        })
        .len()
}

fn unique_canonical_targets(datasets: &[DatasetRecord]) -> usize {
    datasets
        .iter()
        .flat_map(|dataset| dataset.mappings.iter())
        .fold(Vec::<&str>::new(), |mut values, mapping| {
            if !values.contains(&mapping.canonical_target.as_str()) {
                values.push(mapping.canonical_target.as_str());
            }
            values
        })
        .len()
}

fn feature_mapping_count(datasets: &[DatasetRecord]) -> usize {
    datasets
        .iter()
        .flat_map(|dataset| dataset.mappings.iter())
        .filter(|mapping| mapping.feature_name.is_some())
        .count()
}

fn first_canonical_target(datasets: &[DatasetRecord]) -> String {
    datasets
        .iter()
        .flat_map(|dataset| dataset.mappings.iter())
        .next()
        .map(|mapping| mapping.canonical_target.clone())
        .unwrap_or_else(|| "no canonical mapping".into())
}

fn first_feature_mapping(datasets: &[DatasetRecord]) -> String {
    datasets
        .iter()
        .flat_map(|dataset| dataset.mappings.iter())
        .find_map(|mapping| mapping.feature_name.clone())
        .unwrap_or_else(|| "no feature mapping".into())
}

fn data_quality_summary(health: &[DatasetHealthRecord]) -> String {
    if health.is_empty() {
        return "no health record".into();
    }
    if health
        .iter()
        .any(|item| status_tone(&item.data_quality_status) == "danger")
    {
        return "data blocker".into();
    }
    if health
        .iter()
        .any(|item| status_tone(&item.data_quality_status) == "warning")
    {
        return "review required".into();
    }
    "data ready".into()
}

fn lineage_source_coverage(lineage: &[ModelEvaluationLineageRecord]) -> String {
    let covered = lineage
        .iter()
        .filter(|record| record.source_dataset_id.is_some())
        .count();
    format!("{} source-linked", covered)
}

fn open_lead_count(leads: &[LeadRecord]) -> usize {
    leads
        .iter()
        .filter(|lead| !matches!(lead.status.as_str(), "closed" | "rejected"))
        .count()
}

fn active_case_count(cases: &[CaseRecord]) -> usize {
    cases
        .iter()
        .filter(|case| !matches!(case.status.as_str(), "closed" | "rejected"))
        .count()
}

fn breached_case_count(cases: &[CaseRecord]) -> usize {
    cases
        .iter()
        .filter(|case| case.sla_status == "breached")
        .count()
}

fn lead_status_count(leads: &[LeadRecord], status: &str) -> usize {
    leads.iter().filter(|lead| lead.status == status).count()
}

fn case_status_count(cases: &[CaseRecord], status: &str) -> usize {
    cases.iter().filter(|case| case.status == status).count()
}

fn queue_meter(label: &str, value: usize, total: usize, tone: &str) -> Html {
    let width = if total == 0 {
        "0%".to_string()
    } else {
        percent_width(value as f64 / total as f64)
    };
    html! {
        <div class={classes!("queue-meter", tone.to_string())}>
            <div>
                <span>{label}</span>
                <strong>{value}</strong>
            </div>
            <i><b style={format!("width: {width};")}></b></i>
        </div>
    }
}

fn top_scheme_label(leads: &[LeadRecord]) -> String {
    let mut counts = BTreeMap::new();
    for lead in leads {
        *counts.entry(lead.scheme_family.as_str()).or_insert(0_usize) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(scheme, count)| format!("{} ({})", readable_token(scheme), count))
        .unwrap_or_else(|| "No active pattern".into())
}

fn lead_stage_label(status: &str) -> String {
    match status {
        "new" => "New lead".into(),
        "pending_evidence" => "Needs evidence".into(),
        "triaged" => "Case opened".into(),
        "closed" => "Closed".into(),
        other => readable_token(other),
    }
}

fn lead_stage_tone(status: &str) -> &'static str {
    match status {
        "pending_evidence" => "danger",
        "new" => "warning",
        "triaged" | "closed" => "success",
        _ => "neutral",
    }
}

fn case_stage_label(status: &str) -> String {
    match status {
        "investigating" => "Investigating",
        "pending_evidence" => "Waiting evidence",
        "confirmed" => "Confirmed",
        "closed" => "Closed",
        "rejected" => "Rejected",
        "triage" => "Triage",
        other => return readable_token(other),
    }
    .into()
}

fn case_stage_tone(status: &str) -> &'static str {
    match status {
        "investigating" | "pending_evidence" | "triage" => "warning",
        "confirmed" | "closed" => "success",
        "rejected" => "neutral",
        _ => "neutral",
    }
}

fn priority_label(priority: &str) -> String {
    match priority {
        "high" => "High priority",
        "medium" => "Medium priority",
        "low" => "Low priority",
        other => return readable_token(other),
    }
    .into()
}

fn priority_tone(priority: &str) -> &'static str {
    match priority {
        "high" => "danger",
        "medium" => "warning",
        "low" => "neutral",
        _ => "strong",
    }
}

fn sla_label(status: &str) -> &'static str {
    match status {
        "breached" => "Over SLA",
        "on_track" => "On track",
        _ => "SLA pending",
    }
}

fn sla_tone(status: &str) -> &'static str {
    match status {
        "breached" => "danger",
        "on_track" => "success",
        _ => "neutral",
    }
}

fn routing_review_modes(policies: &[RoutingPolicyRecord]) -> String {
    count_by(policies.iter().map(|policy| policy.review_mode.as_str()))
}

fn count_by<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0_u32) += 1;
    }
    map_counts_label(&counts)
}

fn average_medical_score(items: &[MedicalReviewQueueItem]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    let total = items
        .iter()
        .map(|item| item.medical_reasonableness_score as u32)
        .sum::<u32>();
    total as f64 / items.len() as f64
}

fn text_input(label: &'static str, state: &UseStateHandle<String>) -> Html {
    html! {
        <label>
            {label}
            <input
                value={(**state).clone()}
                oninput={{
                    let state = state.clone();
                    Callback::from(move |event: InputEvent| {
                        state.set(event.target_unchecked_into::<HtmlInputElement>().value());
                    })
                }}
            />
        </label>
    }
}

fn refs_label(refs: &[String]) -> String {
    if refs.is_empty() {
        "none".into()
    } else {
        refs.join(", ")
    }
}

fn refs_count_label(refs: &[String]) -> String {
    if refs.is_empty() {
        "none".into()
    } else {
        format!("{} refs", refs.len())
    }
}

fn parse_tags(tags_text: &str) -> Vec<String> {
    tags_text
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

fn payload_keys_label(value: &Value) -> String {
    value
        .as_object()
        .map(|object| {
            if object.is_empty() {
                "empty object".into()
            } else {
                object.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        })
        .unwrap_or_else(|| display_value(value))
}

fn payload_signal_count_label(value: &Value, noun: &str) -> String {
    value
        .as_object()
        .map(|object| {
            if object.is_empty() {
                "empty object".into()
            } else {
                format!("{} {}", object.len(), noun)
            }
        })
        .unwrap_or_else(|| display_value(value))
}

fn compact_payload_label(value: &Value) -> String {
    value
        .as_object()
        .map(|object| {
            if object.is_empty() {
                "empty object".into()
            } else {
                format!("{} fields", object.len())
            }
        })
        .unwrap_or_else(|| "payload recorded".into())
}

fn empty_label(value: &str) -> &str {
    if value.trim().is_empty() {
        "none"
    } else {
        value
    }
}

fn approval_summary(approvals: &[AgentApprovalView]) -> String {
    if approvals.is_empty() {
        return "none".into();
    }
    approvals
        .iter()
        .map(|approval| {
            format!(
                "{} {}:{} by {} at {} evidence={} reason={}",
                approval.approval_id,
                approval.proposed_action,
                approval.decision,
                approval.approver,
                approval.created_at.as_deref().unwrap_or("unknown"),
                approval.evidence_refs.len(),
                approval.reason
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn approval_count_label(approvals: &[AgentApprovalView]) -> String {
    if approvals.is_empty() {
        "none".into()
    } else {
        format!("{} approval records", approvals.len())
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
