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
        model_service_url: "http://unused".into(),
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
    assert_eq!(created["fields"][1]["semantic_role"], "label");

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
        app,
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
}
