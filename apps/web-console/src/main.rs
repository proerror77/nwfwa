use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
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
