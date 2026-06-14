# FWA Risk And Operations Platform PRD

Date: 2026-05-27
Last updated: 2026-06-13

Status: living product blueprint

## Product Goal

Build a health-insurance FWA risk and operations platform that helps TPA and
insurance operations teams detect suspicious claims, explain the risk evidence,
route cases for review, and keep every rule, model, and agent-assisted output
auditable.

The product should optimize for explainability, workflow traceability, and
operator control before autonomous decisioning.

## Decision Boundary

The platform is assistive by default. It can recommend review actions, surface
suspicious patterns, and prepare evidence packages, but it must not
automatically deny, approve, or accuse a claim without a customer-controlled
adjudication process.

Pre-payment integrations may support customer-approved deterministic
adjudication rules. Those rules are not ML decisions. They are explicit policy,
eligibility, coverage, or clinical-compatibility checks that the customer has
approved for straight-through processing. Any automatic denial or automatic
approval must include rule id, rule version, customer policy authority, input
field evidence, exception-check outcome, audit id, and appeal or reviewer
override path. ML scores, anomaly scores, provider graph signals, similar-case
signals, and Agent outputs must never be the sole authority for automatic
denial or automatic approval.

The product language must distinguish fraud, waste, abuse, improper payment,
documentation issue, and medical necessity issue. A high score, anomaly, or
rule hit is a lead, not a confirmed fraud finding.

### Decision Routing Outcomes

Real-time claim scoring should route each claim into one of the following
outcomes:

- `straight_through`: low-risk or explicitly eligible claim can continue normal
  processing under the customer's policy;
- `auto_deny`: a customer-approved deterministic rule establishes ineligibility,
  exclusion, contradiction, or invalid coverage with no unresolved exception;
- `pending_evidence`: required documents or clinical evidence are missing, so
  payment is paused until the evidence request is resolved;
- `manual_review`: rules, model signals, anomaly signals, provider signals, or
  clinical uncertainty require human review;
- `qa_sample`: low or medium risk claims are sampled for quality assurance,
  calibration, or missed-risk measurement;
- `post_payment_audit`: post-payment mode routes claims to audit, recovery
  review, or provider review instead of pre-payment hold or denial.

The scoring response should expose both the business outcome and the authority
behind it:

- `decision_outcome`;
- `decision_authority`: `customer_policy_rule`, `clinical_policy_rule`,
  `risk_routing_policy`, `human_reviewer`, or `qa_policy`;
- `decision_confidence`: `deterministic`, `high`, `medium`, or `low`;
- `reason_code`;
- `appeal_or_review_required`;
- evidence refs and audit ids.

### Rule Action Classes

Rules are not all equivalent. The rule library must classify each production
rule by action class:

- `hard_deny`: deterministic policy, eligibility, coverage, or clinical
  contradiction rules that can deny only after customer approval and exception
  checks;
- `straight_through`: deterministic eligibility or low-risk rules that can allow
  normal processing only under customer policy;
- `pending_evidence`: evidence sufficiency rules that request documents such as
  X-ray images, prescriptions, medication details, lab reports, or medical
  records;
- `manual_review`: suspicious or clinically uncertain rules that require human
  review;
- `score_only`: rules that contribute risk, prioritization, or monitoring
  signals but cannot adjudicate.

Examples of hard-deny candidates include gender or age contraindications,
coverage exclusion, policy waiting-period violation, expired coverage, duplicate
claim identifiers, and provider or product ineligibility. Each candidate must
carry exception logic; if an exception cannot be resolved deterministically, the
outcome must be `pending_evidence` or `manual_review`, not `auto_deny`.

## Input Box And Canonical Evidence Trace

The raw TPA inbox payload at `/Users/proerror/Downloads/req.json` is a
`reportCase` envelope with `systemCode`, `transDate`, and `transNo`; it is not
the final scoring contract by itself. Runtime scoring should normalize this
shape into `source_system` plus `canonical_claim_context` before risk detection.

Required normalization:

- map `reportCase.id` or `reportCase.reportNo` to
  `canonical_claim_context.claim_header.external_claim_id`;
