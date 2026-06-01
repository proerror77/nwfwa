use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
    }
}

async fn json_request(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, body)
}

fn renewal_dataset_payload(storage_format: &str) -> String {
    format!(
        r#"{{
          "source_key": "renewal_automl_20211105",
          "display_name": "20211105 Renewal AutoML",
          "business_domain": "renewal_retention",
          "owner": "data-ops",
          "description": "Legacy renewal retention sample normalized to parquet.",
          "dataset_key": "renewal_automl_20211105",
          "dataset_version": "v1",
          "sample_grain": "policy_order",
          "label_column": "m_2_keep_status",
          "entity_keys": ["policy_no", "order_no"],
          "manifest_uri": "data/external/renewal_automl_20211105/v1/manifest.json",
          "schema_uri": "data/external/renewal_automl_20211105/v1/schema.json",
          "profile_uri": "data/external/renewal_automl_20211105/v1/profile.json",
          "storage_format": "{storage_format}",
          "schema_hash": "sha256:test",
          "row_count": 88622,
          "status": "draft",
          "splits": [
            {{
              "split_name": "train",
              "data_uri": "data/external/renewal_automl_20211105/v1/split=train/",
              "row_count": 68664,
              "positive_count": 35837,
              "negative_count": 32827,
              "label_distribution_json": {{"1": 35837, "0": 32827}}
            }},
            {{
              "split_name": "validation",
              "data_uri": "data/external/renewal_automl_20211105/v1/split=validation/",
              "row_count": 19958,
              "positive_count": 9342,
              "negative_count": 10616,
              "label_distribution_json": {{"1": 9342, "0": 10616}}
            }}
          ],
          "fields": [
            {{
              "field_name": "policy_no",
              "logical_type": "string",
              "nullable": false,
              "semantic_role": "key",
              "description": "External policy number stored as string to avoid scientific notation corruption.",
              "profile_json": {{"source_type": "legacy_csv_identifier"}}
            }},
            {{
              "field_name": "order_no",
              "logical_type": "string",
              "nullable": false,
              "semantic_role": "key",
              "description": "External order number stored as string.",
              "profile_json": {{"source_type": "legacy_csv_identifier"}}
            }},
            {{
              "field_name": "m_2_keep_status",
              "logical_type": "int8",
              "nullable": false,
              "semantic_role": "label",
              "description": "M+2 renewal retention label.",
              "profile_json": {{"allowed_values": [0, 1]}}
            }}
          ]
        }}"#
    )
}

#[tokio::test]
async fn registers_and_reads_parquet_dataset_catalog() {
    let app = build_app(test_config());

    let (status, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(created["source_key"], "renewal_automl_20211105");
    assert_eq!(created["business_domain"], "renewal_retention");
    assert_eq!(created["storage_format"], "parquet");
    assert_eq!(created["entity_keys"][0], "policy_no");
    assert_eq!(created["splits"][0]["split_name"], "train");
    assert_eq!(created["fields"][2]["semantic_role"], "label");

    let dataset_id = created["dataset_id"].as_str().unwrap();
    let (status, loaded) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/datasets/{dataset_id}"),
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["dataset_key"], "renewal_automl_20211105");

    let (status, listed) = json_request(app, "GET", "/api/v1/ops/datasets", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["datasets"][0]["dataset_id"], dataset_id);
    assert_eq!(listed["health"][0]["dataset_id"], dataset_id);
    assert_eq!(listed["health"][0]["field_count"], 3);
    assert_eq!(listed["health"][0]["label_count"], 1);
    assert_eq!(listed["health"][0]["entity_key_count"], 2);
}

