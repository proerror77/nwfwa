use crate::repository::{DatasetRecord, SchemaFieldRecord};
use fwa_core::canonical_scheme_family;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use super::ops_datasets_types::{
    DatasetHealthRecord, FactorCardRecord, FactorReadinessResponse, FactorSchemeReadinessRecord,
};

pub(super) fn build_factor_readiness(datasets: &[DatasetRecord]) -> FactorReadinessResponse {
    let mut response = FactorReadinessResponse {
        dataset_count: datasets.len() as u32,
        factor_count: 0,
        label_count: 0,
        entity_key_count: 0,
        data_quality_score: 0.0,
        data_quality_status: "empty".into(),
        online_ready_count: 0,
        rule_convertible_count: 0,
        mapped_factor_count: 0,
        high_missing_count: 0,
        unstable_factor_count: 0,
        unowned_factor_count: 0,
        ready_factor_count: 0,
        review_factor_count: 0,
        readiness_issue_counts: Map::new(),
        scheme_readiness: Vec::new(),
        factor_cards: Vec::new(),
    };
    let mut scheme_readiness = BTreeMap::<String, FactorSchemeReadinessRecord>::new();

    for dataset in datasets {
        for field in &dataset.fields {
            response.factor_count += 1;
            let is_label = field.semantic_role == "label";
            let is_entity_key = dataset.entity_keys.contains(&field.field_name);
            let missing_rate = numeric_profile_value(&field.profile_json, "missing_rate");
            if is_label {
                response.label_count += 1;
            }
            if is_entity_key {
                response.entity_key_count += 1;
            }
            if !is_label && !field.nullable && missing_rate.unwrap_or(0.0) <= 0.05 {
                response.online_ready_count += 1;
            }
            if !is_label && is_rule_convertible_type(&field.logical_type) {
                response.rule_convertible_count += 1;
            }
            if missing_rate.unwrap_or(0.0) > 0.20 {
                response.high_missing_count += 1;
            }
            if numeric_profile_value(&field.profile_json, "psi").unwrap_or(0.0) >= 0.25 {
                response.unstable_factor_count += 1;
            }
            if field
                .profile_json
                .get("owner")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                response.unowned_factor_count += 1;
            }
            if dataset
                .mappings
                .iter()
                .any(|mapping| mapping.feature_name.as_deref() == Some(field.field_name.as_str()))
            {
                response.mapped_factor_count += 1;
            }
            let factor_card = build_factor_card(dataset, field);
            if factor_card.readiness_status == "ready" {
                response.ready_factor_count += 1;
            } else {
                response.review_factor_count += 1;
            }
            update_scheme_readiness(&mut scheme_readiness, &factor_card);
            for issue in &factor_card.readiness_issues {
                let count = response
                    .readiness_issue_counts
                    .get(issue)
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    + 1;
                response
                    .readiness_issue_counts
                    .insert(issue.clone(), Value::from(count));
            }
            response.factor_cards.push(factor_card);
        }
    }

    response.scheme_readiness = scheme_readiness.into_values().collect();
    response.data_quality_score = factor_data_quality_score(&response);
    response.data_quality_status = factor_data_quality_status(response.data_quality_score).into();

    response
}

fn update_scheme_readiness(
    scheme_readiness: &mut BTreeMap<String, FactorSchemeReadinessRecord>,
    factor_card: &FactorCardRecord,
) {
    let summary = scheme_readiness
        .entry(factor_card.scheme_family.clone())
        .or_insert_with(|| FactorSchemeReadinessRecord {
            scheme_family: factor_card.scheme_family.clone(),
            factor_count: 0,
            ready_factor_count: 0,
            review_factor_count: 0,
            online_ready_count: 0,
            rule_convertible_count: 0,
            readiness_issue_counts: Map::new(),
        });

    summary.factor_count += 1;
    if factor_card.readiness_status == "ready" {
        summary.ready_factor_count += 1;
    } else {
        summary.review_factor_count += 1;
    }
    if factor_card.online_available {
        summary.online_ready_count += 1;
    }
    if factor_card.rule_convertible {
        summary.rule_convertible_count += 1;
    }
    for issue in &factor_card.readiness_issues {
        let count = summary
            .readiness_issue_counts
            .get(issue)
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        summary
            .readiness_issue_counts
            .insert(issue.clone(), Value::from(count));
    }
}

