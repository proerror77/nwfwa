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
use inbox_validation::*;
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
            return Ok(inbox_response_from_record(existing));
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
                "validation_errors": validation_errors,
                "data_quality_signals": data_quality_signals,
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
            validation_errors: serde_json::to_value(&validation_errors)
                .unwrap_or_else(|_| serde_json::json!([])),
            data_quality_signals: serde_json::json!(data_quality_signals.clone()),
            evidence_refs: serde_json::json!(evidence_refs.clone()),
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
) -> (StatusCode, Json<InboxNormalizeResponse>) {
    let validation_errors = serde_json::from_value(record.validation_errors).unwrap_or_default();
    let data_quality_signals =
        serde_json::from_value(record.data_quality_signals).unwrap_or_default();
    let evidence_refs = serde_json::from_value(record.evidence_refs).unwrap_or_default();
    (
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
    )
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

fn canonical_source_paths(canonical_claim_context: &Value) -> Vec<String> {
    let mut source_paths = Vec::new();
    collect_source_paths_from_array(
        canonical_claim_context,
        &["document_evidence"],
        &mut source_paths,
    );
    collect_source_paths_from_array(
        canonical_claim_context,
        &["itemized_bill_lines"],
        &mut source_paths,
    );
    collect_source_paths_from_array(
        canonical_claim_context,
        &["member_policy_snapshot", "product_liabilities"],
        &mut source_paths,
    );
    source_paths.sort();
    source_paths.dedup();
    source_paths
}

fn collect_source_paths_from_array(value: &Value, path: &[&str], source_paths: &mut Vec<String>) {
    for item in array_items(value, path) {
        if let Some(source_path) = string_at(item, &["source_path"]) {
            source_paths.push(source_path);
        }
        if let Some(source_path) = string_at(item, &["liability_source_path"]) {
            source_paths.push(source_path);
        }
    }
}

fn stable_id_fragment(value: &str) -> String {
    let fragment = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(96)
        .collect::<String>();
    if fragment.is_empty() {
        "unknown".into()
    } else {
        fragment
    }
}

fn build_canonical_claim_context(
    payload: &Value,
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    source_system: &str,
    report_no: &str,
) -> Value {
    let policies = array_items(payload, &["reportCase", "policyList"]);
    let policy = policies.first().copied();
    let invoices = policies
        .iter()
        .enumerate()
        .flat_map(|(policy_index, policy)| {
            array_items(policy, &["invoiceList"])
                .into_iter()
                .enumerate()
                .map(move |(invoice_index, value)| SourceInvoice {
                    policy_index,
                    invoice_index,
                    value,
                })
        })
        .collect::<Vec<_>>();
    let invoice = invoices.first().map(|invoice| invoice.value);
    let medical_records = array_items(payload, &["reportCase", "medicalRecordInfoList"]);
    let medical_record = medical_records.first().copied();
    let product = policy.and_then(|policy| first_array_item(policy, &["productList"]));
    let liability = product.and_then(|product| first_array_item(product, &["claimLiabilityList"]));

    if policy.is_none() {
        validation_errors.push(InboxValidationError {
            field_path: "reportCase.policyList".into(),
            severity: "error".into(),
            remediation: "include at least one policy for coverage mapping".into(),
        });
    }
    if invoice.is_none() {
        validation_errors.push(InboxValidationError {
            field_path: "reportCase.policyList[0].invoiceList".into(),
            severity: "error".into(),
            remediation: "include at least one invoice for bill-line mapping".into(),
        });
    }

    validate_policy_coverage_limits(validation_errors, data_quality_signals, &policies);

    let insured_name = string_at(payload, &["reportCase", "accidentPerson", "insuredName"]);
    let policy_insured_name = policy.and_then(|policy| string_at(policy, &["insuredName"]));
    let mut identity_names = vec![insured_name.as_deref(), policy_insured_name.as_deref()];
    let medical_record_patient_names = medical_records
        .iter()
        .map(|record| string_at(record, &["patientName"]))
        .collect::<Vec<_>>();
    identity_names.extend(medical_record_patient_names.iter().map(Option::as_deref));
    let invoice_person_names = invoices
        .iter()
        .map(|invoice| string_at(invoice.value, &["accidentPersonName"]))
        .collect::<Vec<_>>();
    identity_names.extend(invoice_person_names.iter().map(Option::as_deref));
    if names_mismatch(identity_names) {
        push_signal(data_quality_signals, "identity_mismatch");
    }

    let service_date = invoice
        .and_then(|invoice| epoch_date_at(invoice, &["startDate"]))
        .or_else(|| epoch_date_at(payload, &["reportCase", "accidentDate"]));
    let service_date_raw_epoch_ms = invoice
        .and_then(|invoice| epoch_millis_at(invoice, &["startDate"]))
        .or_else(|| epoch_millis_at(payload, &["reportCase", "accidentDate"]));
    let receive_date = epoch_date_at(payload, &["reportCase", "claimReceiveDate"]);
    let accident_date = epoch_date_at(payload, &["reportCase", "accidentDate"]);
    let policy_start_date = policy.and_then(|policy| epoch_date_at(policy, &["validateDate"]));
    let policy_end_date = policy.and_then(|policy| epoch_date_at(policy, &["expireDate"]));
    let product_start_date = product.and_then(|product| epoch_date_at(product, &["validateDate"]));
    let product_end_date = product.and_then(|product| epoch_date_at(product, &["expireDate"]));
    let liability_start_date =
        liability.and_then(|liability| epoch_date_at(liability, &["validateDate"]));
    let liability_claim_start_date =
        liability.and_then(|liability| epoch_date_at(liability, &["claimValidateDate"]));
    let liability_end_date =
        liability.and_then(|liability| epoch_date_at(liability, &["expireDate"]));
    let coverage_start_date = product_start_date.or(policy_start_date);
    let coverage_end_date = product_end_date.or(policy_end_date);
    let coverage_limit = policy.and_then(|policy| number_at(policy, &["coverageLimit"]));
    if let (Some(service_date), Some(receive_date)) = (service_date, receive_date) {
        if receive_date < service_date {
            validation_errors.push(InboxValidationError {
                field_path: "reportCase.claimReceiveDate".into(),
                severity: "warning".into(),
                remediation: "claim receive date should not be earlier than service date".into(),
            });
            push_signal(data_quality_signals, "date_inconsistency");
        }
    }
    if let (Some(accident_date), Some(receive_date)) = (accident_date, receive_date) {
        if receive_date < accident_date {
            validation_errors.push(InboxValidationError {
                field_path: "reportCase.accidentDate".into(),
                severity: "warning".into(),
                remediation: "accident date should not be later than claim receive date".into(),
            });
            push_signal(data_quality_signals, "date_inconsistency");
        }
    }
    validate_invoice_dates(
        validation_errors,
        data_quality_signals,
        receive_date,
        &invoices,
    );
    validate_medical_record_receive_dates(
        validation_errors,
        data_quality_signals,
        receive_date,
        &medical_records,
    );
    for (policy_index, policy) in policies.iter().enumerate() {
        validate_service_window(
            validation_errors,
            data_quality_signals,
            service_date,
            epoch_date_at(policy, &["validateDate"]),
            epoch_date_at(policy, &["expireDate"]),
            &format!("reportCase.policyList[{policy_index}]"),
            "policy",
        );
        validate_product_liability_windows(
            validation_errors,
            data_quality_signals,
            service_date,
            policy,
            policy_index,
        );
    }
    validate_diagnosis_consistency(
        validation_errors,
        data_quality_signals,
        medical_record,
        &invoices,
    );
    validate_diagnosis_item_support(validation_errors, data_quality_signals, &invoices);
    let invoice_total_amount = total_invoice_amount(&invoices);
    if number_at(payload, &["reportCase", "claimAmount"]).is_none()
        && invoice_total_amount.is_some()
    {
        validation_errors.push(InboxValidationError {
            field_path: "reportCase.claimAmount".into(),
            severity: "warning".into(),
            remediation: "claim amount missing; derive canonical total from source invoice totals"
                .into(),
        });
        push_signal(data_quality_signals, "missing_claim_amount");
    }

    json!({
        "claim_header": {
            "external_claim_id": report_no,
            "source_system": source_system,
            "service_date": service_date.map(|date| date.to_string()),
            "receive_date": receive_date.map(|date| date.to_string()),
            "accident_date": accident_date.map(|date| date.to_string()),
            "source_timezone": SOURCE_BUSINESS_TIMEZONE,
            "service_date_raw_epoch_ms": service_date_raw_epoch_ms,
            "receive_date_raw_epoch_ms": epoch_millis_at(payload, &["reportCase", "claimReceiveDate"]),
            "accident_date_raw_epoch_ms": epoch_millis_at(payload, &["reportCase", "accidentDate"]),
            "accident_reason": string_at(payload, &["reportCase", "accidentReason"]),
            "medical_type": invoice
                .and_then(|invoice| string_at(invoice, &["medicalType"]))
                .or_else(|| medical_record.and_then(|record| string_at(record, &["medicalType"]))),
            "currency": "CNY",
            "total_amount": invoice_total_amount
        },
        "member_policy_snapshot": {
            "masked_member_id": string_at(payload, &["reportCase", "accidentPerson", "insuredNo"])
                .map(|value| mask_identifier(&value)),
            "masked_certificate_id": string_at(payload, &["reportCase", "accidentPerson", "certNo"])
                .map(|value| mask_identifier(&value)),
            "certificate_type": string_at(payload, &["reportCase", "accidentPerson", "certType"]),
            "member_gender": string_at(payload, &["reportCase", "accidentPerson", "gender"]),
            "member_birth_date": epoch_date_at(payload, &["reportCase", "accidentPerson", "birthday"])
                .map(|date| date.to_string()),
            "source_timezone": SOURCE_BUSINESS_TIMEZONE,
            "member_birth_date_raw_epoch_ms": epoch_millis_at(payload, &["reportCase", "accidentPerson", "birthday"]),
            "policy_id": policy.and_then(|policy| string_at(policy, &["policyNo"])),
            "product_code": product.and_then(|product| string_at(product, &["productCode"])),
            "liability_code": liability.and_then(|liability| string_at(liability, &["liabCode"])),
            "liability_name": liability.and_then(|liability| string_at(liability, &["liabName"])),
            "policy_type": policy.and_then(|policy| string_at(policy, &["policyType"])),
            "policy_first_apply_date": policy
                .and_then(|policy| epoch_date_at(policy, &["firstApplyTime"]))
                .map(|date| date.to_string()),
            "policy_first_apply_date_raw_epoch_ms": policy
                .and_then(|policy| epoch_millis_at(policy, &["firstApplyTime"])),
            "insured_with_social_insurance": policy
                .and_then(|policy| bool_at(policy, &["insuredWithSI"])),
            "coverage_limit": coverage_limit,
            "coverage_start_date": coverage_start_date.map(|date| date.to_string()),
            "coverage_end_date": coverage_end_date.map(|date| date.to_string()),
            "coverage_start_date_raw_epoch_ms": product
                .and_then(|product| epoch_millis_at(product, &["validateDate"]))
                .or_else(|| policy.and_then(|policy| epoch_millis_at(policy, &["validateDate"]))),
            "coverage_end_date_raw_epoch_ms": product
                .and_then(|product| epoch_millis_at(product, &["expireDate"]))
                .or_else(|| policy.and_then(|policy| epoch_millis_at(policy, &["expireDate"]))),
            "liability_start_date": liability_start_date.map(|date| date.to_string()),
            "liability_claim_start_date": liability_claim_start_date.map(|date| date.to_string()),
            "waiting_period_end_date": liability_claim_start_date.map(|date| date.to_string()),
            "liability_end_date": liability_end_date.map(|date| date.to_string()),
            "liability_start_date_raw_epoch_ms": liability
                .and_then(|liability| epoch_millis_at(liability, &["validateDate"])),
            "liability_claim_start_date_raw_epoch_ms": liability
                .and_then(|liability| epoch_millis_at(liability, &["claimValidateDate"])),
            "liability_end_date_raw_epoch_ms": liability
                .and_then(|liability| epoch_millis_at(liability, &["expireDate"])),
            "product_liabilities": policies
                .iter()
                .enumerate()
                .flat_map(|(policy_index, policy)| product_liabilities(policy, policy_index))
                .collect::<Vec<_>>()
        },
        "provider_snapshot": {
            "provider_code": invoice.and_then(|invoice| string_at(invoice, &["hospitalCode"])),
            "name": invoice
                .and_then(|invoice| string_at(invoice, &["hospitalName"]))
                .or_else(|| medical_record.and_then(|record| string_at(record, &["hospitalName"]))),
            "class": invoice.and_then(|invoice| string_at(invoice, &["hospitalClass"])),
            "type": invoice.and_then(|invoice| string_at(invoice, &["hospitalProperty"])),
            "city": invoice.and_then(|invoice| string_at(invoice, &["hospitalCityName"])),
            "province": invoice.and_then(|invoice| string_at(invoice, &["hospitalProvinceName"])),
            "network_flags": {
                "is_hospital_institution": invoice.and_then(|invoice| bool_at(invoice, &["isHospitalInstitution"])),
                "primary_care": invoice.and_then(|invoice| bool_at(invoice, &["primaryCare"])),
                "red_flag": invoice.and_then(|invoice| string_at(invoice, &["redFlag"]))
            }
        },
        "itemized_bill_lines": invoices
            .iter()
            .flat_map(itemized_bill_lines)
            .collect::<Vec<_>>(),
        "document_evidence": medical_records
            .iter()
            .enumerate()
            .map(|(record_index, record)| document_evidence(record, record_index))
            .collect::<Vec<_>>()
    })
}
