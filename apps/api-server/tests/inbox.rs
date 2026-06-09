use api_server::app::build_app;
use axum::http::StatusCode;

#[path = "inbox/normalization.rs"]
mod normalization;
#[path = "inbox/support.rs"]
mod support;

use support::{json_request, post_inbox, test_config};

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
    assert_eq!(call["actor_role"], "tpa_system");
    assert_eq!(call["customer_scope_id"], "demo-customer");
}

#[tokio::test]
async fn normalizes_medical_record_ocr_artifacts_before_evidence_output() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "medical-text-hygiene-001",
          "reportCase": {
            "reportNo": "SAAS-TEXT-HYGIENE-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "medicalRecordInfoList": [
              {
                "id": 425840019,
                "hospitalName": "南京同仁医院",
                "departmentName": "口腔科",
                "diagnosisName": "牙周炎",
                "medicalType": "门诊",
                "chiefComplaint": "  \uFEFF要求　洁牙  ",
                "medicalRecordInformation": "\uFEFF诊断：牙周炎\r\n\r\n处理措施\r\n全口　显微镜�下行龈下刮治术\r\n西药：\r\n复方氯己定含漱液"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-TEXT-HYGIENE",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-TEXT-HYGIENE",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
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
    let document = &body["canonical_claim_context"]["document_evidence"][0];
    assert_eq!(document["chief_complaint"], "要求 洁牙");
    assert_eq!(document["extracted_diagnosis"], "牙周炎");
    assert_eq!(document["extracted_procedure"], "全口 显微镜下行龈下刮治术");
    let medical_record_text = document["medical_record_text"].as_str().unwrap();
    assert!(!medical_record_text.contains('\u{feff}'));
    assert!(!medical_record_text.contains('\u{fffd}'));
    assert!(!medical_record_text.contains('\u{3000}'));
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
async fn flags_document_invoice_diagnosis_mismatch_on_non_primary_invoice() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-invoice-diagnosis-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-DIAGNOSIS-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "medicalRecordInfoList": [
              {
                "id": 425840010,
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
                "policyNo": "POL-SECONDARY-DIAGNOSIS",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-DIAGNOSIS-OK",
                    "feeAmount": 100.00,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
                      }
                    ],
                    "feeList": []
                  },
                  {
                    "invoiceNo": "INV-DIAGNOSIS-MISMATCH",
                    "feeAmount": 250.00,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [
                      {
                        "detailCode": "S82.900",
                        "detailName": "下肢骨折"
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
            error["field_path"] == "reportCase.policyList[0].invoiceList[1].diagnosisList"
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
async fn flags_medical_record_patient_name_identity_mismatch() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "patient-name-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-PATIENT-MISMATCH-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "accidentPerson": {
              "insuredName": "LEE, Peter",
              "insuredNo": "D209475(0)"
            },
            "medicalRecordInfoList": [
              {
                "id": 425840011,
                "patientName": "王向龙",
                "diagnosisName": "牙周炎",
                "medicalRecordInformation": "诊断：牙周炎"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-PATIENT",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-PATIENT",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "accidentPersonName": "LEE, Peter",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("identity_mismatch")));
}

#[tokio::test]
async fn flags_non_primary_medical_record_patient_identity_mismatch() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-medical-record-patient-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-MEDICAL-PATIENT-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "accidentPerson": {
              "insuredName": "LEE, Peter",
              "insuredNo": "D209475(0)"
            },
            "medicalRecordInfoList": [
              {
                "id": 425840012,
                "patientName": "LEE, Peter",
                "diagnosisName": "牙周炎",
                "medicalRecordInformation": "诊断：牙周炎"
              },
              {
                "id": 425840013,
                "patientName": "王向龙",
                "diagnosisName": "牙周炎",
                "medicalRecordInformation": "诊断：牙周炎"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-SECONDARY-MEDICAL-PATIENT",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-SECONDARY-MEDICAL-PATIENT",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "accidentPersonName": "LEE, Peter",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("identity_mismatch")));
}

#[tokio::test]
async fn flags_non_primary_invoice_person_identity_mismatch() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-invoice-person-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-INVOICE-PERSON-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "accidentPerson": {
              "insuredName": "LEE, Peter",
              "insuredNo": "D209475(0)"
            },
            "medicalRecordInfoList": [
              {
                "id": 425840012,
                "patientName": "LEE, Peter",
                "diagnosisName": "牙周炎",
                "medicalRecordInformation": "诊断：牙周炎"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-SECONDARY-INVOICE-PERSON",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-PERSON-OK",
                    "feeAmount": 100.00,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "accidentPersonName": "LEE, Peter",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
                      }
                    ],
                    "feeList": []
                  },
                  {
                    "invoiceNo": "INV-PERSON-MISMATCH",
                    "feeAmount": 250.00,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "accidentPersonName": "王向龙",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("identity_mismatch")));
}