fn build_factor_card(dataset: &DatasetRecord, field: &SchemaFieldRecord) -> FactorCardRecord {
    let is_label = field.semantic_role == "label";
    let is_entity_key = dataset.entity_keys.contains(&field.field_name);
    let missing_rate = numeric_profile_value(&field.profile_json, "missing_rate");
    let psi = numeric_profile_value(&field.profile_json, "psi");
    let source_fields = string_array_profile_value(&field.profile_json, "source_fields")
        .unwrap_or_else(|| vec![field.field_name.clone()]);
    let owner = string_profile_value(&field.profile_json, "owner").unwrap_or_default();
    let online_available = !is_label
        && bool_profile_value(&field.profile_json, "online_available").unwrap_or(!field.nullable);
    let readiness_issues =
        factor_readiness_issues(is_label, online_available, missing_rate, psi, &owner);
    let readiness_status = if readiness_issues.is_empty() {
        "ready"
    } else {
        "needs_review"
    };
    let mut evidence_refs = vec![format!(
        "dataset_fields:{}:{}:{}",
        dataset.dataset_key, dataset.dataset_version, field.field_name
    )];
    if let Some(profile_refs) = string_array_profile_value(&field.profile_json, "evidence_refs") {
        evidence_refs.extend(profile_refs);
        evidence_refs.sort();
        evidence_refs.dedup();
    }

    FactorCardRecord {
        dataset_id: dataset.dataset_id.clone(),
        dataset_key: dataset.dataset_key.clone(),
        dataset_version: dataset.dataset_version.clone(),
        factor_name: field.field_name.clone(),
        scheme_family: factor_scheme_family(field),
        chinese_name: string_profile_value(&field.profile_json, "chinese_name")
            .or_else(|| string_profile_value(&field.profile_json, "display_label"))
            .unwrap_or_else(|| titleize(&field.field_name)),
        entity_type: string_profile_value(&field.profile_json, "entity_type")
            .unwrap_or_else(|| dataset.sample_grain.clone()),
        semantic_role: field.semantic_role.clone(),
        logical_type: field.logical_type.clone(),
        calculation_window: string_profile_value(&field.profile_json, "calculation_window")
            .unwrap_or_else(|| dataset.sample_grain.clone()),
        calculation_logic: string_profile_value(&field.profile_json, "calculation_logic")
            .unwrap_or_else(|| "registered_dataset_field".into()),
        source_table: string_profile_value(&field.profile_json, "source_table")
            .unwrap_or_else(|| dataset.dataset_key.clone()),
        source_fields,
        business_meaning: string_profile_value(&field.profile_json, "business_meaning")
            .unwrap_or_else(|| field.description.clone()),
        risk_direction: string_profile_value(&field.profile_json, "risk_direction").unwrap_or_else(
            || {
                if is_label {
                    "label".into()
                } else {
                    "unknown".into()
                }
            },
        ),
        missing_rate,
        iv: numeric_profile_value(&field.profile_json, "iv"),
        auc_gain: numeric_profile_value(&field.profile_json, "auc_gain"),
        lift: numeric_profile_value(&field.profile_json, "lift"),
        psi,
        stability: stability_label(psi).into(),
        model_contribution: numeric_profile_value(&field.profile_json, "model_contribution"),
        rule_convertible: !is_label
            && bool_profile_value(&field.profile_json, "convertible_to_rule")
                .unwrap_or_else(|| is_rule_convertible_type(&field.logical_type)),
        online_available,
        readiness_status: readiness_status.into(),
        readiness_issues,
        version: format_factor_version(field.profile_json.get("version")),
        owner,
        is_label,
        is_entity_key,
        evidence_refs,
    }
}

pub(crate) fn build_dataset_health(datasets: &[DatasetRecord]) -> Vec<DatasetHealthRecord> {
    datasets.iter().map(build_dataset_health_record).collect()
}

pub(crate) fn build_dataset_health_record(dataset: &DatasetRecord) -> DatasetHealthRecord {
    let mut record = DatasetHealthRecord {
        dataset_id: dataset.dataset_id.clone(),
        dataset_key: dataset.dataset_key.clone(),
        dataset_version: dataset.dataset_version.clone(),
        data_quality_score: 0.0,
        data_quality_status: "empty".into(),
        field_count: dataset.fields.len() as u32,
        label_count: 0,
        entity_key_count: 0,
        high_missing_count: 0,
        unstable_field_count: 0,
        unowned_field_count: 0,
        online_ready_count: 0,
        issue_count: 0,
    };

    for field in &dataset.fields {
        let is_label = field.semantic_role == "label";
        let is_entity_key = dataset.entity_keys.contains(&field.field_name);
        let missing_rate = numeric_profile_value(&field.profile_json, "missing_rate");
        if is_label {
            record.label_count += 1;
        }
        if is_entity_key {
            record.entity_key_count += 1;
        }
        if !is_label && !field.nullable && missing_rate.unwrap_or(0.0) <= 0.05 {
            record.online_ready_count += 1;
        }
        if missing_rate.unwrap_or(0.0) > 0.20 {
            record.high_missing_count += 1;
        }
        if numeric_profile_value(&field.profile_json, "psi").unwrap_or(0.0) >= 0.25 {
            record.unstable_field_count += 1;
        }
        if field
            .profile_json
            .get("owner")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            record.unowned_field_count += 1;
        }
    }

    record.issue_count =
        record.high_missing_count + record.unstable_field_count + record.unowned_field_count;
    if record.field_count > 0 {
        record.data_quality_score =
            dataset_data_quality_score(record.field_count, record.issue_count);
        record.data_quality_status = factor_data_quality_status(record.data_quality_score).into();
    }

    record
}

