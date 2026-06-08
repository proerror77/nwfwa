use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
mod api;
mod constants;
mod i18n;
mod pages;
mod routing;
mod state;
mod types;

use api::*;
use constants::*;
use i18n::{
    apply_document_language, brand_description, module_context, module_description, module_label,
    section_label, tr,
};
use pages::*;
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
