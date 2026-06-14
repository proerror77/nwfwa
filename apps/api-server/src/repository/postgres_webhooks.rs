use super::*;

pub(super) async fn list_webhook_events(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<WebhookEventRecord>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Value,
        Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT audit_id, run_id, actor_role, event_type, event_status, summary, payload, evidence_refs, created_at
             FROM audit_events
             ORDER BY created_at, audit_id",
    )
    .fetch_all(&repository.pool)
    .await?;

    let mut events = rows
        .into_iter()
        .filter_map(
            |(
                audit_id,
                run_id,
                actor_role,
                event_type,
                event_status,
                summary,
                payload,
                evidence_refs,
                created_at,
            )| {
                webhook_event_from_audit(
                    None,
                    &AuditHistoryEventRecord {
                        audit_id,
                        run_id,
                        actor_role,
                        event_type,
                        event_status,
                        summary,
                        payload,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        created_at: Some(created_at.to_rfc3339()),
                    },
                )
            },
        )
        .collect::<Vec<_>>();
    let attempt_rows: Vec<(
        String,
        i32,
        String,
        Option<i32>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT event_id, attempt_number, delivery_status, response_status_code, error_message, next_attempt_at, attempted_at
             FROM webhook_delivery_attempts
             ORDER BY event_id, attempt_number",
    )
    .fetch_all(&repository.pool)
    .await?;
    let attempts = attempt_rows
        .into_iter()
        .map(
            |(
                event_id,
                attempt_number,
                delivery_status,
                response_status_code,
                error_message,
                next_attempt_at,
                attempted_at,
            )| WebhookDeliveryAttemptRecord {
                event_id,
                attempt_number: attempt_number.max(0) as u32,
                delivery_status,
                response_status_code: response_status_code.map(|value| value.max(0) as u16),
                error_message,
                next_attempt_at: next_attempt_at.map(|timestamp| timestamp.to_rfc3339()),
                attempted_at: Some(attempted_at.to_rfc3339()),
            },
        )
        .collect::<Vec<_>>();
    apply_webhook_delivery_state(&mut events, &attempts);
    sort_webhook_events(&mut events);
    Ok(events)
}

pub(super) async fn save_webhook_delivery_attempt(
    repository: &PostgresScoringRepository,
    input: WebhookDeliveryAttemptInput,
) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
    let row: (Option<i32>,) = sqlx::query_as(
        "SELECT MAX(attempt_number)
             FROM webhook_delivery_attempts
             WHERE event_id = $1",
    )
    .bind(&input.event_id)
    .fetch_one(&repository.pool)
    .await?;
    let attempt_number = row.0.unwrap_or(0) + 1;
    let attempted_at = chrono::Utc::now();
    let next_attempt_at =
        next_webhook_attempt_at(&input.delivery_status, attempt_number as u32, attempted_at);
    let inserted: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "INSERT INTO webhook_delivery_attempts
             (event_id, attempt_number, delivery_status, response_status_code, error_message, next_attempt_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING attempted_at",
    )
    .bind(&input.event_id)
    .bind(attempt_number)
    .bind(&input.delivery_status)
    .bind(input.response_status_code.map(i32::from))
    .bind(&input.error_message)
    .bind(next_attempt_at)
    .fetch_one(&repository.pool)
    .await?;
    Ok(WebhookDeliveryAttemptRecord {
        event_id: input.event_id,
        attempt_number: attempt_number as u32,
        delivery_status: input.delivery_status,
        response_status_code: input.response_status_code,
        error_message: input.error_message,
        next_attempt_at: next_attempt_at.map(|timestamp| timestamp.to_rfc3339()),
        attempted_at: Some(inserted.0.to_rfc3339()),
    })
}
