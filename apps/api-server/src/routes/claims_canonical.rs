use super::claims::{
    ClaimItemPayload, FullClaimPayload, MemberPayload, PolicyPayload, ProviderPayload,
    ScoreClaimRequest,
};
use crate::error::ApiError;
use chrono::NaiveDate;
use fwa_clinical::ClinicalDocumentEvidence;
use fwa_core::*;
use rust_decimal::Decimal;
use std::str::FromStr;

pub(super) fn duplicate_payload_fields(
    request: &ScoreClaimRequest,
    payload: &FullClaimPayload,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if payload.items.is_some() && request.items.is_some() {
        fields.push("items");
    }
    if payload.member.is_some() && request.member.is_some() {
        fields.push("member");
    }
    if payload.policy.is_some() && request.policy.is_some() {
        fields.push("policy");
    }
    if payload.provider.is_some() && request.provider.is_some() {
        fields.push("provider");
    }
    if payload.documents.is_some() && request.documents.is_some() {
        fields.push("documents");
    }
    if payload.provider_profile.is_some() && request.provider_profile.is_some() {
        fields.push("provider_profile");
    }
    if payload.provider_relationships.is_some() && request.provider_relationships.is_some() {
        fields.push("provider_relationships");
    }
    if payload.scoring_feature_context.is_some() && request.scoring_feature_context.is_some() {
        fields.push("scoring_feature_context");
    }
    fields
}

pub(super) struct CanonicalScoreInput {
    pub(super) context: ClaimContext,
    pub(super) clinical_documents: Vec<ClinicalDocumentEvidence>,
    pub(super) evidence_refs: Vec<serde_json::Value>,
    pub(super) trace: serde_json::Value,
}

pub(super) fn canonical_score_input(
    value: &serde_json::Value,
) -> Result<CanonicalScoreInput, ApiError> {
    let claim_header = object_field(value, "claim_header")?;
    let member_policy = object_field(value, "member_policy_snapshot")?;
    let provider_snapshot = object_field(value, "provider_snapshot")?;
    let mut data_quality_warnings = Vec::new();
    let bill_lines = value
        .get("itemized_bill_lines")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let documents = value
        .get("document_evidence")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let external_claim_id = required_json_string(claim_header, "external_claim_id")?;
    let claim_amount = required_json_decimal(claim_header, "total_amount")?;
    let currency = optional_json_string(claim_header, "currency").unwrap_or_else(|| "CNY".into());
    let service_date = optional_json_date(claim_header, "service_date")?;
    let diagnosis_code = optional_json_string(claim_header, "diagnosis_code")
        .or_else(|| first_bill_line_diagnosis_code(&bill_lines))
        .or_else(|| first_document_diagnosis(&documents));
    let diagnosis_code = canonical_string_or_default(
        diagnosis_code,
        &mut data_quality_warnings,
        "claim_header.diagnosis_code",
        "UNKNOWN",
    );

    let member_payload = MemberPayload {
        external_member_id: canonical_string_or_default(
            optional_json_string(member_policy, "masked_member_id")
                .or_else(|| optional_json_string(member_policy, "masked_certificate_id")),
            &mut data_quality_warnings,
            "member_policy_snapshot.masked_member_id",
            "MBR-INBOX",
        ),
        dob: optional_json_date(member_policy, "member_birth_date")?,
        gender: optional_json_string(member_policy, "member_gender"),
    };
    let coverage_start_date = required_json_date(member_policy, "coverage_start_date")?;
    let coverage_end_date = required_json_date(member_policy, "coverage_end_date")?;
    let policy_payload = PolicyPayload {
        external_policy_id: canonical_string_or_default(
            optional_json_string(member_policy, "policy_id"),
            &mut data_quality_warnings,
            "member_policy_snapshot.policy_id",
            "POL-INBOX",
        ),
        product_code: optional_json_string(member_policy, "product_code"),
        coverage_start_date,
        coverage_end_date,
        coverage_limit: required_json_decimal(member_policy, "coverage_limit")?,
        currency: Some(currency.clone()),
    };
    let provider_payload = ProviderPayload {
        external_provider_id: canonical_string_or_default(
            optional_json_string(provider_snapshot, "provider_id")
                .or_else(|| optional_json_string(provider_snapshot, "provider_code")),
            &mut data_quality_warnings,
            "provider_snapshot.provider_id",
            "PRV-INBOX",
        ),
        name: canonical_string_or_default(
            optional_json_string(provider_snapshot, "name"),
            &mut data_quality_warnings,
            "provider_snapshot.name",
            "Inbox Provider",
        ),
        provider_type: canonical_string_or_default(
            optional_json_string(provider_snapshot, "provider_type")
                .or_else(|| optional_json_string(provider_snapshot, "type")),
            &mut data_quality_warnings,
            "provider_snapshot.provider_type",
            "provider",
        ),
        region: canonical_string_or_default(
            optional_json_string(provider_snapshot, "region")
                .or_else(|| optional_json_string(provider_snapshot, "city"))
                .or_else(|| optional_json_string(provider_snapshot, "province")),
            &mut data_quality_warnings,
            "provider_snapshot.region",
            "UNKNOWN",
        ),
        risk_tier: optional_json_string(provider_snapshot, "risk_tier")
            .and_then(|value| provider_risk_tier_from_str(&value)),
    };

    let items = bill_lines
        .iter()
        .enumerate()
        .map(|(index, line)| canonical_bill_line_item(line, index, &currency))
        .collect::<Result<Vec<_>, _>>()?;
    let clinical_documents = documents
        .iter()
        .enumerate()
        .map(canonical_document_evidence)
        .collect::<Vec<_>>();
    let mut evidence_refs = canonical_evidence_refs(&bill_lines);
    evidence_refs.extend(canonical_document_refs(&documents));
    evidence_refs.sort_by_key(|value| value.to_string());
    evidence_refs.dedup();
    let trace = canonical_claim_context_trace(&bill_lines, &documents, data_quality_warnings);

    Ok(CanonicalScoreInput {
        context: demo_context(FullClaimPayload {
            external_claim_id,
            claim_amount,
            currency,
            claim_amount_peer_percentile: None,
            service_date,
            diagnosis_code: Some(diagnosis_code),
            items: Some(items),
            member: Some(member_payload),
            policy: Some(policy_payload),
            provider: Some(provider_payload),
            documents: None,
            provider_profile: None,
            provider_relationships: None,
            scoring_feature_context: None,
        }),
        clinical_documents,
        evidence_refs,
        trace,
    })
}