fn dataset_data_quality_score(field_count: u32, issue_count: u32) -> f64 {
    let max_issue_count = field_count * 3;
    let score = 1.0 - (issue_count as f64 / max_issue_count as f64);
    score.clamp(0.0, 1.0)
}

fn factor_data_quality_score(response: &FactorReadinessResponse) -> f64 {
    if response.factor_count == 0 {
        return 0.0;
    }
    let issue_count = response.high_missing_count
        + response.unstable_factor_count
        + response.unowned_factor_count;
    dataset_data_quality_score(response.factor_count, issue_count)
}

fn factor_data_quality_status(score: f64) -> &'static str {
    if score >= 0.85 {
        "ready"
    } else if score >= 0.65 {
        "watch"
    } else {
        "blocked"
    }
}

fn numeric_profile_value(profile: &Value, key: &str) -> Option<f64> {
    profile.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_i64().map(|value| value as f64))
    })
}

fn string_profile_value(profile: &Value, key: &str) -> Option<String> {
    profile
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn string_array_profile_value(profile: &Value, key: &str) -> Option<Vec<String>> {
    let values = profile.get(key)?.as_array()?;
    let values = values
        .iter()
        .filter_map(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!values.is_empty()).then_some(values)
}

fn bool_profile_value(profile: &Value, key: &str) -> Option<bool> {
    profile.get(key).and_then(Value::as_bool)
}

fn format_factor_version(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(version)) if !version.is_empty() => version.clone(),
        Some(Value::Number(version)) => version
            .as_u64()
            .map(|version| format!("v{version}"))
            .unwrap_or_else(|| "v1".into()),
        _ => "v1".into(),
    }
}

fn stability_label(psi: Option<f64>) -> &'static str {
    match psi {
        None => "unmeasured",
        Some(value) if value < 0.10 => "stable",
        Some(value) if value < 0.25 => "watch",
        Some(_) => "drift",
    }
}

fn factor_readiness_issues(
    is_label: bool,
    online_available: bool,
    missing_rate: Option<f64>,
    psi: Option<f64>,
    owner: &str,
) -> Vec<String> {
    let mut issues = Vec::new();
    if is_label {
        issues.push("label_field".into());
    }
    if !online_available {
        issues.push("not_online_available".into());
    }
    if missing_rate.unwrap_or(0.0) > 0.05 {
        issues.push("online_missing_rate_above_threshold".into());
    }
    if missing_rate.unwrap_or(0.0) > 0.20 {
        issues.push("high_missing_rate".into());
    }
    if psi.unwrap_or(0.0) >= 0.25 {
        issues.push("unstable_distribution".into());
    }
    if owner.trim().is_empty() {
        issues.push("missing_owner".into());
    }
    issues
}

fn factor_scheme_family(field: &SchemaFieldRecord) -> String {
    if let Some(scheme_family) = string_profile_value(&field.profile_json, "scheme_family")
        .and_then(|value| canonical_scheme_family(&value))
    {
        return scheme_family;
    }

    let factor_name = field.field_name.as_str();
    let description = field.description.to_ascii_lowercase();
    let text = format!("{}_{}", factor_name, description);
    let inferred = if text.contains("duplicate") {
        "duplicate_billing"
    } else if text.contains("diagnosis_procedure") || text.contains("diagnosis procedure") {
        "diagnosis_procedure_mismatch"
    } else if text.contains("clinical_review")
        || text.contains("medical_reasonableness")
        || text.contains("medical necessity")
    {
        "medically_unnecessary_service"
    } else if text.contains("provider") {
        "provider_peer_outlier"
    } else if text.contains("service_count")
        || text.contains("utilization")
        || text.contains("item_count")
    {
        "excessive_utilization"
    } else if text.contains("days_since_policy_start")
        || text.contains("amount_to_limit")
        || text.contains("early")
    {
        "early_high_value_claim"
    } else {
        "high_risk_claim"
    };

    inferred.into()
}

fn titleize(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_rule_convertible_type(logical_type: &str) -> bool {
    matches!(
        logical_type,
        "decimal" | "float" | "float64" | "int" | "int8" | "int32" | "int64" | "boolean"
    )
}
