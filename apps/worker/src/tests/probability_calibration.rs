use super::*;

#[test]
fn builds_probability_calibration_report_for_labeled_holdout() {
    let root = temp_root("probability-calibration");
    let source_uri = root.join("probability-calibration-input.json");
    let mut rows = Vec::new();
    for index in 0..50 {
        rows.push(serde_json::json!({
            "observation_id": format!("LOW-{index}"),
            "predicted_probability": 0.1,
            "actual_label": if index < 5 { 1 } else { 0 }
        }));
    }
    for index in 0..50 {
        rows.push(serde_json::json!({
            "observation_id": format!("HIGH-{index}"),
            "predicted_probability": 0.8,
            "actual_label": if index < 40 { 1 } else { 0 }
        }));
    }
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-13",
            "label_source_uri": "s3://labels/holdout-2026-06-13.json",
            "rows": rows
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report =
        build_probability_calibration_report(&source_uri.to_string_lossy(), &output_dir, Some(10))
            .expect("probability calibration report");

    assert_eq!(report.report_kind, "probability_calibration_report");
    assert_eq!(report.row_count, 100);
    assert_eq!(report.calibration_status, "passed");
    assert_eq!(report.expected_calibration_error, 0.0);
    assert_eq!(report.brier_score, 0.125);
    assert!(report.review_tasks.is_empty());
    assert!(report
        .evidence_refs
        .iter()
        .any(|reference| reference.starts_with("calibration_labels:")));
    assert!(output_dir
        .join("probability_calibration_report.json")
        .exists());
    assert!(output_dir
        .join("probability_calibration_bins.json")
        .exists());
    assert!(output_dir
        .join("probability_calibration_review_tasks.json")
        .exists());
}

#[test]
fn opens_probability_calibration_review_when_raw_probabilities_are_miscalibrated() {
    let root = temp_root("probability-calibration-review");
    let source_uri = root.join("probability-calibration-input.json");
    let rows = (0..100)
        .map(|index| {
            serde_json::json!({
                "observation_id": format!("NEG-{index}"),
                "predicted_probability": 0.9,
                "actual_label": 0
            })
        })
        .collect::<Vec<_>>();
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "as_of_date": "2026-06-13",
            "rows": rows
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report =
        build_probability_calibration_report(&source_uri.to_string_lossy(), &output_dir, Some(10))
            .expect("probability calibration report");

    assert_eq!(report.calibration_status, "needs_calibration_review");
    assert_eq!(report.expected_calibration_error, 0.9);
    assert_eq!(report.brier_score, 0.81);
    assert_eq!(report.review_tasks.len(), 1);
    assert_eq!(report.review_tasks[0].priority, "high");
    assert!(report
        .governance_boundary
        .contains("must not relabel outcomes"));
}
