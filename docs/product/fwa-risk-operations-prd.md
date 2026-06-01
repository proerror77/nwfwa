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

The product language must distinguish fraud, waste, abuse, improper payment,
documentation issue, and medical necessity issue. A high score, anomaly, or
rule hit is a lead, not a confirmed fraud finding.

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

### FWA Scheme Taxonomy

Every rule, feature, case, lead, model evaluation, and report should map to a
stable FWA scheme taxonomy.

Initial scheme families:

- duplicate billing;
- upcoding;
- unbundling;
- medically unnecessary service;
- excessive utilization;
- diagnosis-procedure mismatch;
- laboratory testing abuse;
- telehealth abuse;
- genetic testing abuse;
- opioid, pharmacy, or controlled-substance abuse;
- durable medical equipment, home health, hospice, or rehabilitation risk;
- provider peer outlier;
- suspicious referral, ownership, or relationship concentration.

The taxonomy is a product classification layer, not an ontology. It exists so
operators can route cases, compare rule performance, build evidence packages,
and measure ROI by FWA pattern.

### Provider Risk Profile

Provider risk profile is a first-class FWA capability. It tracks medical service
providers such as hospitals, clinics, doctors, departments, pharmacies, labs,
and rehabilitation facilities.

Required signals:

- claim volume and amount over 30/90/180 day windows;
- peer percentile by specialty, region, service type, and policy type;
- provider enrollment, credential, specialty, location, and network status;
- provider ownership or affiliation concentration when customer data provides
  it;
- provider-member, provider-referral, and high-risk-neighbor graph signals;
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
- scheme family and lead source;
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
- improper_payment;
- insufficient_evidence;
- abuse_not_fraud;
- documentation_issue;
- medical_necessity_issue;
- policy_exclusion;
- amount_prevented;
- amount_recovered;
- lead_disposition;
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
- avoided future exposure;
- deterrence or provider behavior-change indicators;
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
- audit-backed API call records for scoring, investigation, and QA writeback
  observability in Governance.

### Inbound Claim Inbox

Pilot integrations also need an inbound claim inbox before a customer-specific
payload is converted into the canonical scoring request. The inbox is the
boundary for raw TPA or claim-system messages. It should store the raw payload,
validate it, mask PII for downstream tools, and produce a normalized claim
context for `/api/v1/claims/score`.

Reference payload observed on 2026-06-01:

- source envelope resembles an `AiClaim Core` transaction with `systemCode`,
  `transDate`, `transNo`, and a nested `reportCase`;
- `reportCase` carries accident date, claim receive date, accident reason,
  calculate-risk flag, accident person identity, medical records, policies,
  invoices, products, and liability lists;
- dates are epoch milliseconds and must be normalized to UTC dates before
  feature calculation;
- member identity, certificate number, patient name, invoice person name, card
  number, and free-text medical record content are PII-bearing and must not be
  sent to LLM or Agent contexts without masking;
- medical records, invoice diagnoses, fee details, provider identity, policy
  liability windows, and product liability codes must be mapped explicitly
  instead of inferred from one free-text field.
- the reference payload includes one policy with 8 `productList` entries and 12
  `claimLiabilityList` entries; inbox normalization must preserve every
  product/liability coverage window instead of using only the first product and
  first liability.

Correction record for `/Users/proerror/Downloads/req.json`:

- keep the original file as raw intake evidence; do not rewrite customer
  payloads in place before scoring;
- route it through `POST /api/v1/inbox/claims/normalize`, then score only the
  normalized canonical context when `scoring_ready` is true or a reviewer has
  resolved blocking validation findings;
- derive `external_message_id` from `systemCode + transNo + reportNo`, and use
  hashed internal run, audit, raw-payload, and idempotency identifiers so raw
  claim identifiers are not leaked downstream;
- preserve every source medical record, invoice fee detail, product, and
  product-liability window as first-class canonical evidence;
- treat `calculateRisk = N` as a warning-level source hint unless customer
  configuration explicitly allows scoring bypass;
- flag identity mismatches between accident person, insured person, every
  invoice person, and medical-record patient rather than silently overwriting
  names;
- compare each invoice's structured diagnosis list against the medical-record
  diagnosis, including non-primary invoices, and emit
  `document_invoice_mismatch` on the exact `invoiceList[n].diagnosisList` path;
- allow normalized containment matches such as `牙周炎` versus `慢性牙周炎`,
  but do not use loose matching to hide unrelated diagnoses;
- if an invoice has bill lines but no structured diagnosis context, emit
  `diagnosis_item_mismatch` on that invoice's `feeList` path before L5 medical
  reasonableness scoring.
- compute canonical `claim_header.total_amount` as the sum of all source
  invoice `feeAmount` values, not only the primary invoice amount.

Required inbox corrections before scoring:

- idempotency: use `systemCode + transNo + reportNo` as the external message
  identity and reject or upsert duplicate submissions deterministically; internal
  audit ids, run ids, raw payload refs, and idempotency keys must use a stable
  checksum or fingerprint rather than raw external identifiers;
