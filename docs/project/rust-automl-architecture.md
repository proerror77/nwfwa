# Rust Auto MLOps Architecture

This document defines the target ML architecture for `nwfwa`: Rust owns the
full ML lifecycle and production control plane, while ONNX or signed serving
artifacts carry model formats that are not practical to reimplement in Rust.

## Target State

Rust should own:

- dataset registration, schema profiling, label policy, and split validation;
- feature materialization contracts and feature reproducibility hashes;
- candidate training job orchestration and artifact registration;
- model evaluation, backtest, shadow, drift, fairness, and label-delay checks;
- human review queues, approval gates, activation, rollback, and audit events;
- production inference routing through the API server and Rust runtime.

Rust does not need to own every optimizer implementation. For XGBoost,
LightGBM, deep learning, or other complex algorithms, the acceptable path is:

1. Train offline through a reproducible job.
2. Export a governed artifact, preferably ONNX when the algorithm and
   converter support the required operators.
3. Register artifact checksum, feature order, threshold, metrics, and
   explanation evidence.
4. Serve through Rust with pinned runtime versions and parity tests.

This keeps the production surface Rust-governed without forcing lower-quality
custom Rust implementations of mature model libraries.

## Algorithm Portfolio

| Family | Purpose | Rust role | Artifact path | Promotion boundary |
| --- | --- | --- | --- | --- |
| Deterministic rules | Policy, eligibility, clear FWA patterns | Native rule evaluation and backtest | Rule DSL JSON | Can affect disposition only with customer-approved authority |
| Logistic regression | Calibrated baseline and rule-only comparison | Native Rust artifact can serve directly | Rust JSON artifact, optional ONNX | Candidate model only until gates pass |
| Decision tree / shallow tree | Transparent rule-of-thumb mining | Rust evaluates extracted tree paths or imports artifact | Rust JSON or ONNX | Candidate rules require Rule Studio workflow |
| XGBoost | Primary supervised structured-risk challenger | Rust orchestrates, validates, registers, serves exported artifact | ONNX when supported, otherwise signed model endpoint/artifact | No automatic promotion |
| LightGBM | Second GBDT candidate after XGBoost path stabilizes | Same as XGBoost | ONNX when supported, otherwise signed model endpoint/artifact | No automatic promotion |
| Clustering / anomaly | Provider, member, claim, graph, and peer outlier discovery | Rust builds datasets, runs jobs, stores candidate signals | Rust-native algorithm or ONNX where useful | Review candidate only, never fraud truth |
| Deep learning / LLM | Documents, OCR cleanup, note extraction, embeddings, investigation drafting | Rust controls evidence, retrieval, audit, and approval | ONNX, embedding runtime, or external service | Assistive only |

## Closed Loop

The production loop should be:

1. Register immutable labeled and unlabeled Parquet datasets.
2. Profile schemas and enforce label policy.
3. Build feature-set versions with reproducibility hashes.
4. Train candidates offline: logistic baseline, XGBoost, then LightGBM.
5. Export artifacts: Rust JSON for simple models, ONNX for portable complex
   models, or a signed endpoint only when ONNX cannot preserve behavior.
6. Run validation: holdout, out-of-time, leakage, calibration, fairness,
   latency, artifact checksum, and serving parity.
7. Run deterministic backtests against historical rule and review outcomes.
8. Produce explanation artifacts: feature importance, tree paths, or SHAP-style
   contribution summaries.
9. Convert explainable patterns into Rule Studio candidate rules only when they
   can be expressed as deterministic feature predicates.
10. Require rule backtest, human rule review, approval, and publication before
    any extracted pattern enters the active rule library.
11. Run shadow mode against live traffic and QA outcomes.
12. Promote or reject through governed model approval.
13. Monitor drift, calibration, segment performance, reviewer disagreement,
    label delay, latency, and error rate.
14. Trigger retraining proposals, not automatic activation.

Auto MLOps may rank candidates and open review tasks. It must not auto-promote
models, publish rules, or turn unlabeled anomaly clusters into confirmed FWA
labels.

## Rust Worker Responsibilities

The worker is the right control-plane home for scheduled and batch ML work:

- `build-demo-ml-datasets`: generate labeled and unlabeled Parquet datasets for
  pipeline validation.
- `profile-parquet`: validate labeled training manifests and write schema,
  profile, and catalog artifacts.
- `build-training-handoff`: create the reproducible external training contract.
- `run-retraining-job`: claim a candidate job, execute the trainer, and register
  output.
- `build-mlops-monitoring-plan`: define scheduled shadow, drift, fairness,
  reviewer-disagreement, and label-delay checks.

Future worker commands should add:

- `build-feature-set`: materialize feature versions from registered datasets;
- `evaluate-model-artifact`: run offline metrics and serving parity tests;
- `mine-rule-candidates`: translate model explanation patterns into Rule Studio
  draft candidates;
- `run-rule-candidate-backtest`: backtest drafts before human review;
- `rank-automl-candidates`: compare logistic, XGBoost, LightGBM, and anomaly
  candidates without activating them.

## Serving Architecture

Serving should follow a layered runtime:

1. Serving manifest scorer for governed artifact metadata, feature order,
   checksum, version lock, and signature validation.
2. Native Rust scorers for deterministic rules, heuristics, logistic baselines,
   and small transparent artifacts.
3. Rust ONNX scorer for exported XGBoost, LightGBM, and deep models where
   conversion preserves feature order and output parity.
4. HTTP scorer only as a controlled fallback for models that cannot yet be
   exported safely.

Every production model version must record:

- artifact URI and checksum;
- serving runtime kind: `rust_serving_manifest`, `rust_artifact`,
  `rust_onnx`, or `http_model_service`;
- feature set id and ordered feature list;
- threshold and calibration evidence;
- validation, shadow, drift, and fairness report URIs;
- explanation artifact URI;
- approval and rollback references.

## Current Completion Estimate

Current repository completion for this target architecture is approximately:

- 60% for governance skeleton: model jobs, approval gates, worker handoff,
  monitoring-plan contract, and documentation exist.
- 45% for data lifecycle: labeled public/demo manifests and profiling exist;
  Rust-generated labeled/unlabeled demo packs now cover the missing dataset
  shape.
- 35% for model portfolio: logistic and XGBoost training paths exist; LightGBM,
  clustering, and ONNX serving are still future work.
- 30% for Auto MLOps: worker has plan and retraining primitives, but candidate
  ranking, rule mining, parity tests, and automatic review task creation are not
  implemented yet.
- 20% for Rust ONNX serving: architecture is defined, but the runtime scorer and
  parity tests still need implementation.

The runtime now has a serving-manifest boundary for Rust logistic artifacts and
ONNX contract validation. The next highest-leverage implementation is the real
Rust ONNX scorer with parity tests against XGBoost and LightGBM training
artifacts.
