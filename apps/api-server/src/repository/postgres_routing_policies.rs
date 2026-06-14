use super::*;

pub(super) async fn active_routing_policy(
    repository: &PostgresScoringRepository,
    review_mode: &str,
) -> anyhow::Result<Option<RoutingPolicy>> {
    ensure_default_routing_policies_seeded(&repository.pool).await?;
    let row: Option<(Value,)> = sqlx::query_as(
        "SELECT policy_json
         FROM routing_policies
         WHERE status = 'active'
           AND review_mode IN ($1, 'both')
         ORDER BY CASE WHEN review_mode = $1 THEN 0 ELSE 1 END, version DESC
         LIMIT 1",
    )
    .bind(review_mode)
    .fetch_optional(&repository.pool)
    .await?;

    row.map(|row| serde_json::from_value(row.0))
        .transpose()
        .map_err(Into::into)
}

pub(super) async fn list_routing_policies(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
    ensure_default_routing_policies_seeded(&repository.pool).await?;
    let rows: Vec<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT policy_json, status, owner, activated_at::text, created_at::text
         FROM routing_policies
         ORDER BY policy_key, review_mode, version DESC",
    )
    .fetch_all(&repository.pool)
    .await?;

    rows.into_iter()
        .map(routing_policy_record_from_row)
        .collect()
}

pub(super) async fn save_routing_policy_candidate(
    repository: &PostgresScoringRepository,
    policy: RoutingPolicy,
    owner: String,
) -> anyhow::Result<RoutingPolicyRecord> {
    ensure_default_routing_policies_seeded(&repository.pool).await?;
    sqlx::query(
        "INSERT INTO routing_policies
         (policy_key, version, review_mode, status, owner, policy_json)
         VALUES ($1, $2, $3, 'draft', $4, $5)
         ON CONFLICT (policy_key, version, review_mode) DO UPDATE
         SET status = 'draft',
             owner = EXCLUDED.owner,
             policy_json = EXCLUDED.policy_json",
    )
    .bind(&policy.policy_id)
    .bind(policy.version as i32)
    .bind(&policy.review_mode)
    .bind(&owner)
    .bind(serde_json::to_value(&policy)?)
    .execute(&repository.pool)
    .await?;

    Ok(routing_policy_record(policy, "draft", &owner, None, None))
}

pub(super) async fn get_routing_policy(
    repository: &PostgresScoringRepository,
    policy_id: &str,
    version: u32,
    review_mode: &str,
) -> anyhow::Result<Option<RoutingPolicyRecord>> {
    ensure_default_routing_policies_seeded(&repository.pool).await?;
    let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT policy_json, status, owner, activated_at::text, created_at::text
             FROM routing_policies
             WHERE policy_key = $1 AND version = $2 AND review_mode = $3",
    )
    .bind(policy_id)
    .bind(version as i32)
    .bind(review_mode)
    .fetch_optional(&repository.pool)
    .await?;

    row.map(routing_policy_record_from_row).transpose()
}

pub(super) async fn update_routing_policy_status(
    repository: &PostgresScoringRepository,
    policy_id: &str,
    version: u32,
    review_mode: &str,
    status: &str,
) -> anyhow::Result<Option<RoutingPolicyRecord>> {
    ensure_default_routing_policies_seeded(&repository.pool).await?;
    let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "UPDATE routing_policies
             SET status = $4
             WHERE policy_key = $1 AND version = $2 AND review_mode = $3
             RETURNING policy_json, status, owner, activated_at::text, created_at::text",
    )
    .bind(policy_id)
    .bind(version as i32)
    .bind(review_mode)
    .bind(status)
    .fetch_optional(&repository.pool)
    .await?;

    row.map(routing_policy_record_from_row).transpose()
}

pub(super) async fn activate_routing_policy(
    repository: &PostgresScoringRepository,
    policy_id: &str,
    version: u32,
    review_mode: &str,
) -> anyhow::Result<Option<RoutingPolicyRecord>> {
    ensure_default_routing_policies_seeded(&repository.pool).await?;
    let mut tx = repository.pool.begin().await?;
    sqlx::query(
        "UPDATE routing_policies
         SET status = 'approved'
         WHERE review_mode = $1
           AND status = 'active'
           AND NOT (policy_key = $2 AND version = $3)",
    )
    .bind(review_mode)
    .bind(policy_id)
    .bind(version as i32)
    .execute(&mut *tx)
    .await?;

    let row: Option<(Value, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "UPDATE routing_policies
             SET status = 'active', activated_at = now()
             WHERE policy_key = $1 AND version = $2 AND review_mode = $3
             RETURNING policy_json, status, owner, activated_at::text, created_at::text",
    )
    .bind(policy_id)
    .bind(version as i32)
    .bind(review_mode)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    row.map(routing_policy_record_from_row).transpose()
}
