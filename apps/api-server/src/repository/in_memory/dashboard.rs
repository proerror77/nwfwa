use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        let runs = self.runs.lock().await;
        let claims = self.claims.lock().await;
        let pilot_events = self.pilot_audit_events.lock().await;
        let saving_attribution_records = self.saving_attributions.lock().await.clone();

        let mut risk_amount = Decimal::ZERO;
        let mut rag_distribution = BTreeMap::new();
        let mut model_accumulators = BTreeMap::<String, (u32, u32, u32)>::new();
        let mut layer_accumulators = BTreeMap::<String, (String, u32, u32, u32)>::new();
        let mut rule_hits = 0_u32;

        let scoped_runs = runs
            .iter()
            .filter(|run| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&run.audit_event, scope)
                })
            })
            .collect::<Vec<_>>();
        let scoped_pilot_events = pilot_events
            .iter()
            .filter(|(_, event)| {
                customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
            })
            .collect::<Vec<_>>();
        let scoped_claim_ids = scoped_runs
            .iter()
            .map(|run| run.claim_id.clone())
            .chain(
                scoped_pilot_events
                    .iter()
                    .map(|(claim_id, _)| claim_id.clone()),
            )
            .collect::<BTreeSet<_>>();
        let scoped_saving_attributions = saving_attribution_records
            .iter()
            .filter(|attribution| {
                customer_scope_id.is_none() || scoped_claim_ids.contains(&attribution.claim_id)
            })
            .cloned()
            .collect::<Vec<_>>();

        for run in scoped_runs.iter() {
            if run.risk_score >= 70 {
                if let Some(context) = claims.get(&run.claim_id) {
                    risk_amount += context.claim.amount.amount;
                }
            }
            *rag_distribution.entry(run.rag.clone()).or_insert(0) += 1;
            rule_hits += run.rule_runs.len() as u32;

            let model_key = run.model_score["model_key"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let score = run.model_score["score"].as_u64().unwrap_or(0) as u32;
            let entry = model_accumulators.entry(model_key).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += score;
            if score >= 70 {
                entry.2 += 1;
            }

            for layer in run
                .audit_event
                .get("layers")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                let layer_id = layer["layer_id"].as_str().unwrap_or("UNKNOWN").to_string();
                let layer_name = layer["name"].as_str().unwrap_or("Unknown").to_string();
                let layer_score = layer["score"].as_u64().unwrap_or(0) as u32;
                let entry =
                    layer_accumulators
                        .entry(layer_id)
                        .or_insert((layer_name.clone(), 0, 0, 0));
                entry.0 = layer_name;
                entry.1 += 1;
                entry.2 += layer_score;
                if layer_score >= 70 {
                    entry.3 += 1;
                }
            }
        }

        let mut saving_amount = Decimal::ZERO;
        let mut confirmed_fwa = 0_u32;
        let mut investigation_results = 0_u32;
        let mut qa_reviews = 0_u32;
        let mut outcome_labels = Vec::new();
        let mut financial_impacts = Vec::new();
        let feedback_statuses = latest_qa_feedback_statuses(
            &scoped_pilot_events
                .iter()
                .map(|(claim_id, event)| ((*claim_id).clone(), (*event).clone()))
                .collect::<Vec<_>>(),
        );

        for (_, event) in scoped_pilot_events.iter() {
            match event.event_type.as_str() {
                "investigation.result.received" => {
                    investigation_results += 1;
                    if let Ok(record) =
                        serde_json::from_value::<InvestigationResultRecord>(event.payload.clone())
                    {
                        outcome_labels.extend(labels_from_investigation_result(record.clone()));
                        if let Some(impact) = financial_impact_from_investigation(&record) {
                            financial_impacts.push(impact);
                        }
                    }
                    if event.payload["confirmed_fwa"].as_bool().unwrap_or(false) {
                        confirmed_fwa += 1;
                    }
                    if let Some(value) = event.payload["saving_amount"].as_str() {
                        saving_amount += value.parse::<Decimal>().unwrap_or(Decimal::ZERO);
                    }
                }
                "qa.result.received" => {
                    qa_reviews += 1;
                    if let Ok(record) =
                        serde_json::from_value::<QaReviewRecord>(event.payload.clone())
                    {
                        let feedback_id = qa_feedback_id(&record.qa_case_id);
                        let feedback_status = feedback_statuses
                            .get(&feedback_id)
                            .map(|update| update.status.as_str())
                            .unwrap_or("open");
                        outcome_labels.push(label_from_qa_review(record, feedback_status));
                    }
                }
                "medical.review.recorded" => {
                    outcome_labels.extend(labels_from_medical_review_event(event));
                }
                _ => {}
            }
        }
        let runtime_events = self.audit_events.lock().await;
        let scoring_audit_runs = runtime_events
            .iter()
            .filter(|event| {
                event.event_type == "scoring.completed"
                    && event.event_status == "succeeded"
                    && customer_scope_id.is_none_or(|scope| {
                        audit_event_payload_matches_customer_scope(&event.payload, scope)
                    })
            })
            .count() as u32;
        let canonical_trace_audit_runs = runtime_events
            .iter()
            .filter(|event| {
                event.event_type == "scoring.completed"
                    && event.event_status == "succeeded"
                    && customer_scope_id.is_none_or(|scope| {
                        audit_event_payload_matches_customer_scope(&event.payload, scope)
                    })
                    && event
                        .payload
                        .get("canonical_claim_context_trace")
                        .and_then(Value::as_object)
                        .is_some()
            })
            .count() as u32;
        let audit_coverage =
            summarize_dashboard_audit_coverage(scoring_audit_runs, canonical_trace_audit_runs);
        for event in runtime_events.iter().filter(|event| {
            event.event_type == "medical.review.recorded"
                && customer_scope_id.is_none_or(|scope| {
                    audit_event_payload_matches_customer_scope(&event.payload, scope)
                })
        }) {
            let audit_event = AuditHistoryEventRecord {
                audit_id: event.audit_id.clone(),
                run_id: event.run_id.clone(),
                actor_role: event.actor_role.clone(),
                event_type: event.event_type.clone(),
                event_status: event.event_status.clone(),
                summary: event.summary.clone(),
                payload: event.payload.clone(),
                evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                created_at: None,
            };
            outcome_labels.extend(labels_from_medical_review_event(&audit_event));
        }
        let suspected_claims = scoped_runs
            .iter()
            .filter(|run| run.risk_score >= 70)
            .count() as u32;
        let saving_attributions = summarize_saving_attributions(&scoped_saving_attributions);
        drop(runtime_events);
        drop(pilot_events);
        drop(claims);
        drop(runs);

        let audit_samples = self.list_audit_samples(customer_scope_id).await?;
        let qa_review_records = self.list_qa_reviews(customer_scope_id).await?;
        let qa_feedback_items = self.list_qa_feedback_items(customer_scope_id).await?;
        let cases = self.list_cases(customer_scope_id).await?;
        let agent_runs = self.list_agent_runs(customer_scope_id).await?;
        let models = self.list_models().await?;
        let model_evaluations = self.list_model_evaluations().await?;
        let rules = self.list_rules().await?;
        let rule_performance = self.rule_performance().await?;
        let leads = self.list_leads(customer_scope_id).await?;
        let scheme_distribution = leads
            .iter()
            .fold(BTreeMap::new(), |mut distribution, lead| {
                *distribution.entry(lead.scheme_family.clone()).or_insert(0) += 1;
                distribution
            });
        let saving_segments = summarize_saving_segments(&scoped_saving_attributions, &leads);
        let false_positive_count = rule_performance
            .iter()
            .map(|record| record.false_positive_count)
            .sum::<u32>();
        let value_measurement = summarize_dashboard_value_measurement(
            &financial_impacts,
            rule_hits,
            false_positive_count,
        );

        Ok(DashboardSummaryRecord {
            suspected_claims,
            confirmed_fwa,
            risk_amount: risk_amount.to_string(),
            saving_amount: saving_amount.to_string(),
            rag_distribution,
            scheme_distribution,
            rule_hits,
            model_scores: model_accumulators
                .into_iter()
                .map(|(model_key, (scored_runs, score_sum, high_risk_count))| {
                    let average_score = if scored_runs == 0 {
                        0.0
                    } else {
                        score_sum as f64 / scored_runs as f64
                    };
                    (
                        model_key,
                        DashboardModelScoreRecord {
                            scored_runs,
                            average_score,
                            high_risk_count,
                        },
                    )
                })
                .collect(),
            layer_scores: layer_accumulators
                .into_iter()
                .map(
                    |(layer_id, (name, scored_runs, score_sum, high_risk_count))| {
                        let average_score = if scored_runs == 0 {
                            0.0
                        } else {
                            score_sum as f64 / scored_runs as f64
                        };
                        (
                            layer_id,
                            DashboardLayerScoreRecord {
                                name,
                                scored_runs,
                                average_score,
                                high_risk_count,
                            },
                        )
                    },
                )
                .collect(),
            saving_attributions,
            saving_segments,
            value_measurement,
            audit_coverage,
            label_pool: summarize_dashboard_label_pool(&outcome_labels),
            qa_queue: summarize_dashboard_qa_queue(
                &audit_samples,
                &qa_review_records,
                &qa_feedback_items,
            ),
            case_sla: summarize_dashboard_case_sla(&cases),
            agent_governance: summarize_dashboard_agent_governance(&agent_runs),
            model_governance: summarize_dashboard_model_governance(&models, &model_evaluations),
            rule_governance: summarize_dashboard_rule_governance(&rules, &rule_performance),
            investigation_results,
            qa_reviews,
        })
    }

    pub(super) async fn in_memory_provider_risk_summary(
        &self,
    ) -> anyhow::Result<ProviderRiskSummaryRecord> {
        let runs = self.runs.lock().await;
        Ok(summarize_provider_risk_profiles(
            runs.iter().map(|run| &run.audit_event),
        ))
    }
}
