use crate::scorer_artifact::ArtifactModelScorer;
use crate::scorer_heuristic::HeuristicModelScorer;
use crate::scorer_http::HttpModelScorer;
use crate::scorer_manifest::{
    onnx_session_cache_key, positive_probability_from_tensor, ServingManifest,
    ServingManifestModelScorer,
};
use crate::types::{ModelRuntimeError, ModelScoreRequest, ModelScorer};
use crate::verify::to_hex;
use fwa_core::{ClaimId, ScoringRunId};
use fwa_features::FeatureValue;
use hmac::{Hmac, Mac};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[tokio::test]
async fn heuristic_scorer_maps_amount_ratio_to_score() {
    let mut features = BTreeMap::new();
    features.insert(
        "claim_amount_to_limit_ratio".into(),
        FeatureValue {
            name: "claim_amount_to_limit_ratio".into(),
            version: 1,
            value: serde_json::json!(0.82),
            evidence_refs: vec![],
        },
    );

    let scorer = HeuristicModelScorer;
    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_test"),
            claim_id: ClaimId::from_external("CLM-1"),
            model_key: "baseline_fwa".into(),
            model_version: "0.1.0".into(),
            endpoint_url: None,
            features,
        })
        .await
        .unwrap();

    assert_eq!(result.score, 82);
    assert_eq!(result.model_version, "0.1.0");
    assert_eq!(result.runtime_kind, "heuristic");
    assert_eq!(
        result.metadata["fraud_probability"],
        serde_json::json!(0.82)
    );
    assert_eq!(
        result.metadata["abuse_probability"],
        serde_json::json!(0.574)
    );
    assert_eq!(
        result.metadata["waste_probability"],
        serde_json::json!(0.328)
    );
}

#[test]
fn http_scorer_normalizes_base_url() {
    let scorer = HttpModelScorer::new("http://localhost:8001/").unwrap();
    assert_eq!(scorer.base_url, "http://localhost:8001");
}

#[tokio::test]
async fn http_scorer_records_request_latency() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 2048];
        let _ = stream.read(&mut buffer);
        thread::sleep(Duration::from_millis(5));
        let body = r#"{"model_key":"baseline_fwa","model_version":"0.1.0","score":74,"label":"HIGH_RISK","explanations":[],"metadata":{"fraud_probability":0.74}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let scorer = HttpModelScorer::new(format!("http://{address}")).unwrap();
    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_http_latency"),
            claim_id: ClaimId::from_external("CLM-HTTP-LATENCY"),
            model_key: "baseline_fwa".into(),
            model_version: "0.1.0".into(),
            endpoint_url: None,
            features: BTreeMap::new(),
        })
        .await
        .unwrap();

    server.join().unwrap();
    assert_eq!(result.score, 74);
    assert!(result.latency_ms >= 5);
}

#[tokio::test]
async fn http_scorer_rejects_mismatched_model_version() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 2048];
        let _ = stream.read(&mut buffer);
        let body = r#"{"model_key":"baseline_fwa","model_version":"0.2.0","score":74,"label":"HIGH_RISK","explanations":[],"metadata":{}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let scorer = HttpModelScorer::new(format!("http://{address}")).unwrap();
    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_http_mismatch"),
            claim_id: ClaimId::from_external("CLM-HTTP-MISMATCH"),
            model_key: "baseline_fwa".into(),
            model_version: "0.1.0".into(),
            endpoint_url: None,
            features: BTreeMap::new(),
        })
        .await;

    server.join().unwrap();
    assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
}

#[tokio::test]
async fn http_scorer_rejects_out_of_range_score() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 2048];
        let _ = stream.read(&mut buffer);
        let body = r#"{"model_key":"baseline_fwa","model_version":"0.1.0","score":101,"label":"HIGH_RISK","explanations":[],"metadata":{}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let scorer = HttpModelScorer::new(format!("http://{address}")).unwrap();
    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_http_score_range"),
            claim_id: ClaimId::from_external("CLM-HTTP-SCORE-RANGE"),
            model_key: "baseline_fwa".into(),
            model_version: "0.1.0".into(),
            endpoint_url: None,
            features: BTreeMap::new(),
        })
        .await;

    server.join().unwrap();
    assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
}

