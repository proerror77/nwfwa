use api_server::app::build_app;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use super::support::test_config;

#[tokio::test]
async fn scores_inbox_canonical_claim_context() {
    let app = build_app(test_config()).unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "canonical_claim_context": {
                "claim_header": {
                  "external_claim_id": "CLM-INBOX-CANONICAL",
                  "total_amount": 8800,
                  "currency": "CNY",
                  "service_date": "2026-01-06"
                },
                "member_policy_snapshot": {
                  "masked_member_id": "masked-member-1",
                  "masked_certificate_id": "masked-cert-1",
                  "member_birth_date": "1988-03-12",
                  "member_gender": "F",
                  "policy_id": "POL-INBOX-CANONICAL",
                  "product_code": "MED",
                  "coverage_start_date": "2026-01-01",
                  "coverage_end_date": "2026-12-31",
                  "coverage_limit": 10000
                },
                "provider_snapshot": {
                  "provider_id": "PRV-INBOX-CANONICAL",
                  "name": "Inbox Hospital",
                  "provider_type": "hospital",
                  "region": "SH",
                  "risk_tier": "High"
                },
                "itemized_bill_lines": [
                  {
                    "item_name": "High cost imaging",
                    "fee_category": "procedure",
                    "amount": 8800,
                    "diagnosis_list": [
                      { "code": "J10", "name": "Influenza" }
                    ],
                    "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                    "evidence_refs": ["invoice:INV-1:fee_detail:LINE-1"]
                  }
                ],
                "document_evidence": [
                  {
                    "document_id": "MR-INBOX-1",
                    "medical_record_type": "outpatient_record",
                    "source_refs": ["medical_record:MR-INBOX-1"]
                  }
                ]
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["claim_id"], "CLM-INBOX-CANONICAL");
    assert_eq!(body["scores"]["final_score"], body["risk_score"]);
    assert!(body["feature_values"]
        .as_array()
        .unwrap()
        .iter()
        .any(|feature| feature["name"] == "claim_amount_to_limit_ratio"
            && feature["value"] == serde_json::json!(0.88)));
    assert!(body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("invoice:INV-1:fee_detail:LINE-1")));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-INBOX-CANONICAL")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.clone().oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    assert_eq!(scoring_event["payload"]["claim_id"], "CLM-INBOX-CANONICAL");
    assert!(scoring_event["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("invoice:INV-1:fee_detail:LINE-1")));
    assert!(
        scoring_event["payload"]["canonical_claim_context_trace"]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-1:fee_detail:LINE-1"))
    );
    assert!(
        scoring_event["payload"]["canonical_claim_context_trace"]["source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]"
            ))
    );
    assert!(
        scoring_event["payload"]["canonical_claim_context_trace"]["source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("medical_record:MR-INBOX-1"))
    );
}

#[tokio::test]
async fn canonical_claim_context_audits_defaulted_field_warnings() {
    let app = build_app(test_config()).unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "canonical_claim_context": {
                "claim_header": {
                  "external_claim_id": "CLM-CANONICAL-WARNINGS",
                  "total_amount": 3200,
                  "currency": "CNY",
                  "service_date": "2026-01-06"
                },
                "member_policy_snapshot": {
                  "member_birth_date": "1988-03-12",
                  "member_gender": "F",
                  "product_code": "MED",
                  "coverage_start_date": "2026-01-01",
                  "coverage_end_date": "2026-12-31",
                  "coverage_limit": 10000
                },
                "provider_snapshot": {},
                "itemized_bill_lines": [
                  {
                    "item_name": "Outpatient medicine",
                    "fee_category": "pharmacy",
                    "amount": 3200,
                    "source_path": "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]",
                    "evidence_refs": ["invoice:INV-WARN:fee_detail:LINE-1"]
                  }
                ],
                "document_evidence": []
              }
            }"#,
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-CANONICAL-WARNINGS")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    let warnings = scoring_event["payload"]["canonical_claim_context_trace"]
        ["data_quality_warnings"]
        .as_array()
        .expect("canonical trace should include data quality warnings");
    for field_path in [
        "claim_header.diagnosis_code",
        "member_policy_snapshot.masked_member_id",
        "member_policy_snapshot.policy_id",
        "provider_snapshot.provider_id",
        "provider_snapshot.name",
        "provider_snapshot.provider_type",
        "provider_snapshot.region",
    ] {
        assert!(
            warnings
                .iter()
                .any(|warning| warning["field_path"] == field_path
                    && warning["severity"] == "warning"),
            "missing warning for {field_path}"
        );
    }
}

