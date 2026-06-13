use crate::{
    app::AppState,
    auth::AuthenticatedApiPrincipal,
    error::ApiError,
    repository::{PersistedAuditEvent, PersistedInboxClaimRun},
};
use axum::{extract::State, http::StatusCode, Json};
use fwa_core::AuditEventId;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[path = "inbox_utils.rs"]
mod inbox_utils;
use inbox_utils::*;
#[path = "inbox_validation.rs"]
mod inbox_validation;
#[path = "inbox_mapping.rs"]
mod inbox_mapping;
use inbox_mapping::*;

const MAPPING_VERSION: &str = "aiclaim-core-v1";
const SOURCE_BUSINESS_TIMEZONE: &str = "Asia/Shanghai";
const SOURCE_BUSINESS_UTC_OFFSET_SECONDS: i32 = 8 * 60 * 60;

#[derive(Debug, Serialize)]
pub struct InboxNormalizeResponse {
    pub run_id: String,
    pub audit_id: String,
    pub external_message_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub raw_payload_checksum: String,
    pub mapping_version: &'static str,
    pub validation_result: String,
    pub scoring_ready: bool,
    pub raw_payload_ref: Option<String>,
    pub validation_errors: Vec<InboxValidationError>,
    pub canonical_claim_context: Value,
    pub data_quality_signals: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxValidationError {
    pub field_path: String,
    pub severity: String,
    pub remediation: String,
}

#[derive(Clone, Copy)]
struct SourceInvoice<'a> {
    policy_index: usize,
    invoice_index: usize,
    value: &'a Value,
}

impl SourceInvoice<'_> {
    fn field_path(&self, field_name: &str) -> String {
        format!(
            "reportCase.policyList[{}].invoiceList[{}].{field_name}",
            self.policy_index, self.invoice_index
        )
    }
}