#[tokio::test]
async fn artifact_scorer_scores_logistic_regression_json() {
    let artifact_path = write_artifact(
        "rust-logistic",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "intercept": -2.0,
            "coefficients": {
                "claim_amount_to_limit_ratio": 4.0,
                "provider_profile_score": 0.01
            }
        }),
    );
    let expected_sha256 = artifact_sha256(&artifact_path);
    let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy());
    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_artifact"),
            claim_id: ClaimId::from_external("CLM-ARTIFACT"),
            model_key: "baseline_fwa".into(),
            model_version: "0.2.0-rust".into(),
            endpoint_url: None,
            features: features([
                ("claim_amount_to_limit_ratio", 0.8),
                ("provider_profile_score", 20.0),
            ]),
        })
        .await
        .unwrap();

    assert_eq!(result.model_key, "baseline_fwa");
    assert_eq!(result.model_version, "0.2.0-rust");
    assert_eq!(result.runtime_kind, "rust_logistic_regression");
    assert_eq!(result.execution_provider, "cpu");
    assert_eq!(result.score, 80);
    assert_eq!(result.label, "HIGH_RISK");
    assert_eq!(
        result.metadata["artifact_sha256"],
        serde_json::json!(expected_sha256)
    );
    assert_eq!(
        result.metadata["artifact_integrity_status"],
        serde_json::json!("passed")
    );
    assert_eq!(
        result.metadata["serving_version_lock_status"],
        serde_json::json!("not_configured")
    );
    assert_eq!(result.metadata["feature_count"], serde_json::json!(2));

    fs::remove_file(artifact_path).unwrap();
}

#[tokio::test]
async fn artifact_scorer_reuses_loaded_artifact_between_scores() {
    let artifact_path = write_artifact(
        "rust-logistic-cached",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "intercept": -2.0,
            "coefficients": {
                "claim_amount_to_limit_ratio": 4.0,
                "provider_profile_score": 0.01
            }
        }),
    );
    let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy());
    let request = ModelScoreRequest {
        run_id: ScoringRunId::from_external("run_artifact_cached"),
        claim_id: ClaimId::from_external("CLM-ARTIFACT-CACHED"),
        model_key: "baseline_fwa".into(),
        model_version: "0.2.0-rust".into(),
        endpoint_url: None,
        features: features([
            ("claim_amount_to_limit_ratio", 0.8),
            ("provider_profile_score", 20.0),
        ]),
    };

    scorer.score(request.clone()).await.unwrap();
    fs::remove_file(artifact_path).unwrap();
    let result = scorer.score(request).await.unwrap();

    assert_eq!(result.model_key, "baseline_fwa");
    assert_eq!(result.model_version, "0.2.0-rust");
}

#[tokio::test]
async fn artifact_scorer_recovers_poisoned_cache_lock() {
    let artifact_path = write_artifact(
        "rust-logistic-poisoned-cache",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": 0.0,
            "coefficients": {
                "claim_amount_to_limit_ratio": 1.0
            }
        }),
    );
    let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy());
    scorer.poison_artifact_cache_for_test();

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_artifact_poisoned_cache"),
            claim_id: ClaimId::from_external("CLM-ARTIFACT-POISONED-CACHE"),
            model_key: "baseline_fwa".into(),
            model_version: "0.2.0-rust".into(),
            endpoint_url: None,
            features: features([("claim_amount_to_limit_ratio", 0.8)]),
        })
        .await
        .unwrap();

    assert_eq!(result.model_key, "baseline_fwa");
    assert_eq!(result.model_version, "0.2.0-rust");
    fs::remove_file(artifact_path).unwrap();
}

#[tokio::test]
async fn artifact_scorer_rejects_checksum_mismatch() {
    let artifact_path = write_artifact(
        "rust-logistic-checksum",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": 0.0,
            "coefficients": {"claim_amount_to_limit_ratio": 1.0}
        }),
    );
    let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy())
        .with_expected_sha256("sha256:wrong");

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_artifact_checksum"),
            claim_id: ClaimId::from_external("CLM-ARTIFACT-CHECKSUM"),
            model_key: "baseline_fwa".into(),
            model_version: "0.2.0-rust".into(),
            endpoint_url: None,
            features: BTreeMap::new(),
        })
        .await;

    assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
    fs::remove_file(artifact_path).unwrap();
}