#[tokio::test]
async fn returns_factor_readiness_summary_from_profiled_fields() {
    let app = build_app(test_config());
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet").replace(
            r#""profile_json": {"allowed_values": [0, 1]}"#,
            r#""profile_json": {"allowed_values": [0, 1], "missing_rate": 0.0}"#,
        )
        .replace(
            r#""profile_json": {"source_type": "legacy_csv_identifier"}"#,
            r#""profile_json": {"source_type": "legacy_csv_identifier", "scheme_family": "provider_outlier", "evidence_refs": ["profiles:renewal_automl_20211105:v1:policy_no"]}"#,
        ),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, _) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "policy_no",
          "canonical_target": "feature.policy_no",
          "feature_name": "policy_no",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, readiness) = json_request(app, "GET", "/api/v1/ops/factors/readiness", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(readiness["dataset_count"], 1);
    assert_eq!(readiness["factor_count"], 3);
    assert_eq!(readiness["label_count"], 1);
    assert_eq!(readiness["entity_key_count"], 2);
    assert_eq!(readiness["data_quality_score"], 0.6666666666666667);
    assert_eq!(readiness["data_quality_status"], "watch");
    assert_eq!(readiness["online_ready_count"], 2);
    assert_eq!(readiness["rule_convertible_count"], 0);
    assert_eq!(readiness["mapped_factor_count"], 1);
    assert_eq!(readiness["high_missing_count"], 0);
    assert_eq!(readiness["unowned_factor_count"], 3);
    assert_eq!(readiness["ready_factor_count"], 0);
    assert_eq!(readiness["review_factor_count"], 3);
    assert_eq!(readiness["readiness_issue_counts"]["missing_owner"], 3);
    assert_eq!(readiness["readiness_issue_counts"]["label_field"], 1);
    assert_eq!(readiness["factor_cards"].as_array().unwrap().len(), 3);
    assert!(readiness["factor_cards"]
        .as_array()
        .unwrap()
        .iter()
        .all(|card| card["scheme_family"]
            .as_str()
            .is_some_and(|value| !value.is_empty())));
    assert_eq!(readiness["factor_cards"][0]["factor_name"], "policy_no");
    assert_eq!(
        readiness["factor_cards"][0]["scheme_family"],
        "provider_peer_outlier"
    );
    assert_eq!(readiness["factor_cards"][0]["chinese_name"], "Policy No");
    assert_eq!(readiness["factor_cards"][0]["entity_type"], "policy_order");
    assert_eq!(
        readiness["factor_cards"][0]["calculation_logic"],
        "registered_dataset_field"
    );
    assert_eq!(
        readiness["factor_cards"][0]["source_table"],
        "renewal_automl_20211105"
    );
    assert_eq!(
        readiness["factor_cards"][0]["source_fields"][0],
        "policy_no"
    );
    assert_eq!(
        readiness["factor_cards"][0]["business_meaning"],
        "External policy number stored as string to avoid scientific notation corruption."
    );
    assert_eq!(readiness["factor_cards"][0]["risk_direction"], "unknown");
    assert_eq!(readiness["factor_cards"][0]["iv"], serde_json::Value::Null);
    assert_eq!(
        readiness["factor_cards"][0]["auc_gain"],
        serde_json::Value::Null
    );
    assert_eq!(
        readiness["factor_cards"][0]["lift"],
        serde_json::Value::Null
    );
    assert_eq!(readiness["factor_cards"][0]["psi"], serde_json::Value::Null);
    assert_eq!(readiness["factor_cards"][0]["stability"], "unmeasured");
    assert_eq!(
        readiness["factor_cards"][0]["model_contribution"],
        serde_json::Value::Null
    );
    assert_eq!(readiness["factor_cards"][0]["rule_convertible"], false);
    assert_eq!(readiness["factor_cards"][0]["online_available"], true);
    assert_eq!(
        readiness["factor_cards"][0]["readiness_status"],
        "needs_review"
    );
    assert!(readiness["factor_cards"][0]["readiness_issues"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_owner")));
    assert_eq!(readiness["factor_cards"][0]["version"], "v1");
    assert_eq!(readiness["factor_cards"][0]["owner"], "");
    assert!(readiness["factor_cards"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "dataset_fields:renewal_automl_20211105:v1:policy_no"
        )));
    assert!(readiness["factor_cards"][0]["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "profiles:renewal_automl_20211105:v1:policy_no"
        )));
}

