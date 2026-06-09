use crate::routes::pii::redact_text;
use serde_json::{json, Value};

use super::{
    inbox_utils::{
        bool_at, epoch_date_at, epoch_millis_at, extract_diagnosis, extract_next_line_after_label,
        normalize_medical_text, normalized_redacted_text_at, number_at, string_at,
    },
    SourceInvoice, SOURCE_BUSINESS_TIMEZONE,
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
