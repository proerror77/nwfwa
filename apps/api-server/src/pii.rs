use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn mask_audit_payload(payload: Value) -> Value {
    match payload {
        Value::Array(values) => Value::Array(values.into_iter().map(mask_audit_payload).collect()),
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| {
                    let masked_value = match key.as_str() {
                        "external_member_id"
                        | "member_id"
                        | "masked_member_id"
                        | "masked_certificate_id" => mask_identifier(value),
                        "dob" | "member_birth_date" | "birth_date" | "date_of_birth" => {
                            mask_date(value)
                        }
                        "gender" | "member_gender" => Value::String("MASKED".into()),
                        _ => mask_audit_payload(value),
                    };
                    (key, masked_value)
                })
                .collect(),
        ),
        other => other,
    }
}

fn mask_identifier(value: Value) -> Value {
    match value {
        Value::String(raw) if !raw.trim().is_empty() => {
            let digest = Sha256::digest(raw.as_bytes());
            Value::String(format!("sha256:{:x}", digest))
        }
        other => mask_audit_payload(other),
    }
}

fn mask_date(value: Value) -> Value {
    match value {
        Value::String(raw) => raw
            .get(0..4)
            .filter(|year| year.chars().all(|ch| ch.is_ascii_digit()))
            .map(|year| Value::String(format!("{year}-XX-XX")))
            .unwrap_or(Value::String("MASKED".into())),
        other => mask_audit_payload(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn masks_member_identifiers_dates_and_gender_recursively() {
        let payload = json!({
            "external_member_id": "MBR-12345",
            "dob": "1988-03-12",
            "gender": "F",
            "nested": {
                "member_birth_date": "1991-07-09",
                "member_gender": "M",
                "items": [
                    { "masked_certificate_id": "ID-987654" }
                ]
            }
        });

        let masked = mask_audit_payload(payload);

        assert_ne!(masked["external_member_id"], "MBR-12345");
        assert!(masked["external_member_id"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"));
        assert_eq!(masked["dob"], "1988-XX-XX");
        assert_eq!(masked["gender"], "MASKED");
        assert_eq!(masked["nested"]["member_birth_date"], "1991-XX-XX");
        assert_eq!(masked["nested"]["member_gender"], "MASKED");
        assert_ne!(
            masked["nested"]["items"][0]["masked_certificate_id"],
            "ID-987654"
        );
    }

    #[test]
    fn preserves_non_pii_fields() {
        let payload = json!({
            "claim_id": "CLM-1",
            "risk_score": 72,
            "routing_reason": "manual review"
        });

        assert_eq!(mask_audit_payload(payload.clone()), payload);
    }
}
