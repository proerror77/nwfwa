# Architecture Gap Review - 2026-06-12

This document records the June 12 architecture review for the `nwfwa` FWA
platform. It is a gap ledger, not implementation status. Use
`docs/project/prd-coverage.md` for repository evidence and current completion
boundaries.

## Executive Summary

The platform has a solid product architecture: the seven-layer scoring model is
explainable, rule governance is explicit, the agentic investigation scaffold is
assistive-only, and audit/event tables already cover many operational surfaces.

The production gaps cluster in three areas:

- scoring data quality: several important signals are still baseline,
  heuristic, or proxy-backed, and layer weights do not always degrade safely
  when real data is absent;
- worker data pipelines: PRD-required rollups for sanctions, provider profile
  windows, peer percentile benchmarks, graph signals, episode aggregation, PSI
  actions, and rule hit-rate trending are not all implemented as autonomous
  worker commands;
- agentic control plane: database, audit, deterministic specialist dispatch,
  and crate-level cancellation checkpoint concepts exist, but LLM-backed
  specialist execution and external tool-call runtime mediation remain
  incomplete.

## Post-Review Implementation Notes

As of the P1/P2 remediation commits after this review:

- scoring missing-data reweighting, broader confidence scoring, field-path PHI
  response masking, sanctions sync contract, provider profile windows, PSI
  actioning, and rule hit-rate trending are tracked as implemented in
  `docs/project/prd-coverage.md`;
- provider graph signal rollups for billing-ring membership, temporal
  co-billing, and referral entropy are implemented as local worker contracts
  with a permission-gated submit/write path;
- peer percentile benchmark and member-provider episode aggregation are
  implemented as local worker contracts with permission-gated submit/write
  paths;
- `agent_registry` and `investigations` now exist as additive schema/runtime
  contracts, agent audit events use stable investigation ids, and deterministic
  agent audit records include non-empty PHI field names without values;
- the deterministic agent route now enforces active registry capability and PHI
  field allowlists before executing `knowledge.search_similar`.
- Agent run cancellation now has an API control-plane contract: queued/running
  runs can be marked `cancelled`, cancellation requires run evidence, and
  `agent.run.cancelled` is emitted as a governance audit event.
- `fwa-agent` now exposes an `InvestigationOrchestrator` trait and deterministic
  specialist-plan contract for intake triage, evidence review, and network
  analysis slots while preserving the assistive-only boundary.
- `fwa-agent` now exposes an investigation cancellation signal contract and
  deterministic execution checkpoints so future long-running/specialist agents
  can stop safely at named boundaries.
- `fwa-agent` now has a deterministic specialist dispatch contract that emits
  intake, evidence-review, and network-analysis executions plus mediated tool
  call contracts without executing external tools.
- The agent investigation API now exposes those deterministic specialist
  executions in the response, persisted run `output_json`, audit payload, and
  OpenAPI schema while keeping contract-only mediated calls out of real
  `tool_calls`.
- `fwa-features` now has a `ClinicalCompatibilityFeatureContext` input path so
  governed ICD-10/CPT compatibility scores can replace the
  `diagnosis_procedure_match_score` heuristic without treating fallback values
  as real clinical compatibility data.
- The worker now has a governed clinical compatibility reference contract that
  validates ICD/CPT-style compatibility rows with policy authority refs and
  evidence refs, then emits records suitable for the clinical compatibility
  feature input path.
- The clinical compatibility reference report now has a permission-gated submit
  path and worker command that persist customer policy reference rows while
  explicitly avoiding claim scoring, fraud-label assignment, claim denial, or
  replacement of medical review.
- The worker now has an unbundling comparator contract that joins governed
  bundled/component code rules with episode procedure-code snapshots and emits
  medical-review candidates without assigning fraud labels.
- The unbundling comparator report now has a permission-gated submit path and
  worker command that persist comparator candidates while explicitly avoiding
  claim scoring, fraud-label assignment, claim denial, case creation, or
  replacement of medical review.
- `fwa-features` and `fwa-scoring` now accept episode-utilization signals for
  member-provider revisit counts, duplicate-claim similarity, procedure
  frequency peer percentile, and unbundling candidate counts; these signals
  contribute to L5 only when worker-owned context is supplied.
- The worker now has a scoring feature context materialization contract that
  maps episode rollups, peer benchmarks, clinical compatibility records, and
  unbundling candidates into claim-level contexts suitable for online scoring
  integration, plus a permission-gated
  `/api/v1/ops/scoring-feature-context-materializations` submit path and
  `submit-scoring-feature-contexts` worker command so those claim-level
  contexts can be audited and persisted before scoring.
- The OIG/SAM sanctions worker contract now has a separate permission-gated
  sanctions sync report submit path and worker command that persist provider
  sanctions from an approved report while explicitly avoiding scoring-policy
  changes, fraud-label assignment, or claim adjudication.
