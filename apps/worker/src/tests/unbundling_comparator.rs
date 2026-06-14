use super::*;

#[test]
fn builds_unbundling_comparator_candidates_from_episode_codes() {
    let root = temp_root("unbundling-comparator");
    let input_uri = root.join("unbundling-input.json");
    write_json(
        input_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-14",
            "rules": [
                {
                    "rule_id": "UNBUNDLE-KNEE-001",
                    "bundled_code": " knee-bundle ",
                    "component_codes": ["scope-a", "scope-b"],
                    "policy_authority_ref": "policy:unbundling:knee:v1",
                    "evidence_refs": ["policy:unbundling:knee:v1"]
                }
            ],
            "episodes": [
                {
                    "episode_key": "MBR-1|PRV-A",
                    "member_id": "MBR-1",
                    "provider_id": "PRV-A",
                    "window_days": 30,
                    "claim_ids": ["CLM-1", "CLM-2"],
                    "procedure_codes": ["KNEE-BUNDLE", "SCOPE-A", "OTHER"]
                },
                {
                    "episode_key": "MBR-2|PRV-B",
                    "member_id": "MBR-2",
                    "provider_id": "PRV-B",
                    "window_days": 30,
                    "claim_ids": ["CLM-3"],
                    "procedure_codes": ["SCOPE-A"]
                }
            ]
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_unbundling_comparator_report(&input_uri.to_string_lossy(), &output_dir)
        .expect("unbundling comparator report");

    assert_eq!(report.report_kind, "unbundling_comparator");
    assert_eq!(report.rule_count, 1);
    assert_eq!(report.episode_count, 2);
    assert_eq!(report.candidate_count, 1);
    let candidate = &report.candidates[0];
    assert_eq!(candidate.rule_id, "UNBUNDLE-KNEE-001");
    assert_eq!(candidate.episode_key, "MBR-1|PRV-A");
    assert_eq!(candidate.bundled_code, "KNEE-BUNDLE");
    assert_eq!(candidate.matched_component_codes, vec!["SCOPE-A"]);
    assert_eq!(candidate.recommended_review, "medical_review_candidate");
    assert!(candidate
        .evidence_refs
        .contains(&"policy:unbundling:knee:v1".into()));
    assert!(candidate.evidence_refs.contains(&"claims:CLM-1".into()));
    assert!(report.governance_boundary.contains("must not assign fraud"));
    assert!(output_dir
        .join("unbundling_comparator_report.json")
        .exists());
    assert!(output_dir
        .join("unbundling_comparator_candidates.json")
        .exists());
}

#[test]
fn rejects_unbundling_rules_without_policy_evidence() {
    let root = temp_root("unbundling-comparator-invalid");
    let input_uri = root.join("unbundling-input.json");
    write_json(
        input_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-14",
            "rules": [
                {
                    "rule_id": "UNBUNDLE-KNEE-001",
                    "bundled_code": "KNEE-BUNDLE",
                    "component_codes": ["SCOPE-A"],
                    "policy_authority_ref": "policy:unbundling:knee:v1",
                    "evidence_refs": []
                }
            ],
            "episodes": []
        }),
    )
    .unwrap();

    let error = build_unbundling_comparator_report(&input_uri.to_string_lossy(), root.join("out"))
        .expect_err("unbundling rule without evidence refs must fail");

    assert!(error.to_string().contains("requires evidence_refs"));
}
