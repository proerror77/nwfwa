from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
import onnxruntime as ort
import pandas as pd
from onnxmltools import convert_lightgbm, convert_xgboost
from onnxmltools.convert.common.data_types import FloatTensorType
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import (
    accuracy_score,
    average_precision_score,
    confusion_matrix,
    f1_score,
    precision_score,
    recall_score,
    roc_auc_score,
)
from sklearn.neural_network import MLPClassifier
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler

from .training_diagnostics import model_input, model_matrix
from .training_utils import safe_path_segment

try:
    from xgboost import XGBClassifier
except ImportError:  # pragma: no cover - exercised only in incomplete envs
    XGBClassifier = None

try:
    from lightgbm import LGBMClassifier
except ImportError:  # pragma: no cover - exercised only in incomplete envs
    LGBMClassifier = None


DEFAULT_THRESHOLD = 0.5
DEFAULT_ALGORITHM = "logistic_regression"
ONNX_ALGORITHMS = {"xgboost", "lightgbm"}
SUPPORTED_ALGORITHMS = {DEFAULT_ALGORITHM, "xgboost", "lightgbm", "deep_learning"}


def normalize_algorithm(value: Any) -> str:
    algorithm = str(value).strip().lower().replace("-", "_")
    if algorithm not in SUPPORTED_ALGORITHMS:
        supported = ", ".join(sorted(SUPPORTED_ALGORITHMS))
        raise ValueError(f"algorithm must be one of: {supported}")
    if algorithm == "xgboost" and XGBClassifier is None:
        raise ValueError("xgboost training requires the xgboost package")
    if algorithm == "lightgbm" and LGBMClassifier is None:
        raise ValueError("lightgbm training requires the lightgbm package")
    return algorithm


def build_pipeline(algorithm: str, labels: pd.Series) -> Pipeline:
    if algorithm == DEFAULT_ALGORITHM:
        return Pipeline(
            [
                ("scale", StandardScaler()),
                (
                    "model",
                    LogisticRegression(
                        class_weight="balanced",
                        max_iter=1000,
                        random_state=42,
                    ),
                ),
            ]
        )
    if algorithm == "deep_learning":
        return Pipeline(
            [
                ("scale", StandardScaler()),
                (
                    "model",
                    MLPClassifier(
                        hidden_layer_sizes=(8, 4),
                        activation="relu",
                        solver="lbfgs",
                        alpha=0.001,
                        max_iter=500,
                        random_state=42,
                    ),
                ),
            ]
        )
    positive_count = int((labels.astype(int) == 1).sum())
    negative_count = int((labels.astype(int) == 0).sum())
    scale_pos_weight = negative_count / positive_count if positive_count else 1.0
    if algorithm == "xgboost":
        model = XGBClassifier(
            n_estimators=8,
            max_depth=3,
            learning_rate=0.08,
            subsample=0.9,
            colsample_bytree=0.9,
            objective="binary:logistic",
            eval_metric="logloss",
            tree_method="hist",
            n_jobs=1,
            min_child_weight=0,
            scale_pos_weight=scale_pos_weight,
            random_state=42,
            verbosity=0,
        )
    else:
        model = LGBMClassifier(
            n_estimators=12,
            max_depth=3,
            learning_rate=0.08,
            objective="binary",
            class_weight="balanced",
            min_child_samples=1,
            min_data_in_bin=1,
            num_leaves=7,
            random_state=42,
            verbose=-1,
        )
    return Pipeline([("model", model)])


def candidate_version(base_model_version: str, job_id: str, algorithm: str) -> str:
    base = safe_path_segment(base_model_version)
    job = safe_path_segment(job_id)
    if algorithm == DEFAULT_ALGORITHM:
        return f"{base}-candidate-{job}"
    return f"{base}-{safe_path_segment(algorithm)}-candidate-{job}"


def runtime_kind_for_algorithm(algorithm: str) -> str:
    return {
        DEFAULT_ALGORITHM: "sklearn_logistic_regression",
        "xgboost": "xgboost_classifier",
        "lightgbm": "lightgbm_classifier",
        "deep_learning": "sklearn_mlp_classifier",
    }[algorithm]


def serving_runtime_kind_for_algorithm(algorithm: str) -> str:
    return {
        DEFAULT_ALGORITHM: "rust_logistic_regression",
        "xgboost": "xgboost_onnx",
        "lightgbm": "lightgbm_onnx",
        "deep_learning": "deep_learning_sklearn_mlp",
    }[algorithm]


def algorithm_family(algorithm: str) -> str:
    return {
        DEFAULT_ALGORITHM: "linear_baseline",
        "xgboost": "gradient_boosted_tree",
        "lightgbm": "gradient_boosted_tree",
        "deep_learning": "deep_learning",
    }[algorithm]


def build_rust_serving_artifact(
    pipeline: Pipeline,
    model_key: str,
    model_version: str,
    feature_columns: list[str],
    threshold: float,
) -> dict[str, Any]:
    scaler = pipeline.named_steps["scale"]
    model = pipeline.named_steps["model"]
    if not isinstance(scaler, StandardScaler):
        raise TypeError("rust serving export requires StandardScaler preprocessing")
    if not isinstance(model, LogisticRegression):
        raise TypeError("rust serving export requires LogisticRegression model")
    coefficients = model.coef_[0]
    scale = scaler.scale_
    mean = scaler.mean_
    raw_coefficients = {
        feature: float(coefficient / scale_value)
        for feature, coefficient, scale_value in zip(
            feature_columns,
            coefficients,
            scale,
            strict=True,
        )
    }
    intercept = float(model.intercept_[0] - sum(coefficients * mean / scale))
    return {
        "model_key": model_key,
        "model_version": model_version,
        "runtime_kind": "rust_logistic_regression",
        "execution_provider": "cpu",
        "threshold": threshold,
        "feature_columns": feature_columns,
        "intercept": intercept,
        "coefficients": raw_coefficients,
    }


