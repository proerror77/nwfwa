# ML Algorithm Strategy

Last researched: 2026-06-02

This document records the current external research and implementation
assessment for the FWA model plan. It should be read with the PRD modeling
section, model APIs, and scoring implementation.

## Executive Assessment

The planned algorithm direction is appropriate for the current product stage:
rule-first, explainable-model-first, and governed promotion before any model can
affect routing.

Automatic claim denial or approval is outside the authority of ML, anomaly
detection, provider graph signals, similar-case search, or Agent outputs. If the
product supports automatic denial or straight-through approval in pre-payment
flows, that decision must come from customer-approved deterministic adjudication
rules with policy authority, evidence refs, exception checks, audit ids, and an
appeal or override path. Models can prioritize, route, explain, and trigger
review; they cannot be the sole denial authority.

The current implementation is not a production machine learning model. It is a
demo and pilot baseline that exercises the model runtime contract, scoring
fusion, evidence refs, model registry, retraining job contract, and promotion
gates. That is the right boundary for the MVP because stable customer labels,
pilot holdouts, and shadow-mode evidence do not exist yet.

## External Research Summary

The current research supports the existing PRD direction.

| Source | Relevant finding | Product implication |
| --- | --- | --- |
| CMS Data Analytics and Systems Group | CMS program integrity analytics include subject-matter collaboration, data mining, behavioral analysis, network analysis, predictive analytics, and machine learning for fraud, waste, and abuse detection. | FWA detection should combine rules, peer/provider behavior, graph or network signals, and ML rather than rely on a single black-box model. |
| CMS FPS2 system description | CMS describes Fraud Prevention System 2 as using predictive analytics, link analysis, geo-mapping, and machine learning to monitor Medicare fee-for-service pre-paid claims. | Pre-payment screening is a valid target, but it must be tied to investigation capture and governed follow-up. |
| GAO 2026 Medicare analytics report | GAO describes Medicare fraud analytics such as billing spikes, peer-to-peer analysis, risk scoring, predictive analytics, and unstructured ML. It also notes that selected Medicare/private payer analytics were not using generative AI for potential fraud identification. | The current seven-layer plan is aligned: peer benchmark, rules, anomaly, provider graph, model score, and routing. Generative AI should remain assistive for evidence packaging and text workflows, not fraud disposition. |
| scikit-learn cross-validation docs | scikit-learn warns that ordinary random splits are unsafe when samples are time-dependent or grouped; it recommends time-series-aware and group-wise validation in those cases. | Model promotion must keep requiring time split plus leakage-sensitive group split across member, policy, provider, and case family. |
| scikit-learn metrics docs | Precision, recall, precision-recall curves, and average precision are first-class classifier metrics. | FWA evaluation should prioritize PR-AUC/AP, precision at review capacity, recall, false-positive burden, and confusion matrix, not accuracy alone. |
| scikit-learn calibration docs | Probability calibration requires careful cross-validation and disjoint calibration data. | `fraud_probability`, `abuse_probability`, and `waste_probability` must not be represented as calibrated probabilities until a calibrated model and calibration evidence exist. |
| XGBoost, LightGBM, and SHAP docs | GBDT libraries support feature importance and contribution/explanation workflows, including Tree SHAP for tree ensembles. | Gradient-boosted trees are appropriate later candidates only when feature importance or SHAP artifacts are registered with validation evidence. |
| NIST AI RMF | NIST frames AI risk management around design, development, deployment, use, evaluation, and risk management across the AI lifecycle. | The model lifecycle should keep explicit governance, measurement, documentation, human review, and monitoring gates. |

Primary sources:

- CMS DASG: <https://www.cms.gov/research-statistics-data-and-systems/data-analytics-and-systems-group>
- CMS DASG business systems / FPS2: <https://www.cms.gov/data-research/computer-data-systems/data-analytics-and-systems-group-dasg/dasg-business-systems>
- GAO-26-107799: <https://files.gao.gov/reports/GAO-26-107799/index.html>
- scikit-learn cross-validation: <https://scikit-learn.org/stable/modules/cross_validation.html>
- scikit-learn model evaluation: <https://scikit-learn.org/stable/modules/model_evaluation.html>
- scikit-learn calibration: <https://scikit-learn.org/stable/modules/calibration.html>
- XGBoost prediction explanations: <https://xgboost.readthedocs.io/en/latest/prediction.html>
- LightGBM Booster API: <https://lightgbm.readthedocs.io/en/latest/pythonapi/lightgbm.Booster.html>
- SHAP TreeExplainer: <https://shap.readthedocs.io/en/stable/generated/shap.TreeExplainer.html>
- NIST AI RMF: <https://www.nist.gov/itl/ai-risk-management-framework>

## Current Implementation Position