- The provider profile 30/90/365 window rollup now has a separate
  permission-gated submit path and worker command that persist provider profile
  windows from an approved rollup report while explicitly avoiding scoring-
  policy changes, fraud-label assignment, or claim adjudication.
- The provider graph signal rollup now has a separate permission-gated submit
  path and worker command that persist billing-ring, temporal co-billing, and
  referral-entropy signals from an approved rollup report while explicitly
  avoiding scoring-policy changes, fraud-label assignment, case creation, or
  claim adjudication.
- The peer percentile benchmark now has a separate permission-gated submit path
  and worker command that persist specialty/region/service-segment benchmark
  groups from an approved benchmark report while explicitly avoiding claim
  scoring, fraud-label assignment, or routing-policy changes.
- The member-provider episode aggregation rollup now has a separate
  permission-gated submit path and worker command that persist episode
  utilization windows from an approved aggregation report while explicitly
  avoiding scoring-policy changes, fraud-label assignment, case creation, claim
  denial, or claim adjudication.
- Worker data pipeline execution reports now carry an explicit readiness gate:
  scheduler run-status artifacts can reference the readiness report, execution
  evidence records `ready`/`blocked`/`missing`, and missing or blocked readiness
  creates an operations review task instead of silently allowing downstream use.
- The worker also emits a run-status template contract so customer schedulers
  can start from the planned job list and readiness report URI instead of
  hand-authoring the execution input JSON.
- The claims scoring API now accepts those materialized worker contexts and
  passes peer, clinical compatibility, and episode-utilization inputs into
  online feature calculation while preserving the assistive-only scoring
  boundary and feature-value trace.
- `ServingManifestModelScorer` now caches the parsed serving manifest, removing
  avoidable per-score manifest file reads while preserving request-time
  identity and feature-order validation.
- Rust artifact and serving-manifest scorers now emit
  `probability_calibration_status = "uncalibrated_raw_sigmoid"` in score
  metadata so downstream users do not confuse raw sigmoid output with a
  calibrated fraud probability.
- the worker now emits an L3 anomaly upgrade readiness report that checks the
  confirmed-FWA label threshold and 30-day recall signal before opening an
  IQR/MAD statistical-baseline evaluation task.
- Rust feature-set manifests now include per-column `is_proxy` and
  `data_source` metadata so training/governance artifacts can distinguish demo
  baselines from worker-owned peer/profile rollups.
- Online scoring `FeatureValue` payloads now include `is_proxy` and
  `data_source` metadata, with OpenAPI coverage and claims-score response tests
  verifying the contract.
- The worker now has an audit-retention dry-run scan contract that computes
  six-year cutoff candidates, legal-hold blocks, and destruction-review
  artifacts without deleting records.
- The worker now has a probability-calibration evidence report that computes
  ECE and Brier score from labeled holdout predictions and opens calibration
  review tasks when raw probabilities are miscalibrated or sample size is
  insufficient.
- Probability calibration reports now have a model-governance submit path and
  worker command that persist calibration evidence and record audit lineage
  while explicitly avoiding calibrated serving activation, threshold changes, or
  label assignment.
- The worker now has a scheduled data-pipeline plan contract that orders the
  build and submit commands for sanctions, provider profiles, graph signals,
  peer benchmarks, episode rollups, clinical references, unbundling candidates,
  scoring feature contexts, and probability-calibration evidence under daily
  or monthly cadence with explicit readiness gates.
- The worker now has a data-pipeline readiness report that checks customer-
  approved artifact URIs, minimum row counts, data-quality status, evidence
  refs, and external OIG/SAM fetch configuration before scheduled writes.
- The worker data-pipeline readiness report now has a permission-gated API
  submit path that persists prerequisite evidence and records governance audit
  lineage while explicitly avoiding external fetch execution or artifact
  submission.
- The worker now has a data-pipeline execution report contract that converts a
  customer scheduler run-status artifact into per-job completion, pending,
  failed, and review-task evidence without running live customer jobs itself.
- The worker data-pipeline execution report now has a permission-gated API
  submit path that persists scheduler evidence and records governance audit
  lineage while explicitly avoiding claim scoring, label assignment, claim
  denial, model activation, or routing-policy changes.

Remaining boundaries after those commits are live scheduler deployment/runtime
execution, live external OIG/SAM fetch, customer claim/history data, LLM-backed
specialist execution, real external tool-call runtime mediation, wiring long-
running/tool-using agents into the cancellation signal, customer-approved
ICD-10/CPT or medical-policy reference data, customer-approved unbundling rule
packs, customer-approved feature lineage/source mappings, calibrated-
probability serving activation, and replacement of the L3 heuristic anomaly
scorer with a validated statistical baseline. Audit retention still needs
customer-environment partitioning, archive storage, legal-hold reconciliation
writes, approved destruction workflow execution, and live routing-impact
validation on customer data.