pub async fn normalize_claim_inbox(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<InboxNormalizeResponse>), ApiError> {
    if !principal.has_permission("tpa:inbox:normalize") {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            "missing permission: tpa:inbox:normalize",
        ));
    }
    let actor = principal.actor;
    let raw_payload_checksum = raw_payload_checksum(&payload);

    let mut validation_errors = Vec::new();
    let system_code = required_string(
        &payload,
        &["systemCode"],
        "systemCode",
        "source system code",
        &mut validation_errors,
    );
    let trans_no = required_string(
        &payload,
        &["transNo"],
        "transNo",
        "source transaction id",
        &mut validation_errors,
    );
    let report_no = required_string(
        &payload,
        &["reportCase", "reportNo"],
        "reportCase.reportNo",
        "claim report number",
        &mut validation_errors,
    );

    if let Some(system_code) = system_code.as_deref() {
        if system_code != actor.source_system {
            validation_errors.push(InboxValidationError {
                field_path: "systemCode".into(),
                severity: "error".into(),
                remediation: "systemCode must match the authenticated API key source system".into(),
            });
        }
    }

    let external_message_id = match (&system_code, &trans_no, &report_no) {
        (Some(system_code), Some(trans_no), Some(report_no)) => {
            Some(format!("{system_code}:{trans_no}:{report_no}"))
        }
        _ => None,
    };

    let mut data_quality_signals = Vec::new();
    let canonical_claim_context = if validation_errors
        .iter()
        .any(|error| error.severity == "error")
    {
        json!({})
    } else {
        build_canonical_claim_context(
            &payload,
            &mut validation_errors,
            &mut data_quality_signals,
            system_code.as_deref().unwrap_or_default(),
            report_no.as_deref().unwrap_or_default(),
        )
    };

    if string_at(&payload, &["reportCase", "calculateRisk"])
        .is_some_and(|value| value.eq_ignore_ascii_case("N"))
    {
        validation_errors.push(InboxValidationError {
            field_path: "reportCase.calculateRisk".into(),
            severity: "warning".into(),
            remediation:
                "treat calculateRisk=N as a source hint; do not bypass FWA scoring without customer config"
                    .into(),
        });
        push_signal(&mut data_quality_signals, "risk_bypass_hint");
    }

    let has_errors = validation_errors
        .iter()
        .any(|error| error.severity == "error");
    let has_warnings = validation_errors
        .iter()
        .any(|error| error.severity == "warning");
    let scoring_ready = !has_errors && !validation_errors.iter().any(blocks_direct_scoring);
    let validation_result = if has_errors {
        "rejected"
    } else if has_warnings {
        "accepted_with_warnings"
    } else {
        "accepted"
    }
    .to_string();
    let status = if has_errors {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::OK
    };
    let external_message_fingerprint = external_message_id
        .as_deref()
        .map(external_message_fingerprint);
    let audit_id = inbox_audit_id(external_message_fingerprint.as_deref());
    let run_id = external_message_fingerprint
        .as_ref()
        .map(|fingerprint| format!("inbox:{fingerprint}"))
        .unwrap_or_else(|| format!("inbox:{audit_id}"));
    let claim_id = report_no.clone().unwrap_or_else(|| "unknown".into());
    let raw_payload_ref = external_message_fingerprint
        .as_ref()
        .map(|fingerprint| format!("inbox://raw-claims/{fingerprint}"));
    let idempotency_key = external_message_fingerprint
        .as_ref()
        .map(|fingerprint| format!("inbox.claim.normalize:{fingerprint}"));

    if let Some(idempotency_key) = idempotency_key.as_deref() {
        if let Some(existing) = state
            .repository
            .get_inbox_claim_run_by_idempotency_key(idempotency_key, Some(&actor.customer_scope_id))
            .await
            .map_err(internal_error("INBOX_RECORD_LOAD_FAILED"))?
        {
            if existing.raw_payload_checksum != raw_payload_checksum {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "INBOX_IDEMPOTENCY_CONFLICT",
                    "same inbox idempotency key was received with a different raw payload checksum",
                ));
            }
            return inbox_response_from_record(existing);
        }
    }

    let evidence_refs = [
        raw_payload_ref.clone(),
        Some(format!("inbox_mappings:{MAPPING_VERSION}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let source_paths = canonical_source_paths(&canonical_claim_context);
    let validation_errors_value =
        serialize_inbox_record_value("validation_errors", &validation_errors)?;
    let data_quality_signals_value =
        serialize_inbox_record_value("data_quality_signals", &data_quality_signals)?;
    let evidence_refs_value = serialize_inbox_record_value("evidence_refs", &evidence_refs)?;

    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: audit_id.clone(),
            run_id: run_id.clone(),
            claim_id: claim_id.clone(),
            source_system: system_code.clone().unwrap_or(actor.source_system.clone()),
            actor_id: actor.actor_id,
            actor_role: actor.actor_role,
            event_type: "inbox.claim.normalized".into(),
            event_status: validation_result.clone(),
            summary: "raw claim inbox payload normalized".into(),
            payload: json!({
                "claim_id": claim_id,
                "source_system": system_code.clone().unwrap_or_else(|| state.config.source_system.clone()),
                "customer_scope_id": actor.customer_scope_id,
                "external_message_fingerprint": external_message_fingerprint,
                "idempotency_key": idempotency_key,
                "raw_payload_checksum": raw_payload_checksum,
                "mapping_version": MAPPING_VERSION,
                "validation_result": validation_result,
                "scoring_ready": scoring_ready,
                "raw_payload_ref": raw_payload_ref,
                "source_paths": source_paths,
                "validation_errors": validation_errors_value.clone(),
                "data_quality_signals": data_quality_signals_value.clone(),
                "status_code": status.as_u16()
            }),
            evidence_refs: evidence_refs
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        })
        .await
        .map_err(internal_error("INBOX_AUDIT_PERSISTENCE_FAILED"))?;
    state
        .repository
        .save_inbox_claim_run(PersistedInboxClaimRun {
            run_id: run_id.clone(),
            audit_id: audit_id.clone(),
            external_message_id: external_message_id.clone(),
            idempotency_key: idempotency_key.clone(),
            external_message_fingerprint: external_message_fingerprint.clone(),
            raw_payload_checksum: raw_payload_checksum.clone(),
            raw_payload_ref: raw_payload_ref.clone(),
            mapping_version: MAPPING_VERSION.into(),
            validation_result: validation_result.clone(),
            scoring_ready,
            claim_id: claim_id.clone(),
            source_system: system_code.clone().unwrap_or(actor.source_system.clone()),
            customer_scope_id: actor.customer_scope_id.clone(),
            canonical_claim_context: canonical_claim_context.clone(),
            validation_errors: validation_errors_value,
            data_quality_signals: data_quality_signals_value,
            evidence_refs: evidence_refs_value,
        })
        .await
        .map_err(internal_error("INBOX_RECORD_PERSISTENCE_FAILED"))?;

    Ok((
        status,
        Json(InboxNormalizeResponse {
            run_id,
            audit_id,
            raw_payload_ref,
            idempotency_key,
            raw_payload_checksum,
            external_message_id,
            mapping_version: MAPPING_VERSION,
            validation_result,
            scoring_ready,
            validation_errors,
            canonical_claim_context,
            data_quality_signals,
            evidence_refs,
        }),
    ))
}

