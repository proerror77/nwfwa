use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies
            .iter()
            .filter(|policy| policy.status == "active")
            .filter(|policy| routing_policy_review_mode_applies(&policy.review_mode, review_mode))
            .max_by_key(|policy| (policy.review_mode == review_mode, policy.version))
            .map(routing_policy_from_record))
    }

    pub(super) async fn in_memory_list_routing_policies(
        &self,
    ) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies.clone())
    }

    pub(super) async fn in_memory_save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord> {
        let record = routing_policy_record(policy, "draft", &owner, None, None);
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        policies.retain(|existing| {
            !(existing.policy_id == record.policy_id
                && existing.version == record.version
                && existing.review_mode == record.review_mode)
        });
        policies.push(record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        Ok(policies
            .iter()
            .find(|policy| {
                policy.policy_id == policy_id
                    && policy.version == version
                    && policy.review_mode == review_mode
            })
            .cloned())
    }

    pub(super) async fn in_memory_update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        let Some(policy) = policies.iter_mut().find(|policy| {
            policy.policy_id == policy_id
                && policy.version == version
                && policy.review_mode == review_mode
        }) else {
            return Ok(None);
        };
        policy.status = status.into();
        Ok(Some(policy.clone()))
    }

    pub(super) async fn in_memory_activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        let mut policies = self.routing_policies.lock().await;
        seed_default_routing_policy_records(&mut policies);
        if !policies.iter().any(|policy| {
            policy.policy_id == policy_id
                && policy.version == version
                && policy.review_mode == review_mode
        }) {
            return Ok(None);
        }
        for policy in policies
            .iter_mut()
            .filter(|policy| policy.review_mode == review_mode && policy.status == "active")
        {
            policy.status = "approved".into();
        }
        let policy = policies
            .iter_mut()
            .find(|policy| {
                policy.policy_id == policy_id
                    && policy.version == version
                    && policy.review_mode == review_mode
            })
            .expect("routing policy existence checked before activation");
        policy.status = "active".into();
        Ok(Some(policy.clone()))
    }
}
