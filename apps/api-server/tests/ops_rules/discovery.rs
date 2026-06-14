use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, public_mvp_parquet_fixture_uri, test_config};

#[tokio::test]
async fn discovers_candidate_rules_from_labeled_samples() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/discover",
        r#"{
          "min_support": 1,
          "samples": [
            {
              "external_claim_id": "CLM-FWA-HIGH-1",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-01-05",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-FWA-HIGH-1",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-FWA-HIGH-2",
              "claim_amount": "8500",
              "currency": "CNY",
              "service_date": "2026-01-06",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-FWA-HIGH-2",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-NORMAL-LATE-LOW",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-NORMAL-LATE-LOW",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-NORMAL-LATE-MEDIUM",
              "claim_amount": "2000",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-NORMAL-LATE-HIGH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            }
          ]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["sample_count"], 4);
    assert_eq!(body["positive_count"], 2);
    let candidate = body["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| {
            candidate["rule"]["conditions"][0]["field"] == "claim_amount_to_limit_ratio"
        })
        .expect("missing amount ratio candidate");
    assert!(candidate["rule"]["rule_id"]
        .as_str()
        .unwrap()
        .starts_with("candidate_mined_claim_amount_to_limit_ratio_gte_"));
    assert_eq!(
        candidate["rule"]["conditions"][0]["field"],
        "claim_amount_to_limit_ratio"
    );
    assert_eq!(candidate["rule"]["conditions"][0]["operator"], ">=");
    assert_eq!(candidate["support"], 2);
    assert_eq!(candidate["precision"], 1.0);
    assert!(candidate["lift"].as_f64().unwrap() > 1.0);
    assert_eq!(candidate["false_positive_rate"], 0.0);
    assert_eq!(candidate["estimated_saving"], "1750.00");
    assert!(candidate["condition_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .starts_with("rule_conditions:candidate_mined_claim_amount_to_limit_ratio_gte_")));
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("单层决策树阈值规则"));
}

#[tokio::test]
async fn discovers_candidate_rules_from_parquet_dataset() {
    let app = build_app(test_config()).unwrap();
    let dataset_uri = public_mvp_parquet_fixture_uri("rule-discovery");
    let payload = r#"{
          "min_support": 2,
          "dataset_uri": "__DATASET_URI__",
          "label_column": "confirmed_fwa",
          "claim_id_column": "claim_id",
          "candidate_feature_fields": ["claim_amount_to_limit_ratio"],
          "samples": []
        }"#
    .replace("__DATASET_URI__", &dataset_uri);

    let (status, body) = json_request(app, "POST", "/api/v1/ops/rules/discover", &payload).await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["sample_count"], 18);
    assert_eq!(body["positive_count"], 13);
    let candidate = &body["candidates"][0];
    assert_eq!(
        candidate["rule"]["conditions"][0]["field"],
        "claim_amount_to_limit_ratio"
    );
    assert!(candidate["rule"]["rule_id"]
        .as_str()
        .unwrap()
        .starts_with("candidate_mined_claim_amount_to_limit_ratio_"));
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("负样本标准差"));
    assert!(candidate["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!("dataset:{dataset_uri}"))));
}

#[tokio::test]
async fn empty_candidate_feature_fields_discovers_all_parquet_features() {
    let app = build_app(test_config()).unwrap();
    let dataset_uri = public_mvp_parquet_fixture_uri("rule-discovery-all-features");
    let payload = r#"{
          "min_support": 2,
          "dataset_uri": "__DATASET_URI__",
          "label_column": "confirmed_fwa",
          "claim_id_column": "claim_id",
          "candidate_feature_fields": [],
          "samples": []
        }"#
    .replace("__DATASET_URI__", &dataset_uri);

    let (status, body) = json_request(app, "POST", "/api/v1/ops/rules/discover", &payload).await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["sample_count"], 18);
    assert_eq!(body["positive_count"], 13);
    assert!(
        !body["candidates"].as_array().unwrap().is_empty(),
        "empty UI feature field selection should not filter every parquet feature"
    );
}