fn inbox_response_from_record(
    record: PersistedInboxClaimRun,
) -> Result<(StatusCode, Json<InboxNormalizeResponse>), ApiError> {
    let validation_errors = decode_inbox_record_json(
        &record.run_id,
        "validation_errors",
        record.validation_errors,
    )?;
    let data_quality_signals = decode_inbox_record_json(
        &record.run_id,
        "data_quality_signals",
        record.data_quality_signals,
    )?;
    let evidence_refs =
        decode_inbox_record_json(&record.run_id, "evidence_refs", record.evidence_refs)?;
    Ok((
        status_for_validation_result(&record.validation_result),
        Json(InboxNormalizeResponse {
            run_id: record.run_id,
            audit_id: record.audit_id,
            external_message_id: record.external_message_id,
            idempotency_key: record.idempotency_key,
            raw_payload_checksum: record.raw_payload_checksum,
            mapping_version: MAPPING_VERSION,
            validation_result: record.validation_result,
            scoring_ready: record.scoring_ready,
            raw_payload_ref: record.raw_payload_ref,
            validation_errors,
            canonical_claim_context: record.canonical_claim_context,
            data_quality_signals,
            evidence_refs,
        }),
    ))
}

fn decode_inbox_record_json<T>(
    run_id: &str,
    field: &'static str,
    value: serde_json::Value,
) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value).map_err(|error| {
        tracing::warn!(
            run_id,
            field,
            error = %error,
            "stored inbox idempotency payload is not decodable"
        );
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INBOX_RECORD_CORRUPT",
            "stored inbox normalization record is corrupt",
        )
    })
}

fn serialize_inbox_record_value<T: Serialize>(
    field: &'static str,
    value: &T,
) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(|error| {
        tracing::error!(
            field,
            error = %error,
            "inbox normalization record field failed to serialize"
        );
        ApiError::internal("INBOX_RECORD_SERIALIZATION_FAILED", error)
    })
}

fn status_for_validation_result(validation_result: &str) -> StatusCode {
    if validation_result == "rejected" {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::OK
    }
}

fn internal_error(
    code: &'static str,
) -> impl FnOnce(anyhow::Error) -> ApiError + Send + Sync + 'static {
    move |error| ApiError::internal(code, error)
}

fn raw_payload_checksum(payload: &Value) -> String {
    let mut hasher = Sha256::new();
    let bytes = serde_json::to_vec(payload).unwrap_or_default();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn blocks_direct_scoring(error: &InboxValidationError) -> bool {
    if error.severity != "warning" {
        return false;
    }
    let path = error.field_path.as_str();
    is_policy_level_blocking_path(path)
        || (path.starts_with("reportCase.policyList[")
            && path.contains(".productList[")
            && matches!(
                path.rsplit('.').next(),
                Some("validateDate" | "claimValidateDate" | "expireDate")
            ))
}

fn is_policy_level_blocking_path(path: &str) -> bool {
    if !path.starts_with("reportCase.policyList[")
        || path.contains(".productList[")
        || path.contains(".invoiceList[")
    {
        return false;
    }
    matches!(
        path.rsplit('.').next(),
        Some("coverageLimit" | "validateDate" | "expireDate")
    )
}

fn inbox_audit_id(external_message_fingerprint: Option<&str>) -> String {
    external_message_fingerprint
        .map(|fingerprint| format!("aud_inbox_{}", stable_id_fragment(fingerprint)))
        .unwrap_or_else(|| AuditEventId::new().to_string())
}

fn external_message_fingerprint(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{digest:x}")
}
