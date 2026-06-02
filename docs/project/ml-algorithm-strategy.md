# ML Algorithm Strategy

Last researched: 2026-06-02

This document records the current external research and implementation
assessment for the FWA model plan. It should be read with the PRD modeling
section, model APIs, and scoring implementation.

## Executive Assessment

The planned algorithm direction is appropriate for the current product stage:
rule-first, explainable-model-first, and governed promotion before any model can
affect routing.

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
| Python ML service | `apps/ml-service/app/scorer.py` keeps the deterministic baseline fallback and can load a trained `.joblib` artifact through `FWA_MODEL_ARTIFACT_URI`. `apps/ml-service/app/training.py` trains a logistic-regression baseline from a Parquet manifest and writes model, validation, and feature-importance artifacts. | Minimum production ML slice is now artifact-backed for local/offline use. It is still not a full feature store, shadow evaluation system, or monitored production serving platform. |
| Rust model runtime | `crates/fwa-ml-runtime` supports HTTP scoring, heuristic fallback, model identity checks, score-range validation, explanations, metadata, and latency. | Good runtime boundary for pilot integration. Production still needs pinned serving identity and artifact checksum enforcement. |
| Feature layer | `crates/fwa-features` emits claim amount ratio, peer-percentile baseline, item count, high-cost item ratio, diagnosis/procedure match, and provider tier/profile features. | Suitable for demo. Production needs rolling provider/member/history, peer cohort, duplicate/similarity, graph, and label-delay features. |
| Anomaly layer | `crates/fwa-anomaly` uses explainable threshold signals. | Reasonable explainable anomaly baseline. Not yet an unsupervised anomaly model. |
| Risk fusion | `crates/fwa-scoring` combines seven layers with explicit weights and evidence refs. | Good demo and pilot decision-support structure. Weights are policy defaults, not learned coefficients. |
| Retraining worker | `apps/worker` can keep the deterministic mock candidate output for demo smoke, or call `python -m app.train` when `run-retraining-job` receives `--training-manifest`. | Real candidate registration can now be driven by a local training manifest. Demo fallback remains available. |
| Model governance | `apps/api-server/src/routes/ops_models.rs` enforces dataset, holdout, out-of-time, split, leakage, explanation, shadow, data quality, label, drift, feedback, and approval gates. | Strong governance skeleton. It still depends on real evaluation evidence being produced later. |

## What The Demo Can Claim

The demo can claim:

- end-to-end scoring runs through a model runtime boundary;
- risk is assembled from explainable rule, anomaly, provider, clinical, model,
  and routing layers;
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
- keep agent and ML outputs assistive-only.

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
3. XGBoost or LightGBM only if it beats rule-only and baseline models under
   strict validation, and only with feature importance or SHAP artifacts.

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
- clear SLOs for model service availability and fallback behavior.

## Open Implementation Gaps

The current plan is reasonable, but these gaps remain before production ML:

- real offline training pipeline;
- real model artifact generation and loading;
- real feature store or reproducible feature materialization;
- calibrated probability outputs;
- real shadow traffic evaluation;
- model monitoring dashboards and alerts;
- segment-level drift and fairness review;
- reviewer disagreement and label-delay handling;
- production object storage and artifact retention policy.

## Decision

Keep the current algorithm architecture. Do not replace it with a deep-learning
first approach. The next correct step is to preserve the demo baseline while
building the real offline training and evaluation pipeline behind the existing
governance gates.
