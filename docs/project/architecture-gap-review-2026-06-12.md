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
- `/api/v1/claims/score` can now load persisted clinical compatibility
  references by diagnosis prefix and procedure code when no inline or
  materialized clinical context is supplied, preserving non-proxy source
  metadata and evidence refs in the scoring response.
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
- `/api/v1/claims/score` can now load persisted member-provider episode
  rollups directly when no inline or materialized episode context is supplied,
  feeding 30-day revisit count and duplicate-claim similarity into L5 with
  evidence refs.
- `/api/v1/claims/score` can also load the latest persisted unbundling
  comparator candidates by member/provider and feed `unbundling_candidate_count`
  into L5 when no inline or materialized episode context supplies it.
- The worker now has a scoring feature context materialization contract that
  maps episode rollups, peer benchmarks, clinical compatibility records, and
  unbundling candidates into claim-level contexts suitable for online scoring
  integration, plus a permission-gated
  `/api/v1/ops/scoring-feature-context-materializations` submit path and
  `submit-scoring-feature-contexts` worker command so those claim-level
  contexts can be audited and persisted before scoring. The submit path now
  requires the claims, episode, peer, clinical, and unbundling source URIs,
  matching top-level source evidence refs, and per-context evidence refs.
- The OIG/SAM sanctions worker contract now has a separate permission-gated
  sanctions sync report submit path and worker command that persist provider
  sanctions from an approved report while explicitly avoiding scoring-policy
  changes, fraud-label assignment, or claim adjudication.
- The worker now also has a configured-endpoint OIG/SAM-compatible snapshot
  fetcher that writes the governed sanctions snapshot artifact used by the
  existing dry-run/report/submit path. The scheduled worker data pipeline now
  exposes that fetcher as an artifact-only daily job before
  `sync-oig-sam-sanctions`; execution evidence treats a successful snapshot
  artifact as complete without requiring API submission. Official feed
  configuration, credentials, and customer scheduler execution remain external.
- Claims scoring can now load persisted provider sanctions by provider and
  merge OIG/SAM hits into the provider profile input, including sanctions-only
  provider review when no inline `provider_profile` payload is supplied.
- The provider profile 30/90/365 window rollup now has a separate
  permission-gated submit path and worker command that persist provider profile
  windows from an approved rollup report while explicitly avoiding scoring-
  policy changes, fraud-label assignment, or claim adjudication.
- The claims scoring API can now load the latest persisted provider profile
  window rollup by provider when no inline `provider_profile` payload is
  supplied, preserving rollup evidence refs in the scoring evidence trace.
- The provider graph signal rollup now has a separate permission-gated submit
  path and worker command that persist billing-ring, temporal co-billing, and
  referral-entropy signals from an approved rollup report while explicitly
  avoiding scoring-policy changes, fraud-label assignment, case creation, or
  claim adjudication.
- The provider graph signal contract now also carries high-risk neighbor ratio,
  provider-patient overlap, connected confirmed FWA counts, referral
  concentration score, and optional network component risk. Claims scoring can
  consume the latest persisted graph signal by provider when those complete
  fields are present and no inline `provider_relationships` payload is
  supplied.
- The peer percentile benchmark now has a separate permission-gated submit path
  and worker command that persist specialty/region/service-segment benchmark
  groups from an approved benchmark report while explicitly avoiding claim
  scoring, fraud-label assignment, or routing-policy changes.
- `/api/v1/claims/score` can now load the latest persisted peer benchmark
  group by provider specialty, provider region, and explicit service segment
  when no inline or materialized peer context is supplied, preserving non-proxy
  source metadata and evidence refs in the scoring response.
- The member-provider episode aggregation rollup now has a separate
  permission-gated submit path and worker command that persist episode
  utilization windows from an approved aggregation report while explicitly
  avoiding scoring-policy changes, fraud-label assignment, case creation, claim
  denial, or claim adjudication.
- Worker data pipeline execution reports now carry an explicit readiness gate:
  scheduler run-status artifacts can reference the readiness report, execution
  evidence records `ready`/`blocked`/`missing`, and missing or blocked readiness
  creates an operations review task instead of silently allowing downstream use.
  Execution evidence also records dependency blockers and marks downstream jobs
  `dependency_not_completed` when planned upstream artifacts are missing or
  incomplete.