#[tokio::test]
async fn returns_dataset_health_from_profiled_fields() {
    let app = build_app(test_config());
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet").replace(
            r#""profile_json": {"allowed_values": [0, 1]}"#,
            r#""profile_json": {"allowed_values": [0, 1], "missing_rate": 0.0}"#,
        ),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, listed) = json_request(app, "GET", "/api/v1/ops/datasets", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["health"][0]["dataset_id"], dataset_id);
    assert_eq!(
        listed["health"][0]["dataset_key"],
        "renewal_automl_20211105"
    );
    assert_eq!(listed["health"][0]["dataset_version"], "v1");
    assert_eq!(
        listed["health"][0]["data_quality_score"],
        0.6666666666666667
    );
    assert_eq!(listed["health"][0]["data_quality_status"], "watch");
    assert_eq!(listed["health"][0]["field_count"], 3);
    assert_eq!(listed["health"][0]["label_count"], 1);
    assert_eq!(listed["health"][0]["entity_key_count"], 2);
    assert_eq!(listed["health"][0]["high_missing_count"], 0);
    assert_eq!(listed["health"][0]["unstable_field_count"], 0);
    assert_eq!(listed["health"][0]["unowned_field_count"], 3);
    assert_eq!(listed["health"][0]["online_ready_count"], 2);
    assert_eq!(listed["health"][0]["issue_count"], 3);
}

#[tokio::test]
async fn rejects_non_parquet_dataset_registration() {
    let app = build_app(test_config());

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("csv"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "DATASET_FORMAT_NOT_SUPPORTED");
}

#[tokio::test]
async fn rejects_csv_split_uri_even_when_storage_format_says_parquet() {
    let app = build_app(test_config());
    let payload = renewal_dataset_payload("parquet").replace(
        "data/external/renewal_automl_20211105/v1/split=train/",
        "data/external/renewal_automl_20211105/v1/train.csv",
    );

    let (status, body) = json_request(app, "POST", "/api/v1/ops/datasets", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "DATASET_SPLIT_FORMAT_INVALID");
}

#[tokio::test]
async fn requires_split_row_counts_to_match_dataset_total() {
    let app = build_app(test_config());
    let payload =
        renewal_dataset_payload("parquet").replace("\"row_count\": 88622", "\"row_count\": 1");

    let (status, body) = json_request(app, "POST", "/api/v1/ops/datasets", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "DATASET_ROW_COUNT_MISMATCH");
}

#[tokio::test]
async fn requires_entity_keys_to_be_string_fields() {
    let app = build_app(test_config());
    let payload = renewal_dataset_payload("parquet").replace(
        "\"logical_type\": \"string\"",
        "\"logical_type\": \"float64\"",
    );

    let (status, body) = json_request(app, "POST", "/api/v1/ops/datasets", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "DATASET_ENTITY_KEY_TYPE_INVALID");
}

#[tokio::test]
async fn rejects_pii_in_dataset_factor_metadata() {
    let app = build_app(test_config());

    let payload = renewal_dataset_payload("parquet").replace(
        "External policy number stored as string to avoid scientific notation corruption.",
        "External policy number from alice@example.com.",
    );

    let (status, body) = json_request(app, "POST", "/api/v1/ops/datasets", &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PII_NOT_ALLOWED_IN_DATASET_METADATA");
}

#[tokio::test]
async fn adds_external_field_mapping_to_dataset() {
    let app = build_app(test_config());
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": " ",
          "canonical_target": "feature.sum_premium",
          "feature_name": "sum_premium",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FIELD_MAPPING");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "sum_premium",
          "canonical_target": "feature.sum_premium",
          "feature_name": " ",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FIELD_MAPPING");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "sum_premium",
          "canonical_target": "feature.sum_premium",
          "feature_name": "sum_premium",
          "transform_kind": "script",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FIELD_MAPPING");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "sum_premium",
          "canonical_target": "feature.sum_premium",
          "feature_name": "sum_premium",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "unknown"
        }"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FIELD_MAPPING");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        &format!("/api/v1/ops/datasets/{dataset_id}/mappings"),
        r#"{
          "external_field": "sum_premium",
          "canonical_target": "feature.sum_premium",
          "feature_name": "sum_premium",
          "transform_kind": "direct",
          "transform_json": {},
          "status": "active"
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["mapping"]["dataset_id"], dataset_id);
    assert_eq!(body["mapping"]["external_field"], "sum_premium");
    assert_eq!(body["mapping"]["feature_name"], "sum_premium");

    let (status, audit_events) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?event_type=dataset.field_mapping.added&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        audit_events["events"][0]["payload"]["external_field"],
        "sum_premium"
    );
    assert_eq!(
        audit_events["events"][0]["payload"]["feature_name"],
        "sum_premium"
    );
}