#[tokio::test]
async fn preserves_all_medical_records_as_document_evidence() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "multiple-medical-records-001",
          "reportCase": {
            "reportNo": "SAAS-MULTI-DOC-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "medicalRecordInfoList": [
              {
                "id": 425840012,
                "hospitalName": "南京同仁医院",
                "departmentName": "口腔科",
                "diagnosisName": "牙周炎",
                "medicalType": "门诊",
                "visitDate": 1766620800000,
                "firstHappenDate": 1703520000000,
                "operationStartDate": 1766678400000,
                "medicalRecordInformation": "诊断：牙周炎"
              },
              {
                "id": 425840013,
                "hospitalName": "南京同仁医院",
                "departmentName": "口腔科",
                "diagnosisName": "龋齿",
                "medicalType": "门诊",
                "visitDate": 1766620800000,
                "medicalRecordInformation": "诊断：龋齿"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-MULTI-DOC",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-MULTI-DOC",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
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
    let documents = body["canonical_claim_context"]["document_evidence"]
        .as_array()
        .expect("document_evidence should be an array");
    assert_eq!(documents.len(), 2);
    let periodontal_document = documents
        .iter()
        .find(|document| document["document_id"] == "425840012")
        .expect("periodontal medical record should be preserved");
    assert_eq!(periodontal_document["visit_date"], "2025-12-25");
    assert_eq!(periodontal_document["first_happen_date"], "2023-12-26");
    assert_eq!(periodontal_document["operation_start_date"], "2025-12-26");
    assert_eq!(
        periodontal_document["source_path"],
        "reportCase.medicalRecordInfoList[0]"
    );
    assert!(documents.iter().any(|document| {
        document["document_id"] == "425840013"
            && document["extracted_diagnosis"] == "龋齿"
            && document["source_path"] == "reportCase.medicalRecordInfoList[1]"
    }));
    assert!(documents.iter().any(|document| {
        document["source_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("medical_record:425840013"))
    }));
}

#[tokio::test]
async fn preserves_bill_lines_from_all_invoices() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "multiple-invoices-001",
          "reportCase": {
            "reportNo": "SAAS-MULTI-INVOICE-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-MULTI-INVOICE",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-MULTI-001",
                    "feeAmount": 100.00,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
                      }
                    ],
                    "feeList": [
                      {
                        "feeCategory": "westernMedicineFee",
                        "medicareAmount": 10.0,
                        "feeDetailList": [
                          {
                            "id": 1001,
                            "name": "复方氯己定含漱液",
                            "amount": 100.00
                          }
                        ]
                      }
                    ]
                  },
                  {
                    "invoiceNo": "INV-MULTI-002",
                    "feeAmount": 250.00,
                    "startDate": 1766620800000,
                    "hospitalCode": "HSP-SECONDARY",
                    "hospitalName": "南京口腔医院",
                    "hospitalClass": "二级",
                    "hospitalProperty": "01",
                    "hospitalCityName": "南京",
                    "hospitalProvinceName": "江苏",
                    "isHospitalInstitution": true,
                    "primaryCare": false,
                    "redFlag": "Y",
                    "diagnosisList": [
                      {
                        "detailCode": "K02.900",
                        "detailName": "龋齿"
                      }
                    ],
                    "feeList": [
                      {
                        "feeCategory": "treatmentFee",
                        "medicareAmount": 25.0,
                        "feeDetailList": [
                          {
                            "id": 2002,
                            "name": "龋齿充填术",
                            "amount": 250.00
                          }
                        ]
                      }
                    ]
                  }
                ]
              },
              {
                "policyNo": "POL-MULTI-INVOICE-SECONDARY",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 5000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-MULTI-003",
                    "feeAmount": 300.00,
                    "startDate": 1766620800000,
                    "hospitalCode": "HSP-SECOND-POLICY",
                    "hospitalName": "南京第二医院",
                    "diagnosisList": [
                      {
                        "detailCode": "J06.900",
                        "detailName": "急性上呼吸道感染"
                      }
                    ],
                    "feeList": [
                      {
                        "feeCategory": "inspectionFee",
                        "medicareAmount": 30.0,
                        "feeDetailList": [
                          {
                            "id": 3003,
                            "name": "血常规",
                            "amount": 300.00
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
        body["canonical_claim_context"]["claim_header"]["total_amount"],
        650.0
    );
    let bill_lines = body["canonical_claim_context"]["itemized_bill_lines"]
        .as_array()
        .expect("itemized_bill_lines should be an array");
    assert_eq!(bill_lines.len(), 3);
    assert!(bill_lines.iter().any(|line| {
        line["invoice_id"] == "INV-MULTI-002"
            && line["item_name"] == "龋齿充填术"
            && line["diagnosis_list"][0]["name"] == "龋齿"
            && line["social_insurance_amount"] == 25.0
            && line["invoice_provider_code"] == "HSP-SECONDARY"
            && line["invoice_provider_name"] == "南京口腔医院"
            && line["invoice_provider_class"] == "二级"
            && line["invoice_provider_type"] == "01"
            && line["invoice_provider_city"] == "南京"
            && line["invoice_provider_province"] == "江苏"
            && line["invoice_is_hospital_institution"] == true
            && line["invoice_primary_care"] == false
            && line["invoice_red_flag"] == "Y"
            && line["source_path"]
                == "reportCase.policyList[0].invoiceList[1].feeList[0].feeDetailList[0]"
    }));
    assert!(bill_lines.iter().any(|line| {
        line["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("invoice:INV-MULTI-002:fee_detail:2002"))
    }));
    assert!(bill_lines.iter().any(|line| {
        line["invoice_id"] == "INV-MULTI-003"
            && line["item_name"] == "血常规"
            && line["diagnosis_list"][0]["name"] == "急性上呼吸道感染"
            && line["invoice_provider_code"] == "HSP-SECOND-POLICY"
            && line["invoice_provider_name"] == "南京第二医院"
            && line["source_path"]
                == "reportCase.policyList[1].invoiceList[0].feeList[0].feeDetailList[0]"
            && line["evidence_refs"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("invoice:INV-MULTI-003:fee_detail:3003"))
    }));
}

#[tokio::test]
async fn flags_bill_lines_without_diagnosis_on_non_primary_invoice() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-invoice-diagnosis-item-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-DIAGNOSIS-ITEM-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-SECONDARY-DIAGNOSIS-ITEM",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-DIAGNOSIS-OK",
                    "feeAmount": 100.00,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [
                      {
                        "detailCode": "K05.300",
                        "detailName": "牙周炎"
                      }
                    ],
                    "feeList": []
                  },
                  {
                    "invoiceNo": "INV-DIAGNOSIS-MISSING",
                    "feeAmount": 250.00,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "diagnosisList": [],
                    "feeList": [
                      {
                        "feeCategory": "treatmentFee",
                        "feeDetailList": [
                          {
                            "id": 2002,
                            "name": "龋齿充填术",
                            "amount": 250.00
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
            error["field_path"] == "reportCase.policyList[0].invoiceList[1].feeList"
                && error["severity"] == "warning"
                && error["remediation"].as_str().unwrap().contains("diagnosis")
        }));
}

#[tokio::test]
async fn preserves_all_product_liability_windows_in_canonical_context() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "product-liability-list-001",
          "reportCase": {
            "reportNo": "SAAS-PRODUCT-LIABILITY-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-PRODUCTS",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "productList": [
                  {
                    "productCode": "YBYL",
                    "productName": "一般医疗保险金",
                    "validateDate": 1735689600000,
                    "expireDate": 1798675200000,
                    "claimLiabilityList": [
                      {
                        "liabCode": "YBYL01",
                        "liabName": "住院医疗费用",
                        "validateDate": 1735689600000,
                        "claimValidateDate": 1740787200000,
                        "expireDate": 1798675200000,
                        "isSeriousDiseaseLiability": "N",
                        "mainLiab": false
                      },
                      {
                        "liabCode": "YBYL02",
                        "liabName": "特定门诊医疗费用",
                        "validateDate": 1735689600000,
                        "claimValidateDate": 1735689600000,
                        "expireDate": 1798675200000,
                        "isSeriousDiseaseLiability": "N",
                        "mainLiab": true
                      }
                    ]
                  },
                  {
                    "productCode": "TDJB",
                    "productName": "特定疾病医疗保险金",
                    "validateDate": 1735689600000,
                    "expireDate": 1798675200000,
                    "claimLiabilityList": [
                      {
                        "liabCode": "TDJB01",
                        "liabName": "特定疾病住院医疗费用",
                        "validateDate": 1735689600000,
                        "claimValidateDate": 1740787200000,
                        "expireDate": 1798675200000,
                        "isSeriousDiseaseLiability": "Y",
                        "mainLiab": false
                      }
                    ]
                  },
                  {
                    "productCode": "ZFXM",
                    "productName": "自费项目补充保险金",
                    "validateDate": 1735689600000,
                    "expireDate": 1798675200000,
                    "claimLiabilityList": []
                  }
                ],
                "invoiceList": [
                  {
                    "invoiceNo": "INV-PRODUCTS",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "feeList": []
                  }
                ]
              },
              {
                "policyNo": "POL-EXTRA-PRODUCTS",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 5000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "productList": [
                  {
                    "productCode": "EJYL",
                    "productName": "二级医疗保险金",
                    "validateDate": 1735689600000,
                    "expireDate": 1798675200000,
                    "claimLiabilityList": [
                      {
                        "liabCode": "EJYL01",
                        "liabName": "二级医疗门诊费用",
                        "validateDate": 1735689600000,
                        "claimValidateDate": 1735689600000,
                        "expireDate": 1798675200000,
                        "isSeriousDiseaseLiability": "N",
                        "mainLiab": false
                      }
                    ]
                  }
                ],
                "invoiceList": []
              }
            ]
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let product_liabilities = body["canonical_claim_context"]["member_policy_snapshot"]
        ["product_liabilities"]
        .as_array()
        .expect("canonical policy snapshot should preserve product/liability windows");
    assert_eq!(product_liabilities.len(), 5);
    assert!(product_liabilities.iter().any(|liability| {
        liability["product_code"] == "YBYL"
            && liability["liability_code"] == "YBYL02"
            && liability["policy_id"] == "POL-PRODUCTS"
            && liability["source_path"] == "reportCase.policyList[0].productList[0]"
            && liability["liability_source_path"]
                == "reportCase.policyList[0].productList[0].claimLiabilityList[1]"
            && liability["source_timezone"] == "Asia/Shanghai"
            && liability["product_start_date_raw_epoch_ms"] == 1735689600000_i64
            && liability["product_end_date_raw_epoch_ms"] == 1798675200000_i64
            && liability["liability_start_date_raw_epoch_ms"] == 1735689600000_i64
            && liability["liability_claim_start_date_raw_epoch_ms"] == 1735689600000_i64
            && liability["liability_end_date_raw_epoch_ms"] == 1798675200000_i64
            && liability["liability_claim_start_date"] == "2025-01-01"
            && liability["waiting_period_end_date"] == "2025-01-01"
            && liability["is_serious_disease_liability"] == false
            && liability["main_liability"] == true
    }));
    assert!(product_liabilities.iter().any(|liability| {
        liability["product_code"] == "TDJB"
            && liability["product_name"] == "特定疾病医疗保险金"
            && liability["liability_code"] == "TDJB01"
            && liability["is_serious_disease_liability"] == true
            && liability["main_liability"] == false
    }));
    assert!(product_liabilities.iter().any(|liability| {
        liability["policy_id"] == "POL-EXTRA-PRODUCTS"
            && liability["product_code"] == "EJYL"
            && liability["product_name"] == "二级医疗保险金"
            && liability["liability_code"] == "EJYL01"
            && liability["liability_name"] == "二级医疗门诊费用"
    }));
    assert!(product_liabilities.iter().any(|liability| {
        liability["policy_id"] == "POL-PRODUCTS"
            && liability["product_code"] == "ZFXM"
            && liability["product_name"] == "自费项目补充保险金"
            && liability["source_path"] == "reportCase.policyList[0].productList[2]"
            && liability["liability_source_path"].is_null()
            && liability["source_timezone"] == "Asia/Shanghai"
            && liability["product_start_date_raw_epoch_ms"] == 1735689600000_i64
            && liability["product_end_date_raw_epoch_ms"] == 1798675200000_i64
            && liability["liability_start_date_raw_epoch_ms"].is_null()
            && liability["liability_claim_start_date_raw_epoch_ms"].is_null()
            && liability["liability_end_date_raw_epoch_ms"].is_null()
            && liability["liability_code"].is_null()
            && liability["liability_name"].is_null()
    }));
}

#[tokio::test]
async fn flags_non_primary_product_liability_window_mismatches() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-window-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-WINDOW-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-SECONDARY-WINDOW",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "productList": [
                  {
                    "productCode": "YBYL",
                    "validateDate": 1735689600000,
                    "expireDate": 1798675200000,
                    "claimLiabilityList": [
                      {
                        "liabCode": "YBYL02",
                        "liabName": "特定门诊医疗费用",
                        "validateDate": 1735689600000,
                        "claimValidateDate": 1735689600000,
                        "expireDate": 1798675200000
                      }
                    ]
                  },
                  {
                    "productCode": "TDJB",
                    "validateDate": 1767225600000,
                    "expireDate": 1798675200000,
                    "claimLiabilityList": [
                      {
                        "liabCode": "TDJB01",
                        "liabName": "特定疾病住院医疗费用",
                        "validateDate": 1767225600000,
                        "claimValidateDate": 1767225600000,
                        "expireDate": 1798675200000
                      }
                    ]
                  }
                ],
                "invoiceList": [
                  {
                    "invoiceNo": "INV-SECONDARY-WINDOW",
                    "feeAmount": 397.06,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "feeList": []
                  }
                ]
              },
              {
                "policyNo": "POL-SECONDARY-WINDOW-EXTRA",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "validateDate": 1767225600000,
                "expireDate": 1798675200000,
                "productList": [
                  {
                    "productCode": "EJYL",
                    "validateDate": 1767225600000,
                    "expireDate": 1798675200000,
                    "claimLiabilityList": [
                      {
                        "liabCode": "EJYL01",
                        "liabName": "二级医疗门诊费用",
                        "validateDate": 1767225600000,
                        "claimValidateDate": 1767225600000,
                        "expireDate": 1798675200000
                      }
                    ]
                  }
                ],
                "invoiceList": []
              }
            ]
          }
        }"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["validation_result"], "accepted_with_warnings");
    assert_eq!(body["scoring_ready"], false);
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("coverage_window_mismatch")));
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_coverage_limit")));
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("policy_liability_mismatch")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[0].productList[1].validateDate"
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
                == "reportCase.policyList[0].productList[1].claimLiabilityList[0].claimValidateDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("claim eligibility date")
        }));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[1].coverageLimit"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("coverage limit")
        }));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[1].validateDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("policy window")
        }));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[1].productList[0].validateDate"
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
                == "reportCase.policyList[1].productList[0].claimLiabilityList[0].claimValidateDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("claim eligibility date")
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
                "insuredWithSI": true,
                "firstApplyTime": 1514764800000,
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
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["policy_first_apply_date"],
        "2018-01-01"
    );
    assert_eq!(
        body["canonical_claim_context"]["member_policy_snapshot"]["insured_with_social_insurance"],
        true
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
async fn flags_accident_date_after_claim_receive_date() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "accident-after-receive-001",
          "reportCase": {
            "reportNo": "SAAS-ACCIDENT-DATE-001",
            "accidentDate": 1767312000000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-ACCIDENT-DATE",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-ACCIDENT-DATE",
                    "feeAmount": 100.00,
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("date_inconsistency")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.accidentDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("claim receive date")
        }));
}

