use crate::display_value;
use serde_json::Value;
use yew::prelude::*;

pub(crate) fn comma_separated_values(input: &UseStateHandle<String>) -> Vec<String> {
    (**input)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn parse_json_array(input: &str, label: &str) -> Result<Vec<Value>, String> {
    match serde_json::from_str::<Value>(input.trim()) {
        Ok(Value::Array(items)) if !items.is_empty() => Ok(items),
        Ok(Value::Array(_)) => Err(format!("{label} must include at least one sample")),
        Ok(_) => Err(format!("{label} must be a JSON array")),
        Err(error) => Err(format!("{label} JSON is invalid: {error}")),
    }
}

pub(crate) fn response_rule_id(response: &Value) -> Option<String> {
    response
        .get("saved_draft_rule_id")
        .and_then(Value::as_str)
        .or_else(|| {
            response
                .get("summary")
                .and_then(|summary| summary.get("rule_id"))
                .and_then(Value::as_str)
        })
        .map(str::to_string)
}

pub(crate) fn push_unique(mut values: Vec<String>, value: String) -> Vec<String> {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
    values
}

pub(crate) fn remove_id(values: Vec<String>, value: &str) -> Vec<String> {
    values
        .into_iter()
        .filter(|existing| existing != value)
        .collect()
}

pub(crate) fn refs_label(refs: &[String]) -> String {
    if refs.is_empty() {
        "none".into()
    } else {
        refs.join(", ")
    }
}

pub(crate) fn refs_count_label(refs: &[String]) -> String {
    if refs.is_empty() {
        "none".into()
    } else {
        format!("{} refs", refs.len())
    }
}

pub(crate) fn parse_tags(tags_text: &str) -> Vec<String> {
    tags_text
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn payload_keys_label(value: &Value) -> String {
    value
        .as_object()
        .map(|object| {
            if object.is_empty() {
                "empty object".into()
            } else {
                object.keys().cloned().collect::<Vec<_>>().join(", ")
            }
        })
        .unwrap_or_else(|| display_value(value))
}

pub(crate) fn payload_signal_count_label(value: &Value, noun: &str) -> String {
    value
        .as_object()
        .map(|object| {
            if object.is_empty() {
                "empty object".into()
            } else {
                format!("{} {}", object.len(), noun)
            }
        })
        .unwrap_or_else(|| display_value(value))
}

pub(crate) fn compact_payload_label(value: &Value) -> String {
    value
        .as_object()
        .map(|object| {
            if object.is_empty() {
                "empty object".into()
            } else {
                format!("{} fields", object.len())
            }
        })
        .unwrap_or_else(|| "payload recorded".into())
}

pub(crate) fn empty_label(value: &str) -> &str {
    if value.trim().is_empty() {
        "none"
    } else {
        value
    }
}