- source trace: persist raw payload URI or checksum/fingerprint, normalized claim
  id, mapping version, validation result, and evidence refs;
- date normalization: convert all epoch-millisecond dates and detect impossible
  or inconsistent accident, visit, invoice, policy, product, liability, and
  receive windows across the full source list, not only the primary product or
  liability;
- identity consistency: compare accident person, insured person, patient name,
  and every invoice person after masking, and raise a review signal when they
  do not align;
- medical consistency: map diagnosis codes, diagnosis names, department,
  medical type, fee categories, drugs, procedures, and medical record text into
  L5 medical-reasonableness inputs;
- policy coverage: map policy, product, and liability lists into coverage,
  waiting-period, limit, and liability eligibility features;
- text hygiene: normalize literal `/n` separators, OCR artifacts, missing
  spaces, empty fields, and mixed-language medical text before evidence
  extraction;
- risk intent: treat `calculateRisk = N` as a source-system hint, not as an
  instruction to bypass FWA scoring unless the customer config explicitly
  allows bypass;
- error handling: return structured inbox validation errors with field paths,
  severity, and remediation hints instead of failing silently or dropping
  fields.

The inbox should output a canonical payload with:

- claim header: external claim id, source system, service date, receive date,
  accident reason, medical type, currency, and claim-level total amount summed
  from all source invoices;
- member and policy snapshot: masked member id, policy id, product code,
  primary product/liability codes, all product-liability windows, policy type,
  and coverage constraints;
- provider snapshot: hospital/provider code, name, class, type, city, province,
  and network flags;
- itemized bill lines: every source invoice fee detail with invoice id,
  diagnosis list, fee category, item name, amount, self-pay, social-insurance
  amount, and evidence refs;
- document evidence: every source medical record with medical record text,
  extracted diagnosis, procedure, prescription, department, visit date, and
  source refs;
- data-quality signals: identity mismatch, missing fields, date inconsistency,
  document-invoice mismatch, diagnosis-item mismatch, and policy-liability
  mismatch.

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
- provider enrollment, credentialing, ownership, and network management systems
  where available;
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
- cross-payer or partner data collaboration where allowed by customer contracts,
  privacy rules, and governance approval.

Core rule evaluation, scoring aggregation, audit, and model governance must stay
inside the FWA platform. External systems provide data, documents, workflow
destinations, and outcome feedback.

Partner collaboration must be privacy-preserving and evidence-controlled. Shared
signals should use approved identifiers, aggregated patterns, hashed references,
or customer-approved exchange formats rather than raw PII.

## Infrastructure And Agentic Operating Model

The platform needs a staged infrastructure foundation before it needs a large
collection of specialist databases. The detailed engineering baseline is
documented in `docs/engineering/infrastructure-architecture.md`.

Required infrastructure principles:

- PostgreSQL is the transactional source of truth for claims, providers,
  policies, rules, models, cases, labels, jobs, audit events, and agent run
  metadata.
- Object storage is required for durable artifacts such as Parquet datasets,
  feature matrices, document evidence, OCR output, model artifacts, backtest
  reports, and evidence packages.
- Async worker jobs are first-class product infrastructure for imports,
  backtests, embeddings, graph projections, model evaluations, exports, and
  agent-run continuation.
- Agentic workflows must operate through governed platform tools, context
  snapshots, evidence refs, audit events, and approval gates. Agents may prepare
  evidence and propose actions, but they must not autonomously deny claims,
  publish rules, promote models, delete audit records, or export sensitive data.
- Optional infrastructure such as Redis, ClickHouse, Neo4j, OpenSearch, Qdrant,
  LanceDB, or Kubernetes is adopted only when a defined workload requires it.

Staged infrastructure roadmap:

- MVP: PostgreSQL, Rust API server, Rust worker, Python ML service, React
  console, migrations, seed scripts, and CI checks.
- Pilot foundation: object storage, durable job state, backups, structured
  logs, minimum metrics, secret management, data masking, retention policy, and
  customer network controls.
- AI evidence foundation: document registry, chunk registry, embedding jobs,
  retrieval audit, vector search starting with `pgvector` where sufficient, and
  agent run/step/context/approval records.
- Analytics scale: derived analytical event store, optionally ClickHouse, for
  high-volume scoring, rule, model, case, graph, SLA, and ROI reporting.
- Production hardening: infrastructure as code, SSO/RBAC, managed secrets,
  network isolation, OpenTelemetry dashboards, alerting, disaster recovery, and
  release/rollback runbooks.

The system should support agent-native operation by giving agents parity with
operator-readable capabilities through approved tools, while reserving
high-impact writes for human approval. This keeps the architecture compatible
with modern agentic workflows without turning the FWA platform into an
uncontrolled autonomous adjudication system.

## Lead Generation Lifecycle

Analytics creates leads. It does not directly create fraud conclusions.

Required lifecycle:

```text
signal -> lead -> triage -> case -> investigation -> outcome -> feedback
```

Definitions:

- signal: a rule hit, anomaly, model score, peer deviation, document finding, or
  external alert;
- lead: a review candidate with score, scheme family, source, and evidence refs;
- triage: operator review that accepts, rejects, merges, or requests more
  evidence;