- The worker now emits a readiness input template from the scheduled plan so
  customer operators can fill artifact URIs, approvals, row counts, quality
  status, external-fetch configuration, and evidence refs without inventing the
  schema; the generated template remains blocked until that evidence is filled.
- The worker also emits a run-status template contract so customer schedulers
  can start from the planned job list, readiness report URI, build commands,
  source inputs, dependency list, artifact-only markers, and planned targets
  instead of hand-authoring the execution input JSON.
- Worker data-pipeline readiness now requires customer evidence for fresh
  source data and a positive coverage window before a job can be marked ready,
  so stale peer/profile/graph/episode/calibration inputs cannot be submitted as
  production-ready prerequisites.
- The worker data-pipeline readiness submit API now also rejects ready jobs
  without non-empty per-job evidence refs, preventing direct API submissions
  from bypassing the worker-generated evidence contract.
- The claims scoring API now accepts inline materialized worker contexts and
  also loads the latest persisted scoring-feature-context materialization by
  claim when no inline context is supplied, passing peer, clinical
  compatibility, and episode-utilization inputs into online feature
  calculation while preserving the assistive-only scoring boundary and
  feature-value trace.
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
- The production readiness validator now deep-checks retention/legal-hold
  evidence for six-year retention, policy ids, archive storage, completed
  legal-hold reconciliation, human destruction approval, and disabled
  automated destruction.
- The production readiness validator now deep-checks model-serving SLO
  evidence for model identity, p95 latency, error rate, checksum/signature
  verification, fallback health, rollback readiness, active calibrated-
  probability serving, and calibration evidence refs.
- The production readiness validator now deep-checks customer-data governance
  evidence for approved dataset provenance, label provenance, holdout split,
  shadow-traffic plan, positive validation sample counts, and evidence refs.
- The production readiness validator now deep-checks OCR/vector/analytics
  execution evidence for completed OCR, embedding/vector, retrieval,
  ClickHouse export, dashboard access, analytics retention/backup, positive
  counts, no raw PHI export, and stage evidence refs.
- The worker now has a probability-calibration evidence report that computes
  ECE and Brier score from labeled holdout predictions and opens calibration
  review tasks when raw probabilities are miscalibrated or sample size is
  insufficient.
- Probability calibration reports now have a model-governance submit path and
  worker command that persist calibration evidence and record audit lineage
  while explicitly avoiding calibrated serving activation, threshold changes, or
  label assignment.
- Model promotion gates now consume the latest same-version probability
  calibration report and block activation when calibration evidence is missing
  or failing, without rewriting probabilities or activating calibrated serving.
- The worker now has a scheduled data-pipeline plan contract that orders the
  build and submit commands for sanctions, provider profiles, graph signals,
  peer benchmarks, episode rollups, clinical references, unbundling candidates,
  scoring feature contexts, and probability-calibration evidence under daily
  or monthly cadence with explicit readiness gates. OIG/SAM sanctions now start
  with a configured-endpoint snapshot fetch job and then feed the governed
  snapshot into the sanctions sync submit path.
- The worker now has a data-pipeline readiness report that checks customer-
  approved artifact URIs, minimum row counts, data-quality status, evidence
  refs, and external OIG/SAM fetch configuration before scheduled writes. A
  generated readiness input template gives customer operators the exact evidence
  fields to fill while preserving a default blocked state.
- The worker data-pipeline readiness report now has a permission-gated API
  submit path that persists prerequisite evidence and records governance audit
  lineage while explicitly avoiding external fetch execution or artifact
  submission.
- The worker now has a data-pipeline execution report contract that converts a
  customer scheduler run-status artifact into per-job completion, pending,
  failed, dependency-blocked, and review-task evidence without running live
  customer jobs itself.
- The worker data-pipeline execution report now has a permission-gated API
  submit path that persists scheduler evidence and records governance audit
  lineage while explicitly avoiding claim scoring, label assignment, claim
  denial, model activation, or routing-policy changes.
- Worker data-pipeline readiness and execution submit paths now validate
  optional review-task `required_permission` scopes, including endpoint-family
  matching when a review task carries an `api_path`, before persisting scheduler
  evidence.
- Worker-generated readiness/execution review tasks now carry the planned
  `api_path` alongside `required_permission`, so operator review evidence and
  API permission validation use the same endpoint-family contract.