| Area | Current implementation | Assessment |
| --- | --- | --- |
| Python ML service | `apps/ml-service/app/scorer.py` keeps the deterministic baseline fallback and can load a trained `.joblib` artifact through `FWA_MODEL_ARTIFACT_URI`. `apps/ml-service/app/training.py` trains logistic, XGBoost, and LightGBM candidates from a Parquet manifest and writes model, validation, feature-importance, serving-manifest, and ONNX parity artifacts where applicable. | Training/export and HTTP demo compatibility are in place. Python remains a training and fallback surface; the long-term production control plane and serving contract are Rust-governed. |
| Rust model runtime | `crates/fwa-ml-runtime` supports HTTP scoring, heuristic fallback, local JSON logistic artifact scoring, Rust ONNX serving for `rust_onnx`, `xgboost_onnx`, `lightgbm_onnx`, and `deep_learning_onnx`, model identity checks, checksum validation, optional HMAC signature verification, version-lock metadata, explanations, latency, and bounded HTTP model-service calls that bypass host proxy settings. | Production-oriented serving has moved into Rust for logistic and ONNX artifacts. Production still needs broader real ONNX fixture coverage, service-level SLOs, active failure-rate monitoring, and environment-specific timeout policy. |
| Feature layer | `crates/fwa-features` emits claim amount ratio, peer-percentile baseline, item count, high-cost item ratio, diagnosis/procedure match, and provider tier/profile features. | Suitable for demo. Production needs rolling provider/member/history, peer cohort, duplicate/similarity, graph, and label-delay features. |
| Anomaly layer | `crates/fwa-anomaly` uses explainable threshold signals, and `apps/worker` has Rust-native provider-peer, provider graph-community, and claim-entity clustering demo workflows over unlabeled manifests. | Reasonable explainable anomaly baseline plus demo clustering. Production still needs customer-scale unlabeled feature materialization and governance review volume controls. |
| Risk fusion | `crates/fwa-scoring` combines seven layers with explicit weights and evidence refs. | Good demo and pilot decision-support structure. Weights are policy defaults, not learned coefficients. |
| Retraining worker | `apps/worker` can keep the deterministic mock candidate output for demo smoke, or call `python -m app.train` when `run-retraining-job` receives `--training-manifest`. | Real candidate registration can now be driven by a local training manifest. Demo fallback remains available. |
| Model governance | `apps/api-server/src/routes/ops_models.rs` enforces dataset, holdout, out-of-time, split, leakage, explanation, shadow, data quality, label, drift, feedback, and approval gates. | Strong governance skeleton. It still depends on real evaluation evidence being produced later. |

## What The Demo Can Claim

The demo can claim:

- end-to-end scoring runs through a model runtime boundary;
- risk is assembled from explainable rule, anomaly, provider, clinical, model,
  and routing layers;
- deterministic customer-approved rules are the only valid future authority for
  automatic denial or straight-through approval;
- every promoted model must satisfy dataset, feature, metric, split, leakage,
  shadow, drift, label, and approval gates;
- current model outputs are assistive risk signals and do not adjudicate,
  deny, approve, or accuse fraud.

The demo must not claim:

- the Python scorer is a trained production ML model;
- current metadata probabilities are statistically calibrated probabilities;
- the anomaly layer is a full unsupervised model;
- retraining jobs train real candidate models unless a `--training-manifest`
  path and trainer Python runtime are provided;
- model promotion evidence is real customer holdout or shadow evidence unless
  such evidence has been registered from an actual pilot.

## Algorithm Roadmap

### Stage 0: Current Demo Baseline

Keep the current deterministic scorer and seven-layer fusion. The purpose is to
prove the scoring contract, evidence trail, UI/API surfaces, and governance
workflow.

Required language:

- call the Python service a "demo scoring boundary" or "heuristic baseline";
- call sub-probability metadata "baseline risk components" unless calibration
  evidence exists;
- keep agent and ML outputs assistive-only;
- call auto-denial candidates "customer-approved deterministic adjudication
  rules", not model decisions.

### Stage 0A: Deterministic Adjudication Policy Design

Before adding production automatic denial or straight-through approval, define a
separate adjudication policy layer outside the ML model:

- action classes: `hard_deny`, `straight_through`, `pending_evidence`,
  `manual_review`, and `score_only`;
- output fields: `decision_outcome`, `decision_authority`,
  `decision_confidence`, `reason_code`, `appeal_or_review_required`, evidence
  refs, and audit ids;
- rule metadata: customer approval status, policy or clinical authority refs,
  applicability scope, exception checks, effective dates, rollback plan, and
  override route;
- examples: gender or age contraindication, coverage exclusion, waiting-period
  violation, expired coverage, duplicate claim identifier, provider
  ineligibility, and product ineligibility;
- fallback rule: if the exception check is unresolved or evidence is missing,
  route to `pending_evidence` or `manual_review` instead of `auto_deny`.

This layer can use model and anomaly outputs as supporting context, but they
must not determine `auto_deny` by themselves.

### Stage 1: Pilot Label Collection

Before training, collect stable labels and QA feedback:

- confirmed FWA, false positive, insufficient evidence, recovered amount, and
  reviewer disposition;
- label provenance and reviewer source;
- case family identifiers for leakage control;
- review mode, scheme family, provider, member, policy, product, region, and
  service-date context;
