import json
from pathlib import Path

import pandas as pd

from app.training import train_from_manifest


def write_split(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    pd.DataFrame(rows).to_parquet(path, index=False)


def test_training_pipeline_writes_artifacts_and_validation_payload(tmp_path: Path):
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
            {**common_rows[0], "claim_id": "CLM-5", "member_id": "MBR-5", "policy_id": "POL-5", "provider_id": "PRV-5"},
            {**common_rows[1], "claim_id": "CLM-6", "member_id": "MBR-6", "policy_id": "POL-6", "provider_id": "PRV-6"},
        ],
    )
    write_split(
        dataset_root / "out_of_time.parquet",
        [
            {**common_rows[2], "claim_id": "CLM-7", "member_id": "MBR-7", "policy_id": "POL-7", "provider_id": "PRV-7"},
            {**common_rows[3], "claim_id": "CLM-8", "member_id": "MBR-8", "policy_id": "POL-8", "provider_id": "PRV-8"},
        ],
    )
    manifest = {
        "dataset_key": "claims_model",
        "dataset_version": "2026-06-02",
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

    payload = train_from_manifest(
        manifest_path=manifest_path,
        artifact_base_uri=tmp_path / "artifacts",
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="model_retraining_job_1",
        actor="trainer-worker",
    )

    assert payload["candidate_model_version"] == "0.1.0-candidate-model_retraining_job_1"
    assert payload["artifact_uri"].endswith("/model.joblib")
    assert payload["validation_report_uri"].endswith("/validation.json")
    assert payload["feature_importance_uri"].endswith("/feature_importance.parquet")
    assert Path(payload["artifact_uri"]).exists()
    assert Path(payload["validation_report_uri"]).exists()
    assert Path(payload["feature_importance_uri"]).exists()
    assert payload["metrics_json"]["time_group_split_status"] == "passed"
    assert payload["metrics_json"]["leakage_check_status"] == "passed"
    assert payload["metrics_json"]["shadow_comparison_status"] == "pending"
    assert payload["metrics_json"]["label_provenance_status"] == "passed"
    assert payload["auc"] is not None
    assert payload["precision"] is not None
    assert payload["recall"] is not None
    assert payload["threshold"] == "0.5000"