#[tokio::test]
async fn registers_feature_set_model_dataset_and_evaluation_trace() {
    let app = build_app(test_config());
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, feature_set) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age", "sum_premium", "issue_rate"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let feature_set_id = feature_set["feature_set_id"].as_str().unwrap();
    assert_eq!(feature_set["dataset_id"], dataset_id);

    let (status, model_dataset) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-datasets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "task_type": "binary_classification",
              "label_name": "renewal_m2_keep_status",
              "feature_set_id": "{feature_set_id}",
              "train_uri": "data/features/renewal_automl_20211105/v1/split=train/",
              "validation_uri": "data/features/renewal_automl_20211105/v1/split=validation/",
              "test_uri": null,
              "row_counts_json": {{"train": 68664, "validation": 19958}},
              "label_distribution_json": {{"train": {{"1": 35837, "0": 32827}}, "validation": {{"1": 9342, "0": 10616}}}},
              "status": "draft"
            }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let model_dataset_id = model_dataset["model_dataset_id"].as_str().unwrap();
    assert_eq!(model_dataset["feature_set_id"], feature_set_id);

    let (status, evaluation) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &format!(
            r#"{{
              "evaluation_run_id": "eval_renewal_v1",
              "model_key": "renewal_baseline",
              "model_version": "0.1.0",
              "model_dataset_id": "{model_dataset_id}",
              "scheme_family": "diagnosis_procedure_mismatch",
              "auc": "0.81",
              "ks": "0.42",
              "precision": "0.73",
              "recall": "0.68",
              "f1": "0.70",
              "accuracy": "0.74",
              "threshold": "0.50",
              "confusion_matrix_json": {{"tp": 10, "fp": 2, "tn": 12, "fn": 3}},
              "feature_importance_uri": "data/predictions/renewal_automl_20211105/v1/feature_importance.parquet",
              "metrics_json": {{"data_status": "validation"}}
            }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        evaluation["evaluation"]["evaluation_run_id"],
        "eval_renewal_v1"
    );
    assert_eq!(
        evaluation["evaluation"]["model_dataset_id"],
        model_dataset_id
    );
    assert_eq!(
        evaluation["evaluation"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );

    let (status, loaded) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/model-evaluations/eval_renewal_v1",
        "{}",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["evaluation"]["model_key"], "renewal_baseline");
    assert_eq!(loaded["evaluation"]["model_dataset_id"], model_dataset_id);
    assert_eq!(
        loaded["evaluation"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );

    let (status, listed) =
        json_request(app.clone(), "GET", "/api/v1/ops/model-evaluations", "{}").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        listed["evaluations"][0]["evaluation_run_id"],
        "eval_renewal_v1"
    );
    assert_eq!(
        listed["evaluations"][0]["model_dataset_id"],
        model_dataset_id
    );
    assert_eq!(
        listed["evaluations"][0]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert_eq!(listed["lineage"][0]["evaluation_run_id"], "eval_renewal_v1");
    assert_eq!(listed["lineage"][0]["model_key"], "renewal_baseline");
    assert_eq!(listed["lineage"][0]["source_dataset_id"], dataset_id);
    assert_eq!(
        listed["lineage"][0]["source_dataset_key"],
        "renewal_automl_20211105"
    );
    assert_eq!(listed["lineage"][0]["source_dataset_version"], "v1");
    assert_eq!(listed["lineage"][0]["source_data_quality_status"], "watch");

    let (status, audit_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_group=governance&limit=20",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let audit_event_types = audit_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["event_type"].as_str().unwrap())
        .collect::<Vec<_>>();
    for event_type in [
        "dataset.registered",
        "feature_set.registered",
        "model_dataset.registered",
        "model_evaluation.registered",
    ] {
        assert!(
            audit_event_types.contains(&event_type),
            "missing governance audit event {event_type}"
        );
    }
    let model_evaluation_event = audit_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "model_evaluation.registered")
        .unwrap();
    assert_eq!(
        model_evaluation_event["payload"]["evaluation_run_id"],
        "eval_renewal_v1"
    );
    assert_eq!(
        model_evaluation_event["payload"]["scheme_family"],
        "diagnosis_procedure_mismatch"
    );
    assert_eq!(
        model_evaluation_event["evidence_refs"][0],
        "model_evaluations:eval_renewal_v1"
    );

    let (status, dataset_events) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?dataset_id={dataset_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(dataset_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "dataset.registered"
            && event["payload"]["dataset_id"] == dataset_id));

    let (status, feature_set_events) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?feature_set_id={feature_set_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(feature_set_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "feature_set.registered"
            && event["payload"]["feature_set_id"] == feature_set_id));

    let (status, model_dataset_events) = json_request(
        app.clone(),
        "GET",
        &format!("/api/v1/ops/audit-events?model_dataset_id={model_dataset_id}&limit=10"),
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(model_dataset_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "model_dataset.registered"
            && event["payload"]["model_dataset_id"] == model_dataset_id));

    let (status, evaluation_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?evaluation_run_id=eval_renewal_v1&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(evaluation_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"] == "model_evaluation.registered"
            && event["payload"]["evaluation_run_id"] == "eval_renewal_v1"));

    let (status, missing_dataset_events) = json_request(
        app,
        "GET",
        "/api/v1/ops/audit-events?dataset_id=missing-dataset&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(missing_dataset_events["events"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn rejects_csv_feature_matrix_uri() {
    let app = build_app(test_config());
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": " ",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FEATURE_SET");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": [],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FEATURE_SET");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age"],
              "row_count": 0,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FEATURE_SET");

    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "unknown"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_FEATURE_SET");

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/features.csv",
              "feature_list_json": ["member_age"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "FEATURE_SET_FORMAT_INVALID");
}

