use crate::types::ModelRuntimeError;
use hmac::{Hmac, Mac};

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

pub(crate) fn verify_artifact_signature(
    model_key: &str,
    model_version: &str,
    artifact_sha256: &str,
    expected_signature: Option<&str>,
    signing_key: Option<&str>,
) -> Result<&'static str, ModelRuntimeError> {
    let Some(expected_signature) = expected_signature else {
        return Ok("not_configured");
    };
    let Some(signing_key) = signing_key else {
        return Err(ModelRuntimeError::InvalidResponse(
            "model artifact signature key missing".into(),
        ));
    };

    // Parse "hmac-sha256:<hex>" and decode the hex bytes for constant-time
    // verification via hmac::Mac::verify_slice, which is timing-safe.
    let hex_part = expected_signature
        .strip_prefix("hmac-sha256:")
        .ok_or_else(|| {
            ModelRuntimeError::InvalidResponse(
                "expected_signature must use hmac-sha256:<hex> format".into(),
            )
        })?;
    let expected_bytes = hex_to_bytes(hex_part).map_err(|_| {
        ModelRuntimeError::InvalidResponse("expected_signature contains invalid hex".into())
    })?;

    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(signing_key.as_bytes())
        .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
    mac.update(format!("{model_key}:{model_version}:{artifact_sha256}").as_bytes());

    // verify_slice performs a constant-time comparison — no timing side-channel.
    mac.verify_slice(&expected_bytes).map_err(|_| {
        ModelRuntimeError::InvalidResponse("model artifact signature mismatch".into())
    })?;

    Ok("passed")
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, ()> {
    if !hex.len().is_multiple_of(2) {
        return Err(());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

#[allow(dead_code)]
pub(crate) fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
