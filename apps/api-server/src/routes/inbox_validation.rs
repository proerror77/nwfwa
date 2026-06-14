use chrono::NaiveDate;
use serde_json::Value;

use super::{
    inbox_utils::{epoch_date_at, number_at, push_signal, string_at},
    InboxValidationError, SourceInvoice,
};

pub(super) fn validate_policy_coverage_limits(
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
            severity: "warning".into(),
            remediation: "map policy or liability coverage limit before direct scoring".into(),
        });
        push_signal(data_quality_signals, "missing_coverage_limit");
    }
}

pub(super) fn validate_product_liability_windows(
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

pub(super) fn validate_diagnosis_consistency(
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
                severity: "warning".into(),
                remediation: "invoice diagnosis should align with medical record diagnosis".into(),
            });
            push_signal(data_quality_signals, "document_invoice_mismatch");
        }
    }
}

pub(super) fn validate_diagnosis_item_support(
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
            severity: "warning".into(),
            remediation:
                "bill lines should include diagnosis context before medical reasonableness scoring"
                    .into(),
        });
        push_signal(data_quality_signals, "diagnosis_item_mismatch");
    }
}

pub(super) fn total_invoice_amount(invoices: &[SourceInvoice<'_>]) -> Option<f64> {
    let amounts = invoices
        .iter()
        .filter_map(|invoice| number_at(invoice.value, &["feeAmount"]));
    let (count, total) = amounts.fold((0, 0.0), |(count, total), amount| {
        (count + 1, total + amount)
    });
    (count > 0).then_some(total)
}

pub(super) fn validate_invoice_dates(
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
                    severity: "warning".into(),
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
                    severity: "warning".into(),
                    remediation: "claim receive date should not be earlier than invoice start date"
                        .into(),
                });
                push_signal(data_quality_signals, "date_inconsistency");
            }
        }
    }
}

pub(super) fn validate_medical_record_receive_dates(
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
                    severity: "warning".into(),
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

pub(super) fn validate_service_window(
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
                severity: "warning".into(),
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
            severity: "warning".into(),
            remediation: format!("service date must fall within the {window_label} window"),
        });
        push_signal(data_quality_signals, "coverage_window_mismatch");
    } else if end_date.is_some_and(|end_date| service_date > end_date) {
        validation_errors.push(InboxValidationError {
            field_path: format!("{field_prefix}.expireDate"),
            severity: "warning".into(),
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
                severity: "warning".into(),
                remediation:
                    "service date must not be earlier than liability claim eligibility date".into(),
            });
            push_signal(data_quality_signals, "policy_liability_mismatch");
        }
    }
}