- map policy/member/provider snapshots from `reportCase.policyList`,
  `reportCase.accidentPerson`, and available organization/provider fields;
- map each invoice fee detail under
  `reportCase.policyList[*].invoiceList[*].feeList[*].feeDetailList[*]` into
  an itemized bill line with amount, code/name/category, source path, and a
  stable evidence ref such as `invoice:{invoice_id}:fee_detail:{line_id}`;
- map `reportCase.medicalRecordInfoList` into document evidence with source
  refs such as `medical_record:{document_id}`;
- keep raw source paths in `canonical_claim_context_trace.source_refs` and
  keep generated evidence refs in `canonical_claim_context_trace.evidence_refs`.

The canonical trace is part of the audit contract. Once a claim has a successful
`scoring.completed` audit event with `canonical_claim_context_trace`, downstream
QA result writeback, investigation result writeback, medical review result
writeback, Agent context snapshots, and knowledge case publish must preserve
those canonical evidence refs instead of replacing them with later workflow-only
refs. This lets an auditor start from a confirmed knowledge case or QA outcome
and trace back to the original normalized inbox invoice line or medical record.

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
- sudden volume increase for new or previously low-activity providers;
- OIG exclusion list and SAM.gov debarment status; these signals must be
  refreshed at minimum daily and evaluated before claim scoring begins; claims
  from an excluded or debarred provider must be flagged regardless of
  claim-level risk score;
- peer percentile signals must derive from statistical distributions computed
  over actual peer claim populations segmented by specialty, region, and service
  type; synthetic proxies such as amount-to-limit ratios must not substitute for
  real peer percentiles in any production scoring layer; peer benchmarks must be
  recomputed at minimum monthly with p25, p50, p75, p90, and p99 quantiles
  stored per segment;
- billing ring membership score: provider sets sharing elevated patient overlap
  within rolling 30-day windows, flagged when pairwise patient-overlap ratio
  exceeds a configurable threshold;
- temporal co-billing frequency: rate at which the provider and a secondary
  provider both appear in the same member's claims within a 7-day window, used
  to detect coordinated billing patterns without requiring multi-hop graph
  inference;
- referral concentration entropy: Shannon entropy of the provider's
  referral-source distribution; low entropy flags concentrated referral chains
  as a higher-risk signal than raw referral volume alone;
- multi-window risk scores must use a weighted combination that gives higher
  weight to shorter windows for near-term surveillance while using longer windows
  as background context; the maximum of all window scores must not be the sole
  aggregation method.

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
action class, backtest result, estimated saving, false-positive history, and
evidence refs. Hard-deny and straight-through rules must also keep customer
approval status, policy or clinical authority refs, exception-check logic,
effective dates, rollback plan, and appeal or override route.

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
- ineligible_service;
- clinical_contraindication;
- auto_deny_upheld;
- auto_deny_overturned;
- pending_evidence_resolved;
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
  relationships;
- episode-level aggregation features spanning all claims for a member-provider
  pair within 30, 90, and 365-day windows, covering total episode cost, episode
  DRG mix deviation from peer episodes, and episode-level diagnosis-procedure
  coherence; episode-level features complement claim-level features and are
  required for unbundling and utilization-pattern detection;
- ICD-10 and CPT unbundling detection features: presence of component codes
  alongside bundled-code comparators within the same episode window,
  supplemented by fee-detail line count and service-category concentration;
- peer percentile production requirement: all peer deviation features must derive
  from statistical distributions over actual peer claim populations; ratio-to-limit
  proxies are acceptable as labeled temporary baselines only and must be clearly
  marked in the feature registry until replaced by real peer data.

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
- rule-level and model-level saving attribution;
- feature distribution drift using Population Stability Index (PSI) computed
  monthly over key scoring features; PSI above 0.25 on any tier-1 feature must
  generate a review task for the model or rule that uses it;
- rule hit rate trending: a 7-day rolling hit rate and a 90-day rolling hit rate
  must be maintained per production rule; when the 7-day rate falls below 50
  percent of the 90-day baseline without a seasonal explanation, the platform
  must open a drift review task and may automatically trigger a shadow rerun
  with relaxed thresholds to detect potential provider evasion.

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
- dates are epoch milliseconds and must be normalized with the source business
  timezone before feature calculation, while preserving the raw epoch value for
  audit trace;
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