#[tokio::test]
async fn rejects_invalid_model_dataset_registration() {
    let app = build_app(test_config());
    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/datasets",
        &renewal_dataset_payload("parquet"),
    )
    .await;
    let dataset_id = created["dataset_id"].as_str().unwrap();

    let (status, feature_set) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/feature-sets",
        &format!(
            r#"{{
              "business_domain": "renewal_retention",
              "feature_set_key": "renewal_features",
              "version": "v1",
              "dataset_id": "{dataset_id}",
              "features_uri": "data/features/renewal_automl_20211105/v1/",
              "feature_list_json": ["member_age"],
              "row_count": 88622,
              "label_column": "m_2_keep_status",
              "status": "draft"
            }}"#
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let feature_set_id = feature_set["feature_set_id"].as_str().unwrap();

    let valid_request = serde_json::json!({
        "business_domain": "renewal_retention",
        "task_type": "binary_classification",
        "label_name": "renewal_m2_keep_status",
        "feature_set_id": feature_set_id,
        "train_uri": "data/features/renewal_automl_20211105/v1/split=train/",
        "validation_uri": "data/features/renewal_automl_20211105/v1/split=validation/",
        "test_uri": null,
        "row_counts_json": {"train": 68664, "validation": 19958},
        "label_distribution_json": {
            "train": {"1": 35837, "0": 32827},
            "validation": {"1": 9342, "0": 10616}
        },
        "status": "draft"
    });

    let mut blank_business_domain = valid_request.clone();
    blank_business_domain["business_domain"] = serde_json::json!(" ");
    let payload = blank_business_domain.to_string();
    let (status, body) =
        json_request(app.clone(), "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");

    let mut blank_test_uri = valid_request.clone();
    blank_test_uri["test_uri"] = serde_json::json!(" ");
    let payload = blank_test_uri.to_string();
    let (status, body) =
        json_request(app.clone(), "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");

    let mut empty_row_counts = valid_request.clone();
    empty_row_counts["row_counts_json"] = serde_json::json!({});
    let payload = empty_row_counts.to_string();
    let (status, body) =
        json_request(app.clone(), "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");

    let mut empty_label_distribution = valid_request.clone();
    empty_label_distribution["label_distribution_json"] = serde_json::json!({});
    let payload = empty_label_distribution.to_string();
    let (status, body) =
        json_request(app.clone(), "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");

    let mut invalid_status = valid_request.clone();
    invalid_status["status"] = serde_json::json!("unknown");
    let payload = invalid_status.to_string();
    let (status, body) = json_request(app, "POST", "/api/v1/ops/model-datasets", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_DATASET");
}

