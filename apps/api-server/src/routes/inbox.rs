use crate::{
    app::AppState, error::ApiError, repository::PersistedAuditEvent, routes::pii::redact_text,
};
use axum::{extract::State, http::HeaderMap, http::StatusCode, Json};
use chrono::{DateTime, NaiveDate};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::AuditEventId;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const MAPPING_VERSION: &str = "aiclaim-core-v1";

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
    matches!(
        path,
        "reportCase.policyList[0].coverageLimit"
            | "reportCase.policyList[0].validateDate"
            | "reportCase.policyList[0].expireDate"
    ) || (path.starts_with("reportCase.policyList[0].productList[")
        && matches!(
            path.rsplit('.').next(),
            Some("validateDate" | "claimValidateDate" | "expireDate")
        ))
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
    let policy = first_array_item(payload, &["reportCase", "policyList"]);
    let invoices = policy
        .map(|policy| array_items(policy, &["invoiceList"]))
        .unwrap_or_default();
    let invoice = policy.and_then(|policy| first_array_item(policy, &["invoiceList"]));
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

    if policy
        .and_then(|policy| number_at(policy, &["coverageLimit"]))
        .is_none()
    {
        validation_errors.push(InboxValidationError {
            field_path: "reportCase.policyList[0].coverageLimit".into(),
            severity: "warning",
            remediation: "map policy or liability coverage limit before direct scoring".into(),
        });
        push_signal(data_quality_signals, "missing_coverage_limit");
    }

    let insured_name = string_at(payload, &["reportCase", "accidentPerson", "insuredName"]);
    let policy_insured_name = policy.and_then(|policy| string_at(policy, &["insuredName"]));
    let invoice_person_name =
        invoice.and_then(|invoice| string_at(invoice, &["accidentPersonName"]));
    let medical_record_patient_name =
        medical_record.and_then(|record| string_at(record, &["patientName"]));
    if names_mismatch([
        insured_name.as_deref(),
        policy_insured_name.as_deref(),
        invoice_person_name.as_deref(),
        medical_record_patient_name.as_deref(),
    ]) {
        push_signal(data_quality_signals, "identity_mismatch");
    }

    let service_date = invoice
        .and_then(|invoice| epoch_date_at(invoice, &["startDate"]))
        .or_else(|| epoch_date_at(payload, &["reportCase", "accidentDate"]));
    let receive_date = epoch_date_at(payload, &["reportCase", "claimReceiveDate"]);
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
    validate_service_window(
        validation_errors,
        data_quality_signals,
        service_date,
        policy_start_date,
        policy_end_date,
        "reportCase.policyList[0]",
        "policy",
    );
    if let Some(policy) = policy {
        validate_product_liability_windows(
            validation_errors,
            data_quality_signals,
            service_date,
            policy,
        );
    }
    validate_diagnosis_consistency(
        validation_errors,
        data_quality_signals,
        medical_record,
        &invoices,
    );
    validate_diagnosis_item_support(validation_errors, data_quality_signals, &invoices);

    json!({
        "claim_header": {
            "external_claim_id": report_no,
            "source_system": source_system,
            "service_date": service_date.map(|date| date.to_string()),
            "receive_date": receive_date.map(|date| date.to_string()),
            "accident_reason": string_at(payload, &["reportCase", "accidentReason"]),
            "medical_type": invoice
                .and_then(|invoice| string_at(invoice, &["medicalType"]))
                .or_else(|| medical_record.and_then(|record| string_at(record, &["medicalType"]))),
            "currency": "CNY",
            "total_amount": invoice.and_then(|invoice| number_at(invoice, &["feeAmount"]))
        },
        "member_policy_snapshot": {
            "masked_member_id": string_at(payload, &["reportCase", "accidentPerson", "insuredNo"])
                .map(|value| mask_identifier(&value)),
            "policy_id": policy.and_then(|policy| string_at(policy, &["policyNo"])),
            "product_code": product.and_then(|product| string_at(product, &["productCode"])),
            "liability_code": liability.and_then(|liability| string_at(liability, &["liabCode"])),
            "liability_name": liability.and_then(|liability| string_at(liability, &["liabName"])),
            "policy_type": policy.and_then(|policy| string_at(policy, &["policyType"])),
            "coverage_limit": coverage_limit,
            "coverage_start_date": coverage_start_date.map(|date| date.to_string()),
            "coverage_end_date": coverage_end_date.map(|date| date.to_string()),
            "liability_start_date": liability_start_date.map(|date| date.to_string()),
            "liability_claim_start_date": liability_claim_start_date.map(|date| date.to_string()),
            "waiting_period_end_date": liability_claim_start_date.map(|date| date.to_string()),
            "liability_end_date": liability_end_date.map(|date| date.to_string()),
            "product_liabilities": policy
                .map(product_liabilities)
                .unwrap_or_default()
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
            .flat_map(|invoice| itemized_bill_lines(invoice))
            .collect::<Vec<_>>(),
        "document_evidence": medical_records
            .iter()
            .map(|record| document_evidence(record))
            .collect::<Vec<_>>()
    })
}

