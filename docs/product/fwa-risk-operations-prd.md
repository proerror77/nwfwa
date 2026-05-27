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

## FWA Core Capability Roadmap

The product must move from a generic scoring platform into a domain-specific FWA
operations system. The following capabilities are required to strengthen the
core product.

### Provider Risk Profile

Provider risk profile is a first-class FWA capability. It tracks medical service
providers such as hospitals, clinics, doctors, departments, pharmacies, labs,
and rehabilitation facilities.

Required signals:

- claim volume and amount over 30/90/180 day windows;
- peer percentile by specialty, region, service type, and policy type;
- high-cost code usage rate;
- duplicate or repeated service patterns;
- diagnosis-procedure mismatch rate;
- review failure, confirmed FWA, and false-positive history;
- sudden volume increase for new or previously low-activity providers.

### FWA Rule Pack

The rule library should evolve from demo rules into reusable FWA rule packs.

Required rule families:

- duplicate claim;
- upcoding;
- unbundling;
- medically unnecessary service;
- excessive utilization;
- early high-value claim after policy start;
- provider peer outlier;
- same member repeated service;
- diagnosis-procedure mismatch;
- suspicious provider-member or referral concentration.

Each rule must keep version, owner, lifecycle status, applicability scope,
backtest result, estimated saving, false-positive history, and evidence refs.

### Investigation Case Management

Scoring is not enough. FWA operators need a case workflow for triage,
investigation, review, and closure.

Required workflow fields:

- case id, claim id, member id, provider id, and source system;
- status: new, triage, investigating, pending evidence, confirmed, rejected,
  closed;
- assignee, reviewer, SLA, priority, and reason for routing;
- evidence package with claim, rule, model, anomaly, document, and similar-case
  references;
- reviewer notes and final outcome writeback.

### Feedback And Label Governance

Human review outcomes must become structured labels. These labels are the basis
for rule tuning, model evaluation, and future training.

Required labels:

- confirmed_fwa;
- false_positive;
- insufficient_evidence;
- abuse_not_fraud;
- documentation_issue;
- policy_exclusion;
- amount_prevented;
- amount_recovered;
- feedback_target: rules, model, features, provider_profile, or workflow.

### Feature Factory And Peer Benchmark

The platform needs durable feature families, not one-off fields.

Required feature groups:

- claim-level amount, timing, and diagnosis/procedure features;
- member-level recurrence and utilization features;
- provider-level aggregation and peer deviation features;
- policy-level coverage, waiting-period, and limit features;
- time-window features over 7/30/90/180 days;
- network concentration features for provider-member and provider-agent
  relationships.

### Outcome, Drift, And ROI Monitoring

FWA value must be measurable.

Required monitoring:

- rule hit rate and false-positive rate;
- model shadow-mode performance and drift;
- precision at review capacity;
- reviewer disagreement rate;
- prevented payment;
- recovered amount;
- review cost;
- rule-level and model-level saving attribution.

## Integration Roadmap

The product should integrate with TPAs and customer insurance systems through
explicit adapters. It should not hard-code one vendor, one data format, or one
claim administration platform into the core runtime.

### MVP Integration

MVP supports a generic TPA scoring integration:

- inbound claim scoring through `POST /api/v1/claims/score`;
- stored claim scoring by `claim_id`;
- API-key authentication;
- scoring response with risk score, alerts, recommended action, evidence refs,
  and audit ids;
- investigation and QA result writeback for pilot feedback.

### Pilot Integration Targets

Pilot customers may connect some or all of the following systems:

- TPA claim administration system for claim intake and scoring responses;
- policy administration system for coverage, limits, waiting periods, and
  policy status;
- member eligibility system for member identity, plan, and enrollment status;
- provider master data system for provider identity, specialty, region, and
  network status;
- document management or OCR system for medical records, invoices, receipts,
  prescriptions, and discharge summaries;
- payment or remittance system for paid amount, denied amount, recovery amount,
  and payment status;
- investigation, SIU, QA, or case-management tools for reviewer workflow and
  outcome writeback;