#[tokio::test]
async fn rejects_invalid_model_evaluation_registration() {
    let app = build_app(test_config());
    let valid_request = serde_json::json!({
        "evaluation_run_id": "eval_renewal_v1",
        "model_key": "renewal_baseline",
        "model_version": "0.1.0",
        "model_dataset_id": "model_dataset_1",
        "scheme_family": "diagnosis_procedure_mismatch",
        "auc": "0.81",
        "ks": "0.42",
        "precision": "0.73",
        "recall": "0.68",
        "f1": "0.70",
        "accuracy": "0.74",
        "threshold": "0.50",
        "confusion_matrix_json": {"tp": 10, "fp": 2, "tn": 12, "fn": 3},
        "feature_importance_uri": "data/predictions/renewal_automl_20211105/v1/feature_importance.parquet",
        "metrics_json": {"data_status": "validation"}
    });

    let mut blank_evaluation_run_id = valid_request.clone();
    blank_evaluation_run_id["evaluation_run_id"] = serde_json::json!(" ");
    let payload = blank_evaluation_run_id.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut invalid_scheme_family = valid_request.clone();
    invalid_scheme_family["scheme_family"] = serde_json::json!("not_a_scheme");
    let payload = invalid_scheme_family.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut invalid_metric = valid_request.clone();
    invalid_metric["auc"] = serde_json::json!("1.01");
    let payload = invalid_metric.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut empty_confusion_matrix = valid_request.clone();
    empty_confusion_matrix["confusion_matrix_json"] = serde_json::json!({});
    let payload = empty_confusion_matrix.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut empty_metrics = valid_request.clone();
    empty_metrics["metrics_json"] = serde_json::json!({});
    let payload = empty_metrics.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut blank_feature_importance_uri = valid_request.clone();
    blank_feature_importance_uri["feature_importance_uri"] = serde_json::json!(" ");
    let payload = blank_feature_importance_uri.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "INVALID_MODEL_EVALUATION");

    let mut csv_feature_importance_uri = valid_request.clone();
    csv_feature_importance_uri["feature_importance_uri"] =
        serde_json::json!("data/predictions/feature_importance.csv");
    let payload = csv_feature_importance_uri.to_string();
    let (status, body) = json_request(
        app.clone(),
        "POST",
        "/api/v1/ops/model-evaluations",
        &payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "MODEL_EVALUATION_FEATURE_IMPORTANCE_FORMAT_INVALID"
    );

    let mut txt_feature_importance_uri = valid_request.clone();
    txt_feature_importance_uri["feature_importance_uri"] =
        serde_json::json!("data/predictions/feature_importance.txt");
    let payload = txt_feature_importance_uri.to_string();
    let (status, body) = json_request(app, "POST", "/api/v1/ops/model-evaluations", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["code"],
        "MODEL_EVALUATION_FEATURE_IMPORTANCE_FORMAT_INVALID"
    );
}