#[tokio::test]
async fn flags_non_primary_invoice_after_claim_receive_date() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-invoice-date-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-INVOICE-DATE-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-PRIMARY-INVOICE-DATE",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-DATE-OK",
                    "feeAmount": 100.00,
                    "startDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "feeList": []
                  }
                ]
              },
              {
                "policyNo": "POL-SECONDARY-INVOICE-DATE",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-DATE-MISMATCH",
                    "feeAmount": 250.00,
                    "startDate": 1767312000000,
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("date_inconsistency")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[1].invoiceList[0].startDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("claim receive date")
        }));
}

#[tokio::test]
async fn flags_non_primary_invoice_end_date_before_start_date() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-invoice-window-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-INVOICE-WINDOW-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767312000000,
            "calculateRisk": "Y",
            "policyList": [
              {
                "policyNo": "POL-SECONDARY-INVOICE-WINDOW",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-WINDOW-OK",
                    "feeAmount": 100.00,
                    "startDate": 1766620800000,
                    "endDate": 1766620800000,
                    "hospitalName": "南京同仁医院",
                    "feeList": []
                  },
                  {
                    "invoiceNo": "INV-WINDOW-MISMATCH",
                    "feeAmount": 250.00,
                    "startDate": 1767225600000,
                    "endDate": 1766620800000,
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("date_inconsistency")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.policyList[0].invoiceList[1].endDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("invoice end date")
        }));
}

