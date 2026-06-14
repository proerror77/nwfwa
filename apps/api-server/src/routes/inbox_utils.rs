use crate::routes::pii::redact_text;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde_json::Value;

use super::{InboxValidationError, SOURCE_BUSINESS_UTC_OFFSET_SECONDS};

pub(super) fn normalized_redacted_text_at(value: &Value, path: &[&str]) -> Option<String> {
    string_at(value, path)
        .map(|value| normalize_medical_text(&value))
        .map(|value| redact_text(&value))
}

pub(super) fn required_string(
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
            severity: "error".into(),
            remediation: format!("include {label}"),
        });
        None
    }
}

pub(super) fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(|value| match value {
            Value::String(value) => Some(value.trim().to_string()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.trim().is_empty())
}

pub(super) fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_f64)
}

pub(super) fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
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

pub(super) fn epoch_date_at(value: &Value, path: &[&str]) -> Option<NaiveDate> {
    epoch_millis_at(value, path).and_then(|millis| {
        let source_timezone = FixedOffset::east_opt(SOURCE_BUSINESS_UTC_OFFSET_SECONDS)?;
        DateTime::from_timestamp_millis(millis)
            .map(|date| date.with_timezone(&source_timezone).date_naive())
    })
}

pub(super) fn epoch_millis_at(value: &Value, path: &[&str]) -> Option<i64> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_i64)
}

pub(super) fn first_array_item<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
}

pub(super) fn array_items<'a>(value: &'a Value, path: &[&str]) -> Vec<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

pub(super) fn names_mismatch<'a>(names: impl IntoIterator<Item = Option<&'a str>>) -> bool {
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

pub(super) fn normalize_medical_text(value: &str) -> String {
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

pub(super) fn normalized_medical_text_character(character: char) -> Option<char> {
    match character {
        '\u{feff}' | '\u{fffd}' => None,
        '\u{00a0}' | '\u{3000}' => Some(' '),
        _ => Some(character),
    }
}

pub(super) fn extract_diagnosis(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        strip_label_value(line, "诊断：").or_else(|| strip_label_value(line, "诊断:"))
    })
}

pub(super) fn extract_next_line_after_label(text: &str, label: &str) -> Option<String> {
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

pub(super) fn strip_label_value(line: &str, label: &str) -> Option<String> {
    line.trim()
        .strip_prefix(label)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn mask_identifier(value: &str) -> String {
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

pub(super) fn push_signal(signals: &mut Vec<String>, signal: &str) {
    if !signals.iter().any(|existing| existing == signal) {
        signals.push(signal.into());
    }
}
