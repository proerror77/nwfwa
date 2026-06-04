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
- `build-demo-automl-lifecycle-evidence`: generate a checked-in golden-path
  evidence pack from the demo datasets, including ranking, rule backtest,
  clustering, monitoring, and lifecycle closure reports.
- `profile-parquet`: validate labeled training manifests and write schema,
  profile, and catalog artifacts.
- `build-feature-set`: materialize feature-set manifests from labeled Parquet
  datasets, bind ordered numeric feature columns, split summaries, and a
  reproducibility hash before training.
- `build-training-handoff`: create the reproducible external training contract,
  including algorithm-aware logistic, XGBoost, and LightGBM artifact semantics.
- `run-retraining-job`: claim a candidate job, execute the trainer, and register
  output, enriching the trainer payload with the Rust feature-set manifest URI
  and reproducibility hash before API registration; when the trainer returns a
  serving manifest, it also runs Rust serving artifact evaluation and attaches
  the report URI plus serving latency/status evidence. ONNX runtime kinds must
  also carry a passed trainer ONNX parity report before the Rust serving gate
  can pass.
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
- `cluster-provider-graph`: run Rust-native provider graph-community
  clustering over unlabeled provider graph features, then create graph anomaly
  review tasks without assigning labels.
- `cluster-claim-entities`: run Rust-native claim/member/provider entity
  clustering over unlabeled claims, then create anomaly review tasks or
  rule-candidate backtest preparation without assigning labels or writing rules.
- `build-mlops-monitoring-plan`: define scheduled shadow, drift, fairness,
  reviewer-disagreement, and label-delay checks.
- `build-mlops-monitoring-report`: combine Rust artifact evaluation, shadow,
  drift, and fairness reports into one monitoring decision with review tasks and
  retraining preparation triggers.
- `submit-mlops-monitoring-report`: submit the monitoring decision into the API
  governance audit surface without automatically creating retraining jobs,
  activating models, or rolling models back.
- `build-mlops-scheduler-execution-report`: turn a scheduled monitoring plan
  and monitoring report into scheduler execution evidence plus external
  alert-delivery tasks.
- `submit-mlops-alert-delivery-tasks`: submit scheduler alert-router handoff
  evidence into API governance audit without creating retraining jobs,
  activating models, rolling models back, or assigning fraud labels.
- `run-mlops-monitoring-cycle`: execute the Rust governance cycle from a
  scheduled plan plus runtime reports, then optionally submit monitoring and
  alert-router handoff evidence into the API audit surface.
- `deliver-mlops-alert-receiver-webhook`: POST queued MLOps alert tasks to a
  customer receiver webhook and write delivery evidence without triggering model
  lifecycle actions.
- `build-automl-lifecycle-closure-report`: summarize dataset, candidate
  ranking, ONNX Rust-serving, rule-backtest, clustering, and monitoring
  evidence into one closure report without auto-activating models or writing
  rules.

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
  `rust_onnx`, `xgboost_onnx`, `lightgbm_onnx`, `deep_learning_onnx`, or
  `http_model_service`;
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
- 64% for model portfolio: logistic has a native Rust JSON serving artifact;
  XGBoost and LightGBM training now emit governed ONNX serving artifacts with
  probability-parity reports, the Rust runtime can execute XGBoost, LightGBM,
  generic Rust ONNX, and `deep_learning_onnx` manifests after contract
  validation, and provider-peer, provider graph-community, and
  claim/member/provider entity clustering have Rust-native demo workflows.
- 96% for Auto MLOps: worker can build feature-set manifests, create
  algorithm-aware training handoffs, enrich
  retraining outputs with Rust feature-set and Rust serving evaluation evidence,
  rank candidates, evaluate serving artifacts, require ONNX parity evidence for
  XGBoost/LightGBM gates, mine explainable rule candidates, backtest those
  candidates into human-review evidence, summarize live monitoring reports into
  review/retraining triggers, submit those reports into API governance audit,
  produce scheduler execution and alert-delivery evidence, submit scheduler
  alert-router handoff evidence into API governance audit, run the Rust
  monitoring cycle executor from plan plus runtime reports, POST queued alert
  tasks to a customer receiver webhook with bearer auth, HMAC signature, and
  bounded retry evidence, produce a lifecycle closure report, and generate a
  checked-in demo lifecycle evidence pack, while API promotion gates now require
  Rust feature-set materialization evidence, worker ranking requires Rust
  feature-set and Rust serving evaluation evidence, and trainer-side ONNX parity
  reports and unlabeled anomaly review tasks exist; API retraining output now
  accepts governed serving manifests, and the console has provider model
  release, promotion review, activation, and rollback actions. Customer-side
  runtime report producers and production cron deployment still need
  environment-specific wiring.
- 78% for Rust ONNX serving: serving-manifest validation, checksum/signature
  checks, feature-order binding, CPU ONNX Runtime execution, and probability
  extraction are implemented, and the worker now creates Rust serving evaluation
  evidence automatically on retraining registration when a serving manifest is
  present; for XGBoost, LightGBM, and deep-learning ONNX candidates it also
  requires a passed ONNX parity report before the gate can pass. Production
  cache/reuse strategy, broader real ONNX fixture tests, and live latency
  monitoring still need hardening.

The runtime now has a serving-manifest boundary for Rust logistic artifacts and
real Rust ONNX scoring for generated XGBoost and LightGBM artifacts. The worker
now has an artifact-evaluation gate for Rust serving parity and latency
evidence before a candidate can enter activation review, and `run-retraining-job`
attaches that evidence before candidate registration when the trainer returns a
serving manifest. The API now records serving-manifest evidence in retraining
output registration and the console exposes the human release actions. The next
highest-leverage implementation is richer artifact report drill-down and live
latency/drift monitoring around those existing gates.
