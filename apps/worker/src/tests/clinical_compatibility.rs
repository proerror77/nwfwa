use super::*;

#[test]
fn builds_clinical_compatibility_reference_report() {
    let root = temp_root("clinical-compatibility");
    let reference_uri = root.join("clinical-reference.json");
    write_json(
        reference_uri.clone(),
        &serde_json::json!({
            "reference_version": "clinical-policy-2026-06",
            "effective_date": "2026-06-01",
            "source_authority": "customer-medical-policy-board",
            "records": [
                {
                    "diagnosis_code_prefix": " j ",
                    "procedure_code": " img-900 ",
                    "compatibility_score": 0.25,
                    "policy_authority_ref": "policy:clinical:J:IMG-900",
                    "rationale": "Respiratory diagnosis requires additional support for this imaging procedure.",
                    "evidence_refs": ["policy:clinical:J:IMG-900", "medical_policy:v2026-06"]
                },
                {
                    "diagnosis_code_prefix": "S",
                    "procedure_code": "XR-100",
                    "compatibility_score": 0.85,
                    "policy_authority_ref": "policy:clinical:S:XR-100",
                    "rationale": "Trauma diagnosis generally supports X-ray review when documented.",
                    "evidence_refs": ["medical_policy:v2026-06"]
                }
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_clinical_compatibility_reference_report(
        &reference_uri.to_string_lossy(),
        &output_dir,
    )
    .expect("clinical compatibility reference report");

    assert_eq!(report.report_kind, "clinical_compatibility_reference");
    assert_eq!(report.reference_version, "clinical-policy-2026-06");
    assert_eq!(report.record_count, 2);
    assert_eq!(report.review_tasks.len(), 1);
    assert_eq!(
        report.review_tasks[0].task_type,
        "clinical_policy_review_candidate"
    );
    let imaging = report
        .records
        .iter()
        .find(|record| record.compatibility_key == "J|IMG-900")
        .expect("normalized imaging record");
    assert_eq!(imaging.diagnosis_code_prefix, "J");
    assert_eq!(imaging.procedure_code, "IMG-900");
    assert_eq!(imaging.diagnosis_procedure_match_score, 0.25);
    assert_eq!(
        imaging.data_source,
        "worker.icd_cpt_compatibility_reference:clinical-policy-2026-06"
    );
    let trauma = report
        .records
        .iter()
        .find(|record| record.compatibility_key == "S|XR-100")
        .expect("normalized trauma record");
    assert!(trauma
        .evidence_refs
        .contains(&"policy:clinical:S:XR-100".into()));
    assert!(report
        .governance_boundary
        .contains("ClinicalCompatibilityFeatureContext"));
    assert!(output_dir
        .join("clinical_compatibility_reference_report.json")
        .exists());
    assert!(output_dir
        .join("clinical_compatibility_records.json")
        .exists());
}

#[test]
fn rejects_clinical_compatibility_reference_rows_without_evidence() {
    let root = temp_root("clinical-compatibility-invalid");
    let reference_uri = root.join("clinical-reference.json");
    write_json(
        reference_uri.clone(),
        &serde_json::json!({
            "reference_version": "clinical-policy-2026-06",
            "effective_date": "2026-06-01",
            "source_authority": "customer-medical-policy-board",
            "records": [
                {
                    "diagnosis_code_prefix": "J",
                    "procedure_code": "IMG-900",
                    "compatibility_score": 0.25,
                    "policy_authority_ref": "policy:clinical:J:IMG-900",
                    "rationale": "Missing evidence should fail reference ingestion.",
                    "evidence_refs": []
                }
            ]
        }),
    )
    .unwrap();

    let error = build_clinical_compatibility_reference_report(
        &reference_uri.to_string_lossy(),
        root.join("out"),
    )
    .expect_err("missing evidence refs must be rejected");

    assert!(error.to_string().contains("requires evidence_refs"));
}

#[test]
fn builds_clinical_compatibility_reference_submission() {
    let root = temp_root("clinical-compatibility-submission");
    let report_uri = root.join("clinical_compatibility_reference_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "clinical_compatibility_reference",
            "report_version": 1,
            "reference_version": "clinical-policy-2026-06",
            "effective_date": "2026-06-01",
            "source_authority": "customer-medical-policy-board",
            "source_uri": "local://inputs/clinical-reference.json",
            "record_count": 1,
            "records": [
                {
                    "compatibility_key": "J|IMG-900",
                    "diagnosis_code_prefix": "J",
                    "procedure_code": "IMG-900",
                    "diagnosis_procedure_match_score": 0.25,
                    "data_source": "worker.icd_cpt_compatibility_reference:clinical-policy-2026-06",
                    "policy_authority_ref": "policy:clinical:J:IMG-900",
                    "rationale": "Respiratory diagnosis requires additional support for this imaging procedure.",
                    "evidence_refs": ["policy:clinical:J:IMG-900"]
                }
            ],
            "review_tasks": [],
            "evidence_refs": [
                "clinical_compatibility_reference:local://inputs/clinical-reference.json",
                "clinical_policy_authority:customer-medical-policy-board"
            ],
            "governance_boundary": "clinical compatibility reference data can feed ClinicalCompatibilityFeatureContext; it must not deny claims or replace medical review without customer-approved policy authority"
        }),
    )
    .unwrap();

    let submission = build_clinical_compatibility_reference_submission(
        &report_uri.to_string_lossy(),
        "worker:build-clinical-compatibility-reference",
        "customer policy board approved reference",
    )
    .expect("clinical compatibility submission");

    assert_eq!(submission.report_kind, "clinical_compatibility_reference");
    assert_eq!(submission.reference_version, "clinical-policy-2026-06");
    assert_eq!(submission.record_count, 1);
    assert_eq!(submission.records[0].compatibility_key, "J|IMG-900");
    assert!(submission.evidence_refs.contains(&format!(
        "clinical_compatibility_references:{}",
        report_uri.to_string_lossy()
    )));
}

#[test]
fn rejects_clinical_compatibility_submission_without_source_evidence() {
    let root = temp_root("clinical-compatibility-submission-missing-source");
    let report_uri = root.join("clinical_compatibility_reference_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "clinical_compatibility_reference",
            "report_version": 1,
            "reference_version": "clinical-policy-2026-06",
            "effective_date": "2026-06-01",
            "source_authority": "customer-medical-policy-board",
            "source_uri": "local://inputs/clinical-reference.json",
            "record_count": 1,
            "records": [
                {
                    "compatibility_key": "J|IMG-900",
                    "diagnosis_code_prefix": "J",
                    "procedure_code": "IMG-900",
                    "diagnosis_procedure_match_score": 0.25,
                    "data_source": "worker.icd_cpt_compatibility_reference:clinical-policy-2026-06",
                    "policy_authority_ref": "policy:clinical:J:IMG-900",
                    "rationale": "Respiratory diagnosis requires additional support for this imaging procedure.",
                    "evidence_refs": ["policy:clinical:J:IMG-900"]
                }
            ],
            "review_tasks": [],
            "evidence_refs": ["clinical_policy_authority:customer-medical-policy-board"],
            "governance_boundary": "clinical compatibility reference data can feed ClinicalCompatibilityFeatureContext; it must not deny claims or replace medical review without customer-approved policy authority"
        }),
    )
    .unwrap();

    let error = build_clinical_compatibility_reference_submission(
        &report_uri.to_string_lossy(),
        "worker:build-clinical-compatibility-reference",
        "customer policy board approved reference",
    )
    .expect_err("missing source evidence must fail");

    assert!(error
        .to_string()
        .contains("clinical_compatibility_reference:"));
}