#[tokio::test]
async fn artifact_scorer_rejects_version_lock_mismatch() {
    let artifact_path = write_artifact(
        "rust-logistic-version",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": 0.0,
            "coefficients": {"claim_amount_to_limit_ratio": 1.0}
        }),
    );
    let scorer =
        ArtifactModelScorer::new(artifact_path.to_string_lossy()).with_version_lock("0.3.0-active");

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_artifact_version"),
            claim_id: ClaimId::from_external("CLM-ARTIFACT-VERSION"),
            model_key: "baseline_fwa".into(),
            model_version: "0.2.0-rust".into(),
            endpoint_url: None,
            features: BTreeMap::new(),
        })
        .await;

    assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
    fs::remove_file(artifact_path).unwrap();
}

#[tokio::test]
async fn artifact_scorer_verifies_hmac_signature() {
    let artifact_path = write_artifact(
        "rust-logistic-signature",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": 0.0,
            "coefficients": {"claim_amount_to_limit_ratio": 1.0}
        }),
    );
    let artifact_sha256 = artifact_sha256(&artifact_path);
    let signature = artifact_signature(
        "baseline_fwa",
        "0.2.0-rust",
        &artifact_sha256,
        "test-signing-key",
    );
    let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy())
        .with_signature(signature, "test-signing-key");

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_artifact_signature"),
            claim_id: ClaimId::from_external("CLM-ARTIFACT-SIGNATURE"),
            model_key: "baseline_fwa".into(),
            model_version: "0.2.0-rust".into(),
            endpoint_url: None,
            features: BTreeMap::new(),
        })
        .await
        .unwrap();

    assert_eq!(
        result.metadata["artifact_signature_status"],
        serde_json::json!("passed")
    );
    fs::remove_file(artifact_path).unwrap();
}

#[tokio::test]
async fn artifact_scorer_rejects_signature_without_key() {
    let artifact_path = write_artifact(
        "rust-logistic-missing-key",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": 0.0,
            "coefficients": {"claim_amount_to_limit_ratio": 1.0}
        }),
    );
    let scorer = ArtifactModelScorer::from_env(
        artifact_path.to_string_lossy(),
        None,
        None,
        Some("hmac-sha256:configured".into()),
        None,
    );

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_artifact_missing_key"),
            claim_id: ClaimId::from_external("CLM-ARTIFACT-MISSING-KEY"),
            model_key: "baseline_fwa".into(),
            model_version: "0.2.0-rust".into(),
            endpoint_url: None,
            features: BTreeMap::new(),
        })
        .await;

    assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
    fs::remove_file(artifact_path).unwrap();
}

#[tokio::test]
async fn serving_manifest_scorer_scores_rust_logistic_artifact() {
    let artifact_path = write_artifact(
        "serving-manifest-rust-logistic-artifact",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-active",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "intercept": -1.5,
            "coefficients": {
                "claim_amount_to_limit_ratio": 3.0,
                "provider_profile_score": 0.02
            }
        }),
    );
    let artifact_sha256 = artifact_sha256(&artifact_path);
    let signature = artifact_signature(
        "baseline_fwa",
        "0.3.0-active",
        &artifact_sha256,
        "test-signing-key",
    );
    let manifest_path = write_artifact(
        "serving-manifest-rust-logistic",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-active",
            "runtime_kind": "rust_logistic_regression",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256,
            "artifact_signature": signature,
            "signature_algorithm": "hmac-sha256",
            "version_lock": "0.3.0-active",
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "threshold": 0.5,
            "training_artifact_uri": "/tmp/model.joblib"
        }),
    );
    let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy())
        .with_signing_key("test-signing-key");

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_serving_manifest"),
            claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST"),
            model_key: "baseline_fwa".into(),
            model_version: "0.3.0-active".into(),
            endpoint_url: None,
            features: features([
                ("claim_amount_to_limit_ratio", 0.82),
                ("provider_profile_score", 18.0),
            ]),
        })
        .await
        .unwrap();

    assert_eq!(result.runtime_kind, "rust_logistic_regression");
    assert_eq!(
        result.metadata["serving_manifest_status"],
        serde_json::json!("passed")
    );
    assert_eq!(
        result.metadata["serving_runtime_kind"],
        serde_json::json!("rust_logistic_regression")
    );
    assert_eq!(
        result.metadata["training_artifact_uri"],
        serde_json::json!("/tmp/model.joblib")
    );
    assert_eq!(
        result.metadata["artifact_signature_status"],
        serde_json::json!("passed")
    );

    fs::remove_file(artifact_path).unwrap();
    fs::remove_file(manifest_path).unwrap();
}

