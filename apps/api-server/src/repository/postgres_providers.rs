use super::*;

pub(super) async fn provider_risk_summary(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<ProviderRiskSummaryRecord> {
    let rows: Vec<(Value,)> = sqlx::query_as(
        "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'",
    )
    .fetch_all(&repository.pool)
    .await?;
    Ok(summarize_provider_risk_profiles(
        rows.iter().map(|(payload,)| payload),
    ))
}
