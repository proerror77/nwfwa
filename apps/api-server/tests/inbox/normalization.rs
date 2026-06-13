use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, post_inbox, test_config};

#[tokio::test]
async fn normalizes_aiclaim_inbox_payload_with_data_quality_signals() {
    let app = build_app(test_config()).unwrap();
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
              "certType": "I",
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
                "claimNature": "疾病",
                "medicalRecordType": "13",
                "chiefComplaint": "要求洁牙",
                "currentMedicalHistory": "患者定期口腔卫生保健/n现要求洁牙",
                "pastHistory": "否认系统病史，否认药敏史。",
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
                    "departmentName": "口腔科",
                    "claimNature": "1",
                    "billType": "socialSecurityBill",
                    "documentType": "original",
                    "socialInsuranceType": "2",
                    "medicareAmount": 133.99,
                    "selfPayAmount": 108.82,
                    "ownExpenseAmount": 0,
                    "otherAmount": 0,
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
                        "feeAmount": 51.51,
                        "otherAmount": 0,
                        "feeDetailList": [
                          {
                            "name": "双氯芬酸二乙胺乳胶剂",
                            "amount": 51.51,
                            "selfPayAmount": 5.15,
                            "ownExpenseAmount": 0,
                            "medicalCategory": "1",
                            "medicareProrated": "10.00"
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
        "2025-12-26"
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["receive_date"],
        "2026-05-27"
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["accident_date"],
        "2025-12-26"
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["source_timezone"],
        "Asia/Shanghai"
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["service_date_raw_epoch_ms"],
        1766678400000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["receive_date_raw_epoch_ms"],
        1779811200000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["accident_date_raw_epoch_ms"],
        1766678400000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["masked_certificate_id"],
        "***5(0)"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["certificate_type"],
        "I"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["member_gender"],
        "M"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["member_birth_date"],
        "2004-09-05"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["source_timezone"],
        "Asia/Shanghai"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["member_birth_date_raw_epoch_ms"],
        1094313600000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]
            ["policy_first_apply_date_raw_epoch_ms"],
        serde_json::Value::Null
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]
            ["coverage_start_date_raw_epoch_ms"],
        1735747200000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["coverage_end_date_raw_epoch_ms"],
        1767283200000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]
            ["liability_start_date_raw_epoch_ms"],
        1735747200000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]
            ["liability_claim_start_date_raw_epoch_ms"],
        serde_json::Value::Null
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]
            ["liability_end_date_raw_epoch_ms"],
        1767283200000_i64
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
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_bill_type"],
        "socialSecurityBill"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_document_type"],
        "original"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["social_insurance_type"],
        "2"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["department"],
        "口腔科"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["medical_type"],
        "门诊"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_claim_nature"],
        "1"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_start_date"],
        "2025-12-26"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_end_date"],
        "2025-12-26"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["source_timezone"],
        "Asia/Shanghai"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]
            ["invoice_start_date_raw_epoch_ms"],
        1766678400000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_end_date_raw_epoch_ms"],
        1766678400000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]
            ["invoice_social_insurance_amount"],
        133.99
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_self_pay_amount"],
        108.82
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_own_expense_amount"],
        0.0
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_other_amount"],
        0.0
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["fee_group_amount"],
        51.51
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["fee_group_other_amount"],
        0.0
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["medicare_prorated"],
        "10.00"
    );
    assert_eq!(
        body["canonical_claim_context"]["claim_header"]["total_amount"],
        397.06
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["source_timezone"],
        "Asia/Shanghai"
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["visit_date_raw_epoch_ms"],
        1766678400000_i64
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["first_happen_date_raw_epoch_ms"],
        serde_json::Value::Null
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]
            ["operation_start_date_raw_epoch_ms"],
        serde_json::Value::Null
    );
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_claim_amount")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.claimAmount"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("invoice totals")
        }));
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_provider_code"],
        "HSP-001"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_provider_name"],
        "南京同仁医院"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_provider_class"],
        "三级"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_provider_type"],
        "02"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_provider_city"],
        "南京"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_provider_province"],
        "江苏"
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]
            ["invoice_is_hospital_institution"],
        true
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_primary_care"],
        true
    );
    assert_eq!(
        body["canonical_claim_context"]["itemized_bill_lines"][0]["invoice_red_flag"],
        "N"
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
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["claim_nature"],
        "疾病"
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["medical_record_type"],
        "13"
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["chief_complaint"],
        "要求洁牙"
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["current_medical_history"],
        "患者定期口腔卫生保健 现要求洁牙"
    );
    assert_eq!(
        body["canonical_claim_context"]["document_evidence"][0]["past_history"],
        "否认系统病史，否认药敏史。"
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
    assert_eq!(event["payload"]["customer_scope_id"], "demo-customer");
    assert!(event["payload"]["external_message_id"].is_null());
    assert!(event["payload"]["external_message_fingerprint"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    let audit_source_paths = event["payload"]["source_paths"]
        .as_array()
        .expect("audit payload should summarize canonical source paths");
    assert!(audit_source_paths.contains(&serde_json::json!("reportCase.medicalRecordInfoList[0]")));
    assert!(audit_source_paths.contains(&serde_json::json!(
        "reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]"
    )));
    assert!(audit_source_paths.contains(&serde_json::json!(
        "reportCase.policyList[0].productList[0].claimLiabilityList[0]"
    )));
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
    assert_eq!(call["actor_role"], "tpa_system");
    assert_eq!(call["customer_scope_id"], "demo-customer");
    assert_eq!(call["claim_id"], "SAAS0300040388200349");
    assert_eq!(call["audit_id"], body["audit_id"]);
    assert_eq!(call["idempotency_key"], body["idempotency_key"]);
}
