#!/usr/bin/env python3
"""Build a Kaggle provider-fraud demo dataset and TPA inbox payloads.

The Kaggle label is provider-level PotentialFraud. This script maps it to a
weak claim-level pipeline label only so the demo training and scoring loop can
run. It is not customer production evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any
from zipfile import ZipFile


LABEL_POLICY = "weak_provider_level_label_not_claim_level_production_evidence"
DEFAULT_ARCHIVE = Path("/Users/proerror/Downloads/archive.zip")

TRAIN_LABELS = "Train-1542865627584.csv"
TRAIN_BENEFICIARY = "Train_Beneficiarydata-1542865627584.csv"
TRAIN_INPATIENT = "Train_Inpatientdata-1542865627584.csv"
TRAIN_OUTPATIENT = "Train_Outpatientdata-1542865627584.csv"
REQUIRED_ARCHIVE_MEMBERS = [
    TRAIN_LABELS,
    TRAIN_BENEFICIARY,
    TRAIN_INPATIENT,
    TRAIN_OUTPATIENT,
]


def main() -> int:
    args = parse_args()
    pd = require_pandas()
    archive_path = Path(args.archive)
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    with ZipFile(archive_path) as archive:
        validate_archive(archive)
        claims = build_claim_frame(
            pd=pd,
            archive=archive,
            max_claims=args.max_claims,
        )

    write_dataset(claims, output_dir, args.dataset_version)
    write_tpa_payloads(
        claims=claims,
        output_dir=output_dir,
        dataset_version=args.dataset_version,
        max_payloads=args.max_tpa_payloads,
        source_system=args.source_system,
    )
    write_sources(output_dir, archive_path, args.dataset_version, len(claims))

    payload = {
        "dataset_key": "kaggle_provider_fraud_claims",
        "dataset_version": args.dataset_version,
        "label_policy": LABEL_POLICY,
        "manifest_uri": str(output_dir / "manifest.json"),
        "tpa_payloads_uri": str(output_dir / "tpa_claims.jsonl"),
        "claim_count": len(claims),
    }
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Materialize Kaggle provider-fraud MVP Parquet and TPA JSON payloads."
    )
    parser.add_argument("--archive", default=str(DEFAULT_ARCHIVE))
    parser.add_argument("--output-dir", default="data/kaggle-provider-fraud")
    parser.add_argument("--dataset-version", default=date.today().isoformat())
    parser.add_argument(
        "--max-claims",
        type=int,
        default=5000,
        help="Deterministic claim cap for local demo size. Use 0 for all train claims.",
    )
    parser.add_argument(
        "--max-tpa-payloads",
        type=int,
        default=100,
        help="Number of TPA inbox JSON payloads to write. Use 0 to skip JSONL output.",
    )
    parser.add_argument("--source-system", default="AiClaim Core")
    return parser.parse_args()


def require_pandas() -> Any:
    try:
        import pandas as pd
    except ImportError as error:
        raise SystemExit(
            "pandas and pyarrow are required; run through apps/ml-service dev environment"
        ) from error
    return pd


def validate_archive(archive: ZipFile) -> None:
    names = set(archive.namelist())
    missing = [name for name in REQUIRED_ARCHIVE_MEMBERS if name not in names]
    if missing:
        raise SystemExit(f"Kaggle archive missing required files: {', '.join(missing)}")


def build_claim_frame(pd: Any, archive: ZipFile, max_claims: int) -> Any:
    provider_labels = read_csv(pd, archive, TRAIN_LABELS)
    beneficiary = read_csv(pd, archive, TRAIN_BENEFICIARY)
    inpatient = read_csv(pd, archive, TRAIN_INPATIENT)
    outpatient = read_csv(pd, archive, TRAIN_OUTPATIENT)

    provider_labels["confirmed_fwa"] = (
        provider_labels["PotentialFraud"].astype(str).str.lower().eq("yes").astype(int)
    )
    provider_labels = provider_labels[["Provider", "PotentialFraud", "confirmed_fwa"]]

    inpatient["claim_type"] = "inpatient"
    outpatient["claim_type"] = "outpatient"
    claims = pd.concat([inpatient, outpatient], ignore_index=True, sort=False)
    claims = claims.merge(provider_labels, on="Provider", how="inner")
    claims = claims.merge(beneficiary, on="BeneID", how="left")

    claims["ClaimStartDt"] = pd.to_datetime(claims["ClaimStartDt"], errors="coerce")
    claims = claims.dropna(subset=["ClaimID", "BeneID", "Provider", "ClaimStartDt"])
    claims = claims.sort_values(["ClaimStartDt", "ClaimID"]).reset_index(drop=True)
    if max_claims > 0:
        claims = balanced_head(claims, max_claims)

    feature_frame = pd.DataFrame(
        {
            "claim_id": claims["ClaimID"].astype(str),
            "member_id": claims["BeneID"].astype(str),
            "policy_id": claims["BeneID"].astype(str).map(mask_policy_id),
            "provider_id": claims["Provider"].astype(str),
            "service_date": claims["ClaimStartDt"].dt.date.astype(str),
            "service_date_ord": claims["ClaimStartDt"].map(lambda value: value.date().toordinal()),
            "claim_amount_to_limit_ratio": (
                numeric_series(claims, "InscClaimAmtReimbursed") / 25_000.0
            ).clip(lower=0, upper=2).round(6),
            "provider_profile_score": provider_profile_score(claims).round(6),
            "high_cost_item_ratio": (
                numeric_series(claims, "InscClaimAmtReimbursed") / 10_000.0
            ).clip(lower=0, upper=2).round(6),
            "provider_peer_payment_zscore": provider_peer_zscore(claims).round(6),
            "leie_excluded_provider": 0,
            "beneficiary_age": beneficiary_age(claims).round(6),
            "chronic_condition_count": chronic_condition_count(claims),
            "diagnosis_code_count": diagnosis_code_count(claims),
            "procedure_code_count": procedure_code_count(claims),
            "deductible_paid": numeric_series(claims, "DeductibleAmtPaid").round(6),
            "claim_type_inpatient": claims["claim_type"].eq("inpatient").astype(int),
            "confirmed_fwa": claims["confirmed_fwa"].astype(int),
            "claim_type": claims["claim_type"].astype(str),
            "provider_fraud_label": claims["PotentialFraud"].astype(str),
        }
    )
    return feature_frame.reset_index(drop=True)


def read_csv(pd: Any, archive: ZipFile, name: str) -> Any:
    with archive.open(name) as handle:
        return pd.read_csv(handle, low_memory=False)


def balanced_head(frame: Any, max_rows: int) -> Any:
    if len(frame) <= max_rows:
        return frame.copy()
    fraud = frame[frame["confirmed_fwa"] == 1]
    non_fraud = frame[frame["confirmed_fwa"] == 0]
    fraud_target = min(len(fraud), max_rows // 2)
    non_fraud_target = max_rows - fraud_target
    selected = [fraud.head(fraud_target), non_fraud.head(non_fraud_target)]
    return frame.iloc[sorted(pd_index for part in selected for pd_index in part.index)].copy()


def numeric_series(frame: Any, column: str) -> Any:
    return frame[column].replace("NA", 0).fillna(0).astype(float)


def provider_profile_score(claims: Any) -> Any:
    provider_amount = numeric_series(claims, "InscClaimAmtReimbursed").groupby(
        claims["Provider"]
    )
    average_amount = provider_amount.transform("mean")
    claim_count = claims.groupby("Provider")["ClaimID"].transform("count")
    return (average_amount / 500.0 + claim_count / 50.0).clip(lower=0, upper=100)


def provider_peer_zscore(claims: Any) -> Any:
    amount = numeric_series(claims, "InscClaimAmtReimbursed")
    grouped = amount.groupby(claims["Provider"])
    provider_average = grouped.transform("mean")
    global_std = max(float(amount.std()), 1.0)
    return (provider_average - float(amount.mean())) / global_std


def beneficiary_age(claims: Any) -> Any:
    dob = claims["DOB"].pipe(lambda series: series.replace("NA", None))
    dob = dob.map(parse_year)
    service_year = claims["ClaimStartDt"].dt.year
    return (service_year - dob).fillna(0).clip(lower=0, upper=120)


def parse_year(value: object) -> float | None:
    if value is None:
        return None
    try:
        return float(str(value)[:4])
    except ValueError:
        return None


def chronic_condition_count(claims: Any) -> Any:
    columns = [column for column in claims.columns if column.startswith("ChronicCond_")]
    if not columns:
        return 0
    return claims[columns].apply(
        lambda row: sum(1 for value in row if str(value).strip() == "1"),
        axis=1,
    )


def diagnosis_code_count(claims: Any) -> Any:
    columns = [column for column in claims.columns if column.startswith("ClmDiagnosisCode_")]
    return claims[columns].apply(lambda row: count_present(row), axis=1)


def procedure_code_count(claims: Any) -> Any:
    columns = [column for column in claims.columns if column.startswith("ClmProcedureCode_")]
    return claims[columns].apply(lambda row: count_present(row), axis=1)


def count_present(row: Any) -> int:
    return sum(1 for value in row if str(value).strip() not in {"", "NA", "nan", "None"})


def mask_policy_id(member_id: str) -> str:
    digest = hashlib.sha256(member_id.encode("utf-8")).hexdigest()[:10]
    return f"KAG-POL-{digest}"


def write_dataset(claims: Any, output_dir: Path, dataset_version: str) -> None:
    split_frames = split_claims(claims)
    parquet_columns = [
        "claim_id",
        "member_id",
        "policy_id",
        "provider_id",
        "service_date",
        "service_date_ord",
        "claim_amount_to_limit_ratio",
        "provider_profile_score",
        "high_cost_item_ratio",
        "provider_peer_payment_zscore",
        "leie_excluded_provider",
        "beneficiary_age",
        "chronic_condition_count",
        "diagnosis_code_count",
        "procedure_code_count",
        "deductible_paid",
        "claim_type_inpatient",
        "confirmed_fwa",
    ]
    for split_name, frame in split_frames.items():
        split_dir = output_dir / f"split={split_name}"
        split_dir.mkdir(parents=True, exist_ok=True)
        frame[parquet_columns].to_parquet(split_dir / "part-00000.parquet", index=False)

    manifest = {
        "dataset_key": "kaggle_provider_fraud_claims",
        "dataset_version": dataset_version,
        "business_domain": "health_fwa",
        "sample_grain": "claim",
        "label_column": "confirmed_fwa",
        "label_policy": LABEL_POLICY,
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date_ord",
        "group_split_fields": ["member_id", "policy_id", "provider_id"],
        "public_data_boundary": {
            "purpose": "demo_training_pipeline_and_tpa_json_validation",
            "source": "kaggle_healthcare_provider_fraud_detection_analysis",
            "label_grain": "provider",
            "not_valid_for": [
                "customer_production_model_evidence",
                "claim_level_fraud_truth",
                "fraud_accusation",
                "automatic_claim_denial",
            ],
        },
        "splits": [
            {"split_name": "train", "data_uri": "split=train/"},
            {"split_name": "validation", "data_uri": "split=validation/"},
            {"split_name": "out_of_time", "data_uri": "split=out_of_time/"},
        ],
    }
    (output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def split_claims(claims: Any) -> dict[str, Any]:
    if len(claims) < 6:
        raise SystemExit("Kaggle provider-fraud MVP needs at least 6 claim rows")
    train_end = max(2, int(len(claims) * 0.6))
    validation_end = max(train_end + 1, int(len(claims) * 0.8))
    return {
        "train": claims.iloc[:train_end].copy(),
        "validation": claims.iloc[train_end:validation_end].copy(),
        "out_of_time": claims.iloc[validation_end:].copy(),
    }


def write_tpa_payloads(
    claims: Any,
    output_dir: Path,
    dataset_version: str,
    max_payloads: int,
    source_system: str,
) -> None:
    payload_count = max(0, min(max_payloads, len(claims)))
    payloads = [
        build_tpa_payload(row, dataset_version, source_system)
        for _, row in claims.head(payload_count).iterrows()
    ]
    (output_dir / "tpa_claims.jsonl").write_text(
        "".join(json.dumps(payload, sort_keys=True, ensure_ascii=False) + "\n" for payload in payloads),
        encoding="utf-8",
    )
    sample = payloads[0] if payloads else {}
    (output_dir / "tpa_claim_sample.json").write_text(
        json.dumps(sample, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    (output_dir / "tpa_claims_index.json").write_text(
        json.dumps(
            {
                "artifact_kind": "kaggle_provider_fraud_tpa_claim_payloads",
                "dataset_key": "kaggle_provider_fraud_claims",
                "dataset_version": dataset_version,
                "payload_count": payload_count,
                "files": ["tpa_claims.jsonl", "tpa_claim_sample.json"],
                "boundary": "demo only; no PHI; provider-level labels are not claim-level production evidence",
            },
            indent=2,
            sort_keys=True,
            ensure_ascii=False,
        )
        + "\n",
        encoding="utf-8",
    )


def build_tpa_payload(row: Any, dataset_version: str, source_system: str) -> dict[str, Any]:
    claim_id = row["claim_id"]
    member_id = row["member_id"]
    provider_id = row["provider_id"]
    service_date = str(row["service_date"])
    claim_amount = round(float(row["claim_amount_to_limit_ratio"]) * 25_000.0, 2)
    high_cost_amount = round(float(row["high_cost_item_ratio"]) * min(claim_amount, 10_000.0), 2)
    validate_date = "2009-01-01"
    expire_date = "2010-12-31"
    return {
        "systemCode": source_system,
        "transNo": f"KAGGLE-MVP-{claim_id}",
        "reportCase": {
            "reportNo": claim_id,
            "claimReceiveDate": epoch_ms(service_date),
            "accidentDate": epoch_ms(service_date),
            "accidentReason": "kaggle_provider_fraud_demo_health_claim",
            "claimAmount": claim_amount,
            "calculateRisk": "Y",
            "accidentPerson": {
                "insuredNo": f"masked-{member_id}",
                "insuredName": f"Demo Member {str(member_id)[-4:]}",
                "certNo": f"masked-cert-{str(member_id)[-4:]}",
                "certType": "masked_demo_id",
                "gender": "U",
                "birthday": epoch_ms("1940-01-01"),
            },
            "policyList": [
                {
                    "policyNo": row["policy_id"],
                    "insuredName": f"Demo Member {str(member_id)[-4:]}",
                    "coverageLimit": 25_000,
                    "validateDate": epoch_ms(validate_date),
                    "expireDate": epoch_ms(expire_date),
                    "productList": [
                        {
                            "productCode": "HEALTH-KAGGLE-DEMO",
                            "productName": "Kaggle Demo Health Insurance",
                            "validateDate": epoch_ms(validate_date),
                            "expireDate": epoch_ms(expire_date),
                            "claimLiabilityList": [
                                {
                                    "liabilityCode": "MEDICAL-EXPENSE",
                                    "liabilityName": "Demo Medical Expense",
                                    "claimValidateDate": epoch_ms(validate_date),
                                    "validateDate": epoch_ms(validate_date),
                                    "expireDate": epoch_ms(expire_date),
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
                            "hospitalCode": provider_id,
                            "hospitalName": f"Demo Provider {str(provider_id)[-5:]}",
                            "medicalType": str(row["claim_type"]),
                            "diagnosisList": [
                                {
                                    "detailCode": "KAGGLE-DEMO",
                                    "detailName": "Demo diagnosis bucket",
                                }
                            ],
                            "feeList": [
                                {
                                    "feeCategory": "treatmentFee",
                                    "medicareAmount": round(claim_amount * 0.5, 2),
                                    "feeDetailList": [
                                        {
                                            "name": "Demo reimbursed service",
                                            "amount": round(max(claim_amount - high_cost_amount, 0.0), 2),
                                        },
                                        {
                                            "name": "Demo high cost component",
                                            "amount": high_cost_amount,
                                        },
                                    ],
                                }
                            ],
                        }
                    ],
                }
            ],
            "medicalRecordInfoList": [
                {
                    "medicalRecordNo": f"MR-{claim_id}",
                    "claimNature": "health",
                    "medicalRecordType": "demo_summary",
                    "visitDate": epoch_ms(service_date),
                    "chiefComplaint": "redacted demo complaint",
                    "medicalRecordInformation": "Kaggle provider fraud demo note. No PHI.\nFor pipeline validation only.",
                }
            ],
        },
        "demo_metadata": {
            "source_dataset_key": "kaggle_provider_fraud_claims",
            "source_dataset_version": dataset_version,
            "label_policy": LABEL_POLICY,
            "provider_level_potential_fraud": str(row["provider_fraud_label"]),
            "weak_label_confirmed_fwa": int(row["confirmed_fwa"]),
            "not_valid_for": [
                "customer_production_model_evidence",
                "claim_level_fraud_truth",
                "fraud_accusation",
                "automatic_claim_denial",
            ],
        },
    }


def epoch_ms(iso_date: str) -> int:
    return int(datetime.fromisoformat(iso_date).replace(tzinfo=timezone.utc).timestamp() * 1000)


def write_sources(output_dir: Path, archive_path: Path, dataset_version: str, claim_count: int) -> None:
    (output_dir / "sources.json").write_text(
        json.dumps(
            {
                "source_bundle": "kaggle_provider_fraud_mvp",
                "dataset_key": "kaggle_provider_fraud_claims",
                "dataset_version": dataset_version,
                "label_policy": LABEL_POLICY,
                "claim_count": claim_count,
                "archive_path": str(archive_path),
                "archive_sha256": "sha256:" + hashlib.sha256(archive_path.read_bytes()).hexdigest(),
                "source_files": REQUIRED_ARCHIVE_MEMBERS,
                "source_url": "https://www.kaggle.com/datasets/rohitrox/healthcare-provider-fraud-detection-analysis",
                "boundary": "Kaggle research/demo data; provider-level PotentialFraud is not claim-level adjudication truth",
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    raise SystemExit(main())
