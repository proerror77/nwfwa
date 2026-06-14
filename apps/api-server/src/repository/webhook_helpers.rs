use super::{
    is_terminal_case_status, AuditHistoryEventRecord, WebhookDeliveryAttemptRecord,
    WebhookEventRecord,
};
use serde_json::Value;

const WEBHOOK_MAX_ATTEMPTS: u32 = 3;

pub(super) fn webhook_event_from_audit(
    source_claim_id: Option<&str>,
    event: &AuditHistoryEventRecord,
) -> Option<WebhookEventRecord> {
    if event.event_status != "succeeded" {
        return None;
    }
    let event_type = match event.event_type.as_str() {
        "scoring.completed" => "fwa.score.completed",
        "lead.triaged"
            if event.payload["decision"].as_str() == Some("open_case")
                && event.payload["case_id"].as_str().is_some() =>
        {
            "fwa.case.routed"
        }
        "lead.triaged" => return None,
        "investigation.result.received" => "fwa.investigation.closed",
        "qa.result.received" => "fwa.qa.reviewed",
        "medical.review.recorded" => "fwa.medical.reviewed",
        "case.status.updated" => {
            let to_status = event.payload["to_status"].as_str().unwrap_or_default();
            if is_terminal_case_status(to_status) {
                "fwa.investigation.closed"
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let claim_id = event.payload["claim_id"]
        .as_str()
        .or(source_claim_id)
        .unwrap_or_default()
        .to_string();
    if claim_id.is_empty() {
        return None;
    }
    let event_id = format!("webhook_{}", event.audit_id);
    let idempotency_key = format!("fwa-webhook:{}:{}", event_type, event.audit_id);
    let customer_scope_id = event
        .payload
        .get("customer_scope_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let signature_base_string = format!(
        "{}.{}.{}.{}",
        event_type, event.audit_id, event.run_id, claim_id
    );
    Some(WebhookEventRecord {
        event_id,
        event_type: event_type.into(),
        source_event_type: event.event_type.clone(),
        source_audit_id: event.audit_id.clone(),
        customer_scope_id,
        claim_id,
        run_id: event.run_id.clone(),
        delivery_status: "pending".into(),
        retry_count: 0,
        max_attempts: WEBHOOK_MAX_ATTEMPTS,
        next_attempt_at: event.created_at.clone(),
        last_attempt_at: None,
        last_response_status_code: None,
        last_error_message: None,
        idempotency_key,
        signature_key_id: "tpa-webhook-v1".into(),
        signature_algorithm: "hmac-sha256".into(),
        signature_base_string,
        payload: event.payload.clone(),
        evidence_refs: event.evidence_refs.clone(),
        occurred_at: event.created_at.clone(),
    })
}

pub(super) fn apply_webhook_delivery_state(
    events: &mut [WebhookEventRecord],
    attempts: &[WebhookDeliveryAttemptRecord],
) {
    for event in events {
        let mut event_attempts = attempts
            .iter()
            .filter(|attempt| attempt.event_id == event.event_id)
            .collect::<Vec<_>>();
        event_attempts.sort_by_key(|attempt| attempt.attempt_number);
        let Some(latest) = event_attempts.last() else {
            continue;
        };
        event.retry_count = event_attempts.len() as u32;
        event.last_attempt_at = latest.attempted_at.clone();
        event.last_response_status_code = latest.response_status_code;
        event.last_error_message = latest.error_message.clone();
        event.next_attempt_at = latest.next_attempt_at.clone();
        event.delivery_status = if latest.delivery_status == "delivered" {
            "delivered".into()
        } else if event.retry_count >= event.max_attempts {
            "failed".into()
        } else {
            "retry_wait".into()
        };
    }
}

pub(super) fn next_webhook_attempt_at(
    delivery_status: &str,
    attempt_number: u32,
    attempted_at: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    if delivery_status != "failed" || attempt_number >= WEBHOOK_MAX_ATTEMPTS {
        return None;
    }
    let delay_minutes = match attempt_number {
        1 => 5,
        2 => 15,
        _ => 60,
    };
    Some(attempted_at + chrono::Duration::minutes(delay_minutes))
}

pub(super) fn sort_webhook_events(events: &mut [WebhookEventRecord]) {
    events.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
}