- Worker data-pipeline plans, readiness input templates, and run-status
  templates now expose per-job `required_evidence_prefixes`; readiness reports
  keep jobs blocked until required prefixes are non-blank and customer evidence
  includes those prefixes, including scoring-context source lineage for episode,
  peer, clinical, and unbundling artifacts.
- Worker data-pipeline readiness submit API now also rejects ready jobs with
  blank required evidence prefixes or evidence refs that do not satisfy the
  declared prefixes before persisting prerequisite evidence.
- Worker data-pipeline execution reports now also consume those required
  evidence prefixes, mark succeeded jobs with missing prefixes as
  `artifact_missing_evidence`, and the execution submit API rejects completed
  jobs whose evidence refs do not satisfy the declared prefixes.

Remaining boundaries after those commits are live scheduler deployment/runtime
execution, official OIG/SAM feed configuration and credentials, customer
claim/history data, LLM-backed specialist execution, real external tool-call
runtime mediation, wiring long-
running/tool-using agents into the cancellation signal, customer-approved
ICD-10/CPT or medical-policy reference data, customer-approved unbundling rule
packs, customer-approved feature lineage/source mappings, calibrated-
probability serving activation, and replacement of the L3 heuristic anomaly
scorer with a validated statistical baseline. Audit retention still needs
customer-environment partitioning, archive storage, legal-hold reconciliation
writes, approved destruction workflow execution, and live routing-impact
validation on customer data. The production readiness contract now includes a
dedicated `worker_data_pipeline_execution` gate so customer scheduler evidence
for the governed worker write paths is required before production readiness can
be claimed; that gate now carries acceptance checks for ready readiness status,
completed scheduler status, zero pending or failed jobs, zero review tasks,
completed governed job kinds, submitted governed write jobs, source snapshot
artifact evidence, scheduler-reported job success without dependency blockers,
expected API paths and permission scopes, required execution URIs, required
per-completed-job artifact URI/evidence refs, required evidence refs, and the
no-adjudication boundary. It also checks scoring-context materialization source
lineage against the same episode, peer, clinical, and unbundling evidence refs
required by the API submit path. Worker-generated plan/readiness/run-status
templates now surface those required evidence prefixes before scheduler
execution, and the production readiness
validator can execute those checks when given the customer evidence directory.

## A. Scoring Layer Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| A-1 | L1 peer percentile fallback is explicitly marked as proxy data when real peer data is absent; real peer percentile can now come from inline/materialized context or the latest persisted benchmark group when provider specialty, region, and explicit service segment are available. | Keep proxy paths excluded or down-weighted when those real peer inputs are missing, and validate persisted peer benchmarks against customer claim history before production use. |
| A-2 | `diagnosis_procedure_match_score` has a real clinical compatibility input path, worker reference contract, permission-gated clinical reference submit/write path, and claim-time persisted-reference lookup. The default remains a hard-coded heuristic fallback when neither inline/materialized context nor a matching governed reference exists. | Populate the persisted reference set from customer-approved ICD-10/CPT compatibility or medical-policy data and validate it against customer claims before using this layer as production clinical consistency. |
| A-3 | Confidence scoring now includes broader multi-layer evidence instead of only rule/anomaly/model agreement. | Validate confidence thresholds against customer routing queues and live reviewer capacity before production routing. |
| A-4 | Provider graph input now carries billing-ring, temporal co-billing, referral entropy, and related persisted graph signals. | Validate graph rollups against customer claim/referral history before treating them as production network-risk evidence. |
| A-5 | Seven-layer scoring now distinguishes missing data from real zero scores and renormalizes across available layers. | Recalibrate layer weights against customer labels and routing impact before production activation. |
| A-6 | Provider history counts now use the same 30/90/365 recency weighting philosophy as provider profile risk, producing effective counts instead of letting a 365-day max dominate current scoring. | Validate effective-count interpretation with customer review policy before exposing it as an operational KPI. |
| A-7 | L3 anomaly upgrade readiness now has a worker report contract with quantified label/recall gates; the online scorer remains heuristic until validated statistical replacement data exists. | Replace the heuristic scorer with IQR/MAD or ensemble scoring only after customer labels/history meet the readiness gates. |
| A-8 | FWA feature families for revisit frequency, duplicate-claim similarity, procedure frequency vs. peers, and unbundling candidates now have optional feature/scoring inputs, a worker materialization contract, persisted API ingestion and claim-time lookup support, direct persisted episode-rollup lookup for member-provider utilization, direct persisted unbundling comparator lookup for candidate counts, an episode-rollup submit/write path, a clinical-compatibility reference submit/write path, and an unbundling comparator candidate submit/write path. Production scheduling and customer-data validation remain open. | Connect scheduled worker artifact persistence and customer-approved data sources before treating these schemes as production-covered. |