#[test]
fn rejects_clinical_compatibility_submission_with_template_record_evidence() {
    let root = temp_root("clinical-compatibility-submission-template-record");
    let report_uri = root.join("clinical_compatibility_reference_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "clinical_compatibility_reference",
            "report_version": 1,
            "reference_version": "clinical-policy-2026-06",
            "effective_date": "2026-06-01",
            "source_authority": "customer-medical-policy-board",
            "source_uri": "local://inputs/clinical-reference.json",
            "record_count": 1,
            "records": [
                {
                    "compatibility_key": "J|IMG-900",
                    "diagnosis_code_prefix": "J",
                    "procedure_code": "IMG-900",
                    "diagnosis_procedure_match_score": 0.25,
                    "data_source": "worker.icd_cpt_compatibility_reference:clinical-policy-2026-06",
                    "policy_authority_ref": "policy:clinical:J:IMG-900",
                    "rationale": "Respiratory diagnosis requires additional support for this imaging procedure.",
                    "evidence_refs": [
                        "policy:clinical:J:IMG-900",
                        "medical_policy:local://template/clinical/policy.json"
                    ]
                }
            ],
            "review_tasks": [],
            "evidence_refs": [
                "clinical_compatibility_reference:local://inputs/clinical-reference.json",
                "clinical_policy_authority:customer-medical-policy-board"
            ],
            "governance_boundary": "clinical compatibility reference data can feed ClinicalCompatibilityFeatureContext; it must not deny claims or replace medical review without customer-approved policy authority"
        }),
    )
    .unwrap();

    let error = build_clinical_compatibility_reference_submission(
        &report_uri.to_string_lossy(),
        "worker:build-clinical-compatibility-reference",
        "customer policy board approved reference",
    )
    .expect_err("template record evidence must fail");

    assert!(error.to_string().contains(
        "clinical compatibility record evidence_refs must not use local://template evidence"
    ));
}