Reference payload refreshed on 2026-06-02 from
`/Users/proerror/Downloads/req.json`:

- source envelope has top-level `systemCode`, `transNo`, `transDate`, and
  `reportCase`; `reportCase.reportNo` remains the claim-level external
  reference;
- `calculateRisk` is `N`; this should be kept as a validation warning and
  routing hint, not treated as permission to drop the claim from the FWA
  normalization/audit path;
- the sample has one policy, one invoice, 8 products, and 12 liability windows;
  the invoice has 2 fee groups and 2 fee-detail rows, so bill-line evidence
  must be read from `invoiceList[].feeList[].feeDetailList[]`;
- medical records are carried under root-level
  `reportCase.medicalRecordInfoList`, not under each policy. The inbox adapter
  must not assume `policyList[].medicalRecordList` is the only medical-record
  path;
- claim-level amount fields may be absent or null. Canonical claim amount must
  fall back to source invoice totals and retain fee-detail evidence instead of
  inventing a claim header amount;
- invoice-level provider context includes hospital code, name, class, property,
  city, province, institution flag, primary-care flag, and red-flag marker.
  These values must remain attached to provider snapshot and invoice-derived
  bill lines for L6 Provider/Graph Risk attribution.
- date fields in this sample are China-business dates encoded as epoch
  milliseconds. For example, `1766678400000` is `2025-12-25 16:00:00 UTC` but
  `2025-12-26 00:00:00 Asia/Shanghai`, matching the medical-record text
  service date. Inbox normalization must therefore use the source business
  timezone, not a raw UTC date cut, or service, accident, invoice, and visit
  dates will shift one day earlier.
- the current sample also has an identity inconsistency:
  `reportCase.accidentPerson.insuredName` and policy `insuredName` are
  `LEE, Peter`, while `invoiceList[0].accidentPersonName` is `王向龙` and
  `medicalRecordInfoList[0].patientName` is empty. The adapter should flag this
  for review instead of overwriting invoice or patient names.

Correction record for `/Users/proerror/Downloads/req.json`:

- keep the original file as raw intake evidence; do not rewrite customer
  payloads in place before scoring;
- route it through `POST /api/v1/inbox/claims/normalize`, then score only the
  normalized canonical context when `scoring_ready` is true or a reviewer has
  resolved blocking validation findings;
- use the mock client's `--normalize-only` mode as the correction gate. When
  `scoring_ready = false`, the printed JSON must include `correction_hints`
  that identify blocking fields and next actions, including matching the local
  API key source-system config to `systemCode = "AiClaim Core"` and mapping
  `reportCase.policyList[0].coverageLimit` before direct scoring. For supported
  blocking fields, the same output should include `correction_overlay_template`
  so operators can copy a minimal overlay scaffold instead of editing the raw
  intake file. The overlay template must cover policy coverage limits and
  policy/product/liability date-window fields such as `validateDate`,
  `expireDate`, and `claimValidateDate`. Operators may use
  `--write-correction-template` in normalize-only mode to save this scaffold to
  a local correction file; existing correction files must not be overwritten
  unless `--overwrite-correction-template` is explicitly passed;
- support local correction overlays via `--inbox-correction-file` so operators
  can validate fixes without rewriting the raw customer payload. The overlay is
  merged in memory before normalization; object fields merge by key and arrays
  merge by index, allowing minimal patches such as
  `reportCase.policyList[0].coverageLimit = 20000`;
- when the normalized scoring run later enters QA, merge its canonical evidence
  refs into `POST /api/v1/qa/results` so QA conclusions and audit events remain
  traceable to the original bill-line and document evidence;
- when the normalized scoring run later enters Investigation Result writeback,
  merge its canonical evidence refs into `POST /api/v1/investigations/results`
  so confirmed FWA labels, saving attribution, and investigation audit events
  remain traceable to the original fee-detail evidence;
- when the normalized scoring run enters Medical Review, expose canonical
  source/evidence refs in `/api/v1/ops/medical-review/queue` and merge
  canonical evidence refs into `/api/v1/ops/medical-review/results` so L5
  medical conclusions remain traceable to the same original fee-detail evidence;
