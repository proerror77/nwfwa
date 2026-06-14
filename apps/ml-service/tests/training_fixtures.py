import json
from pathlib import Path

import pandas as pd


BASE_FEATURES = {
    "claim_amount_to_limit_ratio",
    "provider_profile_score",
    "high_cost_item_ratio",
}


def write_split(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    pd.DataFrame(rows).to_parquet(path, index=False)


def write_training_manifest(tmp_path: Path) -> Path:
    dataset_root = tmp_path / "dataset"
    common_rows = [
        {
            "claim_id": "CLM-1",
            "member_id": "MBR-1",
            "policy_id": "POL-1",
            "provider_id": "PRV-1",
            "service_date_ord": 1,
            "claim_amount_to_limit_ratio": 0.12,
            "provider_profile_score": 10.0,
            "high_cost_item_ratio": 0.0,
            "confirmed_fwa": 0,
        },
        {
            "claim_id": "CLM-2",
            "member_id": "MBR-2",
            "policy_id": "POL-2",
            "provider_id": "PRV-2",
            "service_date_ord": 2,
            "claim_amount_to_limit_ratio": 0.88,
            "provider_profile_score": 80.0,
            "high_cost_item_ratio": 1.0,
            "confirmed_fwa": 1,
        },
        {
            "claim_id": "CLM-3",
            "member_id": "MBR-3",
            "policy_id": "POL-3",
            "provider_id": "PRV-3",
            "service_date_ord": 3,
            "claim_amount_to_limit_ratio": 0.2,
            "provider_profile_score": 15.0,
            "high_cost_item_ratio": 0.0,
            "confirmed_fwa": 0,
        },
        {
            "claim_id": "CLM-4",
            "member_id": "MBR-4",
            "policy_id": "POL-4",
            "provider_id": "PRV-4",
            "service_date_ord": 4,
            "claim_amount_to_limit_ratio": 0.95,
            "provider_profile_score": 85.0,
            "high_cost_item_ratio": 1.0,
            "confirmed_fwa": 1,
        },
    ]
    write_split(dataset_root / "train.parquet", common_rows)
    write_split(
        dataset_root / "validation.parquet",
        [
            {
                **common_rows[0],
                "claim_id": "CLM-5",
                "member_id": "MBR-5",
                "policy_id": "POL-5",
                "provider_id": "PRV-5",
                "service_date_ord": 5,
            },
            {
                **common_rows[1],
                "claim_id": "CLM-6",
                "member_id": "MBR-6",
                "policy_id": "POL-6",
                "provider_id": "PRV-6",
                "service_date_ord": 6,
            },
            {
                **common_rows[2],
                "claim_id": "CLM-7",
                "member_id": "MBR-7",
                "policy_id": "POL-7",
                "provider_id": "PRV-7",
                "service_date_ord": 7,
            },
            {
                **common_rows[3],
                "claim_id": "CLM-8",
                "member_id": "MBR-8",
                "policy_id": "POL-8",
                "provider_id": "PRV-8",
                "service_date_ord": 8,
            },
        ],
    )
    write_split(
        dataset_root / "out_of_time.parquet",
        [
            {
                **common_rows[0],
                "claim_id": "CLM-9",
                "member_id": "MBR-9",
                "policy_id": "POL-9",
                "provider_id": "PRV-9",
                "service_date_ord": 9,
            },
            {
                **common_rows[1],
                "claim_id": "CLM-10",
                "member_id": "MBR-10",
                "policy_id": "POL-10",
                "provider_id": "PRV-10",
                "service_date_ord": 10,
            },
            {
                **common_rows[2],
                "claim_id": "CLM-11",
                "member_id": "MBR-11",
                "policy_id": "POL-11",
                "provider_id": "PRV-11",
                "service_date_ord": 11,
            },
            {
                **common_rows[3],
                "claim_id": "CLM-12",
                "member_id": "MBR-12",
                "policy_id": "POL-12",
                "provider_id": "PRV-12",
                "service_date_ord": 12,
            },
        ],
    )
    manifest = {
        "dataset_key": "claims_model",
        "dataset_version": "2026-06-02",
        "dataset_usage_scope": "pilot_validated",
        "pilot_validation_status": "passed",
        "label_column": "confirmed_fwa",
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date_ord",
        "group_split_fields": ["member_id", "policy_id", "provider_id"],
        "splits": [
            {"split_name": "train", "data_uri": "train.parquet"},
            {"split_name": "validation", "data_uri": "validation.parquet"},
            {"split_name": "out_of_time", "data_uri": "out_of_time.parquet"},
        ],
    }
    manifest_path = dataset_root / "manifest.json"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    return manifest_path


def imbalanced_rows(start_ord: int) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for index in range(20):
        confirmed_fwa = index >= 18
        rows.append(
            {
                "claim_id": f"CLM-IMB-{start_ord + index}",
                "member_id": f"MBR-IMB-{start_ord + index}",
                "policy_id": f"POL-IMB-{start_ord + index}",
                "provider_id": f"PRV-IMB-{start_ord + index}",
                "service_date_ord": start_ord + index,
                "claim_amount_to_limit_ratio": 0.9
                if confirmed_fwa
                else 0.08 + (index % 6) * 0.02,
                "provider_profile_score": 88.0
                if confirmed_fwa
                else 8.0 + (index % 5) * 2.0,
                "high_cost_item_ratio": 1.0 if confirmed_fwa else 0.0,
                "confirmed_fwa": int(confirmed_fwa),
            }
        )
    return rows


def write_imbalanced_training_manifest(tmp_path: Path) -> Path:
    dataset_root = tmp_path / "imbalanced_dataset"
    write_split(dataset_root / "train.parquet", imbalanced_rows(1))
    write_split(dataset_root / "validation.parquet", imbalanced_rows(101))
    write_split(dataset_root / "out_of_time.parquet", imbalanced_rows(201))
    manifest = {
        "dataset_key": "claims_model_imbalanced",
        "dataset_version": "2026-06-09",
        "dataset_usage_scope": "pilot_validated",
        "pilot_validation_status": "passed",
        "label_column": "confirmed_fwa",
        "entity_keys": ["claim_id", "member_id", "policy_id", "provider_id"],
        "time_split_field": "service_date_ord",
        "group_split_fields": ["member_id", "policy_id", "provider_id"],
        "splits": [
            {"split_name": "train", "data_uri": "train.parquet"},
            {"split_name": "validation", "data_uri": "validation.parquet"},
            {"split_name": "out_of_time", "data_uri": "out_of_time.parquet"},
        ],
    }
    manifest_path = dataset_root / "manifest.json"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    return manifest_path
