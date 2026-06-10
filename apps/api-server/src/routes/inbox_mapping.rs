use crate::routes::pii::redact_text;
use serde_json::{json, Value};

use super::{
    inbox_utils::{
        array_items, bool_at, epoch_date_at, epoch_millis_at, extract_diagnosis,
        extract_next_line_after_label, first_array_item, mask_identifier, names_mismatch,
        normalize_medical_text, normalized_redacted_text_at, number_at, push_signal, string_at,
    },
    inbox_validation::{
        total_invoice_amount, validate_diagnosis_consistency, validate_diagnosis_item_support,
        validate_invoice_dates, validate_medical_record_receive_dates,
        validate_policy_coverage_limits, validate_product_liability_windows,
        validate_service_window,
    },
    InboxValidationError, SourceInvoice, SOURCE_BUSINESS_TIMEZONE,
};

pub(super) fn product_liabilities(policy: &Value, policy_index: usize) -> Vec<Value> {
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
                    "source_timezone": SOURCE_BUSINESS_TIMEZONE,
                    "product_start_date_raw_epoch_ms": epoch_millis_at(product, &["validateDate"]),
                    "product_end_date_raw_epoch_ms": epoch_millis_at(product, &["expireDate"]),
                    "liability_id": null,
                    "liability_code": null,
                    "liability_name": null,
                    "liability_start_date": null,
                    "liability_claim_start_date": null,
                    "waiting_period_end_date": null,
                    "liability_end_date": null,
                    "liability_start_date_raw_epoch_ms": null,
                    "liability_claim_start_date_raw_epoch_ms": null,
                    "liability_end_date_raw_epoch_ms": null,
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
                        "source_timezone": SOURCE_BUSINESS_TIMEZONE,
                        "product_start_date_raw_epoch_ms": epoch_millis_at(product, &["validateDate"]),
                        "product_end_date_raw_epoch_ms": epoch_millis_at(product, &["expireDate"]),
                        "liability_id": liability_id,
                        "liability_code": string_at(liability, &["liabCode"]),
                        "liability_name": string_at(liability, &["liabName"]),
                        "liability_start_date": liability_start_date.map(|date| date.to_string()),
                        "liability_claim_start_date": liability_claim_start_date.map(|date| date.to_string()),
                        "waiting_period_end_date": liability_claim_start_date.map(|date| date.to_string()),
                        "liability_end_date": liability_end_date.map(|date| date.to_string()),
                        "liability_start_date_raw_epoch_ms": epoch_millis_at(liability, &["validateDate"]),
                        "liability_claim_start_date_raw_epoch_ms": epoch_millis_at(liability, &["claimValidateDate"]),
                        "liability_end_date_raw_epoch_ms": epoch_millis_at(liability, &["expireDate"]),
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

pub(super) fn itemized_bill_lines(invoice: &SourceInvoice<'_>) -> Vec<Value> {
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
                        "source_timezone": SOURCE_BUSINESS_TIMEZONE,
                        "invoice_start_date_raw_epoch_ms": epoch_millis_at(invoice_value, &["startDate"]),
                        "invoice_end_date_raw_epoch_ms": epoch_millis_at(invoice_value, &["endDate"]),
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

pub(super) fn document_evidence(record: &Value, record_index: usize) -> Value {
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
        "source_timezone": SOURCE_BUSINESS_TIMEZONE,
        "visit_date_raw_epoch_ms": epoch_millis_at(record, &["visitDate"]),
        "first_happen_date_raw_epoch_ms": epoch_millis_at(record, &["firstHappenDate"]),
        "operation_start_date_raw_epoch_ms": epoch_millis_at(record, &["operationStartDate"]),
        "medical_record_text": text,
        "source_refs": [
            format!(
                "medical_record:{}",
                string_at(record, &["id"]).unwrap_or_else(|| "unknown".into())
            )
        ]
    })
}

pub(super) fn stable_id_fragment(value: &str) -> String {
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

pub(super) fn canonical_source_paths(canonical_claim_context: &Value) -> Vec<String> {
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

pub(super) fn build_canonical_claim_context(
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
