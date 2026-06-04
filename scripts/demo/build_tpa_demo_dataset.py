#!/usr/bin/env python3
import argparse
import json
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any


DATASET_KEY = "tpa_rule_funnel_demo"
DATASET_VERSION = "2026-06-rule-funnel-demo"
SOURCE_SYSTEM = "tpa-demo"


def epoch_ms(value: str) -> int:
    return int(
        datetime.fromisoformat(value)
        .replace(tzinfo=timezone.utc)
        .timestamp()
        * 1000
    )


def money(value: int | float) -> str:
    return f"{value:.2f}"


def direct_payload(
    claim_id: str,
    claim_amount: int,
    coverage_limit: int,
    item_code: str,
    item_type: str,
    description: str,
    *,
    service_date: str = "2026-01-15",
    provider_risk_tier: str = "Low",
    documents: list[dict[str, Any]] | None = None,
    provider_profile: dict[str, Any] | None = None,
    provider_relationships: dict[str, Any] | None = None,
) -> dict[str, Any]:
    payload = {
        "source_system": SOURCE_SYSTEM,
        "claim": {
            "external_claim_id": claim_id,
            "claim_amount": money(claim_amount),
            "currency": "CNY",
            "service_date": service_date,
            "diagnosis_code": "Z00",
        },
        "items": [
            {
                "item_code": item_code,
                "item_type": item_type,
                "description": description,
                "quantity": 1,
                "unit_amount": money(claim_amount),
                "total_amount": money(claim_amount),
            }
        ],
        "member": {
            "external_member_id": f"MBR-{claim_id}",
            "gender": "U",
        },
        "policy": {
            "external_policy_id": f"POL-{claim_id}",
            "product_code": "DEMO-HEALTH",
            "coverage_start_date": "2026-01-01",
            "coverage_end_date": "2026-12-31",
            "coverage_limit": money(coverage_limit),
            "currency": "CNY",
        },
        "provider": {
            "external_provider_id": f"PRV-{claim_id}",
            "name": "Demo Provider",
            "provider_type": "clinic",
            "region": "SH",
            "risk_tier": provider_risk_tier,
        },
    }
    if documents is not None:
        payload["documents"] = documents
    if provider_profile is not None:
        payload["provider_profile"] = provider_profile
    if provider_relationships is not None:
        payload["provider_relationships"] = provider_relationships
    return payload


def inbox_payload(
    claim_id: str,
    claim_amount: int,
    coverage_limit: int | None,
    fee_name: str,
    *,
    service_date: str = "2026-01-15",
    claim_receive_date: str = "2026-01-20",
    include_medical_record: bool = True,
) -> dict[str, Any]:
    policy: dict[str, Any] = {
        "policyNo": f"POL-{claim_id}",
        "insuredName": "Demo Member",
        "validateDate": epoch_ms("2026-01-01"),
        "expireDate": epoch_ms("2026-12-31"),
        "productList": [
            {
                "productCode": "DEMO-HEALTH",
                "productName": "Demo Health Product",
                "validateDate": epoch_ms("2026-01-01"),
                "expireDate": epoch_ms("2026-12-31"),
                "claimLiabilityList": [
                    {
                        "liabilityCode": "MEDICAL-EXPENSE",
                        "liabilityName": "Demo Medical Expense",
                        "claimValidateDate": epoch_ms("2026-01-01"),
                        "validateDate": epoch_ms("2026-01-01"),
                        "expireDate": epoch_ms("2026-12-31"),
                    }
                ],
            }
        ],
        "invoiceList": [
            {
                "invoiceNo": f"INV-{claim_id}",
                "feeAmount": claim_amount,
                "startDate": epoch_ms(service_date),
                "endDate": epoch_ms(service_date),
                "hospitalCode": f"PRV-{claim_id}",
                "hospitalName": "Demo Provider",
                "medicalType": "outpatient",
                "diagnosisList": [{"detailCode": "Z00", "detailName": "Demo diagnosis"}],
                "feeList": [
                    {
                        "feeCategory": "treatmentFee",
                        "medicareAmount": round(claim_amount * 0.4, 2),
                        "feeDetailList": [
                            {
                                "detailId": f"LINE-{claim_id}",
                                "name": fee_name,
                                "amount": claim_amount,
                            }
                        ],
                    }
                ],
            }
        ],
    }
    if coverage_limit is not None:
        policy["coverageLimit"] = coverage_limit

    report_case: dict[str, Any] = {
        "reportNo": claim_id,
        "claimReceiveDate": epoch_ms(claim_receive_date),
        "accidentDate": epoch_ms(service_date),
        "accidentReason": "demo health claim",
        "claimAmount": claim_amount,
        "calculateRisk": "Y",
        "accidentPerson": {
            "insuredNo": f"MASKED-{claim_id}",
            "insuredName": "Demo Member",
            "certNo": f"MASKED-CERT-{claim_id}",
            "certType": "demo_id",
            "gender": "U",
            "birthday": epoch_ms("1980-01-01"),
        },
        "policyList": [policy],
    }
    if include_medical_record:
        report_case["medicalRecordInfoList"] = [
            {
                "medicalRecordNo": f"MR-{claim_id}",
                "patientName": "Demo Member",
                "medicalRecordType": "demo_summary",
                "visitDate": epoch_ms(service_date),
                "medicalRecordInformation": "Demo medical note. No PHI.",
            }
        ]

    return {
        "systemCode": SOURCE_SYSTEM,
        "transNo": f"TPA-DEMO-{claim_id}",
        "reportCase": report_case,
    }


