use super::*;
use parquet::arrow::ArrowWriter;
use std::fs::File;

#[test]
fn builds_rust_demo_ml_datasets_with_labeled_and_unlabeled_manifests() {
    let root = temp_root("demo-ml-datasets");
    let pack = build_demo_ml_datasets(&root, "2026-06-rust-demo").expect("demo ML datasets");

    assert_eq!(pack.pack_kind, "rust_automl_demo_datasets");
    assert_eq!(pack.dataset_version, "2026-06-rust-demo");
    assert_eq!(pack.dataset_manifests.len(), 3);
    assert_eq!(pack.unlabeled_manifest_uris.len(), 2);
    assert!(root.join("index.json").is_file());

    let labeled_manifest_path = root.join("labeled_claim_risk/manifest.json");
    let scoring_manifest_path = root.join("unlabeled_shadow_scoring/manifest.json");
    let provider_manifest_path = root.join("unlabeled_provider_peer_clustering/manifest.json");
    assert!(labeled_manifest_path.is_file());
    assert!(scoring_manifest_path.is_file());
    assert!(provider_manifest_path.is_file());
    assert!(root
        .join("labeled_claim_risk/split=train/part-00000.parquet")
        .is_file());
    assert!(root
        .join("labeled_claim_risk/split=validation/part-00000.parquet")
        .is_file());
    assert!(root
        .join("labeled_claim_risk/split=out_of_time/part-00000.parquet")
        .is_file());
    assert!(root
        .join("unlabeled_shadow_scoring/split=scoring/part-00000.parquet")
        .is_file());
    assert!(root
        .join("unlabeled_provider_peer_clustering/split=analysis/part-00000.parquet")
        .is_file());

    let labeled_manifest = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&labeled_manifest_path).unwrap(),
    )
    .unwrap();
    assert_eq!(labeled_manifest["label_column"], "confirmed_fwa");
    assert_eq!(
        labeled_manifest["label_policy"],
        "weak_rust_demo_label_not_production_evidence"
    );
    assert_eq!(labeled_manifest["splits"].as_array().unwrap().len(), 3);

    let scoring_manifest = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&scoring_manifest_path).unwrap(),
    )
    .unwrap();
    assert!(scoring_manifest.get("label_column").is_none());
    assert_eq!(
        scoring_manifest["label_policy"],
        "unlabeled_shadow_scoring_only"
    );

    let provider_manifest = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&provider_manifest_path).unwrap(),
    )
    .unwrap();
    assert!(provider_manifest.get("label_column").is_none());
    assert_eq!(
        provider_manifest["label_policy"],
        "unlabeled_clustering_discovery_only"
    );

    let profile_dir = root.join("profile");
    let profile = profile_manifest_file(&labeled_manifest_path, &profile_dir).unwrap();
    assert_eq!(profile.profile.row_count_by_split["train"], 8);
    assert_eq!(profile.profile.row_count_by_split["validation"], 4);
    assert_eq!(profile.profile.row_count_by_split["out_of_time"], 4);
    assert_eq!(profile.profile.label_distribution_by_split["train"]["1"], 4);
    assert_eq!(profile.profile.label_distribution_by_split["train"]["0"], 4);
    assert!(profile_dir.join("schema.json").is_file());
    assert!(profile_dir.join("profile.json").is_file());
    assert!(profile_dir.join("catalog.json").is_file());
    assert!(pack
        .next_worker_commands
        .iter()
        .any(|command| command.contains("build-feature-set")));
    assert!(pack
        .next_worker_commands
        .iter()
        .any(|command| command.contains("cluster-provider-peers")));
    assert!(pack
        .next_worker_commands
        .iter()
        .any(|command| command.contains("cluster-provider-graph")));
    assert!(pack
        .next_worker_commands
        .iter()
        .any(|command| command.contains("cluster-claim-entities")));
}

