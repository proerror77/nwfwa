use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let backtests = self.rule_backtests.lock().await.clone();
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let mut rules = details
            .into_iter()
            .map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                if let Some(backtest) = latest_rule_backtest_for(
                    &backtests,
                    &detail.summary.rule_id,
                    detail.summary.latest_version,
                ) {
                    apply_rule_backtest_metadata(&mut detail.summary, Some(backtest));
                }
                detail.summary
            })
            .collect::<Vec<_>>();
        rules.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
        Ok(rules)
    }

    pub(super) async fn in_memory_list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        let statuses = self.rule_statuses.lock().await;
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        details
            .into_iter()
            .filter_map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                (detail.summary.status == "active").then_some(detail)
            })
            .map(runtime_rule_from_detail)
            .collect()
    }

    pub(super) async fn in_memory_get_rule(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Option<RuleDetailRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let backtests = self.rule_backtests.lock().await.clone();
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let audit_events = self.in_memory_rule_audit_history(rule_id).await?;
        Ok(details
            .into_iter()
            .find(|detail| detail.summary.rule_id == rule_id)
            .map(|mut detail| {
                apply_rule_status(&mut detail, &statuses);
                if let Some(backtest) = latest_rule_backtest_for(
                    &backtests,
                    &detail.summary.rule_id,
                    detail.summary.latest_version,
                ) {
                    apply_rule_backtest_metadata(&mut detail.summary, Some(backtest));
                }
                detail.audit_events = audit_events;
                detail
            }))
    }

    pub(super) async fn in_memory_rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        Ok(self
            .audit_events
            .lock()
            .await
            .iter()
            .filter(|event| event.payload["rule_id"].as_str() == Some(rule_id))
            .map(|event| AuditHistoryEventRecord {
                audit_id: event.audit_id.clone(),
                run_id: event.run_id.clone(),
                actor_role: event.actor_role.clone(),
                event_type: event.event_type.clone(),
                event_status: event.event_status.clone(),
                summary: event.summary.clone(),
                payload: event.payload.clone(),
                evidence_refs: evidence_values_to_strings(&event.evidence_refs),
                created_at: None,
            })
            .collect())
    }

    pub(super) async fn in_memory_save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        let detail = rule_detail_from_rule(rule, "draft", owner);
        self.candidate_rules
            .lock()
            .await
            .insert(detail.summary.rule_id.clone(), detail.clone());
        Ok(detail)
    }

    pub(super) async fn in_memory_update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        if self.in_memory_get_rule(rule_id).await?.is_none() {
            return Ok(None);
        }
        self.rule_statuses
            .lock()
            .await
            .insert(rule_id.to_string(), status.to_string());
        Ok(self
            .in_memory_get_rule(rule_id)
            .await?
            .map(|detail| detail.summary))
    }

    pub(super) async fn in_memory_list_rule_conditions(
        &self,
    ) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
        let statuses = self.rule_statuses.lock().await;
        let mut details = default_rule_details();
        details.extend(self.candidate_rules.lock().await.values().cloned());
        let mut records = Vec::new();
        for mut detail in details {
            apply_rule_status(&mut detail, &statuses);
            records.extend(rule_condition_records_from_detail(&detail)?);
        }
        records.sort_by(|left, right| left.condition_key.cmp(&right.condition_key));
        Ok(records)
    }

    pub(super) async fn in_memory_rule_performance(
        &self,
    ) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        let rules = self.in_memory_list_rules().await?;
        let runs = self.runs.lock().await;
        let pilot_events = self.pilot_audit_events.lock().await;
        let mut outcomes = HashMap::new();
        for (_, event) in pilot_events.iter() {
            if event.event_type != "investigation.result.received" {
                continue;
            }
            let Some(claim_id) = event.payload["claim_id"].as_str() else {
                continue;
            };
            outcomes.insert(
                claim_id.to_string(),
                InvestigationOutcome {
                    confirmed_fwa: event.payload["confirmed_fwa"].as_bool().unwrap_or(false),
                    saving_amount: decimal_from_json(&event.payload["saving_amount"]),
                },
            );
        }

        let mut accumulators = rule_accumulators_from_rules(&rules);
        for run in runs.iter() {
            for rule_run in &run.rule_runs {
                let Some(rule_id) = rule_run["rule_id"].as_str() else {
                    continue;
                };
                let Some(accumulator) = accumulators.get_mut(rule_id) else {
                    continue;
                };
                accumulator.trigger_count += 1;
                accumulator.triggered_claim_ids.insert(run.claim_id.clone());
            }
        }

        Ok(rule_performance_records(
            accumulators,
            &outcomes,
            runs.len() as u32,
        ))
    }

    pub(super) async fn in_memory_save_rule_backtest(
        &self,
        mut record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.rule_backtests.lock().await.push(record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        Ok(self
            .rule_backtests
            .lock()
            .await
            .iter()
            .rev()
            .find(|record| record.rule_id == rule_id && record.rule_version == rule_version)
            .cloned())
    }

    pub(super) async fn in_memory_save_rule_shadow_run(
        &self,
        mut record: RuleShadowRunRecord,
    ) -> anyhow::Result<RuleShadowRunRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.rule_shadow_runs.lock().await.push(record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_latest_rule_shadow_run(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleShadowRunRecord>> {
        Ok(self
            .rule_shadow_runs
            .lock()
            .await
            .iter()
            .rev()
            .find(|record| record.rule_id == rule_id && record.rule_version == rule_version)
            .cloned())
    }

    pub(super) async fn in_memory_save_rule_promotion_review(
        &self,
        mut record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        record.created_at = Some(chrono::Utc::now().to_rfc3339());
        self.rule_promotion_reviews
            .lock()
            .await
            .push(record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        Ok(self
            .rule_promotion_reviews
            .lock()
            .await
            .iter()
            .rev()
            .find(|review| review.rule_id == rule_id && review.rule_version == rule_version)
            .cloned())
    }
}