def build_cases() -> list[dict[str, Any]]:
    return [
        {
            "case_id": "straight_through_low_risk",
            "purpose": "完整低风险案件，预期可直接通过或低风险抽样",
            "direct": direct_payload(
                "CLM-DEMO-LOW-001",
                300,
                20_000,
                "CONS-100",
                "consultation",
                "Routine outpatient consultation",
                documents=[],
            ),
            "inbox": inbox_payload("CLM-DEMO-LOW-001", 300, 20_000, "Routine consultation"),
            "expected": {
                "decision_outcome": "straight_through",
                "required_evidence": [],
                "business_assertion": "low risk with complete required baseline fields",
            },
        },
        {
            "case_id": "pending_dental_xray",
            "purpose": "牙科植牙缺 X 光片，规则应进入补件",
            "direct": direct_payload(
                "CLM-DEMO-DENTAL-001",
                8_000,
                20_000,
                "DEN-IMPLANT-01",
                "dental",
                "Dental implant",
                documents=[],
            ),
            "inbox": inbox_payload("CLM-DEMO-DENTAL-001", 8_000, 20_000, "Dental implant"),
            "expected": {
                "decision_outcome": "pending_evidence",
                "required_evidence": ["dental_xray", "medical_record"],
                "business_assertion": "dental treatment needs X-ray and medical-record support",
            },
        },
        {
            "case_id": "pending_prescription_detail",
            "purpose": "药品缺处方/药单明细，规则应进入补件",
            "direct": direct_payload(
                "CLM-DEMO-DRUG-001",
                1_200,
                20_000,
                "DRUG-100",
                "drug",
                "Prescription medication",
                documents=[],
            ),
            "inbox": inbox_payload("CLM-DEMO-DRUG-001", 1_200, 20_000, "Prescription medication"),
            "expected": {
                "decision_outcome": "pending_evidence",
                "required_evidence": ["medication_order", "prescription_detail"],
                "business_assertion": "pharmacy claim needs medication order and prescription detail",
            },
        },
        {
            "case_id": "pending_operation_record",
            "purpose": "手术缺手术记录，规则应进入补件",
            "direct": direct_payload(
                "CLM-DEMO-SURG-001",
                15_000,
                30_000,
                "SURG-100",
                "surgery",
                "Complex operation",
                documents=[],
            ),
            "inbox": inbox_payload("CLM-DEMO-SURG-001", 15_000, 30_000, "Complex operation"),
            "expected": {
                "decision_outcome": "pending_evidence",
                "required_evidence": ["operation_record", "medical_record", "invoice"],
                "business_assertion": "surgery claim needs operation record before adjudication",
            },
        },
        {
            "case_id": "manual_review_provider_pattern",
            "purpose": "Provider / graph risk 高，但不是确定性拒赔，只能人工复核",
            "direct": direct_payload(
                "CLM-DEMO-PROVIDER-001",
                4_000,
                20_000,
                "CONS-200",
                "consultation",
                "Routine outpatient consultation",
                provider_risk_tier="High",
                documents=[],
                provider_profile={
                    "risk_score": 92,
                    "peer_amount_percentile": 99,
                    "evidence_refs": ["provider_profiles:PRV-DEMO-PROVIDER-001"],
                },
                provider_relationships={
                    "graph_risk_score": 88,
                    "high_risk_neighbor_count": 3,
                    "relationship_count": 12,
                    "evidence_refs": ["provider_graph:PRV-DEMO-PROVIDER-001"],
                },
            ),
            "inbox": inbox_payload("CLM-DEMO-PROVIDER-001", 4_000, 20_000, "Routine consultation"),
            "expected": {
                "decision_outcome": "manual_review",
                "required_evidence": [],
                "business_assertion": "provider risk routes review but does not auto-deny",
            },
        },
        {
            "case_id": "manual_review_high_amount",
            "purpose": "高额度/接近保额但资料不缺，进入人工审核而不是补件",
            "direct": direct_payload(
                "CLM-DEMO-HIGH-001",
                18_000,
                20_000,
                "ROOM-100",
                "room",
                "Inpatient room and board",
                documents=[],
            ),
            "inbox": inbox_payload("CLM-DEMO-HIGH-001", 18_000, 20_000, "Inpatient room and board"),
            "expected": {
                "decision_outcome": "manual_review",
                "required_evidence": [],
                "business_assertion": "high amount rules should review, not request unrelated documents",
            },
        },
        {
            "case_id": "inbox_missing_coverage_limit",
            "purpose": "TPA 原始包缺保额，normalize 应 scoring_ready=false",
            "direct": direct_payload(
                "CLM-DEMO-INBOX-BLOCK-001",
                1_000,
                20_000,
                "CONS-300",
                "consultation",
                "Routine outpatient consultation",
                documents=[],
            ),
            "inbox": inbox_payload(
                "CLM-DEMO-INBOX-BLOCK-001",
                1_000,
                None,
                "Routine consultation",
            ),
            "expected": {
                "normalize_scoring_ready": False,
                "validation_field_path": "reportCase.policyList[0].coverageLimit",
                "business_assertion": "basic TPA field blockers are fixed before scoring",
            },
        },
    ]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def build_dataset(output_dir: Path, overwrite: bool) -> dict[str, Any]:
    if output_dir.exists() and any(output_dir.iterdir()) and not overwrite:
        raise FileExistsError(f"{output_dir} is not empty; pass --overwrite")
    output_dir.mkdir(parents=True, exist_ok=True)
    cases = build_cases()
    expected = {}
    manifest_cases = []
    for case in cases:
        case_id = case["case_id"]
        direct_path = output_dir / "direct_scoring_payloads" / f"{case_id}.json"
        inbox_path = output_dir / "inbox_payloads" / f"{case_id}.json"
        write_json(direct_path, case["direct"])
        write_json(inbox_path, case["inbox"])
        expected[case_id] = case["expected"]
        manifest_cases.append(
            {
                "case_id": case_id,
                "purpose": case["purpose"],
                "direct_scoring_payload": str(direct_path.relative_to(output_dir)),
                "inbox_payload": str(inbox_path.relative_to(output_dir)),
                "expected": case["expected"],
            }
        )

    write_json(output_dir / "expected_outcomes.json", expected)
    manifest = {
        "dataset_key": DATASET_KEY,
        "dataset_version": DATASET_VERSION,
        "generated_at": date.today().isoformat(),
        "source_system": SOURCE_SYSTEM,
        "artifact_kind": "tpa_rule_funnel_demo_dataset",
        "label_policy": "business_expected_outcomes_for_demo_validation_not_production_labels",
        "public_data_boundary": {
            "cms_synpuf": "field-shape reference only; synthetic public-use claims data has limited inferential value",
            "kaggle_provider_fraud": "weak provider-level label source only; not claim-level production truth",
        },
        "cases": manifest_cases,
    }
    write_json(output_dir / "manifest.json", manifest)
    write_json(
        output_dir / "source_notes.json",
        {
            "cms_synpuf_url": "https://www.cms.gov/data-research/statistics-trends-and-reports/medicare-claims-synthetic-public-use-files",
            "repo_public_data_builders": [
                "scripts/data/build_public_data_mvp.py",
                "scripts/data/build_kaggle_provider_fraud_mvp.py",
            ],
            "boundary": "Use this dataset to demo TPA normalization, rule-driven pending evidence, review routing, and ML candidate plumbing. Do not use it as production model evidence.",
        },
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default="data/tpa-rule-funnel-demo")
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args()

    manifest = build_dataset(Path(args.output_dir), args.overwrite)
    print(json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
