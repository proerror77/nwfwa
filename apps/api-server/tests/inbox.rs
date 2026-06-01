use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "AiClaim Core".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
    }
}

async fn json_request(
    app: Router,
    method: &str,
    path: &str,
    body: &str,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .header("x-api-key", "dev-secret")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

async fn post_inbox(app: Router, body: &str) -> (StatusCode, serde_json::Value) {
    json_request(app, "POST", "/api/v1/inbox/claims/normalize", body).await
}

#[tokio::test]
async fn normalizes_aiclaim_inbox_payload_with_data_quality_signals() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app.clone(),
        r#"{
          "systemCode": "AiClaim Core",
          "transDate": "2026-05-27 21:22:31",
          "transNo": "f8d0e88391ac4685929d0ca1cb411e7a",
          "reportCase": {
            "reportNo": "SAAS0300040388200349",
            "accidentDate": 1766678400000,
            "claimReceiveDate": 1779811200000,
            "accidentReason": "outpatient",
            "calculateRisk": "N",
            "accidentPerson": {
              "insuredName": "LEE, Peter",
              "insuredNo": "D209475(0)",
              "certNo": "D209475(0)",
              "gender": "M",
              "birthday": 1094313600000
            },
            "medicalRecordInfoList": [
              {
                "id": 425840008,
                "hospitalName": "南京同仁医院",
                "departmentName": "口腔科",
                "diagnosisName": "牙周炎",
                "medicalType": "门诊",
                "visitDate": 1766678400000,
                "patientName": "",
                "medicalRecordInformation": "南京同仁医院/n门急诊病历/n卡号：00002602523/n诊断：牙周炎/n处理措施/n全口显微镜下行龈下刮治术，抛光，双氧水冲洗牙周袋。/n医嘱：/n西药：/n复方氯己定含漱液(300ml)1瓶/n用药天数：1"
              }
            ],
            "policyList": [
              {
                "policyNo": "PNSR039",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "validateDate": 1514822400000,
                "expireDate": 4070966400000,
                "productList": [
                  {
                    "productCode": "YBYL",
                    "productName": "一般医疗保险金",
                    "validateDate": 1735747200000,
                    "expireDate": 1767283200000,
                    "claimLiabilityList": [
                      {
                        "liabCode": "YBYL02",
                        "liabName": "特定门诊医疗费用",
                        "validateDate": 1735747200000,
                        "expireDate": 1767283200000
                      }
                    ]
                  }
                ],
                "invoiceList": [
                  {
                    "invoiceNo": "1111111111",
                    "feeAmount": 397.06,
                    "startDate": 1766678400000,
                    "endDate": 1766678400000,
                    "hospitalCode": "HSP-001",
                    "hospitalName": "南京同仁医院",
                    "hospitalClass": "三级",
                    "hospitalProperty": "02",
                    "hospitalCityName": "南京",
                    "hospitalProvinceName": "江苏",
                    "isHospitalInstitution": true,
                    "primaryCare": true,
                    "redFlag": "N",
                    "medicalType": "门诊",
                    "accidentPersonName": "王向龙",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "慢性牙周炎",
                        "icd": "K05.3",
                        "name": "慢性牙周炎",
                        "primary": true
                      }
                    ],
                    "feeList": [
                      {
                        "feeCategory": "westernMedicineFee",
                        "medicareAmount": 21.55,
                        "feeDetailList": [
                          {
                            "name": "双氯芬酸二乙胺乳胶剂",
                            "amount": 51.51,
                            "selfPayAmount": 5.15,
                            "ownExpenseAmount": 0,
                            "medicalCategory": "1"
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
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["external_message_id"],
        "AiClaim Core:f8d0e88391ac4685929d0ca1cb411e7a:SAAS0300040388200349"
    );
    assert_eq!(body["validation_result"], "accepted_with_warnings");
    assert_eq!(body["scoring_ready"], false);
    assert!(body["run_id"].as_str().unwrap().starts_with("inbox:"));
    assert!(body["audit_id"]
        .as_str()
        .unwrap()
        .starts_with("aud_inbox_sha256_"));
    assert!(body["idempotency_key"]
        .as_str()
        .unwrap()
        .starts_with("inbox.claim.normalize:sha256:"));
    assert!(body["raw_payload_ref"]
        .as_str()
        .unwrap()
        .starts_with("inbox://raw-claims/sha256:"));
    assert!(
        !body["run_id"]
            .as_str()
            .unwrap()
            .contains("f8d0e88391ac4685929d0ca1cb411e7a"),
        "run_id must not expose raw source transaction ids"
    );
    assert!(
        !body["idempotency_key"]
            .as_str()
            .unwrap()
            .contains("SAAS0300040388200349"),
        "idempotency key must not expose raw claim report ids"
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["service_date"],
        "2025-12-25"
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["receive_date"],
        "2026-05-26"
    );
    assert_eq!(
        body["canonical_claim_context"]["provider_snapshot"]["name"],
        "南京同仁医院"
    );
    assert_eq!(
        body["canonical_claim_context"]["provider_snapshot"]["network_flags"]
            ["is_hospital_institution"],
        true
    );
    assert_eq!(
        body["canonical_claim_context"]["provider_snapshot"]["network_flags"]["primary_care"],
        true
    );
    assert_eq!(
        body["canonical_claim_context"]["provider_snapshot"]["network_flags"]["red_flag"],
        "N"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["item_name"],
        "双氯芬酸二乙胺乳胶剂"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["social_insurance_amount"],
        21.55
    );
    assert!(
        body["canonical_claim_context"]["document_evidence"][0]["medical_record_text"]
            .as_str()
            .unwrap()
            .contains("[REDACTED_PHONE]")
    );
    assert!(
        !body["canonical_claim_context"]["document_evidence"][0]["medical_record_text"]
            .as_str()
            .unwrap()
            .contains("/n")
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["extracted_diagnosis"],
        "牙周炎"
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["extracted_procedure"],
        "全口显微镜下行龈下刮治术，抛光，双氧水冲洗牙周袋。"
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["extracted_prescription"],
        "复方氯己定含漱液(300ml)1瓶"
    );
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("identity_mismatch")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[0].coverageLimit"
                && error["severity"] == "warning"
        }));

    let (status, audit_events) = json_request(
        app.clone(),
        "GET",
        "/api/v1/ops/audit-events?event_type=inbox.claim.normalized&claim_id=SAAS0300040388200349&limit=10",
        "{}",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let event = audit_events["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["audit_id"] == body["audit_id"])
        .expect("inbox normalization should write an audit event");
    assert_eq!(event["run_id"], body["run_id"]);
    assert_eq!(event["event_status"], "accepted_with_warnings");
    assert_eq!(event["payload"]["mapping_version"], "aiclaim-core-v1");
    assert!(event["payload"]["external_message_id"].is_null());
    assert!(event["payload"]["external_message_fingerprint"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(event["payload"]["status_code"], 200);
    assert!(
        !event["payload"]
            .to_string()
            .contains("f8d0e88391ac4685929d0ca1cb411e7a"),
        "audit payload must not persist raw source transaction ids"
    );
    assert!(
        !event["payload"].to_string().contains("D209475"),
        "audit payload must not persist raw member identifiers"
    );
    assert!(
        !event["payload"].to_string().contains("王向龙"),
        "audit payload must not persist raw invoice person names"
    );

    let (status, api_calls) =
        json_request(app, "GET", "/api/v1/ops/api-calls?limit=20", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let call = api_calls["calls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["event_type"] == "inbox.claim.normalized")
        .expect("inbox normalization should be visible as an API call");
    assert_eq!(call["endpoint"], "/api/v1/inbox/claims/normalize");
    assert_eq!(call["method"], "POST");
    assert_eq!(call["status_code"], 200);
    assert_eq!(call["result"], "accepted_with_warnings");
    assert_eq!(call["claim_id"], "SAAS0300040388200349");
    assert_eq!(call["audit_id"], body["audit_id"]);
    assert_eq!(call["idempotency_key"], body["idempotency_key"]);
}

#[tokio::test]
async fn rejects_inbox_payload_with_structured_field_errors() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app.clone(),
        r#"{
          "systemCode": "AiClaim Core",
          "reportCase": {
            "reportNo": "SAAS0300040388200349"
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["validation_result"], "rejected");
    assert_eq!(body["validation_errors"][0]["field_path"], "transNo");
    assert_eq!(body["validation_errors"][0]["severity"], "error");
    assert!(body["validation_errors"][0]["remediation"]
        .as_str()
        .unwrap()
        .contains("source transaction id"));
    assert!(body["audit_id"].as_str().unwrap().starts_with("aud_"));

    let (status, api_calls) =
        json_request(app, "GET", "/api/v1/ops/api-calls?limit=20", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let call = api_calls["calls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|call| call["audit_id"] == body["audit_id"])
        .expect("rejected inbox normalization should still be audit-visible");
    assert_eq!(call["endpoint"], "/api/v1/inbox/claims/normalize");
    assert_eq!(call["status_code"], 400);
    assert_eq!(call["result"], "rejected");
}

#[tokio::test]
async fn flags_document_invoice_diagnosis_mismatch() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "diagnosis-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-DIAGNOSIS-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "medicalRecordInfoList": [
              {
                "id": 425840009,
                "hospitalName": "南京同仁医院",
                "departmentName": "口腔科",
                "diagnosisName": "牙周炎",
                "medicalType": "门诊",
                "visitDate": 1766620800000,
                "medicalRecordInformation": "诊断：牙周炎"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-DIAGNOSIS",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-DIAGNOSIS",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [
                      {
                        "detailCode": "S82.900",
                        "detailName": "下肢骨折",
                        "icd": "S82.9",
                        "name": "下肢骨折",
                        "primary": true
                      }
                    ],
                    "feeList": []
                  }
                ]
              }
            ]
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["validation_result"], "accepted_with_warnings");
    assert!(body["scoring_ready"].as_bool().unwrap());
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("document_invoice_mismatch")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[0].invoiceList[0].diagnosisList"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("medical record diagnosis")
        }));
}

#[tokio::test]
async fn flags_bill_lines_without_invoice_diagnosis() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "diagnosis-item-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-DIAGNOSIS-ITEM-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-DIAGNOSIS-ITEM",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-DIAGNOSIS-ITEM",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [],
                    "feeList": [
                      {
                        "feeCategory": "westernMedicineFee",
                        "feeDetailList": [
                          {
                            "name": "双氯芬酸二乙胺乳胶剂",
                            "amount": 51.51
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
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["validation_result"], "accepted_with_warnings");
    assert!(body["scoring_ready"].as_bool().unwrap());
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("diagnosis_item_mismatch")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[0].invoiceList[0].feeList"
                && error["severity"] == "warning"
                && error["remediation"].as_str().unwrap().contains("diagnosis")
        }));
}

#[tokio::test]
async fn flags_service_date_outside_product_and_liability_windows() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "window-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-WINDOW-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-WINDOW",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "productList": [
                  {
                    "productCode": "YBYL",
                    "validateDate": 1767225600000,
                    "expireDate": 1798675200000,
                    "claimLiabilityList": [
                      {
                        "liabCode": "YBYL02",
                        "liabName": "特定门诊医疗费用",
                        "validateDate": 1767225600000,
                        "claimValidateDate": 1767225600000,
                        "expireDate": 1798675200000
                      }
                    ]
                  }
                ],
                "invoiceList": [
                  {
                    "invoiceNo": "INV-WINDOW",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "feeList": []
                  }
                ]
              }
            ]
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["validation_result"], "accepted_with_warnings");
    assert_eq!(body["scoring_ready"], false);
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["coverage_start_date"],
        "2026-01-01"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["liability_start_date"],
        "2026-01-01"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["liability_claim_start_date"],
        "2026-01-01"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["waiting_period_end_date"],
        "2026-01-01"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["coverage_limit"],
        20000.0
    );
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("coverage_window_mismatch")));
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("policy_liability_mismatch")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[0].productList[0].validateDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("product window")
        }));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"]
                == "reportCase.policyList[0].productList[0].claimLiabilityList[0].validateDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("liability window")
        }));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"]
                == "reportCase.policyList[0].productList[0].claimLiabilityList[0].claimValidateDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("claim eligibility date")
        }));
}

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