#[tokio::test]
async fn discovers_shallow_tree_rule_candidates_from_labeled_samples() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/discover",
        r#"{
          "min_support": 2,
          "max_tree_depth": 2,
          "candidate_feature_fields": [
            "days_since_policy_start",
            "claim_amount_to_limit_ratio"
          ],
          "samples": [
            {
              "external_claim_id": "CLM-TREE-TP-1",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-01-03",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-TREE-TP-1",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TREE-TP-2",
              "claim_amount": "8500",
              "currency": "CNY",
              "service_date": "2026-01-04",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-TREE-TP-2",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TREE-FP-EARLY-LOW",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-01-04",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-TREE-FP-EARLY-LOW",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TREE-FP-LATE-HIGH",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-02-15",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-TREE-FP-LATE-HIGH",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-TREE-TN",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-02-15",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-TREE-TN",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            }
          ]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let candidate = body["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| {
            candidate["rule"]["rule_id"]
                .as_str()
                .unwrap()
                .starts_with("candidate_tree_")
        })
        .expect("missing shallow tree candidate");
    let conditions = candidate["rule"]["conditions"].as_array().unwrap();
    assert_eq!(conditions.len(), 2);
    assert!(conditions.iter().any(|condition| {
        condition["field"] == "claim_amount_to_limit_ratio" && condition["operator"] == ">="
    }));
    assert!(conditions.iter().any(|condition| {
        condition["field"] == "days_since_policy_start" && condition["operator"] == "<="
    }));
    assert_eq!(candidate["support"], 2);
    assert_eq!(candidate["precision"], 1.0);
    assert_eq!(candidate["false_positive_rate"], 0.0);
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("浅层决策树"));
}

#[tokio::test]
async fn discovers_rule_candidates_from_model_explanations() {
    let app = build_app(test_config()).unwrap();

    let (status, body) = json_request(
        app,
        "POST",
        "/api/v1/ops/rules/discover",
        r#"{
          "min_support": 1,
          "source_model_key": "baseline_fwa",
          "source_model_version": "0.3.0-candidate",
          "feature_importance_uri": "data/eval/baseline_fwa/v3/feature_importance.parquet",
          "model_explanations": [
            {
              "feature": "claim_amount_to_limit_ratio",
              "direction": "increases_risk",
              "contribution": 1.4,
              "reason": "large positive logistic contribution"
            }
          ],
          "samples": [
            {
              "external_claim_id": "CLM-ML-TP",
              "claim_amount": "9000",
              "currency": "CNY",
              "service_date": "2026-01-05",
              "confirmed_fwa": true,
              "policy": {
                "external_policy_id": "POL-ML-TP",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            },
            {
              "external_claim_id": "CLM-ML-TN",
              "claim_amount": "500",
              "currency": "CNY",
              "service_date": "2026-03-01",
              "confirmed_fwa": false,
              "policy": {
                "external_policy_id": "POL-ML-TN",
                "coverage_start_date": "2026-01-01",
                "coverage_end_date": "2026-12-31",
                "coverage_limit": "10000"
              }
            }
          ]
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    let candidates = body["candidates"].as_array().unwrap();
    let candidate = candidates
        .iter()
        .find(|candidate| {
            candidate["rule"]["rule_id"]
                .as_str()
                .unwrap()
                .starts_with("candidate_mined_claim_amount_to_limit_ratio_gte_")
        })
        .expect("missing mined model-supported candidate rule");
    assert_eq!(
        candidate["rule"]["conditions"][0]["field"],
        "claim_amount_to_limit_ratio"
    );
    assert_eq!(candidate["rule"]["conditions"][0]["operator"], ">=");
    assert!(candidate["condition_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference
            .as_str()
            .unwrap()
            .starts_with("rule_conditions:candidate_mined_claim_amount_to_limit_ratio_gte_")));
    assert_eq!(candidate["precision"], 1.0);
    assert!(candidate["explanation"]
        .as_str()
        .unwrap()
        .contains("模型解释备注"));
    assert!(candidate["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "model_versions:baseline_fwa:0.3.0-candidate"
        )));
}
