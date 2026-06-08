use crate::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};

#[function_component(RulesPage)]
pub fn rules_page() -> Html {
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
