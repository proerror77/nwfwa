from __future__ import annotations

import hashlib
import json
import re
from typing import Any


def source_data_quality_score(frames: Any) -> float:
    total_cells = 0
    missing_cells = 0
    for frame in frames:
        total_cells += int(frame.shape[0] * frame.shape[1])
        missing_cells += int(frame.isna().sum().sum())
    if total_cells == 0:
        return 0.0
    return round(1.0 - missing_cells / total_cells, 4)


def feature_reproducibility_hash(
    feature_columns: list[str],
    label_column: str,
    time_split_field: str,
    group_split_fields: list[str],
) -> str:
    payload = json.dumps(
        {
            "feature_columns": feature_columns,
            "label_column": label_column,
            "time_split_field": time_split_field,
            "group_split_fields": group_split_fields,
        },
        sort_keys=True,
    )
    return f"sha256:{hashlib.sha256(payload.encode('utf-8')).hexdigest()}"


def format_metric(value: float) -> str:
    return f"{float(value):.4f}"


def safe_path_segment(value: str) -> str:
    sanitized = re.sub(r"[^A-Za-z0-9_.-]+", "_", value).strip("_")
    return sanitized or "unknown"


def safe_id_segment(value: str) -> str:
    sanitized = re.sub(r"[^A-Za-z0-9]+", "_", value).strip("_")
    return sanitized or "unknown"
