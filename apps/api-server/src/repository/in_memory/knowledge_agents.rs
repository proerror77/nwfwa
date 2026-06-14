use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_list_knowledge_cases(
        &self,
    ) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        let mut cases = default_knowledge_cases()
            .into_iter()
            .map(|case| (case.case_id.clone(), case))
            .collect::<HashMap<_, _>>();
        cases.extend(self.knowledge_cases.lock().await.clone());
        let mut cases = cases.into_values().collect::<Vec<_>>();
        cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        Ok(cases)
    }

    pub(super) async fn in_memory_save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        self.knowledge_cases
            .lock()
            .await
            .insert(record.case_id.clone(), record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        Ok(search_cases(
            self.in_memory_list_knowledge_cases().await?,
            &query,
        ))
    }

    pub(super) async fn in_memory_save_agent_registry(
        &self,
        record: AgentRegistryRecord,
    ) -> anyhow::Result<AgentRegistryRecord> {
        self.agent_registry
            .lock()
            .await
            .insert(record.agent_identity_id.clone(), record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_active_agent_registry(
        &self,
        agent_kind: &str,
        agent_version: u32,
    ) -> anyhow::Result<Option<AgentRegistryRecord>> {
        let registry = self.agent_registry.lock().await;
        let active = registry
            .values()
            .find(|record| {
                record.agent_kind == agent_kind
                    && record.agent_version == agent_version
                    && record.status == "active"
            })
            .cloned();
        if active.is_some() || !registry.is_empty() {
            return Ok(active);
        }
        let default = default_agent_registry_record();
        Ok((default.agent_kind == agent_kind
            && default.agent_version == agent_version
            && default.status == "active")
            .then_some(default))
    }

    pub(super) async fn in_memory_save_agent_run(
        &self,
        run: PersistedAgentRun,
    ) -> anyhow::Result<()> {
        let registry = default_agent_registry_record();
        self.agent_registry
            .lock()
            .await
            .entry(registry.agent_identity_id.clone())
            .or_insert(registry);
        let investigation = agent_investigation_record_for_claim(&run.claim_id);
        self.agent_investigations.lock().await.insert(
            investigation.investigation_id.clone(),
            investigation.clone(),
        );
        let previous_event_hash = self
            .agent_audit_events
            .lock()
            .await
            .iter()
            .rev()
            .find(|event| event.agent_run_id == run.agent_run_id)
            .map(|event| event.event_hash.clone());
        let audit_event =
            agent_audit_event_from_run(&run, &investigation.investigation_id, previous_event_hash);
        self.agent_runs.lock().await.push(run);
        self.agent_audit_events.lock().await.push(audit_event);
        Ok(())
    }

    pub(super) async fn in_memory_list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        let scoped_claim_ids = match customer_scope_id {
            Some(scope) => Some(scoped_claim_ids_from_audit_events(
                self.audit_events.lock().await.iter(),
                scope,
            )),
            None => None,
        };
        let mut runs = self
            .agent_runs
            .lock()
            .await
            .iter()
            .filter(|run| {
                scoped_claim_ids
                    .as_ref()
                    .is_none_or(|claim_ids| claim_ids.contains(&run.claim_id))
            })
            .map(agent_run_log_from_persisted)
            .collect::<Vec<_>>();
        runs.sort_by(|left, right| left.agent_run_id.cmp(&right.agent_run_id));
        Ok(runs)
    }

    pub(super) async fn in_memory_cancel_agent_run(
        &self,
        agent_run_id: &str,
    ) -> anyhow::Result<()> {
        let mut runs = self.agent_runs.lock().await;
        let Some(run) = runs.iter_mut().find(|run| run.agent_run_id == agent_run_id) else {
            anyhow::bail!("agent run not found: {agent_run_id}");
        };
        run.status = "cancelled".into();
        Ok(())
    }

    pub(super) async fn in_memory_save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        let mut runs = self.agent_runs.lock().await;
        let Some(run) = runs
            .iter_mut()
            .find(|run| run.agent_run_id == approval.agent_run_id)
        else {
            anyhow::bail!("agent run not found: {}", approval.agent_run_id);
        };
        if let Some(existing) = run
            .approvals
            .iter_mut()
            .find(|existing| existing.approval_id == approval.approval_id)
        {
            *existing = approval.clone();
        } else {
            run.approvals.push(approval.clone());
        }
        Ok(approval)
    }
}