- derive `external_message_id` from `systemCode + transNo + reportNo`, and use
  hashed internal run, audit, raw-payload, and idempotency identifiers so raw
  claim identifiers are not leaked downstream;
- preserve every source medical record, invoice fee detail, product, and
  product-liability window as first-class canonical evidence;
- read medical records from `reportCase.medicalRecordInfoList` and preserve
  their source paths and source refs even when `policyList[].medicalRecordList`
  is empty or not present;
- read bill lines from the nested fee-detail path
  `policyList[].invoiceList[].feeList[].feeDetailList[]`; do not infer a single
  bill line from invoice totals, and preserve each fee detail's exact source
  path;
- when claim-level amounts are missing, derive canonical totals from invoice
  totals and keep the missing claim header amount as a data-quality condition
  instead of overwriting the raw payload;
- normalize all epoch-millisecond business dates with the configured source
  timezone, defaulting this `AiClaim Core` sample to `Asia/Shanghai`; retain the
  original epoch milliseconds and timezone metadata on canonical evidence for
  audit and dispute review.
- fix the normalizer contract before relying on date features: this sample's
  `accidentDate`, invoice `startDate`/`endDate`, and medical-record `visitDate`
  should normalize to `2025-12-26`, not `2025-12-25`;
- treat `calculateRisk = N` as a warning-level source hint unless customer
  configuration explicitly allows scoring bypass;
- flag identity mismatches between accident person, insured person, every
  invoice person, and every medical-record patient rather than silently
  overwriting names;
- for this sample, emit an identity-review signal because the invoice person
  name differs from the accident/policy insured name and the medical-record
  patient name is blank. Do not fill the blank patient name from the insured
  name unless a customer-approved mapping rule says so;
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
- compare `claimReceiveDate` with `accidentDate`; an accident date after
  receive date must emit `date_inconsistency` on the exact
  `reportCase.accidentDate` path.
- compare `claimReceiveDate` with every invoice `startDate`; non-primary
  invoice dates after receive date must emit `date_inconsistency` on the exact
  `invoiceList[n].startDate` path.
- validate every invoice date window; any `endDate` earlier than `startDate`
  must emit `date_inconsistency` on the exact `invoiceList[n].endDate` path.
- compare `claimReceiveDate` with every medical-record `visitDate`; non-primary
  medical-record visits after receive date must emit `date_inconsistency` on
  the exact `medicalRecordInfoList[n].visitDate` path.
- compare `claimReceiveDate` with every medical-record `firstHappenDate`;
  first-happen dates after receive date must emit `date_inconsistency` on the
  exact `medicalRecordInfoList[n].firstHappenDate` path.
- compare `claimReceiveDate` with every medical-record `operationStartDate`;
  operation dates after receive date must emit `date_inconsistency` on the
  exact `medicalRecordInfoList[n].operationStartDate` path.
- preserve structured medical-record fields from `medicalRecordInfoList` for L5
  medical reasonableness and QA evidence: `claimNature`, `medicalRecordType`,
  `chiefComplaint`, `currentMedicalHistory`, and `pastHistory`. Text fields
  must be normalized and redacted before API-visible output.
- normalize medical-record text before evidence extraction and API output:
  convert literal `/n` separators to line breaks, drop BOM/replacement-character
  OCR artifacts, normalize full-width or non-breaking spaces, collapse repeated
  line-internal whitespace, and remove empty lines before PII redaction.
- preserve raw source paths on every medical-record evidence row:
  `document_evidence[n].source_path` must point to
  `reportCase.medicalRecordInfoList[n]` so QA, Agent summaries, and audit review
  can trace normalized text and extracted diagnoses back to the exact source
  record.
- preserve invoice-level payment context from this payload on every canonical
  bill line: `billType`, `documentType`, `socialInsuranceType`,
  `departmentName`, `medicalType`, `claimNature`, invoice start/end dates, and
  invoice totals for Medicare, self-pay, own-expense, and other-payment
  amounts.