- case: an opened investigation with owner, status, SLA, and evidence package;
- outcome: structured conclusion from investigation, QA, or customer workflow;
- feedback: governed labels and metrics used for rule tuning, feature quality,
  model evaluation, and ROI reporting.

Lead records must preserve why the lead was created and why it was promoted,
rejected, merged, or closed.

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

Evidence sufficiency depends on the scheme family:

| Scheme | Minimum Evidence |
| --- | --- |
| duplicate billing | same member, provider, service date, procedure, amount, and claim lineage |
| upcoding | diagnosis, billed code, lower-complexity comparator, medical record, and coding rationale |
| unbundling | component codes, bundled-code comparator, same episode, and billing timeline |
| medical necessity | diagnosis, order, chart note, treatment context, reviewer finding, and policy rule |
| lab overuse | ordering pattern, diagnosis match, frequency, peer benchmark, and ordering provider |
| provider outlier | peer group definition, time window, specialty, region, and statistical deviation |
| telehealth abuse | visit mode, provider/member location, visit frequency, documentation, and policy rule |
| pharmacy or opioid abuse | prescription, prescriber, fill pattern, dosage, member history, and policy rule |

Clinical review output must remain structured. Free-text notes can explain the
case, but model training and rule tuning must use controlled outcome fields such
as `documentation_issue`, `medical_necessity_review_required`, and
`insufficient_evidence`.

## Sampling And Audit Methodology

The platform should support both targeted FWA leads and statistically defensible
audit sampling.

Required sampling modes:

- risk-ranked sample for high-risk lead review;
- random control sample for baseline false-positive and missed-risk measurement;
- stratified sample by scheme, provider type, region, policy type, and risk
  band;
- post-payment audit sample for recovery and rule discovery;
- QA sample for reviewer consistency and workflow calibration.

Sampling records must store population definition, inclusion criteria, random
seed or deterministic selection method, sample size, reviewer assignment, and
outcome distribution.

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

## Data Quality And Reproducibility Gates

FWA accuracy depends on data quality as much as model choice. Every dataset,
feature set, rule backtest, and model evaluation should be reproducible.

Required gates:

- source data quality score;
- missingness, duplicate, outlier, and coding-distribution profiles;
- diagnosis, procedure, provider, policy, and member identifier normalization;
- provider and member identity-resolution lineage;
- label provenance and reviewer source;
- feature reproducibility hash;
- dataset, split, feature-set, model, rule, and threshold version ids;
- immutable artifact URIs for profiles, backtests, metrics, and feature
  importance.

No model, rule, or ROI report should be promoted from a dataset whose source,
split, feature generation, and label lineage cannot be replayed.

## Anti-Fraud Value Measurement

The product must measure more than recovered money.

Required value measures:

- prevented payment;
- recovered amount;
- avoided future exposure;
- deterrence or provider behavior-change signal;
- review cost and reviewer capacity used;
- false-positive operational cost;
- net value by rule, model, scheme, provider segment, and campaign;
- time to triage, time to investigation closure, and SLA breach rate;
- confidence interval or evidence caveat for estimates where exact attribution
  is not possible.

Value reports must separate observed financial outcomes from estimated impact.
Estimated deterrence and avoided future exposure must be labeled as estimates.

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

- CMS Fraud, Waste & Abuse:
  https://www.cms.gov/fraud
- CMS Center for Program Integrity:
  https://www.cms.gov/medicare/medicaid-coordination/center-program-integrity
- CMS Healthcare Fraud Prevention Partnership white papers:
  https://www.cms.gov/medicare/medicaid-coordination/healthcare-fraud-prevention-partnership/white-papers
- GAO Medicare fraud analytics report:
  https://files.gao.gov/reports/GAO-26-107799/index.html
- HHS OIG Fraud resources:
  https://oig.hhs.gov/fraud/
- Data-Centric AI for Healthcare Fraud Detection:
  https://pubmed.ncbi.nlm.nih.gov/37200563/
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
- No confirmed fraud language without investigation or QA confirmation.
- No partner data sharing without explicit customer, privacy, and governance
  approval.

## Acceptance Criteria

- Every score can be traced to feature values, rule hits, model signals, anomaly
  signals, and audit events.
- Every lead has a scheme family, lead source, evidence refs, and lifecycle
  disposition.
- Rule changes are versioned, backtested, approved, and publishable.
- Model versions are tied to immutable datasets, feature sets, evaluation runs,
  and runtime metadata.
- Candidate models have explicit anti-overfitting gates before production use.
- Pre-payment and post-payment policies are explicit for rules, models,
  thresholds, and recommended actions.
- Clinically sensitive findings are backed by structured evidence and routed to
  reviewers instead of being treated as autonomous conclusions.
- Evidence sufficiency is defined by scheme family.
- Data quality, feature reproducibility, and label provenance are required before
  model or ROI promotion.
- Value reporting separates prevented payment, recovered amount, avoided
  exposure, review cost, and estimated impact.
- Kaggle-inspired work remains an offline research input until validated on
  customer or pilot data.