## B. Feature Engineering And Data Quality Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| B-1 | Peer percentile fallback remains a proxy path unless upstream supplies peer context or a matching persisted peer benchmark group is available by provider specialty, region, and explicit service segment. | Feature metadata must continue to expose `is_proxy`, `data_source`, and source lineage so compliance reports can distinguish real distributions from estimates. |
| B-2 | Episode-level features now have worker-owned 30/90/365 member-provider aggregation, scoring input materialization, a permission-gated submit/write path, and claim-time persisted rollup lookup for member-provider utilization fallback; production scheduling and customer claim-history validation remain open. | Connect scheduled customer-history rollups before treating unbundling and excessive-utilization coverage as production-ready. |
| B-3 | ICD-10/CPT clinical compatibility ingestion now has a permission-gated reference submit/write path and claim-time scoring lookup. Unbundling comparator candidates now also have a permission-gated submit/write path and claim-time candidate-count lookup, but customer-approved rule packs, production scheduling, and customer-data validation remain open. | Connect clinical compatibility references, bundled/component code references, and episode co-occurrence outputs to governed customer data before treating clinical matching or unbundling detection as production-covered. |
| B-4 | Feature records and online scoring responses now expose proxy/source metadata for compliance traceability. | Fill customer-approved feature lineage/source mappings before production compliance reporting. |

## C. Agentic Investigation Control-Plane Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| C-1 | Agent identity registry exists as an additive runtime authority for deterministic agent capability/PHI checks. | Extend the registry to LLM-backed and externally mediated agents before enabling those runtimes. |
| C-2 | Independent `investigations` now group agent runs under stable investigation ids. | Validate multi-run investigation lifecycle with customer operating procedures. |
| C-3 | Registry PHI allowlists are enforced for deterministic knowledge access, and audit events persist accessed PHI field names without values. | Extend enforcement to every future external tool and LLM specialist boundary. |
| C-4 | Kill-switch behavior now includes an API control plane plus crate-level deterministic cancellation checkpoints, but no long-running/tool-using agent runtime is wired to the signal yet. | Wire runtime specialist dispatch and tool calls through the cancellation signal before adding LLM-backed or long-running agents. |
| C-5 | Deterministic investigation now has an orchestrator trait, specialist-plan contract, deterministic specialist dispatch/tool mediation contract, and API/OpenAPI exposure for specialist execution traces, but not LLM-backed specialist execution or real external tool runtime mediation. | Add LLM-backed investigators and real tool mediation only after governance checks and cancellation wiring are stable. |

## D. ML Lifecycle And Governance Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| D-1 | Serving manifest loading now caches parsed manifests after the first score request. | Keep customer deployment evidence focused on cache behavior under live serving load rather than reimplementing manifest parsing. |
| D-2 | Poisoned ONNX/session mutex recovery is implemented with error logging. | Add customer-environment alert routing and incident evidence. |
| D-3 | PSI threshold actioning now creates monitoring/review artifacts. | Wire action artifacts into customer model-governance operations. |
| D-4 | Rule hit-rate trending now computes 7-day vs. 90-day drift and emits review artifacts. | Validate thresholds against customer production traffic and rule governance cadence. |
| D-5 | L3 anomaly scoring is still a heuristic baseline. | Add worker evaluation for IQR/MAD or ensemble anomaly readiness and make the upgrade trigger measurable. |
| D-6 | Raw sigmoid outputs are explicitly marked as `uncalibrated_raw_sigmoid`, calibration evidence report contracts exist, and model activation gates now require same-version passing probability-calibration evidence. | Fit and activate calibrated probability serving only after customer labels, holdout, and governance approval. |

## E. Compliance, Security, And HIPAA Gaps

