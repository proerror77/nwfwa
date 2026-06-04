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
- `build-feature-set`: materialize feature-set manifests from labeled Parquet
  datasets, bind ordered numeric feature columns, split summaries, and a
  reproducibility hash before training.
- `build-training-handoff`: create the reproducible external training contract.
- `run-retraining-job`: claim a candidate job, execute the trainer, and register
  output, enriching the trainer payload with the Rust feature-set manifest URI
  and reproducibility hash before API registration; when the trainer returns a
  serving manifest, it also runs Rust serving artifact evaluation and attaches
  the report URI plus serving latency/status evidence.
- `rank-automl-candidates`: compare validation reports for logistic, XGBoost,
  LightGBM, and anomaly candidates, then open human-review recommendations
  without activating any model.
- `evaluate-model-artifact`: run Rust serving-manifest execution against a
  governed Parquet split, verify optional probability parity, record P95
  latency, and write activation-review evidence without promoting the model.
- `mine-rule-candidates`: translate feature-importance evidence into draft rule
  candidates plus required backtest and human-review work items.
- `run-rule-candidate-backtest`: select deterministic thresholds, calculate
  split-level rule metrics, and create review evidence while keeping
  rule-library writeback blocked.
- `cluster-provider-peers`: run Rust-native clustering over unlabeled provider
  peer features, then create anomaly review tasks without assigning labels.
- `build-mlops-monitoring-plan`: define scheduled shadow, drift, fairness,
  reviewer-disagreement, and label-delay checks.
- `build-mlops-monitoring-report`: combine Rust artifact evaluation, shadow,
  drift, and fairness reports into one monitoring decision with review tasks and
  retraining preparation triggers.

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

- 63% for governance skeleton: model jobs, approval gates, worker handoff,
  Rust feature-set promotion gating, monitoring-plan contract, and
  documentation exist.
- 56% for data lifecycle: labeled public/demo manifests, profiling, and
  Rust-built feature-set manifests exist, and worker-driven retraining now
  injects Rust feature-set evidence into candidate registration; Rust-generated
  labeled/unlabeled demo packs now cover the missing dataset shape.
- 60% for model portfolio: logistic has a native Rust JSON serving artifact;
  XGBoost and LightGBM training now emit governed ONNX serving artifacts with
  probability-parity reports, the Rust runtime can execute those ONNX manifests
  after contract validation, and provider-peer clustering has a Rust-native demo
  workflow; broader graph/member/claim clustering and deep-learning serving
  remain future work.
- 70% for Auto MLOps: worker can build feature-set manifests, enrich
  retraining outputs with Rust feature-set and Rust serving evaluation evidence,
  rank candidates, evaluate serving artifacts, mine explainable rule candidates,
  backtest those candidates into human-review evidence, and summarize live
  monitoring reports into review/retraining triggers, while API promotion gates
  now require Rust feature-set materialization evidence, worker ranking requires
  Rust feature-set and Rust serving evaluation evidence, and trainer-side ONNX
  parity reports and unlabeled anomaly review tasks exist; API retraining output
  now accepts governed serving manifests, and the console has provider model
  release, promotion review, activation, and rollback actions. Broader graph,
  member, and claim clustering still need hardening.
- 72% for Rust ONNX serving: serving-manifest validation, checksum/signature
  checks, feature-order binding, CPU ONNX Runtime execution, and probability
  extraction are implemented, and the worker now creates Rust serving evaluation
  evidence automatically on retraining registration when a serving manifest is
  present; production cache/reuse strategy, broader ONNX fixture tests, and live
  latency monitoring still need hardening.

The runtime now has a serving-manifest boundary for Rust logistic artifacts and
real Rust ONNX scoring for generated XGBoost and LightGBM artifacts. The worker
now has an artifact-evaluation gate for Rust serving parity and latency
evidence before a candidate can enter activation review, and `run-retraining-job`
attaches that evidence before candidate registration when the trainer returns a
serving manifest. The API now records serving-manifest evidence in retraining
output registration and the console exposes the human release actions. The next
highest-leverage implementation is richer artifact report drill-down and live
latency/drift monitoring around those existing gates.