def export_onnx_serving_artifact(
    pipeline: Pipeline,
    algorithm: str,
    feature_columns: list[str],
    validation: pd.DataFrame,
    onnx_artifact_path: Path,
    parity_report_path: Path,
    use_numpy_matrix: bool,
) -> dict[str, Any]:
    model = pipeline.named_steps["model"]
    initial_types = [("float_input", FloatTensorType([None, len(feature_columns)]))]
    if algorithm == "xgboost":
        onnx_model = convert_xgboost(
            model,
            initial_types=initial_types,
            target_opset=15,
        )
    elif algorithm == "lightgbm":
        onnx_model = convert_lightgbm(
            model,
            initial_types=initial_types,
            target_opset=15,
            zipmap=False,
        )
    else:
        raise ValueError(f"ONNX export is not supported for algorithm: {algorithm}")

    onnx_artifact_path.write_bytes(onnx_model.SerializeToString())
    session = ort.InferenceSession(
        str(onnx_artifact_path),
        providers=["CPUExecutionProvider"],
    )
    feature_matrix = model_matrix(validation, feature_columns)
    python_probabilities = pipeline.predict_proba(
        model_input(validation, feature_columns, use_numpy_matrix)
    )[:, 1]
    onnx_outputs = session.run(None, {session.get_inputs()[0].name: feature_matrix})
    onnx_probabilities = extract_positive_probabilities(onnx_outputs)
    if len(onnx_probabilities) != len(python_probabilities):
        raise ValueError(
            "ONNX parity output row count does not match Python model output row count"
        )

    deltas = np.abs(np.asarray(python_probabilities) - np.asarray(onnx_probabilities))
    tolerance = 1e-4
    max_delta = float(deltas.max()) if len(deltas) else 0.0
    average_delta = float(deltas.mean()) if len(deltas) else 0.0
    status = "passed" if max_delta <= tolerance else "failed"
    report = {
        "report_kind": "onnx_probability_parity",
        "report_version": 1,
        "algorithm": algorithm,
        "python_runtime_kind": runtime_kind_for_algorithm(algorithm),
        "serving_runtime_kind": serving_runtime_kind_for_algorithm(algorithm),
        "onnx_export_status": "exported",
        "status": status,
        "sample_count": int(len(python_probabilities)),
        "tolerance": tolerance,
        "max_abs_probability_delta": max_delta,
        "average_abs_probability_delta": average_delta,
        "input_name": session.get_inputs()[0].name,
        "output_names": [output.name for output in session.get_outputs()],
        "feature_columns": feature_columns,
    }
    parity_report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    if status != "passed":
        raise ValueError(
            f"ONNX parity failed for {algorithm}: max probability delta {max_delta} exceeds {tolerance}"
        )
    return report


def extract_positive_probabilities(outputs: list[Any]) -> np.ndarray:
    for output in reversed(outputs):
        array = np.asarray(output)
        if array.ndim == 2 and array.shape[1] >= 2 and np.issubdtype(array.dtype, np.number):
            return array[:, 1].astype(float)
        if array.ndim == 1 and np.issubdtype(array.dtype, np.floating):
            return array.astype(float)
    for output in reversed(outputs):
        if isinstance(output, list) and output and isinstance(output[0], dict):
            return np.asarray(
                [row.get(1, row.get("1", 0.0)) for row in output],
                dtype=float,
            )
    raise ValueError("ONNX output does not expose usable positive-class probabilities")


def evaluate_split(
    pipeline: Pipeline,
    frame: pd.DataFrame,
    feature_columns: list[str],
    label_column: str,
    use_numpy_matrix: bool,
) -> dict[str, Any]:
    y_true = frame[label_column].astype(int)
    probabilities = pipeline.predict_proba(
        model_input(frame, feature_columns, use_numpy_matrix)
    )[:, 1]
    predictions = (probabilities >= DEFAULT_THRESHOLD).astype(int)
    tn, fp, fn, tp = confusion_matrix(y_true, predictions, labels=[0, 1]).ravel()
    auc = safe_auc(y_true, probabilities)
    return {
        "auc": auc,
        "average_precision": average_precision_score(y_true, probabilities),
        "ks": ks_statistic(y_true, probabilities),
        "precision": precision_score(y_true, predictions, zero_division=0),
        "recall": recall_score(y_true, predictions, zero_division=0),
        "f1": f1_score(y_true, predictions, zero_division=0),
        "accuracy": accuracy_score(y_true, predictions),
        "threshold": DEFAULT_THRESHOLD,
        "confusion_matrix": {
            "tp": int(tp),
            "fp": int(fp),
            "tn": int(tn),
            "fn": int(fn),
        },
    }


def safe_auc(y_true: pd.Series, probabilities: Any) -> float:
    if len(set(y_true.astype(int).tolist())) < 2:
        return 0.5
    return roc_auc_score(y_true, probabilities)


def ks_statistic(y_true: pd.Series, probabilities: Any) -> float:
    values = pd.DataFrame({"label": y_true.astype(int), "probability": probabilities})
    positive = values[values["label"] == 1]["probability"].sort_values()
    negative = values[values["label"] == 0]["probability"].sort_values()
    if positive.empty or negative.empty:
        return 0.0
    thresholds = sorted(set(values["probability"].tolist()))
    max_gap = 0.0
    for threshold in thresholds:
        positive_cdf = (positive <= threshold).mean()
        negative_cdf = (negative <= threshold).mean()
        max_gap = max(max_gap, abs(positive_cdf - negative_cdf))
    return float(max_gap)