## A. Scoring Layer Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| A-1 | L1 peer percentile fallback can be confused with a real peer percentile when real peer data is absent. | Mark proxy/baseline paths explicitly and reduce or exclude L1 weight when only proxy data is available. |
| A-2 | `diagnosis_procedure_match_score` has a real clinical compatibility input path, worker reference contract, and permission-gated clinical reference submit/write path, but the default remains a hard-coded heuristic fallback until governed reference data is supplied to scoring. | Populate `ClinicalCompatibilityFeatureContext` from customer-approved ICD-10/CPT compatibility or medical-policy reference data before using this layer as production clinical consistency. |
| A-3 | Confidence scoring overweights rule/anomaly/model agreement and ignores other high-signal layers. | Move to a weighted multi-layer confidence model or high-confidence rule based on two independent high-risk signals. |
| A-4 | Provider graph input needs billing ring, temporal co-billing, and referral entropy signals. | Add fields to the provider graph contract and require worker rollups that compute them from claim/referral history. |
| A-5 | Seven-layer weights are hard-coded and do not renormalize when data is missing. | Represent layer values as data-present vs. actual zero and renormalize across available layers. |
| A-6 | Provider history counts now use the same 30/90/365 recency weighting philosophy as provider profile risk, producing effective counts instead of letting a 365-day max dominate current scoring. | Validate effective-count interpretation with customer review policy before exposing it as an operational KPI. |
| A-7 | L3 anomaly baseline has a comment but no quantified upgrade trigger. | Add a measurable threshold, e.g. upgrade evaluation after at least 500 confirmed FWA labels or poor 30-day recall. |
| A-8 | FWA feature families for revisit frequency, duplicate-claim similarity, procedure frequency vs. peers, and unbundling candidates now have optional feature/scoring inputs, a worker materialization contract, API ingestion support, an episode-rollup submit/write path, a clinical-compatibility reference submit/write path, and an unbundling comparator candidate submit/write path. Production scheduling and customer-data validation remain open. | Connect scheduled worker artifact persistence and customer-approved data sources before treating these schemes as production-covered. |

## B. Feature Engineering And Data Quality Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| B-1 | Peer percentile fallback is still a proxy path unless upstream supplies peer context. | Feature metadata must expose `is_proxy`, `data_source`, and source lineage. |
| B-2 | Episode-level features now have worker-owned 30/90/365 member-provider aggregation, scoring input materialization, and a permission-gated submit/write path; production scheduling and customer claim-history validation remain open. | Connect scheduled customer-history rollups before treating unbundling and excessive-utilization coverage as production-ready. |
| B-3 | ICD-10/CPT clinical compatibility ingestion now has a permission-gated reference submit/write path. Unbundling comparator candidates now also have a permission-gated submit/write path, but customer-approved rule packs, production scheduling, and customer-data validation remain open. | Connect bundled/component code references and episode co-occurrence outputs to governed customer data before treating unbundling detection as production-covered. |
| B-4 | Feature registry lacks proxy/source metadata. | Extend feature records and PRD acceptance criteria so compliance reports can distinguish real distributions from estimates. |

## C. Agentic Investigation Control-Plane Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| C-1 | Agent identity registry is missing as a runtime authority. | Add `agent_registry` with identity, kind, version, capability scope, PHI field allowlist, status, registration, and deprovision timestamps. |
| C-2 | No independent `investigations` entity groups multiple agent runs for the same claim. | Add `investigations` and make agent audit events reference a stable investigation id instead of a run-derived string. |
| C-3 | PHI field access is not enforced by registry policy, and accessed fields can be empty in audit events. | Enforce field allowlists at investigation/tool boundaries and persist actual PHI field names without values. |
| C-4 | Kill-switch behavior now includes an API control plane plus crate-level deterministic cancellation checkpoints, but no long-running/tool-using agent runtime is wired to the signal yet. | Wire runtime specialist dispatch and tool calls through the cancellation signal before adding LLM-backed or long-running agents. |
| C-5 | Deterministic investigation now has an orchestrator trait, specialist-plan contract, deterministic specialist dispatch/tool mediation contract, and API/OpenAPI exposure for specialist execution traces, but not LLM-backed specialist execution or real external tool runtime mediation. | Add LLM-backed investigators and real tool mediation only after governance checks and cancellation wiring are stable. |

