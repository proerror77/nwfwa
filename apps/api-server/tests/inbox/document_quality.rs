use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{json_request, post_inbox, test_config};

#[tokio::test]
async fn rejects_inbox_payload_with_structured_field_errors() {
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
