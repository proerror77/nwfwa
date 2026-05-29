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
            redact_text("email alice@example.com phone:13800138000 id 11010519491231002X"),
            "email [REDACTED_EMAIL] [REDACTED_PHONE] id [REDACTED_ID]"
        );
    }
}
