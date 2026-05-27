# FWA Risk And Operations Platform PRD

Date: 2026-05-27

Status: living product blueprint

## Product Goal

Build a health-insurance FWA risk and operations platform that helps TPA and
insurance operations teams detect suspicious claims, explain the risk evidence,
route cases for review, and keep every rule, model, and agent-assisted output
auditable.

The product should optimize for explainability, workflow traceability, and
operator control before autonomous decisioning.

## Decision Boundary

The platform is assistive. It can recommend review actions, surface suspicious
patterns, and prepare evidence packages, but it must not automatically deny,
approve, or accuse a claim without a customer-controlled adjudication process.

## Product Modules

- FWA Core Runtime: score claims through features, rules, model signals, anomaly
  signals, confidence routing, and audit persistence.
- FWA Operations Studio: let operators inspect scores, rules, datasets, models,
  cases, QA feedback, and pilot readiness.
- TPA Integration API: accept claim payloads or stored claim identifiers and
  return traceable scoring responses.
- Rule Studio: manage rule versions, lifecycle, deterministic backtests, and
  publication status.
- Model Operations: track model versions, feature sets, evaluation runs, shadow
  status, and runtime performance.
- Knowledge And Agent Workflows: support similar-case lookup and deterministic
  investigation packages with evidence references.
- QA And Feedback Loop: capture human review outcomes that improve rules,
  model evaluation, and future pilot thresholds.

## Modeling Strategy

The core FWA decision surface should be rule-first and explainable-model-first.
Deep learning must not be the default model for structured risk scoring.

MVP should use:

- deterministic rules for clear fraud, waste, and abuse patterns;
- simple baseline scoring to exercise the model runtime boundary;
- anomaly scoring to prioritize review candidates;
- human QA feedback to create labeled evidence for later training.

Production model candidates should start with interpretable or inspectable
structured models:

- logistic regression for calibrated baselines;
- decision trees for transparent rules-of-thumb;
- gradient-boosted trees, such as XGBoost or LightGBM, only with feature
  importance, SHAP-style explanation, and strict validation gates.

Large language models or deep models may support OCR cleanup, document summary,
medical-note extraction, clustering, and investigation drafting. They must not
directly decide fraud status or final claim disposition.

## Training Strategy

The platform does not need custom model training to prove the MVP. It needs a
working scoring runtime, rules, audit trail, dataset registration, and human
review loop first.

Training becomes useful after enough customer or pilot data has stable labels:

1. Register immutable Parquet datasets and feature-set versions.
2. Split by time and by leakage-sensitive groups such as member, policy,
   provider, and case family.
3. Train offline only; do not let the trained model influence decisions.
4. Record model dataset version, feature-set version, metrics, threshold, and
   feature importance artifact.
5. Run shadow mode against live traffic and compare against rules and human QA.
6. Promote only when holdout, out-of-time, and pilot review metrics pass.

Overfitting controls are product requirements, not optional data-science notes:

- use time-based holdout and out-of-time validation;
- prevent provider, member, policy, and related-case leakage across splits;
- block post-investigation fields and final adjudication artifacts from feature
  sets unless explicitly approved as labels;
- report PR-AUC, precision at review capacity, recall, false-positive burden,
  confusion matrix, calibration, AUC, and KS;
- compare every candidate against rule-only and previous-model baselines;
- require shadow-mode evidence before active routing impact.

## Kaggle-Inspired Strategy

Public Kaggle fraud and healthcare-provider datasets are useful for research
patterns, not production conclusions. They should inform feature engineering,
validation design, and offline experiments only.

Reusable ideas:

- Provider-level aggregation: evaluate behavior across provider, specialty,
  geography, diagnosis, procedure, and time windows instead of only one claim.
- Peer deviation: compare a provider or claim against similar providers,
  specialties, regions, policy types, and service categories.
- Frequency and ratio features: count repeat services, high-cost codes,
  duplicate claim patterns, claim-to-limit ratios, and same-member recurrence.
- Unsupervised anomaly detection: surface unusual provider or claim clusters as
  review candidates, not as final fraud labels.
- Imbalanced evaluation: optimize for review-capacity precision, recall on
  confirmed FWA, and false-positive cost rather than raw accuracy.

Candidate FWA feature families:

- claim amount to policy limit ratio;
- provider 30/90/180 day claim volume and average claim amount;
- provider peer percentile by specialty and region;
- diagnosis-procedure mismatch flag;
- high-cost code usage rate;
- same member, same provider, same diagnosis recurrence;
- duplicate claim similarity score;
- new policy early claim flag;
- provider concentration by diagnosis or procedure code;
- member and provider network relationship signals.

Borrow with caution:

- Do not ship leaderboard-style ensembles that cannot be explained or replayed.
- Do not use pseudo-labeling in production governance paths.
- Do not trust random train/test splits for FWA; time and group split are
  required.
- Do not optimize only AUC or accuracy.
- Do not use public Kaggle data as proof that a customer production model is
  effective.

Reference anchors:

- Kaggle Healthcare Provider Fraud Detection Analysis dataset:
  https://www.kaggle.com/datasets/rohitrox/healthcare-provider-fraud-detection-analysis
- Kaggle IEEE-CIS Fraud Detection competition:
  https://www.kaggle.com/c/ieee-fraud-detection
- NVIDIA write-up on the IEEE-CIS winning fraud-detection solution:
  https://developer.nvidia.com/blog/leveraging-machine-learning-to-detect-fraud-tips-to-developing-a-winning-kaggle-solution/

## Non-Goals

- No MVP semantic layer or ontology system.
- No autonomous fraud accusation or claim denial.
- No deep-learning-first structured risk scoring.
- No automatic model retraining loop before pilot labels and QA governance exist.
- No production model promotion without dataset, feature, metric, and shadow-mode
  evidence.

## Acceptance Criteria

- Every score can be traced to feature values, rule hits, model signals, anomaly
  signals, and audit events.
- Rule changes are versioned, backtested, approved, and publishable.
- Model versions are tied to immutable datasets, feature sets, evaluation runs,
  and runtime metadata.
- Candidate models have explicit anti-overfitting gates before production use.
- Kaggle-inspired work remains an offline research input until validated on
  customer or pilot data.
