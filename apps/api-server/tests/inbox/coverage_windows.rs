use api_server::app::build_app;
use axum::http::StatusCode;

use super::support::{post_inbox, test_config};

#[tokio::test]
async fn preserves_all_product_liability_windows_in_canonical_context() {
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
    let app = build_app(test_config()).unwrap();
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