#[tokio::test]
async fn serving_manifest_scorer_rejects_missing_ordered_feature() {
    let artifact_path = write_artifact(
        "serving-manifest-missing-feature-artifact",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-active",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": 0.0,
            "coefficients": {"claim_amount_to_limit_ratio": 1.0}
        }),
    );
    let manifest_path = write_artifact(
        "serving-manifest-missing-feature",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-active",
            "runtime_kind": "rust_logistic_regression",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256(&artifact_path),
            "version_lock": "0.3.0-active",
            "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
            "threshold": 0.5
        }),
    );
    let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy());

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_serving_manifest_missing_feature"),
            claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST-MISSING-FEATURE"),
            model_key: "baseline_fwa".into(),
            model_version: "0.3.0-active".into(),
            endpoint_url: None,
            features: features([("claim_amount_to_limit_ratio", 0.82)]),
        })
        .await;

    assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
    fs::remove_file(artifact_path).unwrap();
    fs::remove_file(manifest_path).unwrap();
}

#[tokio::test]
async fn serving_manifest_scorer_rejects_joblib_xgboost_as_rust_serving() {
    let artifact_path = write_artifact("serving-manifest-xgboost-joblib", serde_json::json!({}));
    let manifest_path = write_artifact(
        "serving-manifest-xgboost-joblib-manifest",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-xgboost",
            "runtime_kind": "xgboost_classifier",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256(&artifact_path),
            "version_lock": "0.3.0-xgboost",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "threshold": 0.5,
            "training_artifact_uri": artifact_path.to_string_lossy()
        }),
    );
    let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy());

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_serving_manifest_joblib"),
            claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST-JOBLIB"),
            model_key: "baseline_fwa".into(),
            model_version: "0.3.0-xgboost".into(),
            endpoint_url: None,
            features: features([("claim_amount_to_limit_ratio", 0.82)]),
        })
        .await;

    let Err(ModelRuntimeError::InvalidResponse(message)) = result else {
        panic!("expected invalid response for xgboost joblib serving manifest");
    };
    assert!(message.contains("not a Rust serving runtime"));
    fs::remove_file(artifact_path).unwrap();
    fs::remove_file(manifest_path).unwrap();
}

#[tokio::test]
async fn serving_manifest_scorer_reaches_onnx_runtime_after_contract_validation() {
    let onnx_path = std::env::temp_dir().join(format!(
        "nwfwa-serving-manifest-onnx-{}.onnx",
        ScoringRunId::new()
    ));
    fs::write(&onnx_path, b"fake onnx bytes for contract validation").unwrap();
    let manifest_path = write_artifact(
        "serving-manifest-xgboost-onnx",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-xgboost-onnx",
            "runtime_kind": "xgboost_onnx",
            "artifact_uri": onnx_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256(&onnx_path),
            "version_lock": "0.3.0-xgboost-onnx",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "threshold": 0.5
        }),
    );
    let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy());

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_serving_manifest_onnx"),
            claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST-ONNX"),
            model_key: "baseline_fwa".into(),
            model_version: "0.3.0-xgboost-onnx".into(),
            endpoint_url: None,
            features: features([("claim_amount_to_limit_ratio", 0.82)]),
        })
        .await;

    let Err(ModelRuntimeError::InvalidResponse(message)) = result else {
        panic!("expected invalid response for fake ONNX model");
    };
    assert!(message.contains("ONNX runtime error"));
    fs::remove_file(onnx_path).unwrap();
    fs::remove_file(manifest_path).unwrap();
}