#[tokio::test]
async fn flags_non_primary_medical_record_after_claim_receive_date() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-medical-record-date-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-MEDICAL-DATE-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "medicalRecordInfoList": [
              {
                "id": 425840012,
                "patientName": "LEE, Peter",
                "diagnosisName": "牙周炎",
                "visitDate": 1766620800000,
                "medicalRecordInformation": "诊断：牙周炎"
              },
              {
                "id": 425840013,
                "patientName": "LEE, Peter",
                "diagnosisName": "牙周炎",
                "visitDate": 1767312000000,
                "medicalRecordInformation": "诊断：牙周炎"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-SECONDARY-MEDICAL-DATE",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-SECONDARY-MEDICAL-DATE",
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("date_inconsistency")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.medicalRecordInfoList[1].visitDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("claim receive date")
        }));
}

#[tokio::test]
async fn flags_non_primary_medical_record_operation_after_claim_receive_date() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-medical-record-operation-date-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-MEDICAL-OPERATION-DATE-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "medicalRecordInfoList": [
              {
                "id": 425840012,
                "patientName": "LEE, Peter",
                "diagnosisName": "牙周炎",
                "visitDate": 1766620800000,
                "operationStartDate": 1766620800000,
                "medicalRecordInformation": "诊断：牙周炎"
              },
              {
                "id": 425840013,
                "patientName": "LEE, Peter",
                "diagnosisName": "牙周炎",
                "visitDate": 1766620800000,
                "operationStartDate": 1767312000000,
                "medicalRecordInformation": "诊断：牙周炎"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-SECONDARY-MEDICAL-OPERATION-DATE",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-SECONDARY-MEDICAL-OPERATION-DATE",
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("date_inconsistency")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.medicalRecordInfoList[1].operationStartDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("claim receive date")
        }));
}