- preserve invoice-level provider context from this payload on every canonical
  bill line: `hospitalCode`, `hospitalName`, `hospitalClass`,
  `hospitalProperty`, `hospitalCityName`, `hospitalProvinceName`,
  `isHospitalInstitution`, `primaryCare`, and `redFlag`.
- preserve fee-group and fee-detail payment context separately:
  `feeList[n].feeAmount`, `feeList[n].otherAmount`, and
  `feeDetailList[n].medicareProrated` must not be collapsed into the detail
  `amount`. These fields feed L1 cost deviation, L5 medical reasonableness, and
  L7 routing explanations.
- preserve raw source paths on every bill-line evidence row:
  `itemized_bill_lines[n].source_path` must point to
  `reportCase.policyList[p].invoiceList[i].feeList[f].feeDetailList[d]` so L1,
  L5, QA, Agent summaries, and audit review can trace normalized fee details
  back to the exact source row.
- preserve product-liability routing markers from `claimLiabilityList`: parse
  `isSeriousDiseaseLiability` values such as `Y`/`N` into booleans and keep
  `mainLiab` as `main_liability` so routing can distinguish primary liability
  candidates from secondary coverage evidence.
- preserve raw source paths on every product/liability canonical evidence row:
  product rows must carry `source_path` such as
  `reportCase.policyList[n].productList[m]`, and liability rows must carry
  `liability_source_path` such as
  `reportCase.policyList[n].productList[m].claimLiabilityList[k]`; product-only
  entries without a source `claimLiabilityList` keep `liability_source_path =
  null`.

Required inbox corrections before scoring:

- idempotency: use `systemCode + transNo + reportNo` as the external message
  identity and reject or upsert duplicate submissions deterministically; internal
  audit ids, run ids, raw payload refs, and idempotency keys must use a stable
  checksum or fingerprint rather than raw external identifiers;
- source trace: persist raw payload URI or checksum/fingerprint, normalized claim
  id, mapping version, validation result, PII-safe source-path summary, and
  evidence refs;
- date normalization: convert all epoch-millisecond dates using the configured
  source business timezone, preserve the raw epoch milliseconds and timezone
  metadata for audit, and detect impossible or inconsistent accident, visit,
  invoice, policy, product, liability, and receive windows across the full
  source list, not only the primary product or liability or primary invoice;
- identity consistency: compare accident person, insured person, patient name,
  every medical-record patient, and every invoice person after masking, and
  raise a review signal when they do not align;
- medical consistency: map diagnosis codes, diagnosis names, department,
  medical type, fee categories, drugs, procedures, and medical record text into
  L5 medical-reasonableness inputs;
- policy coverage: map policy, product, and liability lists into coverage,
  waiting-period, limit, main-liability, serious-disease-liability, and
  liability eligibility features with raw
  `policyList/productList/claimLiabilityList` source-path traceability;
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
  accident date, accident reason, medical type, currency, and claim-level total
  amount summed from all source invoices;
- member and policy snapshot: masked member id, masked certificate id,
  certificate type, gender, birth date, policy id, product code, primary
  product/liability codes, all product-liability windows, policy type,
  first-apply date, social-insurance participation flag, coverage constraints,
  and raw product/liability source paths for audit;
- provider snapshot: hospital/provider code, name, class, type, city, province,
  and network flags;
- itemized bill lines: every source invoice fee detail with invoice id,
  invoice bill type, document type, social-insurance type, department, medical
  type, invoice claim nature, invoice start/end dates, diagnosis list, fee
  category, item name, amount, self-pay, own-expense, invoice-level payment
  totals, invoice-level provider context, fee-group amount, fee-group other
  amount, medical category, Medicare prorated percentage, social-insurance
  amount, source path, and evidence refs;
- document evidence: every source medical record with medical record text,
  claim nature, medical record type, chief complaint, current medical history,
  past history, extracted diagnosis, procedure, prescription, department, visit
  date, first-happen date, operation-start date, source path, and source refs;
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
- Agent investigation context snapshots should reuse normalized scoring traces,
  including canonical evidence refs and source refs, when a prior scoring audit
  event is available for the claim.
