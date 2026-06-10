use super::ops_bootstrap_types::*;
use crate::{
    app::AppState,
    auth::AuthenticatedActor,
    error::ApiError,
    repository::{AuditEventListFilter, AuditHistoryEventRecord, LeadRecord, PersistedAuditEvent},
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_core::{AuditEventId, ScoringRunId};
use serde_json::{json, Value};
use std::collections::BTreeMap;

mod backfill;
mod evidence;
mod label;

pub use backfill::{
    create_historical_backfill, list_historical_backfill_leads, list_historical_backfills,
};
pub use evidence::{
    generate_evidence_requests, list_evidence_requests, update_evidence_request_status,
};
pub use label::{label_bootstrap_queue, review_label_bootstrap_item};

fn validate_optional_notes(notes: Option<&str>, code: &'static str) -> Result<(), ApiError> {
    if notes
        .filter(|value| pii::contains_pii(std::iter::once(*value)))
        .is_some()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "notes must not contain PII",
        ));
    }
    Ok(())
}

fn json_array_to_strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn not_found(code: &'static str, message: &'static str) -> impl FnOnce() -> ApiError {
    move || ApiError::new(StatusCode::NOT_FOUND, code, message)
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
