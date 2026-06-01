use api_server::{app::build_app, config::AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
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

async fn post_inbox(body: &str) -> (StatusCode, serde_json::Value) {
    let app = build_app(test_config());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/inbox/claims/normalize")
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

#[tokio::test]
async fn normalizes_aiclaim_inbox_payload_with_data_quality_signals() {
    let (status, body) = post_inbox(
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
                "medicalRecordInformation": "南京同仁医院/n门急诊病历/n卡号：00002602523/n诊断：牙周炎"
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
                    "hospitalCityName": "南京",
                    "hospitalProvinceName": "江苏",
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
        body["canonical_claim_context"]["itemized_bill_lines"][0]["item_name"],
        "双氯芬酸二乙胺乳胶剂"
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
}

#[tokio::test]
async fn rejects_inbox_payload_with_structured_field_errors() {
    let (status, body) = post_inbox(
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
}