- Investigation result writeback should preserve canonical evidence refs from
  the latest successful normalized scoring trace for the claim, appending them
  to the persisted investigation result, outcome labels, saving attribution
  evidence, and `investigation.result.received` audit event.
- QA review queue items should surface normalized scoring trace source refs and
  canonical evidence refs so reviewers can ground feedback in original bill-line
  and document evidence.
- QA result writeback should preserve canonical evidence refs from the latest
  successful normalized scoring trace for the claim, appending them to the
  persisted QA review and `qa.result.received` audit event.
- Medical Review queue and writeback should reuse the referenced normalized
  scoring trace, appending canonical evidence refs to the persisted
  `medical.review.recorded` audit event.
- Optional infrastructure such as Redis, ClickHouse, Neo4j, OpenSearch, Qdrant,
  LanceDB, or Kubernetes is adopted only when a defined workload requires it.

Staged infrastructure roadmap:

- MVP: PostgreSQL, Rust API server, Rust worker, Python ML service, Yew
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

### Agentic Investigation Control Plane

Agentic workflows that access PHI or produce investigation outputs must be
governed by a five-layer control plane. This applies to any agent that reads
claim data, member data, provider data, or evidence packages, regardless of
whether the agent uses an LLM or a deterministic algorithm.

Required control layers:

1. Identity and persona registry: every agent deployment must be registered with
   a unique identity, version, defined capability scope, and enumerated PHI field
   access list before it can run in any environment.

2. Orchestration and cross-domain mediation: an investigation orchestrator manages
   the lifecycle state machine and coordinates specialist agents; only the
   orchestrator holds the full investigation context; specialist agents receive
   only the data their registered scope requires.

3. PHI-bounded context and memory: agents must access only the PHI fields
   declared in their registry entry; field-level access must be enforced at tool
   invocation time and must not depend on agent self-reporting.

4. Runtime policy enforcement with kill-switch: every agent action must be
   validated against the platform policy engine before execution; a kill-switch
   mechanism must be able to halt any running agent without data loss.

5. Lifecycle management and decommissioning: agent versions must be registered,
   monitored for output quality drift, and decommissioned with an auditable
   deprovisioning record.

Agent audit event schema must include at minimum: agent identity and version,
investigation identifier, action type, timestamp, actor context, PHI fields
accessed by field name without values, tool calls with input and output digests,
decision rationale, confidence score, and whether human review was triggered.
Audit records must be append-only with a cryptographic hash chain linking
consecutive records for the same investigation. Audit records must be retained
for at least six years to satisfy HIPAA retention requirements. No standardized
schema for agentic AI audit artifacts exists as of this document's last update;
the platform must define and own its schema until regulatory standards are
finalized by NIST or CMS.

Agent run identifiers must use collision-resistant sortable identifiers such as
ULID or UUID v7 so that audit records across multiple runs for the same claim
remain uniquely traceable. Format strings derived from claim identifiers alone
must not be used as agent run identifiers.

Agent decision boundary must be hardcoded to assistive-only for all versions
until a customer-approved deterministic escalation path with explicit rule
authority is in place. Agents may generate investigation packages, checklist
items, evidence summaries, and disposition recommendations, but the final
adjudication decision must remain with a human reviewer or a customer-approved
deterministic rule with published approval status.

The system should support agent-native operation by giving agents parity with
operator-readable capabilities through approved tools, while reserving
high-impact writes for human approval. This keeps the architecture compatible
with modern agentic workflows without turning the FWA platform into an
uncontrolled autonomous adjudication system.

Agent investigation completion audit events must expose the applied governance
controls directly in the audit payload, including `decision_boundary`,
`agent_policy_id`, `policy_check_id`, `tool_call_id`, and `tool_name`, and the
event evidence refs must include the applied policy ref. The persisted agent run
may hold the fuller step/tool/approval records, but the audit timeline itself
must still be sufficient to trace the policy and allowlisted tool used for the
output.

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
`insufficient_evidence`. The medical review writeback API stores these values in
`clinical_outcomes`; if a reviewer omits the array, the platform derives a
compatible controlled outcome from the review `decision`.

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
- no unresolved backtest blockers such as underpowered reviewed sample count,
  insufficient precision/recall, false-positive burden, or review-capacity
  overflow;
