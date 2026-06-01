use crate::{
    app::AppState, error::ApiError, repository::PersistedAuditEvent, routes::pii::redact_text,
};
use axum::{extract::State, http::HeaderMap, http::StatusCode, Json};
use chrono::{DateTime, FixedOffset, NaiveDate};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::AuditEventId;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const MAPPING_VERSION: &str = "aiclaim-core-v1";
const SOURCE_BUSINESS_UTC_OFFSET_SECONDS: i32 = 8 * 60 * 60;

#[derive(Debug, Serialize)]
pub struct InboxNormalizeResponse {
    pub run_id: String,
    pub audit_id: String,
    pub external_message_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub mapping_version: &'static str,
    pub validation_result: String,
    pub scoring_ready: bool,
    pub raw_payload_ref: Option<String>,
    pub validation_errors: Vec<InboxValidationError>,
    pub canonical_claim_context: Value,
    pub data_quality_signals: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InboxValidationError {
    pub field_path: String,
    pub severity: &'static str,
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
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<InboxNormalizeResponse>), ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    let actor = validate_api_key(
        api_key,
        &ApiKeyConfig {
            key: state.config.api_key.clone(),
            source_system: state.config.source_system.clone(),
        },
    )
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })?;

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
                severity: "error",
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
            severity: "warning",
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
                "external_message_fingerprint": external_message_fingerprint,
                "idempotency_key": idempotency_key,
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

    Ok((
        status,
        Json(InboxNormalizeResponse {
            run_id,
            audit_id,
            raw_payload_ref,
            idempotency_key,
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

fn internal_error(
    code: &'static str,
) -> impl FnOnce(anyhow::Error) -> ApiError + Send + Sync + 'static {
    move |error| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            code,
            format!("{code}: {error}"),
        )
    }
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
            severity: "error",
            remediation: "include at least one policy for coverage mapping".into(),
        });
    }
    if invoice.is_none() {
        validation_errors.push(InboxValidationError {
            field_path: "reportCase.policyList[0].invoiceList".into(),
            severity: "error",
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
                severity: "warning",
                remediation: "claim receive date should not be earlier than service date".into(),
            });
            push_signal(data_quality_signals, "date_inconsistency");
        }
    }
    if let (Some(accident_date), Some(receive_date)) = (accident_date, receive_date) {
        if receive_date < accident_date {
            validation_errors.push(InboxValidationError {
                field_path: "reportCase.accidentDate".into(),
                severity: "warning",
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
            severity: "warning",
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
            "policy_id": policy.and_then(|policy| string_at(policy, &["policyNo"])),
            "product_code": product.and_then(|product| string_at(product, &["productCode"])),
            "liability_code": liability.and_then(|liability| string_at(liability, &["liabCode"])),
            "liability_name": liability.and_then(|liability| string_at(liability, &["liabName"])),
            "policy_type": policy.and_then(|policy| string_at(policy, &["policyType"])),
            "policy_first_apply_date": policy
                .and_then(|policy| epoch_date_at(policy, &["firstApplyTime"]))
                .map(|date| date.to_string()),
            "insured_with_social_insurance": policy
                .and_then(|policy| bool_at(policy, &["insuredWithSI"])),
            "coverage_limit": coverage_limit,
            "coverage_start_date": coverage_start_date.map(|date| date.to_string()),
            "coverage_end_date": coverage_end_date.map(|date| date.to_string()),
            "liability_start_date": liability_start_date.map(|date| date.to_string()),
            "liability_claim_start_date": liability_claim_start_date.map(|date| date.to_string()),
            "waiting_period_end_date": liability_claim_start_date.map(|date| date.to_string()),
            "liability_end_date": liability_end_date.map(|date| date.to_string()),
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

fn product_liabilities(policy: &Value, policy_index: usize) -> Vec<Value> {
    let policy_id = string_at(policy, &["policyNo"]);
    policy
        .get("productList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(|(product_index, product)| {
            let source_path =
                format!("reportCase.policyList[{policy_index}].productList[{product_index}]");
            let product_id = string_at(product, &["id"]);
            let product_code = string_at(product, &["productCode"]);
            let product_name = string_at(product, &["productName"]);
            let plan_code = string_at(product, &["planCode"]);
            let plan_version = string_at(product, &["planVersion"]);
            let product_start_date = epoch_date_at(product, &["validateDate"]);
            let product_end_date = epoch_date_at(product, &["expireDate"]);
            let policy_id = policy_id.clone();
            let liabilities = product
                .get("claimLiabilityList")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if liabilities.is_empty() {
                return vec![json!({
                    "policy_id": policy_id,
                    "source_path": source_path,
                    "liability_source_path": null,
                    "product_id": product_id,
                    "product_code": product_code,
                    "product_name": product_name,
                    "plan_code": plan_code,
                    "plan_version": plan_version,
                    "product_start_date": product_start_date.map(|date| date.to_string()),
                    "product_end_date": product_end_date.map(|date| date.to_string()),
                    "liability_id": null,
                    "liability_code": null,
                    "liability_name": null,
                    "liability_start_date": null,
                    "liability_claim_start_date": null,
                    "waiting_period_end_date": null,
                    "liability_end_date": null,
                    "is_serious_disease_liability": null,
                    "main_liability": null,
                    "evidence_refs": [
                        format!(
                            "product:{}",
                            product_code.as_deref().unwrap_or("unknown")
                        )
                    ]
                })];
            }
            liabilities
                .iter()
                .enumerate()
                .map(|(liability_index, liability)| {
                    let liability_source_path =
                        format!("{source_path}.claimLiabilityList[{liability_index}]");
                    let liability_id = string_at(liability, &["id"]);
                    let liability_start_date = epoch_date_at(liability, &["validateDate"]);
                    let liability_claim_start_date =
                        epoch_date_at(liability, &["claimValidateDate"]);
                    let liability_end_date = epoch_date_at(liability, &["expireDate"]);
                    json!({
                        "policy_id": policy_id,
                        "source_path": source_path,
                        "liability_source_path": liability_source_path,
                        "product_id": product_id,
                        "product_code": product_code,
                        "product_name": product_name,
                        "plan_code": plan_code,
                        "plan_version": plan_version,
                        "product_start_date": product_start_date.map(|date| date.to_string()),
                        "product_end_date": product_end_date.map(|date| date.to_string()),
                        "liability_id": liability_id,
                        "liability_code": string_at(liability, &["liabCode"]),
                        "liability_name": string_at(liability, &["liabName"]),
                        "liability_start_date": liability_start_date.map(|date| date.to_string()),
                        "liability_claim_start_date": liability_claim_start_date.map(|date| date.to_string()),
                        "waiting_period_end_date": liability_claim_start_date.map(|date| date.to_string()),
                        "liability_end_date": liability_end_date.map(|date| date.to_string()),
                        "is_serious_disease_liability": bool_at(liability, &["isSeriousDiseaseLiability"]),
                        "main_liability": bool_at(liability, &["mainLiab"]),
                        "evidence_refs": [
                            format!(
                                "product:{}:liability:{}",
                                product_code.as_deref().unwrap_or("unknown"),
                                string_at(liability, &["liabCode"]).unwrap_or_else(|| "unknown".into())
                            )
                        ]
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn validate_policy_coverage_limits(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    policies: &[&Value],
) {
    for (policy_index, policy) in policies.iter().enumerate() {
        if number_at(policy, &["coverageLimit"]).is_some() {
            continue;
        }

        validation_errors.push(InboxValidationError {
            field_path: format!("reportCase.policyList[{policy_index}].coverageLimit"),
            severity: "warning",
            remediation: "map policy or liability coverage limit before direct scoring".into(),
        });
        push_signal(data_quality_signals, "missing_coverage_limit");
    }
}

fn validate_product_liability_windows(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    service_date: Option<NaiveDate>,
    policy: &Value,
    policy_index: usize,
) {
    for (product_index, product) in policy
        .get("productList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let product_prefix =
            format!("reportCase.policyList[{policy_index}].productList[{product_index}]");
        validate_service_window(
            validation_errors,
            data_quality_signals,
            service_date,
            epoch_date_at(product, &["validateDate"]),
            epoch_date_at(product, &["expireDate"]),
            &product_prefix,
            "product",
        );
        for (liability_index, liability) in product
            .get("claimLiabilityList")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let liability_prefix =
                format!("{product_prefix}.claimLiabilityList[{liability_index}]");
            validate_service_window(
                validation_errors,
                data_quality_signals,
                service_date,
                epoch_date_at(liability, &["validateDate"]),
                epoch_date_at(liability, &["expireDate"]),
                &liability_prefix,
                "liability",
            );
            validate_liability_claim_eligibility(
                validation_errors,
                data_quality_signals,
                service_date,
                epoch_date_at(liability, &["claimValidateDate"]),
                &liability_prefix,
            );
        }
    }
}

fn validate_diagnosis_consistency(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    medical_record: Option<&Value>,
    invoices: &[SourceInvoice<'_>],
) {
    let medical_diagnosis = medical_record.and_then(|record| string_at(record, &["diagnosisName"]));
    if let Some(medical_diagnosis) = medical_diagnosis {
        for invoice in invoices {
            let invoice_diagnoses = invoice_diagnosis_names(invoice.value);
            if invoice_diagnoses.is_empty()
                || invoice_diagnoses
                    .iter()
                    .any(|invoice_diagnosis| diagnoses_match(&medical_diagnosis, invoice_diagnosis))
            {
                continue;
            }

            validation_errors.push(InboxValidationError {
                field_path: invoice.field_path("diagnosisList"),
                severity: "warning",
                remediation: "invoice diagnosis should align with medical record diagnosis".into(),
            });
            push_signal(data_quality_signals, "document_invoice_mismatch");
        }
    }
}

fn validate_diagnosis_item_support(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    invoices: &[SourceInvoice<'_>],
) {
    for invoice in invoices {
        if !invoice_diagnosis_names(invoice.value).is_empty()
            || !invoice_has_bill_lines(invoice.value)
        {
            continue;
        }

        validation_errors.push(InboxValidationError {
            field_path: invoice.field_path("feeList"),
            severity: "warning",
            remediation:
                "bill lines should include diagnosis context before medical reasonableness scoring"
                    .into(),
        });
        push_signal(data_quality_signals, "diagnosis_item_mismatch");
    }
}

fn total_invoice_amount(invoices: &[SourceInvoice<'_>]) -> Option<f64> {
    let amounts = invoices
        .iter()
        .filter_map(|invoice| number_at(invoice.value, &["feeAmount"]));
    let (count, total) = amounts.fold((0, 0.0), |(count, total), amount| {
        (count + 1, total + amount)
    });
    (count > 0).then_some(total)
}

fn validate_invoice_dates(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    receive_date: Option<NaiveDate>,
    invoices: &[SourceInvoice<'_>],
) {
    for invoice in invoices {
        let start_date = epoch_date_at(invoice.value, &["startDate"]);
        let end_date = epoch_date_at(invoice.value, &["endDate"]);

        if let (Some(start_date), Some(end_date)) = (start_date, end_date) {
            if end_date < start_date {
                validation_errors.push(InboxValidationError {
                    field_path: invoice.field_path("endDate"),
                    severity: "warning",
                    remediation: "invoice end date must not be earlier than invoice start date"
                        .into(),
                });
                push_signal(data_quality_signals, "date_inconsistency");
            }
        }

        if let Some(receive_date) = receive_date {
            if start_date.is_some_and(|start_date| receive_date < start_date) {
                validation_errors.push(InboxValidationError {
                    field_path: invoice.field_path("startDate"),
                    severity: "warning",
                    remediation: "claim receive date should not be earlier than invoice start date"
                        .into(),
                });
                push_signal(data_quality_signals, "date_inconsistency");
            }
        }
    }
}

fn validate_medical_record_receive_dates(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    receive_date: Option<NaiveDate>,
    medical_records: &[&Value],
) {
    let Some(receive_date) = receive_date else {
        return;
    };

    for (record_index, record) in medical_records.iter().enumerate() {
        for (field_name, field_label) in [
            ("visitDate", "medical record visit date"),
            ("firstHappenDate", "medical record first happen date"),
            ("operationStartDate", "medical record operation start date"),
        ] {
            if epoch_date_at(record, &[field_name])
                .is_some_and(|field_date| receive_date < field_date)
            {
                validation_errors.push(InboxValidationError {
                    field_path: format!(
                        "reportCase.medicalRecordInfoList[{record_index}].{field_name}"
                    ),
                    severity: "warning",
                    remediation: format!(
                        "claim receive date should not be earlier than {field_label}"
                    ),
                });
                push_signal(data_quality_signals, "date_inconsistency");
            }
        }
    }
}

fn invoice_diagnosis_names(invoice: &Value) -> Vec<String> {
    invoice
        .get("diagnosisList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|diagnosis| {
            string_at(diagnosis, &["detailName"]).or_else(|| string_at(diagnosis, &["name"]))
        })
        .collect()
}

fn invoice_has_bill_lines(invoice: &Value) -> bool {
    invoice
        .get("feeList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|fee| {
            fee.get("feeDetailList")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|detail| string_at(detail, &["name"]).is_some())
        })
}

fn diagnoses_match(left: &str, right: &str) -> bool {
    let left = normalize_match_text(left);
    let right = normalize_match_text(right);
    !left.is_empty() && !right.is_empty() && (left.contains(&right) || right.contains(&left))
}

fn normalize_match_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && !character.is_ascii_punctuation())
        .flat_map(char::to_lowercase)
        .collect()
}

fn validate_service_window(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    service_date: Option<NaiveDate>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    field_prefix: &str,
    window_label: &str,
) {
    if let (Some(start_date), Some(end_date)) = (start_date, end_date) {
        if end_date < start_date {
            validation_errors.push(InboxValidationError {
                field_path: format!("{field_prefix}.expireDate"),
                severity: "warning",
                remediation: format!("{window_label} end date must not be earlier than start date"),
            });
            push_signal(data_quality_signals, "date_inconsistency");
            return;
        }
    }

    let Some(service_date) = service_date else {
        return;
    };
    if start_date.is_some_and(|start_date| service_date < start_date) {
        validation_errors.push(InboxValidationError {
            field_path: format!("{field_prefix}.validateDate"),
            severity: "warning",
            remediation: format!("service date must fall within the {window_label} window"),
        });
        push_signal(data_quality_signals, "coverage_window_mismatch");
    } else if end_date.is_some_and(|end_date| service_date > end_date) {
        validation_errors.push(InboxValidationError {
            field_path: format!("{field_prefix}.expireDate"),
            severity: "warning",
            remediation: format!("service date must fall within the {window_label} window"),
        });
        push_signal(data_quality_signals, "coverage_window_mismatch");
    }
}

fn validate_liability_claim_eligibility(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    service_date: Option<NaiveDate>,
    liability_claim_start_date: Option<NaiveDate>,
    field_prefix: &str,
) {
    if let (Some(service_date), Some(liability_claim_start_date)) =
        (service_date, liability_claim_start_date)
    {
        if service_date < liability_claim_start_date {
            validation_errors.push(InboxValidationError {
                field_path: format!("{field_prefix}.claimValidateDate"),
                severity: "warning",
                remediation:
                    "service date must not be earlier than liability claim eligibility date".into(),
            });
            push_signal(data_quality_signals, "policy_liability_mismatch");
        }
    }
}

fn itemized_bill_lines(invoice: &SourceInvoice<'_>) -> Vec<Value> {
    let invoice_value = invoice.value;
    let invoice_path = format!(
        "reportCase.policyList[{}].invoiceList[{}]",
        invoice.policy_index, invoice.invoice_index
    );
    let invoice_id = string_at(invoice_value, &["invoiceNo"]);
    let invoice_bill_type = string_at(invoice_value, &["billType"]);
    let invoice_document_type = string_at(invoice_value, &["documentType"]);
    let social_insurance_type = string_at(invoice_value, &["socialInsuranceType"]);
    let department = string_at(invoice_value, &["departmentName"]);
    let medical_type = string_at(invoice_value, &["medicalType"]);
    let invoice_claim_nature = string_at(invoice_value, &["claimNature"]);
    let invoice_start_date = epoch_date_at(invoice_value, &["startDate"]);
    let invoice_end_date = epoch_date_at(invoice_value, &["endDate"]);
    let invoice_social_insurance_amount = number_at(invoice_value, &["medicareAmount"]);
    let invoice_self_pay_amount = number_at(invoice_value, &["selfPayAmount"]);
    let invoice_own_expense_amount = number_at(invoice_value, &["ownExpenseAmount"]);
    let invoice_other_amount = number_at(invoice_value, &["otherAmount"]);
    let invoice_provider_code = string_at(invoice_value, &["hospitalCode"]);
    let invoice_provider_name = string_at(invoice_value, &["hospitalName"]);
    let invoice_provider_class = string_at(invoice_value, &["hospitalClass"]);
    let invoice_provider_type = string_at(invoice_value, &["hospitalProperty"]);
    let invoice_provider_city = string_at(invoice_value, &["hospitalCityName"]);
    let invoice_provider_province = string_at(invoice_value, &["hospitalProvinceName"]);
    let invoice_is_hospital_institution = bool_at(invoice_value, &["isHospitalInstitution"]);
    let invoice_primary_care = bool_at(invoice_value, &["primaryCare"]);
    let invoice_red_flag = string_at(invoice_value, &["redFlag"]);
    let diagnoses = invoice_value
        .get("diagnosisList")
        .and_then(Value::as_array)
        .map(|diagnoses| {
            diagnoses
                .iter()
                .map(|diagnosis| {
                    json!({
                        "code": string_at(diagnosis, &["detailCode"])
                            .or_else(|| string_at(diagnosis, &["icd"])),
                        "name": string_at(diagnosis, &["detailName"])
                            .or_else(|| string_at(diagnosis, &["name"]))
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    invoice_value
        .get("feeList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(|(fee_index, fee)| {
            let fee_path = format!("{invoice_path}.feeList[{fee_index}]");
            let fee_category = string_at(fee, &["feeCategory"]);
            let social_insurance_amount = number_at(fee, &["medicareAmount"]);
            let fee_group_amount = number_at(fee, &["feeAmount"]);
            let fee_group_other_amount = number_at(fee, &["otherAmount"]);
            let invoice_id = invoice_id.clone();
            let invoice_bill_type = invoice_bill_type.clone();
            let invoice_document_type = invoice_document_type.clone();
            let social_insurance_type = social_insurance_type.clone();
            let department = department.clone();
            let medical_type = medical_type.clone();
            let invoice_claim_nature = invoice_claim_nature.clone();
            let diagnoses = diagnoses.clone();
            let invoice_provider_code = invoice_provider_code.clone();
            let invoice_provider_name = invoice_provider_name.clone();
            let invoice_provider_class = invoice_provider_class.clone();
            let invoice_provider_type = invoice_provider_type.clone();
            let invoice_provider_city = invoice_provider_city.clone();
            let invoice_provider_province = invoice_provider_province.clone();
            let invoice_red_flag = invoice_red_flag.clone();
            fee.get("feeDetailList")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
                .map(move |(detail_index, detail)| {
                    let source_path = format!("{fee_path}.feeDetailList[{detail_index}]");
                    json!({
                        "invoice_id": invoice_id,
                        "source_path": source_path,
                        "diagnosis_list": diagnoses,
                        "fee_category": fee_category,
                        "item_name": string_at(detail, &["name"]),
                        "amount": number_at(detail, &["amount"]),
                        "self_pay": number_at(detail, &["selfPayAmount"]),
                        "own_expense": number_at(detail, &["ownExpenseAmount"]),
                        "social_insurance_amount": social_insurance_amount,
                        "medical_category": string_at(detail, &["medicalCategory"]),
                        "invoice_bill_type": invoice_bill_type,
                        "invoice_document_type": invoice_document_type,
                        "social_insurance_type": social_insurance_type,
                        "department": department,
                        "medical_type": medical_type,
                        "invoice_claim_nature": invoice_claim_nature,
                        "invoice_start_date": invoice_start_date.map(|date| date.to_string()),
                        "invoice_end_date": invoice_end_date.map(|date| date.to_string()),
                        "invoice_social_insurance_amount": invoice_social_insurance_amount,
                        "invoice_self_pay_amount": invoice_self_pay_amount,
                        "invoice_own_expense_amount": invoice_own_expense_amount,
                        "invoice_other_amount": invoice_other_amount,
                        "invoice_provider_code": invoice_provider_code,
                        "invoice_provider_name": invoice_provider_name,
                        "invoice_provider_class": invoice_provider_class,
                        "invoice_provider_type": invoice_provider_type,
                        "invoice_provider_city": invoice_provider_city,
                        "invoice_provider_province": invoice_provider_province,
                        "invoice_is_hospital_institution": invoice_is_hospital_institution,
                        "invoice_primary_care": invoice_primary_care,
                        "invoice_red_flag": invoice_red_flag,
                        "fee_group_amount": fee_group_amount,
                        "fee_group_other_amount": fee_group_other_amount,
                        "medicare_prorated": string_at(detail, &["medicareProrated"]),
                        "evidence_refs": [
                            format!(
                                "invoice:{}:fee_detail:{}",
                                invoice_id.as_deref().unwrap_or("unknown"),
                                string_at(detail, &["id"]).unwrap_or_else(|| "unknown".into())
                            )
                        ]
                    })
                })
        })
        .collect()
}

fn document_evidence(record: &Value, record_index: usize) -> Value {
    let normalized_text = string_at(record, &["medicalRecordInformation"])
        .map(|value| normalize_medical_text(&value));
    let text = normalized_text.as_deref().map(redact_text);
    json!({
        "document_id": string_at(record, &["id"]),
        "source_path": format!("reportCase.medicalRecordInfoList[{record_index}]"),
        "department": string_at(record, &["departmentName"]),
        "diagnosis": string_at(record, &["diagnosisName"]),
        "claim_nature": string_at(record, &["claimNature"]),
        "medical_record_type": string_at(record, &["medicalRecordType"]),
        "chief_complaint": normalized_redacted_text_at(record, &["chiefComplaint"]),
        "current_medical_history": normalized_redacted_text_at(record, &["currentMedicalHistory"]),
        "past_history": normalized_redacted_text_at(record, &["pastHistory"]),
        "extracted_diagnosis": normalized_text
            .as_deref()
            .and_then(extract_diagnosis)
            .map(|value| redact_text(&value)),
        "extracted_procedure": normalized_text
            .as_deref()
            .and_then(|text| extract_next_line_after_label(text, "处理措施"))
            .map(|value| redact_text(&value)),
        "extracted_prescription": normalized_text
            .as_deref()
            .and_then(|text| extract_next_line_after_label(text, "西药："))
            .map(|value| redact_text(&value)),
        "medical_type": string_at(record, &["medicalType"]),
        "visit_date": epoch_date_at(record, &["visitDate"]).map(|date| date.to_string()),
        "first_happen_date": epoch_date_at(record, &["firstHappenDate"]).map(|date| date.to_string()),
        "operation_start_date": epoch_date_at(record, &["operationStartDate"]).map(|date| date.to_string()),
        "medical_record_text": text,
        "source_refs": [
            format!(
                "medical_record:{}",
                string_at(record, &["id"]).unwrap_or_else(|| "unknown".into())
            )
        ]
    })
}

fn normalized_redacted_text_at(value: &Value, path: &[&str]) -> Option<String> {
    string_at(value, path)
        .map(|value| normalize_medical_text(&value))
        .map(|value| redact_text(&value))
}

fn required_string(
    payload: &Value,
    path: &[&str],
    field_path: &str,
    label: &str,
    validation_errors: &mut Vec<InboxValidationError>,
) -> Option<String> {
    let value = string_at(payload, path);
    if value
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        value
    } else {
        validation_errors.push(InboxValidationError {
            field_path: field_path.into(),
            severity: "error",
            remediation: format!("include {label}"),
        });
        None
    }
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(|value| match value {
            Value::String(value) => Some(value.trim().to_string()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.trim().is_empty())
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_f64)
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(|value| match value {
            Value::Bool(value) => Some(*value),
            Value::String(value) if value.eq_ignore_ascii_case("Y") => Some(true),
            Value::String(value) if value.eq_ignore_ascii_case("N") => Some(false),
            Value::String(value) if value.eq_ignore_ascii_case("true") => Some(true),
            Value::String(value) if value.eq_ignore_ascii_case("false") => Some(false),
            _ => None,
        })
}

fn epoch_date_at(value: &Value, path: &[&str]) -> Option<NaiveDate> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_i64)
        .and_then(|millis| {
            let source_timezone = FixedOffset::east_opt(SOURCE_BUSINESS_UTC_OFFSET_SECONDS)?;
            DateTime::from_timestamp_millis(millis)
                .map(|date| date.with_timezone(&source_timezone).date_naive())
        })
}

fn first_array_item<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
}

fn array_items<'a>(value: &'a Value, path: &[&str]) -> Vec<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn names_mismatch<'a>(names: impl IntoIterator<Item = Option<&'a str>>) -> bool {
    let names = names
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    names
        .first()
        .is_some_and(|first| names.iter().any(|name| name != first))
}

fn normalize_medical_text(value: &str) -> String {
    value
        .replace("/n", "\n")
        .chars()
        .filter_map(normalized_medical_text_character)
        .collect::<String>()
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalized_medical_text_character(character: char) -> Option<char> {
    match character {
        '\u{feff}' | '\u{fffd}' => None,
        '\u{00a0}' | '\u{3000}' => Some(' '),
        _ => Some(character),
    }
}

fn extract_diagnosis(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        strip_label_value(line, "诊断：").or_else(|| strip_label_value(line, "诊断:"))
    })
}

fn extract_next_line_after_label(text: &str, label: &str) -> Option<String> {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line == label {
            return lines
                .find(|candidate| !candidate.trim().is_empty())
                .map(str::trim)
                .map(str::to_string);
        }
    }
    None
}

fn strip_label_value(line: &str, label: &str) -> Option<String> {
    line.trim()
        .strip_prefix(label)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn mask_identifier(value: &str) -> String {
    let value = value.trim();
    let suffix = value
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("***{suffix}")
}

fn push_signal(signals: &mut Vec<String>, signal: &str) {
    if !signals.iter().any(|existing| existing == signal) {
        signals.push(signal.into());
    }
}