#[test]
fn builds_feature_set_manifest_from_labeled_parquet_manifest() {
    let root = temp_root("feature-set");
    let pack = build_demo_ml_datasets(&root, "2026-06-feature-set").expect("demo ML datasets");
    let output_dir = root.join("feature-set-output");

    let feature_set = build_feature_set(
        &pack.labeled_manifest_uri,
        &output_dir,
        Some("claims-risk-demo-features-v1"),
    )
    .expect("feature set");
    let repeated = build_feature_set(
        &pack.labeled_manifest_uri,
        root.join("feature-set-output-repeat"),
        Some("claims-risk-demo-features-v1"),
    )
    .expect("repeat feature set");

    assert_eq!(feature_set.manifest_kind, "rust_feature_set_manifest");
    assert_eq!(feature_set.feature_set_id, "claims-risk-demo-features-v1");
    assert_eq!(feature_set.dataset_key, "rust_demo_claim_risk_labeled");
    assert_eq!(feature_set.label_column, "confirmed_fwa");
    assert_eq!(
        feature_set.entity_keys,
        vec![
            "claim_id".to_string(),
            "member_id".to_string(),
            "policy_id".to_string(),
            "provider_id".to_string()
        ]
    );
    let feature_names = feature_set
        .feature_columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert!(feature_names.contains(&"claim_amount"));
    assert!(feature_names.contains(&"amount_to_limit_ratio"));
    assert!(!feature_names.contains(&"confirmed_fwa"));
    assert!(!feature_names.contains(&"claim_id"));
    assert_eq!(feature_set.split_summaries.len(), 3);
    assert_eq!(feature_set.split_summaries[0].row_count, 8);
    assert!(feature_set
        .feature_reproducibility_hash
        .starts_with("sha256:"));
    assert_eq!(
        feature_set.feature_reproducibility_hash,
        repeated.feature_reproducibility_hash
    );
    assert!(feature_set
        .governance_boundary
        .contains("does not approve labels"));
    assert!(output_dir.join("feature_set_manifest.json").is_file());
    assert!(output_dir.join("feature_columns.json").is_file());
    assert!(output_dir.join("feature_split_summary.json").is_file());
}

#[test]
fn profiles_parquet_manifest_and_writes_schema_and_profile() {
    let root = temp_root("parquet-profile");
    let train_dir = root.join("split=train");
    let validation_dir = root.join("split=validation");
    fs::create_dir_all(&train_dir).unwrap();
    fs::create_dir_all(&validation_dir).unwrap();
    write_fixture_parquet(&train_dir.join("part-00000.parquet"), &["P1", "P2", "P3"]);
    write_fixture_parquet(
        &validation_dir.join("part-00000.parquet"),
        &["P4", "P5", "P6"],
    );
    let manifest_path = root.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::json!({
            "dataset_key": "renewal_automl_20211105",
            "dataset_version": "v1",
            "business_domain": "renewal_retention",
            "sample_grain": "policy_order",
            "label_column": "m_2_keep_status",
            "entity_keys": ["policy_no", "order_no"],
            "splits": [
                { "split_name": "train", "data_uri": "split=train/" },
                { "split_name": "validation", "data_uri": "split=validation/" }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let output_dir = root.join("out");
    let result = profile_manifest_file(&manifest_path, &output_dir).unwrap();

    assert_eq!(result.profile.row_count_by_split["train"], 3);
    assert_eq!(result.profile.row_count_by_split["validation"], 3);
    assert_eq!(result.profile.label_distribution_by_split["train"]["1"], 2);
    assert_eq!(result.profile.label_distribution_by_split["train"]["0"], 1);
    let policy_field = result
        .schema
        .fields
        .iter()
        .find(|field| field.field_name == "policy_no")
        .unwrap();
    assert_eq!(policy_field.logical_type, "Utf8");
    assert_eq!(policy_field.semantic_role, "key");
    let premium_profile = result
        .profile
        .fields
        .iter()
        .find(|field| field.field_name == "sum_premium")
        .unwrap();
    assert_eq!(premium_profile.missing_count_by_split["train"], 1);
    assert_eq!(result.catalog.storage_format, "parquet");
    assert_eq!(result.catalog.row_count, 6);
    assert_eq!(result.catalog.splits[0].positive_count, Some(2));
    assert!(result.catalog.schema_hash.starts_with("fnv64:"));
    assert!(output_dir.join("schema.json").is_file());
    assert!(output_dir.join("profile.json").is_file());
    assert!(output_dir.join("catalog.json").is_file());
}

#[test]
fn rejects_csv_manifest_split() {
    let manifest = ParquetDatasetManifest {
        source_key: None,
        display_name: None,
        owner: None,
        description: None,
        status: None,
        dataset_key: "bad".into(),
        dataset_version: "v1".into(),
        business_domain: "renewal_retention".into(),
        sample_grain: "policy_order".into(),
        label_column: "m_2_keep_status".into(),
        entity_keys: vec!["policy_no".into()],
        splits: vec![ParquetSplitManifest {
            split_name: "train".into(),
            data_uri: "train.csv".into(),
        }],
    };

    let error = profile_manifest(&manifest, Path::new(".")).unwrap_err();

    assert!(error.to_string().contains("rejects csv"));
}

fn write_fixture_parquet(path: &Path, policy_ids: &[&str]) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("policy_no", DataType::Utf8, false),
        Field::new("order_no", DataType::Utf8, false),
        Field::new("sum_premium", DataType::Float64, true),
        Field::new("m_2_keep_status", DataType::Int8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(policy_ids.to_vec())),
            Arc::new(StringArray::from(vec!["O1", "O2", "O3"])),
            Arc::new(Float64Array::from(vec![Some(100.0), None, Some(300.0)])),
            Arc::new(Int8Array::from(vec![Some(1), Some(0), Some(1)])),
        ],
    )
    .unwrap();
    let file = File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}