- estimated saving and expected false-positive burden;
- evidence refs for the pattern the rule claims to detect;
- approval before publish;
- shadow or limited rollout for high-impact rules;
- rollback path to the previous active version.

Model promotion requires:

- immutable dataset and feature-set versions;
- explicit time/group split evidence: `time_group_split_status = passed`,
  non-empty `time_split_field`, and non-empty `group_split_fields`;
- leakage checks across member, policy, provider, and related-case groups;
- holdout and out-of-time metrics;
- threshold selection tied to review capacity;
- explanation artifact such as feature importance or SHAP-style analysis;
- Rust serving artifact evaluation, including manifest contract, feature order,
  checksum/signature, probability parity when available, and latency budget;
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

- logistic regression for calibrated baselines and rule-only comparison;
- decision trees for transparent rules-of-thumb;
- gradient-boosted trees, such as XGBoost or LightGBM, as the primary
  supervised-learning candidates for structured claim-risk scoring, provided
  they carry feature importance or SHAP-style explanation and pass strict
  validation gates.

Unsupervised ensemble scoring is a required complement to supervised ML and is
particularly valuable early in a deployment when labeled data is scarce.
Billing-pattern anomaly scoring using statistical methods such as IQR and
MAD-based deviation, DRG peer-group frequency comparison, and code-diversity
metrics does not require fraud labels and produces significantly higher precision
than random targeting when evaluated against confirmed enforcement records.
Unsupervised ensemble outputs must feed the L3 Unsupervised Anomaly scoring
layer as a first-class signal, replacing heuristic threshold implementations once
sufficient historical claim data is available. Unsupervised scores must carry
evidence refs identifying the statistical signal that contributed to the anomaly
and must not be presented as fraud conclusions.

Explainable supervised model outputs should also feed Rule Studio. High-value
feature contributions, tree paths, or SHAP-style patterns may become candidate
rules, but they must pass deterministic backtest, human promotion review, rule
promotion gates, approval, and publication before entering the active rule
library.

The ML lifecycle should be Rust-owned even when the optimizer is not written in
Rust. Rust should control dataset contracts, feature versions, training job
orchestration, model registration, evaluation, backtest, shadow monitoring,
human review, activation, rollback, and audit. XGBoost, LightGBM, and deep
models may enter production serving through ONNX when feature-order and
prediction-parity tests pass; otherwise they remain governed candidates or use a
controlled fallback scorer until the Rust serving boundary is proven.

Auto MLOps should operate as an evidence and recommendation loop, not an
autonomous model operator. It may schedule monitoring, rank candidates, prepare
retraining proposals, open review tasks, and package rule-candidate evidence. It
must not activate a model, rollback a model, publish a rule, assign a fraud
label, or write extracted patterns into the active rule library without the
required backtest, human review, and approval gates.

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
4. Train a logistic baseline plus at least one gradient-boosted tree candidate
   once stable labels exist.
5. Record model dataset version, feature-set version, algorithm family,
   metrics, threshold, and feature importance or SHAP-style artifact.
6. Validate serving artifacts, including ONNX or Rust-native parity where
   applicable.
7. Send explainable model patterns into Rule Studio as candidate rules when
   they can be expressed as deterministic, auditable rule DSL.
8. Run shadow mode against live traffic and compare against rules and human QA.
9. Promote only when holdout, out-of-time, pilot review, and serving-parity
   metrics pass.

Overfitting controls are product requirements, not optional data-science notes:

- use time-based holdout and out-of-time validation;
- prevent provider, member, policy, and related-case leakage across splits;
- store time/group split evidence in model evaluation metrics so promotion
  gates can block models that only report random train/test splits;
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

## June 2026 Architecture Gap Roadmap

The June 12 architecture review is recorded in
`docs/project/architecture-gap-review-2026-06-12.md`. The items below convert
that review into PRD-level requirements. This section describes product and
architecture intent; repository completion status remains in
`docs/project/prd-coverage.md`.

Immediate correctness and compliance requirements:

- Any scoring layer that uses a proxy, baseline, or placeholder implementation
  must label that status in code comments, feature metadata, and operator-facing
  evidence where it affects interpretation.
