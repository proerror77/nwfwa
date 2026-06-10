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
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(signing_key.as_bytes())
        .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
    mac.update(format!("{model_key}:{model_version}:{artifact_sha256}").as_bytes());
    let actual_signature = format!("hmac-sha256:{}", to_hex(&mac.finalize().into_bytes()));
    if actual_signature != expected_signature {
        return Err(ModelRuntimeError::InvalidResponse(
            "model artifact signature mismatch".into(),
        ));
    }
    Ok("passed")
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
