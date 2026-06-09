use api_server::app::build_app;
use axum::http::StatusCode;

#[path = "inbox/coverage_windows.rs"]
mod coverage_windows;
#[path = "inbox/document_quality.rs"]
mod document_quality;
#[path = "inbox/normalization.rs"]
mod normalization;
#[path = "inbox/support.rs"]
mod support;

use support::{json_request, post_inbox, test_config};

#[tokio::test]
async fn repeated_inbox_payload_upserts_same_audit_trace() {
    let app = build_app(test_config());
    let payload = r#"{
      "systemCode": "AiClaim Core",
      "transNo": "duplicate-message-001",
      "reportCase": {
        "reportNo": "SAAS-DUPLICATE-001",
        "claimReceiveDate": 1779811200000,
        "calculateRisk": "Y",
        "policyList": [
          {
            "policyNo": "POL-DUP",
            "insuredName": "LEE, Peter",
            "invoiceList": [
              {
                "invoiceNo": "INV-DUP",
                "feeAmount": 100.00,
                "startDate": 1766678400000,
                "hospitalName": "南京同仁医院",
                "feeList": []
              }
            ]
          }
        ]
      }
    }"#;

    let (first_status, first) = post_inbox(app.clone(), payload).await;
    let (second_status, second) = post_inbox(app.clone(), payload).await;

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(first["external_message_id"], second["external_message_id"]);
    assert_eq!(first["idempotency_key"], second["idempotency_key"]);
    assert_eq!(first["run_id"], second["run_id"]);
    assert_eq!(first["audit_id"], second["audit_id"]);
    assert_eq!(
        first["raw_payload_checksum"],
        second["raw_payload_checksum"]
    );

    let (status, audit_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_type=inbox.claim.normalized&claim_id=SAAS-DUPLICATE-001&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let matching_events = audit_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["audit_id"] == first["audit_id"])
        .count();
    assert_eq!(matching_events, 1);

    let (status, api_calls) =
        json_request(app, "GET", "/api/v1/ops/api-calls?limit=20", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let matching_calls = api_calls["calls"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|call| call["audit_id"] == first["audit_id"])
        .count();
    assert_eq!(matching_calls, 1);
}

#[tokio::test]
async fn rejects_same_inbox_idempotency_key_with_different_payload_checksum() {
    let app = build_app(test_config());
    let payload = r#"{
      "systemCode": "AiClaim Core",
      "transNo": "duplicate-message-conflict-001",
      "reportCase": {
        "reportNo": "SAAS-DUPLICATE-CONFLICT-001",
        "claimReceiveDate": 1779811200000,
        "calculateRisk": "Y",
        "policyList": [
          {
            "policyNo": "POL-DUP-CONFLICT",
            "insuredName": "LEE, Peter",
            "coverageLimit": 20000,
            "validateDate": 1735689600000,
            "expireDate": 1798675200000,
            "invoiceList": [
              {
                "invoiceNo": "INV-DUP-CONFLICT",
                "feeAmount": 100.00,
                "startDate": 1766678400000,
                "hospitalName": "南京同仁医院",
                "feeList": []
              }
            ]
          }
        ]
      }
    }"#;
    let changed_payload = payload.replace("\"feeAmount\": 100.00", "\"feeAmount\": 101.00");

    let (first_status, first) = post_inbox(app.clone(), payload).await;
    let (second_status, second) = post_inbox(app, &changed_payload).await;

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::CONFLICT);
    assert!(first["raw_payload_checksum"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(second["code"], "INBOX_IDEMPOTENCY_CONFLICT");
}
