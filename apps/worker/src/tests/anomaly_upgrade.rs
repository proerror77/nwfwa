use super::*;

#[test]
fn anomaly_upgrade_readiness_opens_review_when_label_threshold_is_met() {
    let root = temp_root("anomaly-upgrade-ready");
    let source_uri = root.join("anomaly-readiness-input.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-13",
            "confirmed_fwa_label_count": 640,
            "total_labeled_claim_count": 5000,
            "anomaly_recall_30d": 0.62,
            "current_detector": "heuristic_l3_baseline",
            "label_source_uri": "s3://fwa-labels/confirmed-fwa/2026-06-13.json"
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_anomaly_upgrade_readiness_report(&source_uri.to_string_lossy(), &output_dir)
        .expect("anomaly upgrade readiness");

    assert_eq!(report.report_kind, "anomaly_upgrade_readiness_report");
    assert_eq!(
        report.readiness_status,
        "ready_for_statistical_baseline_evaluation"
    );
    assert!(report.label_threshold_met);
    assert!(report.recall_below_threshold);
    assert_eq!(report.minimum_confirmed_fwa_labels, 500);
    assert_eq!(report.review_tasks.len(), 1);
    assert_eq!(report.review_tasks[0].priority, "high");
    assert!(report
        .recommended_actions
        .contains(&"prepare_iqr_mad_statistical_baseline_evaluation".into()));
    assert!(output_dir
        .join("anomaly_upgrade_readiness_report.json")
        .exists());
    assert!(output_dir
        .join("anomaly_upgrade_review_tasks.json")
        .exists());
}

#[test]
fn anomaly_upgrade_readiness_does_not_upgrade_without_enough_confirmed_labels() {
    let root = temp_root("anomaly-upgrade-not-ready");
    let source_uri = root.join("anomaly-readiness-input.json");
    write_json(
        source_uri.clone(),
        &serde_json::json!({
            "as_of_date": "2026-06-13",
            "confirmed_fwa_label_count": 120,
            "anomaly_recall_30d": 0.50
        }),
    )
    .unwrap();

    let output_dir = root.join("out");
    let report = build_anomaly_upgrade_readiness_report(&source_uri.to_string_lossy(), &output_dir)
        .expect("anomaly upgrade readiness");

    assert_eq!(report.readiness_status, "insufficient_confirmed_fwa_labels");
    assert!(!report.label_threshold_met);
    assert!(report.recall_below_threshold);
    assert!(report.review_tasks.is_empty());
    assert!(report
        .recommended_actions
        .contains(&"continue_confirmed_fwa_label_collection".into()));
    assert!(report
        .recommended_actions
        .contains(&"review_low_recall_before_threshold_met".into()));
}