- data warehouse, lakehouse, or object storage for historical claims, Parquet
  datasets, feature matrices, and model evaluation artifacts.

### Later Enterprise Integrations

Later phases can add:

- batch import/export through customer-approved file drops;
- event webhooks for score completed, case routed, investigation closed, and QA
  result received;
- standards-oriented adapters where customers require them, such as claim,
  eligibility, remittance, or clinical-document exchange formats;
- SSO and role-based access control;
- BI export for finance, compliance, and operations reporting;
- alerting and notification systems for SLA breach and high-risk routing.

Core rule evaluation, scoring aggregation, audit, and model governance must stay
inside the FWA platform. External systems provide data, documents, workflow
destinations, and outcome feedback.

## Review Mode Strategy

The platform must distinguish pre-payment and post-payment workflows because
they have different risk tolerance, review capacity, and ROI logic.

Pre-payment review happens before money leaves the payer. It should optimize for
high precision, clear explanations, and low operational harm. Recommended
actions may include manual review, request evidence, hold for reviewer, or allow
with audit flag. A pre-payment intervention must have strong evidence and a
customer-controlled review path.

Post-payment audit happens after payment. It can optimize for broader recall,
pattern discovery, recovery opportunities, and rule improvement. Recommended
actions may include audit queue, provider review, recovery review, rule tuning,
or model evaluation. Post-payment findings may be used for training labels only
after QA or investigation confirms the outcome.

Every rule, model, threshold, and routing policy must declare whether it applies
to pre-payment, post-payment, or both.

## Clinical Evidence And Medical Necessity

FWA scoring must cover more than amount anomalies. Many high-value cases depend
on whether the billed service was medically necessary and supported by
documentation.

Required capabilities:

- compare diagnosis, procedure, medication, and service location for basic
  clinical consistency;
- detect claim items that need additional medical records, invoices,
  prescriptions, discharge summaries, or lab results;
- link document evidence to claim items and rule/model findings through evidence
  refs;
- flag missing or contradictory evidence without making an autonomous clinical
  judgment;
- route clinically sensitive cases to medical or QA reviewers;
- use OCR and LLM assistance only for extraction, summarization, evidence
  organization, and checklist generation.

Clinical review output must remain structured. Free-text notes can explain the
case, but model training and rule tuning must use controlled outcome fields such
as `documentation_issue`, `medical_necessity_review_required`, and
`insufficient_evidence`.

## Promotion Gates

Rules and models must earn the right to affect routing. The product should make
promotion gates visible in Operations Studio instead of treating configuration
changes as immediate production behavior.

Rule promotion requires:

- named owner and applicability scope;
- deterministic backtest against representative samples;
- estimated saving and expected false-positive burden;
- evidence refs for the pattern the rule claims to detect;
- approval before publish;
- shadow or limited rollout for high-impact rules;
- rollback path to the previous active version.

Model promotion requires:

- immutable dataset and feature-set versions;
- leakage checks across member, policy, provider, and related-case groups;
- holdout and out-of-time metrics;
- threshold selection tied to review capacity;
- explanation artifact such as feature importance or SHAP-style analysis;
- shadow-mode comparison against rules, previous model, and QA outcomes;
- approval before the model affects recommended actions.

Promotion gates apply separately to pre-payment and post-payment use. A rule or
model may be acceptable for post-payment audit while still being too risky for
pre-payment routing.

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
- No pre-payment routing impact without explicit promotion gates and rollback.

## Acceptance Criteria

- Every score can be traced to feature values, rule hits, model signals, anomaly
  signals, and audit events.
- Rule changes are versioned, backtested, approved, and publishable.
- Model versions are tied to immutable datasets, feature sets, evaluation runs,
  and runtime metadata.
- Candidate models have explicit anti-overfitting gates before production use.
- Pre-payment and post-payment policies are explicit for rules, models,
  thresholds, and recommended actions.
- Clinically sensitive findings are backed by structured evidence and routed to
  reviewers instead of being treated as autonomous conclusions.
- Kaggle-inspired work remains an offline research input until validated on
  customer or pilot data.
