use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub(crate) fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
}

pub(crate) fn display_value(value: &Value) -> String {
    value
        .as_f64()
        .map(|number| format!("{number:.1}"))
        .or_else(|| value.as_str().map(str::to_string))
        .unwrap_or_else(|| value.to_string())
}

pub(crate) fn numeric_value(value: &Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
        .unwrap_or(0.0)
}

pub(crate) fn readable_token(value: &str) -> String {
    value.replace(['_', '-'], " ")
}

pub(crate) fn titleize_token(value: &str) -> String {
    let readable = readable_token(value.trim());
    if readable.is_empty() {
        return "None".into();
    }
    readable
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| {
                    format!(
                        "{}{}",
                        first.to_ascii_uppercase(),
                        characters.as_str().to_ascii_lowercase()
                    )
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn business_label(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => "None".into(),
        "red" => "High risk".into(),
        "amber" | "yellow" => "Watchlist risk".into(),
        "green" => "Low risk".into(),
        "manual_review" | "review" => "Manual review".into(),
        "request_evidence" | "request_more_evidence" => "Request evidence".into(),
        "open_case" => "Open case".into(),
        "reject_lead" => "Reject lead".into(),
        "merge_lead" => "Merge lead".into(),
        "pre_payment" => "Pre-payment review".into(),
        "post_payment" => "Post-payment review".into(),
        "pending_evidence" => "Waiting evidence".into(),
        "evidence_pending" => "Evidence pending".into(),
        "evidence_sufficient" | "clinical_evidence_sufficient" => "Evidence sufficient".into(),
        "insufficient_evidence" => "Insufficient evidence".into(),
        "documentation_issue" => "Documentation issue".into(),
        "medical_necessity_review_required" => "Medical review required".into(),
        "medical_necessity_issue" => "Medical necessity issue".into(),
        "no_medical_issue" => "No medical issue".into(),
        "no_auto_denial" => "No automatic denial".into(),
        "assistive_only" => "Assistive only".into(),
        "approved" => "Approved".into(),
        "approved_for_training" => "Approved for training".into(),
        "blocked" => "Blocked".into(),
        "breached" => "Over SLA".into(),
        "closed" => "Closed".into(),
        "completed" => "Completed".into(),
        "confirmed" => "Confirmed".into(),
        "created" => "Created".into(),
        "done" => "Done".into(),
        "error" => "Error".into(),
        "failed" => "Failed".into(),
        "hold" | "held" => "Held for review".into(),
        "investigating" => "Investigating".into(),
        "new" => "New".into(),
        "ok" | "passed" | "valid" => "Passed".into(),
        "on_track" => "On track".into(),
        "open" => "Open".into(),
        "pending" => "Pending".into(),
        "queued" => "Queued".into(),
        "ready" | "scoring_ready" => "Ready".into(),
        "received" => "Received".into(),
        "rejected" => "Rejected".into(),
        "triage" => "Triage".into(),
        other if other.starts_with("completed") => "Completed".into(),
        _ => titleize_token(value),
    }
}

pub(crate) fn localized_business_text(value: &str, language: crate::state::Language) -> String {
    if language == crate::state::Language::Zh {
        return value.to_string();
    }

    value
        .replace(
            "关键风险，建议人工审核、医务复核并升级调查",
            "Critical risk: route for manual review, medical review, and investigation escalation",
        )
        .replace(
            "医务复核并升级调查",
            "medical review and investigation escalation",
        )
        .replace(
            "高风险，建议人工审核",
            "High risk: manual review recommended",
        )
        .replace("建议人工审核", "manual review recommended")
        .replace("医务复核", "medical review")
        .replace("升级调查", "escalate investigation")
        .replace("关键风险", "critical risk")
        .replace("高风险", "high risk")
        .replace('，', ", ")
}

pub(crate) fn rag_label(value: &str) -> &'static str {
    match value.trim().to_ascii_uppercase().as_str() {
        "RED" => "High risk",
        "AMBER" | "YELLOW" => "Watchlist risk",
        "GREEN" => "Low risk",
        _ => "Risk pending",
    }
}

pub(crate) fn optional_number(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "none".into())
}

pub(crate) fn issue_counts_label(counts: &Map<String, Value>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(key, value)| format!("{key}={}", display_value(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn map_counts_label(counts: &BTreeMap<String, u32>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn map_counts_business_label(counts: &BTreeMap<String, u32>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(key, value)| format!("{}={value}", business_label(key)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn percent_label(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

pub(crate) fn optional_u32(value: Option<u32>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

pub(crate) fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|error| format!("{label} must be an unsigned integer: {error}"))
}

pub(crate) fn parse_risk_score(value: &str) -> Result<u8, String> {
    let score = value
        .trim()
        .parse::<u8>()
        .map_err(|error| format!("risk score must be an integer from 0 to 100: {error}"))?;
    if score > 100 {
        return Err("risk score must be between 0 and 100".into());
    }
    Ok(score)
}

pub(crate) fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

pub(crate) fn optional_metric(value: &Option<Value>) -> String {
    value
        .as_ref()
        .map(display_value)
        .unwrap_or_else(|| "none".into())
}

pub(crate) fn optional_u8(value: Option<u8>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "none".into())
}

pub(crate) fn value_refs_label(refs: &[Value]) -> String {
    if refs.is_empty() {
        return "none".into();
    }
    refs.iter()
        .map(display_value)
        .collect::<Vec<_>>()
        .join(", ")
}
