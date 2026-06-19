use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorFormat {
    X12,
    Hl7V2,
    FhirJson,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorEnvelopeSummary {
    pub format: ConnectorFormat,
    pub source_system: String,
    pub transaction_kind: String,
    pub message_control_id: Option<String>,
    pub claim_id: Option<String>,
    pub patient_ref: Option<String>,
    pub provider_ref: Option<String>,
    pub evidence_refs: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConnectorError {
    #[error("connector payload is empty")]
    EmptyPayload,
    #[error("source_system is required")]
    MissingSourceSystem,
    #[error("expected X12 interchange envelope")]
    InvalidX12Envelope,
    #[error("expected HL7 v2 message")]
    InvalidHl7V2Message,
    #[error("invalid FHIR JSON: {0}")]
    InvalidFhirJson(String),
    #[error("expected FHIR Bundle resource")]
    InvalidFhirBundle,
}

pub fn summarize_connector_payload(
    format: ConnectorFormat,
    source_system: &str,
    payload: &str,
) -> Result<ConnectorEnvelopeSummary, ConnectorError> {
    match format {
        ConnectorFormat::X12 => summarize_x12_837(source_system, payload),
        ConnectorFormat::Hl7V2 => summarize_hl7_v2(source_system, payload),
        ConnectorFormat::FhirJson => summarize_fhir_bundle(source_system, payload),
    }
}

pub fn summarize_x12_837(
    source_system: &str,
    payload: &str,
) -> Result<ConnectorEnvelopeSummary, ConnectorError> {
    ensure_source_system(source_system)?;
    let payload = required_payload(payload)?;
    let segments = payload
        .split('~')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if !segments
        .first()
        .is_some_and(|segment| segment.starts_with("ISA*"))
    {
        return Err(ConnectorError::InvalidX12Envelope);
    }

    let st = segments
        .iter()
        .find_map(|segment| segment_fields(segment, "ST"))
        .unwrap_or_default();
    let transaction_kind = match st.first().map(String::as_str) {
        Some("837") => "x12_837_claim",
        Some(value) => value,
        None => "x12_interchange",
    }
    .to_string();
    let message_control_id = st.get(1).cloned();
    let claim_id = segments
        .iter()
        .find_map(|segment| segment_fields(segment, "CLM"))
        .and_then(|fields| fields.first().cloned());
    let patient_ref = segments.iter().find_map(|segment| {
        let fields = segment_fields(segment, "NM1")?;
        if fields
            .first()
            .is_some_and(|value| value == "IL" || value == "QC")
        {
            fields.get(8).cloned()
        } else {
            None
        }
    });
    let provider_ref = segments.iter().find_map(|segment| {
        let fields = segment_fields(segment, "NM1")?;
        if fields
            .first()
            .is_some_and(|value| value == "85" || value == "82")
        {
            fields.get(8).cloned()
        } else {
            None
        }
    });
    let warnings = missing_id_warnings(&[
        ("message_control_id", &message_control_id),
        ("claim_id", &claim_id),
        ("patient_ref", &patient_ref),
        ("provider_ref", &provider_ref),
    ]);

    Ok(summary(
        ConnectorFormat::X12,
        source_system,
        transaction_kind,
        message_control_id,
        claim_id,
        patient_ref,
        provider_ref,
        warnings,
    ))
}

pub fn summarize_hl7_v2(
    source_system: &str,
    payload: &str,
) -> Result<ConnectorEnvelopeSummary, ConnectorError> {
    ensure_source_system(source_system)?;
    let payload = required_payload(payload)?;
    let segments = payload
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let msh = segments
        .iter()
        .find(|segment| segment.starts_with("MSH|"))
        .ok_or(ConnectorError::InvalidHl7V2Message)?
        .split('|')
        .collect::<Vec<_>>();
    let transaction_kind = msh.get(8).copied().unwrap_or("hl7_v2_message").to_string();
    let message_control_id = msh.get(9).map(|value| (*value).to_string());
    let patient_ref = segments.iter().find_map(|segment| {
        let fields = pipe_fields(segment, "PID")?;
        fields.get(2).or_else(|| fields.get(3)).cloned()
    });
    let provider_ref = segments.iter().find_map(|segment| {
        let fields = pipe_fields(segment, "PV1")?;
        fields.get(6).cloned()
    });
    let warnings = missing_id_warnings(&[
        ("message_control_id", &message_control_id),
        ("patient_ref", &patient_ref),
    ]);

    Ok(summary(
        ConnectorFormat::Hl7V2,
        source_system,
        transaction_kind,
        message_control_id,
        None,
        patient_ref,
        provider_ref,
        warnings,
    ))
}

pub fn summarize_fhir_bundle(
    source_system: &str,
    payload: &str,
) -> Result<ConnectorEnvelopeSummary, ConnectorError> {
    ensure_source_system(source_system)?;
    let payload = required_payload(payload)?;
    let value: serde_json::Value = serde_json::from_str(payload)
        .map_err(|error| ConnectorError::InvalidFhirJson(error.to_string()))?;
    if value.get("resourceType").and_then(|value| value.as_str()) != Some("Bundle") {
        return Err(ConnectorError::InvalidFhirBundle);
    }

    let message_control_id = value
        .get("identifier")
        .and_then(|identifier| identifier.get("value"))
        .or_else(|| value.get("id"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let mut claim_id = None;
    let mut patient_ref = None;
    let mut provider_ref = None;
    if let Some(entries) = value.get("entry").and_then(|value| value.as_array()) {
        for resource in entries
            .iter()
            .filter_map(|entry| entry.get("resource"))
            .filter_map(|resource| resource.as_object())
        {
            match resource
                .get("resourceType")
                .and_then(|value| value.as_str())
            {
                Some("Claim") => {
                    claim_id = claim_id.or_else(|| string_field(resource, "id"));
                    patient_ref = patient_ref
                        .or_else(|| reference_field(resource, "patient"))
                        .or_else(|| reference_field(resource, "subscriber"));
                    provider_ref = provider_ref.or_else(|| reference_field(resource, "provider"));
                }
                Some("Patient") => {
                    patient_ref = patient_ref.or_else(|| string_field(resource, "id"));
                }
                Some("Practitioner") | Some("Organization") => {
                    provider_ref = provider_ref.or_else(|| string_field(resource, "id"));
                }
                _ => {}
            }
        }
    }
    let warnings = missing_id_warnings(&[
        ("message_control_id", &message_control_id),
        ("claim_id", &claim_id),
        ("patient_ref", &patient_ref),
        ("provider_ref", &provider_ref),
    ]);

    Ok(summary(
        ConnectorFormat::FhirJson,
        source_system,
        "fhir_bundle".into(),
        message_control_id,
        claim_id,
        patient_ref,
        provider_ref,
        warnings,
    ))
}

fn ensure_source_system(source_system: &str) -> Result<(), ConnectorError> {
    if source_system.trim().is_empty() {
        return Err(ConnectorError::MissingSourceSystem);
    }
    Ok(())
}

fn required_payload(payload: &str) -> Result<&str, ConnectorError> {
    let payload = payload.trim();
    if payload.is_empty() {
        return Err(ConnectorError::EmptyPayload);
    }
    Ok(payload)
}

fn segment_fields(segment: &str, tag: &str) -> Option<Vec<String>> {
    let mut fields = segment.split('*');
    if fields.next()? != tag {
        return None;
    }
    Some(fields.map(str::to_string).collect())
}

fn pipe_fields(segment: &str, tag: &str) -> Option<Vec<String>> {
    let mut fields = segment.split('|');
    if fields.next()? != tag {
        return None;
    }
    Some(fields.map(str::to_string).collect())
}

fn string_field(
    resource: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Option<String> {
    resource
        .get(field)
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn reference_field(
    resource: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Option<String> {
    resource
        .get(field)
        .and_then(|value| value.get("reference"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn missing_id_warnings(fields: &[(&str, &Option<String>)]) -> Vec<String> {
    fields
        .iter()
        .filter(|(_, value)| value.as_deref().is_none_or(str::is_empty))
        .map(|(field, _)| format!("missing_{field}"))
        .collect()
}

fn summary(
    format: ConnectorFormat,
    source_system: &str,
    transaction_kind: String,
    message_control_id: Option<String>,
    claim_id: Option<String>,
    patient_ref: Option<String>,
    provider_ref: Option<String>,
    warnings: Vec<String>,
) -> ConnectorEnvelopeSummary {
    let mut evidence_refs = vec![format!(
        "connector_envelope:{source_system}:{}",
        message_control_id.as_deref().unwrap_or("uncontrolled")
    )];
    if let Some(claim_id) = &claim_id {
        evidence_refs.push(format!("connector_claim:{source_system}:{claim_id}"));
    }
    ConnectorEnvelopeSummary {
        format,
        source_system: source_system.trim().into(),
        transaction_kind,
        message_control_id,
        claim_id,
        patient_ref,
        provider_ref,
        evidence_refs,
        warnings,
    }
}

pub fn crate_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_x12_837_claim_envelope_without_raw_payload() {
        let payload = "ISA*00*          *00*          *ZZ*SENDER         *ZZ*RECEIVER       *260619*1200*^*00501*000000905*0*T*:~GS*HC*SENDER*RECEIVER*20260619*1200*1*X*005010X222A1~ST*837*0001*005010X222A1~NM1*85*2*ACME CLINIC*****XX*1999999999~NM1*IL*1*DOE*JANE****MI*MBR-001~CLM*CLM-001*120.00***11:B:1*Y*A*Y*Y~SE*5*0001~GE*1*1~IEA*1*000000905~";

        let summary = summarize_x12_837("customer-tpa", payload).unwrap();

        assert_eq!(summary.format, ConnectorFormat::X12);
        assert_eq!(summary.transaction_kind, "x12_837_claim");
        assert_eq!(summary.message_control_id.as_deref(), Some("0001"));
        assert_eq!(summary.claim_id.as_deref(), Some("CLM-001"));
        assert_eq!(summary.patient_ref.as_deref(), Some("MBR-001"));
        assert_eq!(summary.provider_ref.as_deref(), Some("1999999999"));
        assert!(summary
            .evidence_refs
            .contains(&"connector_claim:customer-tpa:CLM-001".into()));
        assert_eq!(summary.warnings, Vec::<String>::new());
    }

    #[test]
    fn summarizes_hl7_v2_message_control_and_patient() {
        let payload = "MSH|^~\\&|TPA|FAC|FWA|NWFWA|202606191200||ADT^A01|MSG-123|P|2.5\rPID|1||MBR-123^^^MRN||DOE^JANE\rPV1|1|O|||||PRV-42";

        let summary = summarize_hl7_v2("customer-tpa", payload).unwrap();

        assert_eq!(summary.format, ConnectorFormat::Hl7V2);
        assert_eq!(summary.transaction_kind, "ADT^A01");
        assert_eq!(summary.message_control_id.as_deref(), Some("MSG-123"));
        assert_eq!(summary.patient_ref.as_deref(), Some("MBR-123^^^MRN"));
        assert_eq!(summary.provider_ref.as_deref(), Some("PRV-42"));
        assert!(summary.claim_id.is_none());
    }

    #[test]
    fn summarizes_fhir_claim_bundle() {
        let payload = serde_json::json!({
            "resourceType": "Bundle",
            "id": "bundle-1",
            "entry": [
                {
                    "resource": {
                        "resourceType": "Claim",
                        "id": "claim-1",
                        "patient": {"reference": "Patient/patient-1"},
                        "provider": {"reference": "Organization/provider-1"}
                    }
                }
            ]
        })
        .to_string();

        let summary = summarize_fhir_bundle("customer-tpa", &payload).unwrap();

        assert_eq!(summary.format, ConnectorFormat::FhirJson);
        assert_eq!(summary.transaction_kind, "fhir_bundle");
        assert_eq!(summary.message_control_id.as_deref(), Some("bundle-1"));
        assert_eq!(summary.claim_id.as_deref(), Some("claim-1"));
        assert_eq!(summary.patient_ref.as_deref(), Some("Patient/patient-1"));
        assert_eq!(
            summary.provider_ref.as_deref(),
            Some("Organization/provider-1")
        );
    }

    #[test]
    fn rejects_invalid_fhir_resource() {
        let error = summarize_fhir_bundle(
            "customer-tpa",
            r#"{"resourceType":"Claim","id":"claim-outside-bundle"}"#,
        )
        .unwrap_err();

        assert_eq!(error, ConnectorError::InvalidFhirBundle);
    }
}
