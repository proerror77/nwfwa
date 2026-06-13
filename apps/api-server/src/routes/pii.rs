use serde_json::{Map, Value};

pub fn contains_pii(values: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    values
        .into_iter()
        .any(|value| value.as_ref().split_whitespace().any(token_has_pii))
}

pub fn redact_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(redact_pii_token)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn mask_claim_response_payload(value: &Value) -> Value {
    mask_value_at_path(value, &mut Vec::new())
}

fn mask_value_at_path(value: &Value, path: &mut Vec<String>) -> Value {
    match value {
        Value::Object(object) => Value::Object(mask_object(object, path)),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| mask_value_at_path(value, path))
                .collect(),
        ),
        Value::String(_) if should_mask_path(path) => {
            Value::String(response_placeholder_for_path(path).into())
        }
        Value::String(text) => Value::String(redact_text(text)),
        other => other.clone(),
    }
}

fn mask_object(object: &Map<String, Value>, path: &mut Vec<String>) -> Map<String, Value> {
    object
        .iter()
        .map(|(key, value)| {
            path.push(key.clone());
            let masked = mask_value_at_path(value, path);
            path.pop();
            (key.clone(), masked)
        })
        .collect()
}

fn should_mask_path(path: &[String]) -> bool {
    path.last()
        .map(|key| known_phi_field_key(key))
        .unwrap_or(false)
}

fn known_phi_field_key(key: &str) -> bool {
    matches!(
        normalize_field_key(key).as_str(),
        "insuredname"
            | "insuredno"
            | "certno"
            | "certificateno"
            | "certificateid"
            | "memberid"
            | "membername"
            | "patientname"
            | "accidentpersonname"
    )
}

fn response_placeholder_for_path(path: &[String]) -> &'static str {
    match path.last().map(|key| normalize_field_key(key)).as_deref() {
        Some("insuredname" | "membername" | "patientname" | "accidentpersonname") => {
            "[REDACTED_NAME]"
        }
        Some("insuredno" | "certno" | "certificateno" | "certificateid" | "memberid") => {
            "[REDACTED_ID]"
        }
        _ => "[REDACTED_PHI]",
    }
}

fn normalize_field_key(key: &str) -> String {
    key.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn token_has_pii(token: &str) -> bool {
    pii_placeholder(trim_token(token)).is_some()
}

fn redact_pii_token(token: &str) -> String {
    pii_placeholder(trim_token(token))
        .map(str::to_string)
        .unwrap_or_else(|| token.to_string())
}

fn trim_token(token: &str) -> &str {
    let value = token.trim_matches(|character: char| {
        matches!(
            character,
            ',' | '.'
                | ';'
                | ':'
                | '!'
                | '?'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | '，'
                | '。'
                | '；'
                | '：'
                | '！'
                | '？'
                | '（'
                | '）'
                | '【'
                | '】'
        )
    });
    value
}

fn pii_placeholder(value: &str) -> Option<&'static str> {
    pii_value_placeholder(value).or_else(|| {
        value
            .rsplit_once(':')
            .or_else(|| value.rsplit_once('：'))
            .and_then(|(_, suffix)| pii_value_placeholder(suffix))
    })
}

fn pii_value_placeholder(value: &str) -> Option<&'static str> {
    if is_email_like(value) {
        Some("[REDACTED_EMAIL]")
    } else if is_cn_id_like(value) {
        Some("[REDACTED_ID]")
    } else if is_phone_like(value) {
        Some("[REDACTED_PHONE]")
    } else {
        None
    }
}

fn is_email_like(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && domain.len() >= 3
}

fn is_cn_id_like(value: &str) -> bool {
    value.len() == 18
        && value
            .chars()
            .take(17)
            .all(|character| character.is_ascii_digit())
        && value
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_digit() || matches!(character, 'X' | 'x'))
}

fn is_phone_like(value: &str) -> bool {
    value.len() >= 10
        && value.len() <= 15
        && value.chars().all(|character| character.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_pii_tokens() {
        assert!(contains_pii(["email alice@example.com"]));
        assert!(contains_pii(["phone:13800138000"]));
        assert!(contains_pii(["email:alice@example.com"]));
        assert!(contains_pii(["id 11010519491231002X"]));
        assert!(!contains_pii(["audit:scoring.completed"]));
        assert!(!contains_pii(["claim_items:IMG-900"]));
    }

    #[test]
    fn redacts_common_pii_tokens() {
        assert_eq!(
            redact_text(
                "email alice@example.com phone:13800138000 id 11010519491231002X 卡号：00002602523"
            ),
            "email [REDACTED_EMAIL] [REDACTED_PHONE] id [REDACTED_ID] [REDACTED_PHONE]"
        );
    }

    #[test]
    fn masks_claim_response_payload_by_field_key() {
        let payload = serde_json::json!({
            "canonical_claim_context": {
                "insuredName": "LEE, Peter",
                "certificateNo": "D209475(0)",
                "nested": [
                    {
                        "patientName": "王向龙",
                        "accidentPersonName": "王向龙",
                        "hospitalName": "南京同仁医院"
                    }
                ]
            }
        });

        let masked = mask_claim_response_payload(&payload);

        assert_eq!(
            masked["canonical_claim_context"]["insuredName"],
            "[REDACTED_NAME]"
        );
        assert_eq!(
            masked["canonical_claim_context"]["certificateNo"],
            "[REDACTED_ID]"
        );
        assert_eq!(
            masked["canonical_claim_context"]["nested"][0]["patientName"],
            "[REDACTED_NAME]"
        );
        assert_eq!(
            masked["canonical_claim_context"]["nested"][0]["accidentPersonName"],
            "[REDACTED_NAME]"
        );
        assert_eq!(
            masked["canonical_claim_context"]["nested"][0]["hospitalName"],
            "南京同仁医院"
        );
    }
}