| ID | Gap | Required planning response |
| --- | --- | --- |
| E-1 | Validation-error serialization now fails loudly instead of silently storing empty arrays. | Keep audit replay evidence in customer validation runs. |
| E-2 | Canonical claim context and evidence responses now apply field-path PHI/PII masking at API response boundaries. | Confirm customer-specific raw payload adapters and PII field policies. |
| E-3 | Routing policy lifecycle writes now require fine-grained permissions aligned with rule lifecycle governance. | Validate production role assignments and approval workflow. |
| E-4 | Known PHI/PII fields are masked by field path instead of relying only on value-pattern detection. | Extend field-path maps for each customer payload schema. |
| E-5 | Audit retention now has a dry-run scan contract, and production readiness validation deep-checks customer retention/legal-hold evidence; deletion/archive execution is still not performed locally. | Add customer-environment partitioning, cold archive, legal-hold reconciliation, and approved destruction execution. |

## F. Worker Data Pipeline Gaps

| Pipeline | Current gap | Priority |
| --- | --- | --- |
| OIG/SAM daily sanctions sync | Configured-endpoint snapshot fetcher is part of the scheduled pipeline before sanctions sync; local worker contract, submit/write path, and claim-time sanctions consumption exist; official feed configuration and scheduling remain. | P1 |
| Provider profile windows | Local 30/90/365 worker rollup, submit/write path, and claim-time persisted profile consumption exist; customer claim-history validation remains. | P1 |
| Billing ring detection | Local graph-signal rollup and persisted consumption exist; production validation against customer claim/referral history remains. | P2 |
| Temporal co-billing | Local graph-signal rollup and persisted consumption exist; production validation against dated customer claim history remains. | P2 |
| Referral entropy | Local entropy-backed graph signal and persisted consumption exist; production validation against customer referral history remains. | P2 |
| Peer-group percentile benchmark | Local worker rollup, permission-gated submit/write path, and claim-time persisted benchmark lookup exist; production scheduling and customer claim-history validation remain. | P2 |
| Episode aggregation | Local member-provider rollup, submit/write path, and claim-time persisted episode consumption exist; production scheduling and customer history validation remain. | P2 |
| PSI actioning | PSI actioning exists as monitoring/review artifacts. | P1 |
| Rule hit-rate trending | 7-day vs. 90-day computation and drift review artifacts exist; production scheduling remains. | P1 |

## G. Scheme Coverage Implications

The strongest current coverage is provider peer outlier and suspicious provider
  relationship review, and both now have local worker rollups plus claim-time
persisted consumption paths. Duplicate billing, unbundling, excessive
utilization, lab abuse, telehealth abuse, genetic testing abuse,
pharmacy/opioid abuse, and DME/home health abuse still require
customer-approved episode history, code-frequency references, or medical-policy
reference data before they can be treated as production-complete schemes.

China-market-specific patterns also need explicit feature and evidence support:
early high-value claims, split treatment across multiple facilities in one
episode, suspicious beneficiary identity, and organized provider/member rings.

## H. Prioritized Roadmap

### P0 - Immediate Correctness And Safety

- Implemented: proxy/placeholder labels, routing-policy permissions,
  validation-error hard failure, and scorer mutex poison recovery.

### P1 - Current Month Architecture Completion

- Implemented locally: provider graph signal fields/rollups, missing-data-aware
  scoring, broader confidence scoring, provider-profile windows, sanctions
  snapshot fetcher plus sync contract/write path, PSI actioning, rule hit-rate
  trending, and field-path PHI/PII response masking.
- Remaining: official OIG/SAM feed configuration and credentials, customer
  scheduler execution, customer claim-history validation, production role
  assignments, and live routing-impact evidence.

### P2 - Next Quarter Foundation Work

- Implemented locally: billing-ring/temporal/referral graph rollups, peer
  percentile benchmarks, episode aggregation, `agent_registry`,
  `investigations`, PHI allowlist enforcement, populated PHI field audits,
  episode-level features, ICD-10/CPT clinical reference inputs, unbundling
  comparator contracts, feature proxy/source metadata, serving manifest caching,
  and probability-calibration activation gating.
- Remaining: customer-approved reference/rule packs, customer data validation,
  production scheduler execution, and LLM-backed external tool mediation.

### P3 - Medium-Term Agentic And Compliance Hardening

- Continue from the implemented control-plane contracts into LLM-backed
  specialist execution and real external tool mediation.
- Execute audit retention in the customer environment with archive/legal-hold
  reconciliation and approved destruction workflow.
- Replace L3 heuristic anomaly baseline with IQR/MAD or ensemble scoring once
  customer label/history thresholds are met.
- Extend scheme coverage for China-market-specific organized fraud patterns.
