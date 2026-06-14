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
                    "evidence_refs": ["policy:clinical:S:XR-100"]
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