#[tokio::test]
async fn flags_non_primary_medical_record_first_happen_after_claim_receive_date() {
    let app = build_app(test_config());
    let (status, body) = post_inbox(
        app,
        r#"{
          "systemCode": "AiClaim Core",
          "transNo": "secondary-medical-record-first-happen-date-mismatch-001",
          "reportCase": {
            "reportNo": "SAAS-SECONDARY-MEDICAL-FIRST-HAPPEN-DATE-001",
            "accidentDate": 1766620800000,
            "claimReceiveDate": 1767225600000,
            "calculateRisk": "Y",
            "medicalRecordInfoList": [
              {
                "id": 425840012,
                "patientName": "LEE, Peter",
                "diagnosisName": "牙周炎",
                "visitDate": 1766620800000,
                "firstHappenDate": 1766620800000,
                "medicalRecordInformation": "诊断：牙周炎"
              },
              {
                "id": 425840013,
                "patientName": "LEE, Peter",
                "diagnosisName": "牙周炎",
                "visitDate": 1766620800000,
                "firstHappenDate": 1767312000000,
                "medicalRecordInformation": "诊断：牙周炎"
              }
            ],
            "policyList": [
              {
                "policyNo": "POL-SECONDARY-MEDICAL-FIRST-HAPPEN-DATE",
                "policyType": "2",
                "insuredName": "LEE, Peter",
                "coverageLimit": 20000,
                "validateDate": 1735689600000,
                "expireDate": 1798675200000,
                "invoiceList": [
                  {
                    "invoiceNo": "INV-SECONDARY-MEDICAL-FIRST-HAPPEN-DATE",
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
    assert!(body["data_quality_signals"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("date_inconsistency")));
    assert!(body["validation_errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["field_path"] == "reportCase.medicalRecordInfoList[1].firstHappenDate"
                && error["severity"] == "warning"
                && error["remediation"]
                    .as_str()
                    .unwrap()
                    .contains("claim receive date")
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