#[tokio::test]
async fn submits_clinical_compatibility_reference_to_api() {
    use tokio::net::TcpListener;

    let root = temp_root("clinical-compatibility-submit-api");
    let report_uri = root.join("clinical_compatibility_reference_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "clinical_compatibility_reference",
            "report_version": 1,
            "reference_version": "clinical-policy-2026-06",
            "effective_date": "2026-06-01",
            "source_authority": "customer-medical-policy-board",
            "source_uri": "local://inputs/clinical-reference.json",
            "record_count": 1,
            "records": [
                {
                    "compatibility_key": "J|IMG-900",
                    "diagnosis_code_prefix": "J",
                    "procedure_code": "IMG-900",
                    "diagnosis_procedure_match_score": 0.25,
                    "data_source": "worker.icd_cpt_compatibility_reference:clinical-policy-2026-06",
                    "policy_authority_ref": "policy:clinical:J:IMG-900",
                    "rationale": "Respiratory diagnosis requires additional support for this imaging procedure.",
                    "evidence_refs": ["policy:clinical:J:IMG-900"]
                }
            ],
            "review_tasks": [],
            "evidence_refs": [
                "clinical_compatibility_reference:local://inputs/clinical-reference.json",
                "clinical_policy_authority:customer-medical-policy-board"
            ],
            "governance_boundary": "clinical compatibility reference data can feed ClinicalCompatibilityFeatureContext; it must not deny claims or replace medical review without customer-approved policy authority"
        }),
    )
    .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        write_json_response(
            &mut socket,
            serde_json::json!({
                "report_kind": "clinical_compatibility_reference",
                "record_count": 1
            }),
        )
        .await;
        request
    });

    let response = submit_clinical_compatibility_reference(
        &api_url,
        "dataset-write-secret",
        &report_uri.to_string_lossy(),
        "worker:build-clinical-compatibility-reference",
        "customer policy board approved reference",
    )
    .await
    .expect("submit clinical compatibility reference");

    assert_eq!(response["record_count"], 1);
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/v1/ops/clinical-compatibility-references HTTP/1.1"));
    assert!(request.contains("x-api-key: dataset-write-secret"));
    assert!(request.contains(r#""compatibility_key":"J|IMG-900""#));
    assert!(request.contains("clinical_compatibility_references:"));
}
