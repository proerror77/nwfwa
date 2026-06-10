use crate::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlTextAreaElement;

#[path = "rules_view.rs"]
mod rules_view;
use rules_view::RulesView;


#[function_component(RulesPage)]
pub fn rules_page() -> Html {
    let api_key = use_api_key();
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
                    {text_input("Gate Rule ID", &rule_id)}
                    {text_input("Model Key", &model_key)}
                    {text_input("Model Version", &model_version)}
                    {text_input("Explained Feature", &explanation_feature)}
                    {text_input("Contribution", &explanation_contribution)}
                    {text_input("Explanation Artifact", &feature_importance_uri)}
                    {text_input("Mining Dataset URI", &discovery_dataset_uri)}
                    {text_input("Label Column", &discovery_label_column)}
                    {text_input("Claim ID Column", &discovery_claim_id_column)}
                    {text_input("Feature Columns", &discovery_feature_fields)}
                    {text_input("Tree Depth", &discovery_tree_depth)}
                    {text_input("Reviewer", &rule_reviewer)}
                    {text_input("Review Evidence Refs", &rule_review_evidence_refs)}
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