fn object_field<'a>(
    value: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, ApiError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| invalid_canonical_field(field))
}

fn required_json_string(
    value: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, ApiError> {
    optional_json_string(value, field).ok_or_else(|| invalid_canonical_field(field))
}

fn optional_json_string(
    value: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn canonical_string_or_default(
    value: Option<String>,
    data_quality_warnings: &mut Vec<serde_json::Value>,
    field_path: &'static str,
    default_value: &'static str,
) -> String {
    match value {
        Some(value) => value,
        None => {
            data_quality_warnings.push(serde_json::json!({
                "field_path": field_path,
                "severity": "warning",
                "message": "canonical_claim_context defaulted missing field",
                "default_value": default_value
            }));
            default_value.to_string()
        }
    }
}

fn required_json_decimal(
    value: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Decimal, ApiError> {
    let decimal = value
        .get(field)
        .and_then(decimal_from_json)
        .ok_or_else(|| invalid_canonical_field(field))?;
    if decimal > Decimal::ZERO {
        Ok(decimal)
    } else {
        Err(invalid_canonical_field(field))
    }
}

fn optional_json_decimal(
    value: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Option<Decimal> {
    value.get(field).and_then(decimal_from_json)
}

fn decimal_from_json(value: &serde_json::Value) -> Option<Decimal> {
    match value {
        serde_json::Value::String(value) => Decimal::from_str(value).ok(),
        serde_json::Value::Number(value) => Decimal::from_str(&value.to_string()).ok(),
        _ => None,
    }
}

fn required_json_date(
    value: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<NaiveDate, ApiError> {
    optional_json_date(value, field)?.ok_or_else(|| invalid_canonical_field(field))
}

fn optional_json_date(
    value: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<Option<NaiveDate>, ApiError> {
    optional_json_string(value, field)
        .map(|value| {
            NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                .map_err(|_| invalid_canonical_field(field))
        })
        .transpose()
}

fn canonical_bill_line_item(
    line: &serde_json::Value,
    index: usize,
    currency: &str,
) -> Result<ClaimItemPayload, ApiError> {
    let object = line
        .as_object()
        .ok_or_else(|| invalid_canonical_field("itemized_bill_lines"))?;
    let total_amount = optional_json_decimal(object, "amount").unwrap_or(Decimal::ZERO);
    Ok(ClaimItemPayload {
        item_code: optional_json_string(object, "item_code")
            .or_else(|| optional_json_string(object, "source_path"))
            .unwrap_or_else(|| format!("inbox-line-{index}")),
        item_type: optional_json_string(object, "fee_category")
            .or_else(|| optional_json_string(object, "medical_category"))
            .unwrap_or_else(|| "claim_item".into()),
        description: optional_json_string(object, "item_name")
            .or_else(|| optional_json_string(object, "fee_category"))
            .unwrap_or_else(|| "Inbox claim item".into()),
        quantity: 1,
        unit_amount: total_amount,
        total_amount,
        currency: Some(currency.to_string()),
    })
}

fn canonical_document_evidence(
    (index, document): (usize, &serde_json::Value),
) -> ClinicalDocumentEvidence {
    let object = document.as_object();
    ClinicalDocumentEvidence {
        document_id: object
            .and_then(|object| optional_json_string(object, "document_id"))
            .unwrap_or_else(|| format!("inbox-document-{index}")),
        document_type: object
            .and_then(|object| {
                optional_json_string(object, "document_type")
                    .or_else(|| optional_json_string(object, "medical_record_type"))
            })
            .unwrap_or_else(|| "medical_record".into()),
        linked_item_codes: object
            .and_then(|object| object.get("linked_item_codes"))
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn first_bill_line_diagnosis_code(bill_lines: &[serde_json::Value]) -> Option<String> {
    bill_lines.iter().find_map(|line| {
        line.get("diagnosis_list")
            .and_then(serde_json::Value::as_array)
            .and_then(|diagnoses| diagnoses.first())
            .and_then(|diagnosis| {
                diagnosis
                    .get("code")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| diagnosis.get("name").and_then(serde_json::Value::as_str))
            })
            .map(ToOwned::to_owned)
    })
}

fn first_document_diagnosis(documents: &[serde_json::Value]) -> Option<String> {
    documents
        .iter()
        .find_map(|document| {
            document
                .get("diagnosis")
                .and_then(serde_json::Value::as_str)
        })
        .map(ToOwned::to_owned)
}

fn canonical_evidence_refs(bill_lines: &[serde_json::Value]) -> Vec<serde_json::Value> {
    bill_lines
        .iter()
        .flat_map(|line| {
            let source_path = line
                .get("source_path")
                .and_then(serde_json::Value::as_str)
                .map(|value| serde_json::Value::String(value.to_string()));
            let evidence_refs = line
                .get("evidence_refs")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|value| {
                    value
                        .as_str()
                        .map(|value| serde_json::Value::String(value.to_string()))
                });
            source_path
                .into_iter()
                .chain(evidence_refs)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn canonical_document_refs(documents: &[serde_json::Value]) -> Vec<serde_json::Value> {
    documents
        .iter()
        .flat_map(|document| {
            document
                .get("source_refs")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|value| {
                    value
                        .as_str()
                        .map(|value| serde_json::Value::String(value.to_string()))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn canonical_claim_context_trace(
    bill_lines: &[serde_json::Value],
    documents: &[serde_json::Value],
    data_quality_warnings: Vec<serde_json::Value>,
) -> serde_json::Value {
    let mut evidence_refs = bill_lines
        .iter()
        .flat_map(|line| {
            line.get("evidence_refs")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    evidence_refs.sort();
    evidence_refs.dedup();

    let mut source_refs = bill_lines
        .iter()
        .filter_map(|line| {
            line.get("source_path")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .chain(documents.iter().flat_map(|document| {
            document
                .get("source_refs")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        }))
        .collect::<Vec<_>>();
    source_refs.sort();
    source_refs.dedup();

    serde_json::json!({
        "input_mode": "canonical_claim_context",
        "evidence_refs": evidence_refs,
        "source_refs": source_refs,
        "data_quality_warnings": data_quality_warnings
    })
}

fn provider_risk_tier_from_str(value: &str) -> Option<ProviderRiskTier> {
    match value {
        "Low" | "low" => Some(ProviderRiskTier::Low),
        "Medium" | "medium" => Some(ProviderRiskTier::Medium),
        "High" | "high" => Some(ProviderRiskTier::High),
        _ => None,
    }
}

fn invalid_canonical_field(field: &'static str) -> ApiError {
    ApiError::new(
        axum::http::StatusCode::BAD_REQUEST,
        "INVALID_SCORE_REQUEST",
        format!("canonical_claim_context.{field} is invalid"),
    )
}

pub(super) fn demo_context(payload: FullClaimPayload) -> ClaimContext {
    let claim_currency = payload.currency.clone();
    let member_payload = payload.member.clone().unwrap_or(MemberPayload {
        external_member_id: "MBR-DEMO".into(),
        dob: None,
        gender: None,
    });
    let policy_payload = payload.policy.clone().unwrap_or(PolicyPayload {
        external_policy_id: "POL-DEMO".into(),
        product_code: Some("MED".into()),
        coverage_start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        coverage_end_date: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        coverage_limit: Decimal::new(10000, 0),
        currency: Some(payload.currency.clone()),
    });
    let provider_payload = payload.provider.clone().unwrap_or(ProviderPayload {
        external_provider_id: "PRV-DEMO".into(),
        name: "Demo Hospital".into(),
        provider_type: "hospital".into(),
        region: "SH".into(),
        risk_tier: Some(ProviderRiskTier::Medium),
    });
    let member_id = MemberId::from_external(member_payload.external_member_id.clone());
    let policy_id = PolicyId::from_external(policy_payload.external_policy_id.clone());
    let provider_id = ProviderId::from_external(provider_payload.external_provider_id.clone());
    let service_date = payload
        .service_date
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 1, 6).unwrap());
    let items = payload
        .items
        .unwrap_or_default()
        .into_iter()
        .map(|item| {
            let currency = item.currency.unwrap_or_else(|| payload.currency.clone());
            ClaimItem {
                item_code: item.item_code,
                item_type: item.item_type,
                description: item.description,
                quantity: item.quantity,
                unit_amount: Money::new(item.unit_amount, currency.clone()),
                total_amount: Money::new(item.total_amount, currency),
            }
        })
        .collect();

    ClaimContext {
        claim: Claim {
            id: ClaimId::from_external(payload.external_claim_id.clone()),
            external_claim_id: payload.external_claim_id,
            member_id: member_id.clone(),
            policy_id: policy_id.clone(),
            provider_id: provider_id.clone(),
            diagnosis_code: payload.diagnosis_code.unwrap_or_else(|| "J10".into()),
            service_date,
            amount: Money::new(payload.claim_amount, payload.currency),
        },
        items,
        member: Member {
            id: member_id.clone(),
            external_member_id: member_payload.external_member_id,
            dob: member_payload.dob,
            gender: member_payload.gender,
        },
        policy: Policy {
            id: policy_id,
            external_policy_id: policy_payload.external_policy_id,
            member_id,
            product_code: policy_payload.product_code.unwrap_or_else(|| "MED".into()),
            coverage_start_date: policy_payload.coverage_start_date,
            coverage_end_date: policy_payload.coverage_end_date,
            coverage_limit: Money::new(
                policy_payload.coverage_limit,
                policy_payload
                    .currency
                    .unwrap_or_else(|| claim_currency.clone()),
            ),
        },
        provider: Provider {
            id: provider_id,
            external_provider_id: provider_payload.external_provider_id,
            name: provider_payload.name,
            provider_type: provider_payload.provider_type,
            region: provider_payload.region,
            risk_tier: provider_payload
                .risk_tier
                .unwrap_or(ProviderRiskTier::Medium),
        },
    }
}
