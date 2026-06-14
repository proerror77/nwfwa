pub(crate) const API_KEY_DEFAULT: &str = "aiclaim-demo-key";

pub(crate) const API_UNAVAILABLE_MESSAGE: &str =
    "API server is unavailable. Start the API server on 127.0.0.1:8080, then refresh this workspace.";

pub(crate) const SAMPLE_INBOX_PAYLOAD: &str = r#"{
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
        "hospitalName": "Nanjing Tongren Hospital",
        "departmentName": "Dental",
        "diagnosisName": "Periodontitis",
        "medicalType": "outpatient",
        "medicalRecordType": "13",
        "visitDate": 1766678400000,
        "patientName": "",
        "medicalRecordInformation": "periodontal cleaning /n prescription"
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
            "productName": "Medical Benefit",
            "validateDate": 1735747200000,
            "expireDate": 1767283200000,
            "claimLiabilityList": [
              {
                "liabCode": "YBYL02",
                "liabName": "Outpatient Medical",
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
            "hospitalName": "Nanjing Tongren Hospital",
            "hospitalClass": "Level III",
            "hospitalProperty": "02",
            "hospitalCityName": "Nanjing",
            "hospitalProvinceName": "Jiangsu",
            "isHospitalInstitution": true,
            "primaryCare": true,
            "redFlag": "N",
            "medicalType": "outpatient",
            "departmentName": "Dental",
            "claimNature": "1",
            "billType": "socialSecurityBill",
            "documentType": "original",
            "socialInsuranceType": "2",
            "medicareAmount": 133.99,
            "selfPayAmount": 108.82,
            "ownExpenseAmount": 0,
            "otherAmount": 0,
            "accidentPersonName": "Wang",
            "diagnosisList": [
              {
                "detailCode": "K05.300",
                "detailName": "Chronic periodontitis",
                "icd": "K05.3",
                "name": "Chronic periodontitis",
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
                    "name": "Diclofenac diethylamine emulgel",
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
}"#;

pub(crate) const LIVE_TPA_DEMO_PAYLOAD: &str = r#"{
  "systemCode": "AiClaim Core",
  "transNo": "TPA-LIVE-DEMO",
  "reportCase": {
    "reportNo": "CLM-LIVE-DEMO",
    "accidentDate": 1768435200000,
    "claimReceiveDate": 1768867200000,
    "accidentReason": "live demo health claim",
    "calculateRisk": "Y",
    "claimAmount": 18000,
    "accidentPerson": {
      "insuredName": "Demo Member",
      "insuredNo": "MASKED-LIVE-DEMO",
      "certNo": "MASKED-LIVE-DEMO",
      "certType": "demo_id",
      "gender": "U",
      "birthday": 315532800000
    },
    "medicalRecordInfoList": [
      {
        "medicalRecordNo": "MR-LIVE-DEMO",
        "medicalRecordType": "demo_summary",
        "medicalRecordInformation": "Demo medical note. No PHI.",
        "patientName": "Demo Member",
        "visitDate": 1768435200000
      }
    ],
    "policyList": [
      {
        "policyNo": "POL-LIVE-DEMO",
        "insuredName": "Demo Member",
        "coverageLimit": 20000,
        "validateDate": 1767225600000,
        "expireDate": 1798675200000,
        "productList": [
          {
            "productCode": "DEMO-HEALTH",
            "productName": "Demo Health Product",
            "validateDate": 1767225600000,
            "expireDate": 1798675200000,
            "claimLiabilityList": [
              {
                "liabilityCode": "MEDICAL-EXPENSE",
                "liabilityName": "Demo Medical Expense",
                "validateDate": 1767225600000,
                "claimValidateDate": 1767225600000,
                "expireDate": 1798675200000
              }
            ]
          }
        ],
        "invoiceList": [
          {
            "invoiceNo": "INV-LIVE-DEMO",
            "hospitalCode": "PRV-LIVE-DEMO",
            "hospitalName": "Demo Provider",
            "medicalType": "outpatient",
            "startDate": 1768435200000,
            "endDate": 1768435200000,
            "feeAmount": 18000,
            "diagnosisList": [
              {
                "detailCode": "Z00",
                "detailName": "Demo diagnosis"
              }
            ],
            "feeList": [
              {
                "feeCategory": "treatmentFee",
                "medicareAmount": 7200,
                "feeDetailList": [
                  {
                    "detailId": "LINE-LIVE-DEMO",
                    "name": "Inpatient room and board",
                    "amount": 18000
                  }
                ]
              }
            ]
          }
        ]
      }
    ]
  }
}"#;

pub(crate) const LIVE_TPA_DEMO_AMOUNT: &str = "18000.00";

pub(crate) const SAMPLE_RUNTIME_SCORE_REQUEST: &str = r#"{
  "source_system": "AiClaim Core",
  "review_mode": "pre_payment",
  "claim_id": "CLM-0287"
}"#;