- a random control sample to estimate missed-risk and false-positive burden.

### Stage 2: First Real Supervised Models

Train offline only after pilot labels are available.

Candidate order:

1. Logistic regression with calibration for an interpretable baseline.
2. Decision tree or shallow tree ensemble for transparent rules-of-thumb.
3. XGBoost as the first primary supervised-learning challenger for structured
   claim-risk scoring, compared against the logistic and rule-only baselines.
4. LightGBM as the second gradient-boosted-tree candidate when training
   platform constraints or customer stack preference make it a better fit.

XGBoost or LightGBM candidates must carry feature importance or SHAP-style
artifacts, threshold evidence tied to review capacity, strict time/group
validation, ONNX serving artifacts, and probability-parity reports. Their
high-contribution feature patterns may be sent into Rule Studio as candidate
rules, but those candidates still require backtest, human review, promotion
gates, approval, and publication before they become active rules.

Do not make deep learning the default for structured claims scoring. Keep deep
or LLM models limited to OCR cleanup, document summarization, medical-note
extraction, clustering support, and investigation drafting.

### Stage 3: Validation And Promotion Evidence

Every candidate evaluation should record:

- immutable dataset version and feature-set version;
- time split field, group split fields, and split-status evidence;
- leakage check across member, policy, provider, and related case family;
- holdout and out-of-time metrics;
- PR-AUC or average precision, precision at review capacity, recall,
  false-positive burden, confusion matrix, calibration/Brier score, AUC, and KS;
- threshold selection tied to review capacity;
- rule-only and previous-model comparisons;
- feature importance or SHAP artifact URI;
- ONNX artifact URI, ONNX parity report URI, and maximum probability delta for
  XGBoost and LightGBM candidates;
- shadow-mode comparison against live traffic, QA outcomes, and routing impact;
- source data quality score, label provenance, pilot or customer validation,
  drift status, and human approval.

Minimum `metrics_json` fields for promotion-ready model evaluations:

```json
{
  "time_group_split_status": "passed",
  "time_split_field": "service_date",
  "group_split_fields": ["member_id", "policy_id", "provider_id", "case_family_id"],
  "leakage_check_status": "passed",
  "shadow_comparison_status": "passed",
  "review_capacity_threshold_status": "passed",
  "feature_reproducibility_hash": "sha256:<feature-build-hash>",
  "label_provenance_status": "passed",
  "label_reviewer_source": "qa_review",
  "pilot_validation_status": "passed",
  "source_data_quality_score": 0.95
}
```

### Stage 4: Production Serving

Before production impact, replace demo runtime assumptions with:

- artifact-backed model loading or a pinned model-serving endpoint;
- immutable model artifact URI, checksum, dependency lock, and serving image;
- online monitoring for latency, errors, input schema drift, feature drift,
  score drift, segment drift, calibration drift, and reviewer disagreement;
- rollback and previous-active-version audit evidence;
- separate thresholds for pre-payment and post-payment review modes;
- explicit separation between deterministic adjudication rules and model-driven
  review routing;
- clear SLOs for model service availability and fallback behavior.

## Open Implementation Gaps

The current plan is reasonable, but the remaining gaps are now mostly pilot-data
and production-operations gaps rather than missing demo mechanics:

- real customer or pilot labels with provenance, delayed-label handling, and
  reviewer-disagreement measurement;
- production-grade ONNX fixture coverage, latency SLOs, and monitoring around
  XGBoost, LightGBM, and deep-learning ONNX serving; the Rust runtime now has an
  artifact URI plus checksum-bound ONNX session cache, but it still needs
  customer-scale load and failure-mode evidence;
- production clustering and anomaly-discovery jobs over customer-scale unlabeled
  member, claim, provider, and graph features; the Rust worker covers demo
  provider-peer, provider graph-community, and claim-entity clustering with
  output treated as review candidates only;
- production feature store or scheduled feature materialization beyond the
  current manifest-backed offline baseline;
- calibrated probability outputs with calibration evidence and disjoint
  calibration data;
- real shadow traffic evaluation against live routing and QA outcomes, not just
  generated shadow comparison reports;
- model monitoring dashboards, alert routes, and incident response tied to live
  latency, error, drift, calibration, and segment metrics;
- production segment-level drift and fairness review using customer-approved
  cohorts and policy constraints;
- production object storage, artifact retention, legal hold, and signing-key
  management outside local/demo artifact paths;
- production serving registry and rollout automation for pinned serving images
  or endpoints.

## Decision

Keep the current algorithm architecture, but move the lifecycle toward a
Rust-owned Auto MLOps control plane. Logistic regression is the native baseline,
XGBoost is the primary production challenger, LightGBM is the next GBDT
candidate, and clustering/anomaly models support provider and claim discovery.
XGBoost and LightGBM do not need to be reimplemented in Rust; they now enter the
governed serving contract as ONNX artifacts only after parity tests pass, while
`.joblib` remains the training artifact or Python fallback. Deep learning
remains limited to non-structured evidence workflows unless a later
customer-approved validation package proves otherwise.