## D. ML Lifecycle And Governance Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| D-1 | Serving manifest loading now caches parsed manifests after the first score request. | Keep customer deployment evidence focused on cache behavior under live serving load rather than reimplementing manifest parsing. |
| D-2 | Poisoned ONNX/session mutexes can cause persistent scorer failure. | Recover poisoned locks with error logging and alerting instead of requiring process restart. |
| D-3 | PSI calculation exists as a monitoring concept but must drive actions. | PSI above threshold must create monitoring alerts and compliance/model review tasks. |
| D-4 | Rule hit-rate trending is defined as a plan but needs runtime computation. | Compute 7-day and 90-day hit-rate windows and trigger drift review when short-term rates collapse. |
| D-5 | L3 anomaly scoring is still a heuristic baseline. | Add worker evaluation for IQR/MAD or ensemble anomaly readiness and make the upgrade trigger measurable. |
| D-6 | Raw sigmoid outputs may be interpreted as calibrated fraud probability. | Mark probability calibration status explicitly, such as `uncalibrated_raw_sigmoid`. |

## E. Compliance, Security, And HIPAA Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| E-1 | Validation-error serialization must not silently turn failures into empty arrays. | Treat serialization failure as an explicit error or critical audit event. |
| E-2 | Canonical claim context and evidence payloads can expose PHI unless field-level masking is applied. | Mask known PII field paths before API responses and audit payloads; do not rely only on regex value detection. |
| E-3 | Routing policy lifecycle writes need fine-grained permissions. | Align routing policy write/approve/activate/rollback permissions with rule lifecycle governance. |
| E-4 | PII masking by value pattern misses names and non-US identity numbers. | Prefer field-name/path-based masking for known PHI/PII inputs. |
| E-5 | Six-year audit retention is documented but not enforced. | Add audit retention policy, archive workflow, and production evidence for HIPAA retention. |

## F. Worker Data Pipeline Gaps

| Pipeline | Current gap | Priority |
| --- | --- | --- |
| OIG/SAM daily sanctions sync | No autonomous sync command; sanctions status depends on upstream input. | P1 |
| Provider profile windows | 30/90/365 windows are required, but worker rollup ownership must be explicit. | P1 |
| Billing ring detection | Graph clustering needs billing-ring membership or patient-overlap ring detection. | P2 |
| Temporal co-billing | Needs 7-day co-occurrence computation from dated claim history. | P2 |
| Referral entropy | Referral concentration should be entropy or HHI-backed, not an opaque score. | P2 |
| Peer-group percentile benchmark | Monthly p25/p50/p75/p90/p99 by specialty/region/service segment is required. | P2 |
| Episode aggregation | Member-provider 30/90/365 episode rollups are missing for unbundling/utilization schemes. | P2 |
| PSI actioning | PSI must create alerts/tasks, not only report values. | P1 |
| Rule hit-rate trending | 7-day vs. 90-day rule hit-rate computation must be implemented and scheduled. | P1 |

## G. Scheme Coverage Implications

The strongest current coverage is provider peer outlier and suspicious provider
relationship review, but both still depend on real peer distributions and graph
rollups. Duplicate billing, unbundling, excessive utilization, lab abuse,
telehealth abuse, genetic testing abuse, pharmacy/opioid abuse, and DME/home
health abuse all require episode, code-frequency, or policy-reference features
before they can be treated as production-complete schemes.

China-market-specific patterns also need explicit feature and evidence support:
early high-value claims, split treatment across multiple facilities in one
episode, suspicious beneficiary identity, and organized provider/member rings.

## H. Prioritized Roadmap

### P0 - Immediate Correctness And Safety

- Label proxy and placeholder scoring paths for L1 peer percentile, L5
  diagnosis/procedure compatibility, and L3 anomaly baseline.
- Add fine-grained permissions to routing policy lifecycle writes.
- Make inbox validation serialization failures fail loudly.
- Recover scorer mutex poisoning and log/alert the recovery.

### P1 - Current Month Architecture Completion

- Add provider graph contract fields for billing ring membership, temporal
  co-billing, and referral entropy.
- Add missing-data-aware scoring reweighting and improve confidence scoring.
- Add worker commands for provider profile windows and daily OIG/SAM sanctions
  sync.
- Make PSI violations and rule hit-rate drift produce actionable review tasks.
- Add field-path PII masking to canonical claim context and evidence responses.

### P2 - Next Quarter Foundation Work

- Add worker rollups for billing rings, temporal co-billing, referral entropy,
  peer percentile benchmarks, and episode aggregation.
- Add `agent_registry`, `investigations`, PHI field enforcement, and populated
  `phi_fields_accessed` audit records.
- Add episode-level features, ICD-10/CPT unbundling comparators, and feature
  proxy/source metadata.
- Cache serving manifests in the Rust ML runtime.

### P3 - Medium-Term Agentic And Compliance Hardening

- Implement agent kill-switch and specialist-agent orchestration.
- Add audit retention implementation and evidence of six-year retention policy.
- Replace L3 heuristic anomaly baseline with IQR/MAD or ensemble scoring once
  label/history thresholds are met.
- Extend scheme coverage for China-market-specific organized fraud patterns.