fn product_liabilities(policy: &Value) -> Vec<Value> {
    policy
        .get("productList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|product| {
            let product_id = string_at(product, &["id"]);
            let product_code = string_at(product, &["productCode"]);
            let product_name = string_at(product, &["productName"]);
            let plan_code = string_at(product, &["planCode"]);
            let plan_version = string_at(product, &["planVersion"]);
            let product_start_date = epoch_date_at(product, &["validateDate"]);
            let product_end_date = epoch_date_at(product, &["expireDate"]);
            product
                .get("claimLiabilityList")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(move |liability| {
                    let liability_id = string_at(liability, &["id"]);
                    let liability_start_date = epoch_date_at(liability, &["validateDate"]);
                    let liability_claim_start_date =
                        epoch_date_at(liability, &["claimValidateDate"]);
                    let liability_end_date = epoch_date_at(liability, &["expireDate"]);
                    json!({
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
                        "evidence_refs": [
                            format!(
                                "product:{}:liability:{}",
                                product_code.as_deref().unwrap_or("unknown"),
                                string_at(liability, &["liabCode"]).unwrap_or_else(|| "unknown".into())
                            )
                        ]
                    })
                })
        })
        .collect()
}

fn validate_product_liability_windows(
    validation_errors: &mut Vec<InboxValidationError>,
    data_quality_signals: &mut Vec<String>,
    service_date: Option<NaiveDate>,
    policy: &Value,
) {
    for (product_index, product) in policy
        .get("productList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let product_prefix = format!("reportCase.policyList[0].productList[{product_index}]");
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
    invoices: &[&Value],
) {
    let medical_diagnosis = medical_record.and_then(|record| string_at(record, &["diagnosisName"]));
    if let Some(medical_diagnosis) = medical_diagnosis {
        for (invoice_index, invoice) in invoices.iter().enumerate() {
            let invoice_diagnoses = invoice_diagnosis_names(invoice);
            if invoice_diagnoses.is_empty()
                || invoice_diagnoses
                    .iter()
                    .any(|invoice_diagnosis| diagnoses_match(&medical_diagnosis, invoice_diagnosis))
            {
                continue;
            }

            validation_errors.push(InboxValidationError {
                field_path: format!(
                    "reportCase.policyList[0].invoiceList[{invoice_index}].diagnosisList"
                ),
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
    invoices: &[&Value],
) {
    for (invoice_index, invoice) in invoices.iter().enumerate() {
        if !invoice_diagnosis_names(invoice).is_empty() || !invoice_has_bill_lines(invoice) {
            continue;
        }

        validation_errors.push(InboxValidationError {
            field_path: format!("reportCase.policyList[0].invoiceList[{invoice_index}].feeList"),
            severity: "warning",
            remediation:
                "bill lines should include diagnosis context before medical reasonableness scoring"
                    .into(),
        });
        push_signal(data_quality_signals, "diagnosis_item_mismatch");
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

fn itemized_bill_lines(invoice: &Value) -> Vec<Value> {
    let invoice_id = string_at(invoice, &["invoiceNo"]);
    let diagnoses = invoice
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

    invoice
        .get("feeList")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|fee| {
            let fee_category = string_at(fee, &["feeCategory"]);
            let social_insurance_amount = number_at(fee, &["medicareAmount"]);
            let invoice_id = invoice_id.clone();
            let diagnoses = diagnoses.clone();
            fee.get("feeDetailList")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(move |detail| {
                    json!({
                        "invoice_id": invoice_id,
                        "diagnosis_list": diagnoses,
                        "fee_category": fee_category,
                        "item_name": string_at(detail, &["name"]),
                        "amount": number_at(detail, &["amount"]),
                        "self_pay": number_at(detail, &["selfPayAmount"]),
                        "own_expense": number_at(detail, &["ownExpenseAmount"]),
                        "social_insurance_amount": social_insurance_amount,
                        "medical_category": string_at(detail, &["medicalCategory"]),
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

fn document_evidence(record: &Value) -> Value {
    let normalized_text = string_at(record, &["medicalRecordInformation"])
        .map(|value| normalize_medical_text(&value));
    let text = normalized_text.as_deref().map(redact_text);
    json!({
        "document_id": string_at(record, &["id"]),
        "department": string_at(record, &["departmentName"]),
        "diagnosis": string_at(record, &["diagnosisName"]),
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
        "medical_record_text": text,
        "source_refs": [
            format!(
                "medical_record:{}",
                string_at(record, &["id"]).unwrap_or_else(|| "unknown".into())
            )
        ]
    })
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
        .and_then(Value::as_bool)
}

fn epoch_date_at(value: &Value, path: &[&str]) -> Option<NaiveDate> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_i64)
        .and_then(|millis| DateTime::from_timestamp_millis(millis).map(|date| date.date_naive()))
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

fn names_mismatch<const N: usize>(names: [Option<&str>; N]) -> bool {
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
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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