#[tokio::test]
async fn scores_scoring_ready_inbox_run_handoff() {
    let app = build_app(test_config()).unwrap();

    let normalize_request = Request::builder()
        .method("POST")
        .uri("/api/v1/inbox/claims/normalize")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "systemCode": "tpa-demo",
              "transNo": "score-handoff-001",
              "reportCase": {
                "reportNo": "CLM-INBOX-HANDOFF",
                "accidentDate": 1767225600000,
                "claimReceiveDate": 1767312000000,
                "calculateRisk": "Y",
                "medicalRecordInfoList": [
                  {
                    "id": 88001,
                    "hospitalName": "Inbox Hospital",
                    "departmentName": "内科",
                    "diagnosisName": "流感",
                    "medicalType": "门诊",
                    "visitDate": 1767225600000,
                    "medicalRecordInformation": "诊断：流感"
                  }
                ],
                "policyList": [
                  {
                    "policyNo": "POL-INBOX-HANDOFF",
                    "insuredName": "LEE, Peter",
                    "coverageLimit": 10000,
                    "validateDate": 1735689600000,
                    "expireDate": 1798675200000,
                    "invoiceList": [
                      {
                        "invoiceNo": "INV-INBOX-HANDOFF",
                        "feeAmount": 8800,
                        "startDate": 1767225600000,
                        "hospitalCode": "PRV-INBOX-HANDOFF",
                        "hospitalName": "Inbox Hospital",
                        "diagnosisList": [
                          {
                            "detailCode": "J10",
                            "detailName": "流感",
                            "primary": true
                          }
                        ],
                        "feeList": [
                          {
                            "feeCategory": "inspectionFee",
                            "feeDetailList": [
                              {
                                "id": 88001,
                                "name": "High cost imaging",
                                "amount": 8800,
                                "medicalCategory": "procedure"
                              }
                            ]
                          }
                        ]
                      }
                    ]
                  }
                ]
              }
            }"#,
        ))
        .unwrap();
    let normalize_response = app.clone().oneshot(normalize_request).await.unwrap();
    assert_eq!(normalize_response.status(), StatusCode::OK);
    let normalize_body = to_bytes(normalize_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let normalize_body: serde_json::Value = serde_json::from_slice(&normalize_body).unwrap();
    assert_eq!(normalize_body["scoring_ready"], true);

    let score_request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(format!(
            r#"{{
              "source_system": "tpa-demo",
              "inbox_run_id": {}
            }}"#,
            serde_json::to_string(&normalize_body["run_id"]).unwrap()
        )))
        .unwrap();
    let score_response = app.clone().oneshot(score_request).await.unwrap();
    assert_eq!(score_response.status(), StatusCode::OK);
    let score_body = to_bytes(score_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let score_body: serde_json::Value = serde_json::from_slice(&score_body).unwrap();

    assert_eq!(score_body["claim_id"], "CLM-INBOX-HANDOFF");
    assert!(score_body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!(
            "inbox_claim_runs:{}",
            normalize_body["run_id"].as_str().unwrap()
        ))));
    assert!(score_body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(format!(
            "audit_events:{}",
            normalize_body["audit_id"].as_str().unwrap()
        ))));
    assert!(score_body["evidence_refs"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "invoice:INV-INBOX-HANDOFF:fee_detail:88001"
        )));

    let audit_request = Request::builder()
        .method("GET")
        .uri("/api/v1/audit/claims/CLM-INBOX-HANDOFF")
        .header("x-api-key", "dev-secret")
        .body(Body::empty())
        .unwrap();
    let audit_response = app.oneshot(audit_request).await.unwrap();
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audit_body = to_bytes(audit_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let audit_body: serde_json::Value = serde_json::from_slice(&audit_body).unwrap();
    let scoring_event = audit_body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event_type"] == "scoring.completed")
        .expect("audit history should include scoring.completed");
    let trace = &scoring_event["payload"]["canonical_claim_context_trace"];
    assert_eq!(trace["input_mode"], "inbox_run");
    assert_eq!(trace["inbox_run_id"], normalize_body["run_id"]);
    assert_eq!(trace["inbox_audit_id"], normalize_body["audit_id"]);
    assert_eq!(
        trace["inbox_idempotency_key"],
        normalize_body["idempotency_key"]
    );
    assert_eq!(
        trace["raw_payload_checksum"],
        normalize_body["raw_payload_checksum"]
    );
}