- L1 peer benchmark scoring must not treat amount-to-limit ratio as a real peer
  percentile in production. If peer data is missing, the scoring layer must
  either downweight the signal or exclude it from final-score normalization.
- L5 diagnosis/procedure compatibility must identify heuristic outputs as
  placeholder clinical signals until a governed ICD-10/CPT or policy-reference
  comparator is available.
- Routing policy lifecycle writes must require fine-grained permissions for
  submit, approve, activate, rollback, and other high-impact transitions.
- Inbox validation and audit serialization failures must fail loudly or emit
  critical audit events; they must not silently persist empty arrays or
  misleading successful records.
- Runtime scorer shared-state failures, including poisoned locks around model
  sessions, must recover with logging and alerting where safe instead of
  permanently failing all future scoring requests.

Current-month architecture requirements:

- Scoring aggregation must distinguish missing data from a real zero score and
  renormalize layer weights across the available evidence-bearing layers.
- Confidence scoring must use multiple evidence families, including clinical,
  provider, and graph signals, rather than depending only on rule/anomaly/model
  agreement.
- Provider graph contracts must include billing ring membership, temporal
  co-billing frequency, and referral concentration entropy, with worker-owned
  rollups for those values.
- Worker commands must own daily OIG/SAM sanctions refresh, provider
  30/90/365-day profile windows, PSI actioning, and 7-day/90-day rule hit-rate
  trending.
- Canonical claim context and evidence responses must apply field-path PHI/PII
  masking before leaving the API boundary.

Next-quarter foundation requirements:

- Worker pipelines must add billing-ring detection, temporal co-billing,
  referral entropy, monthly peer percentile benchmarks, and episode-level
  member-provider aggregation.
- Feature records must carry proxy/source metadata so reviewers can distinguish
  real peer distributions, customer data, public data, synthetic data, and
  fallback estimates.
- The agentic control plane must add a runtime `agent_registry`, stable
  `investigations` entity, enforced PHI field allowlists, populated
  `phi_fields_accessed` audit records, and cancellation/kill-switch semantics.
- The Rust ML runtime should cache parsed serving manifests and label raw
  sigmoid outputs as uncalibrated until calibration evidence exists.
- The L3 anomaly layer must define a measurable upgrade gate for replacing
  heuristic baselines with IQR, MAD, or unsupervised ensemble methods.

Medium-term requirements:

- Audit retention must move from documentation to executable policy with
  retention evidence, archival workflow, and customer-environment proof.
- Agentic investigation should evolve from one deterministic investigator into
  an orchestrated set of specialist agents, while preserving assistive-only
  decision boundaries and human approval gates.
- Scheme coverage must expand beyond provider outliers into duplicate billing,
  unbundling, excessive utilization, lab, telehealth, genetic testing,
  pharmacy/opioid, DME/home health, and China-market-specific organized fraud
  patterns using episode, policy, and peer-distribution features.

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
- Operations Dashboard reports canonical inbox trace coverage so normalized
  scoring adoption is visible to governance users.
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
- Peer percentile signals in the L1 Peer Benchmark layer are derived from actual
  peer distribution data segmented by specialty and region; ratio-to-limit proxy
  values must not be used as peer percentile substitutes in production scoring.
- Provider graph signals include OIG exclusion status, billing ring membership
  score, temporal co-billing frequency, and referral concentration entropy;
  these signals are refreshed at minimum nightly.
- Feature distribution PSI is computed monthly for tier-1 scoring features; PSI
  violations above 0.25 generate actionable review tasks.
- Rule hit rate trending is maintained with 7-day and 90-day baselines per
  production rule; rates significantly below baseline trigger drift review tasks.
- Agent audit events include agent identity, version, investigation identifier,
  PHI fields accessed by name, tool call records, decision rationale, and
  human-review flag; the audit store is append-only with a cryptographic hash
  chain.
- The L3 Unsupervised Anomaly layer uses statistical IQR, MAD, or
  billing-pattern ensemble methods rather than fixed heuristic thresholds once
  sufficient historical claim data is available; heuristic thresholds are
  acceptable as labeled baselines only.