#[tokio::test]
async fn serving_manifest_accepts_deep_learning_onnx_runtime() {
    let onnx_path = std::env::temp_dir().join(format!(
        "nwfwa-serving-manifest-deep-learning-onnx-{}.onnx",
        ScoringRunId::new()
    ));
    fs::write(
        &onnx_path,
        b"fake deep learning onnx bytes for contract validation",
    )
    .unwrap();
    let manifest_path = write_artifact(
        "serving-manifest-deep-learning-onnx",
        serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.4.0-deep-learning-onnx",
            "runtime_kind": "deep_learning_onnx",
            "artifact_uri": onnx_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256(&onnx_path),
            "version_lock": "0.4.0-deep-learning-onnx",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "threshold": 0.5
        }),
    );
    let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy());

    let result = scorer
        .score(ModelScoreRequest {
            run_id: ScoringRunId::from_external("run_serving_manifest_deep_learning_onnx"),
            claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST-DL-ONNX"),
            model_key: "baseline_fwa".into(),
            model_version: "0.4.0-deep-learning-onnx".into(),
            endpoint_url: None,
            features: features([("claim_amount_to_limit_ratio", 0.82)]),
        })
        .await;

    let Err(ModelRuntimeError::InvalidResponse(message)) = result else {
        panic!("expected invalid response for fake ONNX model");
    };
    assert!(message.contains("ONNX runtime error"));
    fs::remove_file(onnx_path).unwrap();
    fs::remove_file(manifest_path).unwrap();
}

#[test]
fn onnx_session_cache_key_binds_artifact_uri_and_checksum() {
    let base = ServingManifest {
        model_key: "baseline_fwa".into(),
        model_version: "0.3.0-xgboost-onnx".into(),
        runtime_kind: "xgboost_onnx".into(),
        artifact_uri: "/tmp/model-a.onnx".into(),
        artifact_sha256: "sha256:a".into(),
        artifact_signature: None,
        version_lock: "0.3.0-xgboost-onnx".into(),
        feature_columns: vec!["claim_amount_to_limit_ratio".into()],
        threshold: 0.5,
        training_artifact_uri: None,
    };
    let mut changed_checksum = base.clone();
    changed_checksum.artifact_sha256 = "sha256:b".into();
    let mut changed_uri = base.clone();
    changed_uri.artifact_uri = "/tmp/model-b.onnx".into();

    assert_ne!(
        onnx_session_cache_key(&base),
        onnx_session_cache_key(&changed_checksum)
    );
    assert_ne!(
        onnx_session_cache_key(&base),
        onnx_session_cache_key(&changed_uri)
    );
}

#[test]
fn positive_probability_from_tensor_prefers_positive_class_column() {
    assert_eq!(
        positive_probability_from_tensor(&[1, 2], &[0.25, 0.75]),
        Some(0.75)
    );
    let probability = positive_probability_from_tensor(&[3], &[0.61, 0.2, 0.1]).unwrap();
    assert!((probability - 0.61).abs() < 1e-6);
    assert_eq!(positive_probability_from_tensor(&[1, 1], &[0.2]), None);
}

fn features(
    values: impl IntoIterator<Item = (&'static str, f64)>,
) -> BTreeMap<String, FeatureValue> {
    values
        .into_iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                FeatureValue {
                    name: name.to_string(),
                    version: 1,
                    value: serde_json::json!(value),
                    evidence_refs: vec![],
                },
            )
        })
        .collect()
}

fn write_artifact(name: &str, payload: serde_json::Value) -> PathBuf {
    let path = std::env::temp_dir().join(format!("nwfwa-{name}-{}.json", ScoringRunId::new()));
    fs::write(&path, serde_json::to_vec(&payload).unwrap()).unwrap();
    path
}

fn artifact_sha256(path: &PathBuf) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(fs::read(path).unwrap());
    format!("sha256:{digest:x}")
}

fn artifact_signature(
    model_key: &str,
    model_version: &str,
    artifact_sha256: &str,
    signing_key: &str,
) -> String {
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(signing_key.as_bytes()).unwrap();
    mac.update(format!("{model_key}:{model_version}:{artifact_sha256}").as_bytes());
    format!("hmac-sha256:{}", to_hex(&mac.finalize().into_bytes()))
}
