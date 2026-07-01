# FWA Operations Studio UX Research And Service Blueprint

Date: 2026-07-01
Status: product UX blueprint for the web console

## Purpose

This document defines the business logic behind the FWA Operations Studio UI.
It should guide future page structure, labels, component hierarchy, and demo
scripts.

The product is not a generic BI dashboard, a payment engine, or an autonomous
fraud accuser. It is a human review workstation for claim risk, evidence,
workflow, and governance. The interface should help an operations team answer:

1. Which claims need attention now?
2. Why is each claim suspicious or blocked?
3. What evidence exists, what is missing, and who must act?
4. What human decision was made and how is it auditable?
5. Which reviewer outcomes should improve rules, models, and releases?

## Product Thesis

FWA work is a queue and evidence problem before it is an analytics problem.
The UI must optimize for reviewer action, evidence traceability, and governed
handoff. Metrics are useful only when they explain queue pressure, risk value,
review quality, or release readiness.

Design implication: the first screen should not lead with generic charts. It
should lead with actionable work, risk amount, evidence state, SLA state, and
the next human or governed system action.

## Research Inputs

### Repository Sources

- [README.md](../../README.md): product boundary, audience, and assistive
  decision policy.
- [FWA Risk And Operations Platform PRD](fwa-risk-operations-prd.md): routing
  outcomes, rule action classes, case workflow, feedback labels, and governance
  requirements.
- [TPA integration contract](../engineering/tpa-integration-contract.md):
  investigation, QA, medical review, audit history, evidence refs, and error
  boundaries.
- [Demo runbook](../engineering/demo-runbook.md): business-facing demo modules
  and end-to-end smoke coverage.

### External Sources

- CMS Medicare Program Integrity Manual, Chapter 3: medical review for FWA can
  require a 60-day window after requested documentation is received, because it
  may integrate investigative actions and external information.
  <https://www.cms.gov/regulations-and-guidance/guidance/manuals/downloads/pim83c03.pdf>
- CMS Medicare Program Integrity Manual, Chapter 4: potential FWA inquiries
  require logging, document review, referral packages, and handoff to program
  integrity review when FWA is suspected.
  <https://www.cms.gov/regulations-and-guidance/guidance/manuals/downloads/pim83c04.pdf>
- CMS Medicaid Program Integrity Manual, Chapter 1: UPIC functions include
  identifying FWA, investigating allegations, using data mining, and referring
  administrative actions such as overpayments or payment suspensions.
  <https://www.cms.gov/regulations-and-guidance/guidance/manuals/downloads/mpi115c01.pdf>
- HHS-OIG Exclusions Program: excluded individuals and entities cannot receive
  Federal health care program payment for furnished, ordered, or prescribed
  items or services; LEIE checks are an operational control.
  <https://oig.hhs.gov/exclusions/>
- HHS-OIG 2025 Medicaid managed care provider-fraud referral report: plans are
  required to identify and refer potential provider FWA, and training/dedicated
  referral staff correlate with more referrals.
  <https://oig.hhs.gov/reports/all/2025/some-medicaid-managed-care-plans-made-few-or-no-referrals-of-potential-provider-fraud/>
- HHS-OIG civil monetary penalties and exclusions: false or fraudulent claims
  include services not provided, duplicate coverage, improper coding, or lack of
  medical-record support.
  <https://oig.hhs.gov/fraud/enforcement/types-of-civil-monetary-penalties-and-affirmative-exclusions/>

## Primary Users

### 1. TPA Operations Lead

Goal: keep risky claims moving without creating unfair denials or audit gaps.

Needs:
- queue size, SLA pressure, risk amount, and evidence gaps;
- escalation controls for high-risk or delayed items;
- confidence that every action is logged and explainable.

UI priority:
- daily command center;
- queue filters by next action, SLA, amount, scheme family, and evidence state;
- release-readiness view for rules and models.

### 2. FWA Investigator

Goal: determine whether a claim needs confirmation, rejection, more evidence,
medical review, QA, or referral.

Needs:
- one case view with claim facts, risk reasons, evidence refs, audit history,
  similar cases, provider/member context, and next action;
- clear distinction between allegation, suspected pattern, documentation issue,
  abuse, waste, improper payment, and confirmed FWA;
- writeback controls that require evidence before final outcome.

UI priority:
- three-column case workbench: queue, evidence, decision;
- evidence sufficiency and missing evidence panel;
- final decision form with allowed labels and audit refs.

### 3. Medical Reviewer

Goal: decide whether clinical necessity or documentation supports the claim.

Needs:
- clinical facts, medical code context, document availability, and missing
  medical records;
- ability to request more evidence without labeling a claim as fraud.

UI priority:
- medical evidence queue;
- missing-document reason and deadline;
- decision labels such as medical necessity issue, documentation issue, or
  request more evidence.

### 4. QA Reviewer

Goal: calibrate false positives, missed risk, reviewer behavior, and training
signals.

Needs:
- sample cohorts, reviewer outcome, issue type, feedback target, and evidence
  refs;
- ability to send feedback to rules, model labels, routing, or operations.

UI priority:
- QA queue and feedback form;
- calibration metrics;
- links back to the original evidence chain.

### 5. Rule / Model Owner

Goal: promote only changes that have evidence, labels, backtests, shadow
results, drift checks, and rollback plans.

Needs:
- rule/model candidate source, evidence, label count, evaluation metrics,
  gates, blockers, and decision status;
- no ambiguous "deploy" action before governance is satisfied.

UI priority:
- release checklist over decorative topology;
- explicit gate status and blockers;
- traceability from reviewer outcomes to candidate changes.

### 6. Compliance / Customer Approver

Goal: verify the system did not make unauthorized claim decisions.

Needs:
- audit timeline, actor role, customer scope, evidence refs, API call records,
  and decision authority;
- proof that ML, anomaly, similar-case, and agent signals were assistive unless
  customer-approved deterministic policy was used.

UI priority:
- audit trail and policy authority surfaces;
- exported review packet;
- clear decision boundary labels.

## Jobs To Be Done

1. When a risky claim is detected, help me see whether it needs manual review,
   evidence request, medical review, QA sample, post-payment audit, or
   customer-approved deterministic action.
2. When I open a case, help me understand the risk without hunting through rule,
   model, document, and provider pages.
3. When evidence is missing, help me request the right evidence and pause the
   correct workflow state instead of forcing a fraud/reject conclusion.
4. When I close a review, force an outcome label, notes, and evidence refs so
   the result can train the system and survive audit.
5. When rule/model owners prepare a release, show whether the candidate is
   backed by reviewer outcomes, backtests, shadow evidence, QA labels, drift
   checks, and rollback.
6. When compliance audits a claim, provide a single timeline from intake to
   scoring, review, writeback, and governance.

## Experience Principles

1. Action before analytics.
   The top of each page should answer "what do I do now?" before showing
   secondary metrics.

2. Evidence before confidence.
   Risk score and model confidence are not enough. Every serious action needs
   evidence refs, source trace, or a clear evidence gap.

3. Human decision boundary.
   Labels must avoid implying automated guilt. Use "suspected", "risk signal",
   "documentation issue", "medical necessity issue", "confirmed by reviewer",
   and "customer-approved rule" precisely.

4. Workflow state is product state.
   New, triage, investigating, pending evidence, medical review, QA sample,
   confirmed, rejected, closed, and referred are not visual badges only; they
   determine next actions and page priority.

5. Governance is part of the work, not a separate admin afterthought.
   Reviewer outcomes should visibly feed rules, model labels, QA calibration,
   and release packages.

6. Dense but calm.
   This is an operations tool. Use compact grids, restrained color, stable
   cards, predictable controls, and high scan density. Avoid hero marketing,
   decorative diagrams, and vague "AI insight" language.

## Service Blueprint

| Phase | Customer / External Trigger | Frontstage UI | Backstage System | Data / Evidence | Failure Mode | UX Requirement |
|---|---|---|---|---|---|---|
| 1. Intake | TPA sends claim packet or claim id | Inbox / dashboard intake status | Normalize raw payload into canonical claim context | Source refs, evidence refs, audit id | Missing fields, unsupported source shape | Show data quality blocker and source path, not generic error |
| 2. Scoring | Claim needs pre-payment or post-payment review | Risk summary and route outcome | Rules, model, anomaly, provider, clinical, document checks | Rule hits, model version, score layers, evidence refs | Score shown without authority | Pair every route with decision authority and evidence refs |
| 3. Routing | Risk output creates lead or case | Dashboard command row and action queue | Lead/case creation, SLA, assignee, priority | Case id, lead source, scheme family, SLA | Queue becomes a metric-only dashboard | Show next action buckets first: review, evidence, medical, QA, audit |
| 4. Triage | Reviewer opens work queue | Action Queue | Filter and sort by risk, SLA, value, evidence state | Lead/case records | Reviewer cannot tell why an item is first | Explain rank reason and next action in each row |
| 5. Investigation | Investigator opens case | Case Workbench | Load member, provider, audit, similar cases, evidence pack | Claim facts, rule/model/document/similar refs | Reviewer jumps across pages | Keep queue, evidence, and decision visible together on desktop |
| 6. Evidence Request | Required documents or clinical proof missing | Missing Evidence panel | Generate request / pause state / audit event | Missing refs, requested docs, deadline | Missing evidence mislabeled as fraud | Use pending evidence and medical review states before fraud labels |
| 7. Medical Review | Clinical uncertainty or medical necessity issue | Medical Review queue | Medical reviewer writeback | Medical score, document refs, clinical outcome | Clinical review delayed or detached from case | Show reviewer, decision, notes, and evidence refs in case timeline |
| 8. Decision Writeback | Reviewer decides outcome | Decision panel | Persist investigation result | Outcome label, notes, financial impact, evidence refs | Notes or refs missing | Disable final action until required evidence fields are present |
| 9. QA Calibration | Sample or review outcome requires QA | QA queue / feedback | Persist QA result and feedback target | Issue type, target, refs | Feedback cannot improve system | Force target: rules, model, routing, evidence, operations |
| 10. Learning Loop | Outcomes become candidates | Learning / Governance | Mine rules, evaluate models, package releases | Candidate rules, labels, backtests, drift, gates | Improvement list is vague | Connect every candidate to source outcomes and release decision |
| 11. Release Approval | Customer approves rule/model/policy change | Rule / Model release checklist | Apply promotion gates and rollback plan | Version, evidence, approval, rollback | Unsafe model/rule affects routing | Show blockers before publish, require customer authority for adjudication |
| 12. Audit / Referral | Compliance or external investigator checks case | Audit timeline / export packet | Read audit events and API records | Actor, role, customer scope, refs | Cannot reconstruct decision | One timeline from intake to final outcome and release impact |

## Information Architecture

### Current Top-Level Navigation

1. Today / Dashboard
2. Work Queue
3. Case Workbench
4. Learning / Release
5. Rule Lab
6. Model Versions

This is directionally right. It should stay workflow-based, not technology-based.

### Recommended Navigation Refinement

| Nav Item | User Question | Primary User | Required First Screen |
|---|---|---|---|
| Today | What needs attention now? | Ops lead | Action buckets, risk amount, SLA, evidence blockers |
| Queue | Which item should I open next? | Ops lead, investigator | Sorted worklist with next action and rank reason |
| Case | What happened and what should I do? | Investigator | Queue + evidence + decision workbench |
| Medical | Is clinical evidence sufficient? | Medical reviewer | Missing docs, clinical issue, review writeback |
| QA | Was the review/routing correct? | QA reviewer | Sample queue, feedback target, label controls |
| Learning | What should become a governed change? | Rule/model owner | Outcome-to-rule/model candidates |
| Releases | Can this rule/model affect routing? | Approver | Gate checklist, blockers, approval, rollback |
| Audit | Can we prove what happened? | Compliance | Claim timeline, actors, refs, export packet |

The current app combines Medical, QA, Learning, Rules, Models, and Audit in
several places. That is acceptable for demo, but the product IA should move
toward the table above as workflows mature.

## Page-Level UX Requirements

### Today Dashboard

Must show:
- action buckets, not just counts;
- risk amount / prevented amount / recovered amount with clear definitions;
- SLA at risk;
- evidence blockers;
- queue change reason;
- recent high-priority arrivals;
- link into work queue and case workbench.

Avoid:
- large decorative charts;
- "AI found fraud" wording;
- metrics without a next action.

### Work Queue

Must show each row as an action card:
- claim / provider / member identifiers;
- scheme family;
- risk and value;
- current state;
- next action;
- evidence status;
- SLA;
- rank reason;
- owner.

Default sort:
1. patient harm or excluded provider signal;
2. SLA breach;
3. high financial impact;
4. high-risk score with sufficient evidence;
5. missing evidence approaching deadline;
6. QA calibration samples.

### Case Workbench

Desktop structure:
- left: queue and case context;
- center: evidence stack and explanation;
- right: decision, missing evidence, writeback.

Center evidence stack:
1. canonical claim facts;
2. top risk reasons;
3. rule hits;
4. model / anomaly contribution;
5. provider and member context;
6. document and medical evidence;
7. similar confirmed cases;
8. audit timeline.

Decision panel:
- recommended next action;
- allowed outcome labels;
- missing evidence checklist;
- required evidence refs;
- reviewer notes;
- financial impact;
- final writeback button.

### Medical Review

Must separate:
- "not enough documentation";
- "medical necessity issue";
- "clinical contradiction";
- "needs reviewer escalation";
- "no medical issue found".

The page should not imply fraud when the correct state is missing medical
documentation.

### QA Review

Must capture:
- false positive;
- missed risk;
- alert handling incomplete;
- documentation issue;
- abuse not fraud;
- improper payment;
- feedback target.

QA output should visibly feed the learning loop.

### Rule Lab

Must expose:
- rule action class;
- customer policy authority for adjudication rules;
- exception logic;
- backtest result;
- estimated saving;
- false positive history;
- lifecycle status;
- evidence refs;
- rollback path.

The primary action is not "publish" until gates are satisfied. Before that it
is "save candidate", "run backtest", "submit shadow evidence", or "request
approval".

### Model Versions

Must expose:
- candidate model;
- model version and runtime boundary;
- feature set;
- latest evaluation;
- drift;
- label readiness;
- release gates;
- blockers;
- rollback target.

Models must remain assistive unless a customer-approved deterministic policy
owns the decision.

### Learning / Governance

Must connect:
- reviewer outcomes;
- QA labels;
- candidate rules;
- model evaluation;
- customer release package.

The page should not be a generic improvement backlog.

### Audit

Must reconstruct:
- source system;
- claim id;
- actor and role;
- customer scope;
- route outcome and authority;
- evidence refs;
- scoring audit;
- investigation writeback;
- QA / medical writeback;
- final outcome;
- any rule/model release that affected routing.

## Component Model

### Action Card

Use for queue rows and dashboard next-action buckets.

Required fields:
- title;
- state;
- next action;
- why now;
- SLA;
- evidence status;
- owner.

### Evidence Ref Chip

Use wherever the UI references proof.

States:
- present;
- missing;
- canonical;
- generated;
- reviewer-added.

### Decision Authority Badge

Allowed values:
- customer policy rule;
- clinical policy rule;
- risk routing policy;
- human reviewer;
- QA policy;
- assistive signal only.

### Gate Checklist

Use for rule/model release.

Fields:
- gate;
- status;
- evidence source;
- blocker;
- owner;
- next action.

### Audit Timeline Row

Fields:
- timestamp;
- actor;
- role;
- event;
- status;
- evidence refs;
- API / webhook id when relevant.

## Copy Rules

Use:
- "suspected risk";
- "risk signal";
- "manual review";
- "request evidence";
- "medical necessity issue";
- "documentation issue";
- "confirmed by reviewer";
- "customer-approved policy rule";
- "assistive model signal".

Avoid:
- "AI found fraud";
- "fraudulent claim" before human confirmation;
- "auto deny" without policy authority;
- "black box";
- "prediction says".

## Visual Direction

Tone: quiet, dense, trustworthy operations system.

Layout:
- compact panels;
- stable grid;
- no nested card stacks;
- restrained borders;
- high scan density;
- few charts, mostly action tables and evidence rows.

Color:
- neutral surface for work;
- blue for route/context;
- teal/green for completed or safe;
- amber for missing evidence / pending action;
- red only for breach, high risk, or hard blocker.

Typography:
- small operational labels;
- clear numeric hierarchy;
- avoid hero-scale headings inside work surfaces.

Signature interaction:
- evidence chain should be visible across the journey: every major action can
  reveal "why this is here" and "what evidence supports it".

## Success Metrics

Operational:
- time from risk lead to first reviewer action;
- percent of queue with clear next action;
- percent of closed cases with evidence refs;
- pending evidence aging;
- SLA breach rate;
- review throughput by role.

Quality:
- false positive rate;
- confirmed FWA rate;
- insufficient evidence rate;
- QA issue rate;
- medical review reversal rate;
- missed-risk sample rate.

Governance:
- candidate rules with backtest evidence;
- model releases blocked by missing labels or drift;
- releases with rollback target;
- audit timeline completeness;
- customer-approved adjudication rules with policy authority refs.

Business:
- confirmed prevented amount;
- recovered amount;
- estimated impact;
- provider referral count;
- post-payment audit recovery;
- avoided manual review on safe deterministic routes.

## UX Gaps In Current Web Console

1. Medical review and QA are present but not yet first-class top-level reviewer
   lanes in the simplified navigation.
2. Evidence refs appear in many pages, but the app lacks one consistent
   evidence chain component.
3. Dashboard has improved action-first hierarchy, but queue ranking logic still
   needs visible "why now" explanations on every row.
4. Case workbench needs stronger missing-evidence controls before final
   writeback.
5. Rule/model release readiness is directionally right, but releases should be
   separated from learning once pilot usage grows.
6. Audit timeline should become a claim-level export packet, not only page-local
   details.

## Recommended Implementation Sequence

1. Add a reusable evidence chain component.
   Covers case, QA, medical review, rules, models, and audit.

2. Make queue rows action-first.
   Add next action, why now, evidence state, SLA, owner.

3. Strengthen case decision panel.
   Require outcome label, evidence refs, missing evidence resolution, and
   financial impact before writeback.

4. Promote Medical and QA lanes in IA.
   They are distinct reviewer roles, not secondary pages.

5. Split Learning and Releases.
   Learning explains candidate generation. Releases decide whether governed
   changes can affect routing.

6. Build claim audit export packet.
   One page or modal that reconstructs intake to final outcome.

## Acceptance Criteria For Future UI Work

A UI change is business-aligned only if it preserves these invariants:

- every high-risk item has a next action;
- every final review outcome has evidence refs;
- missing evidence can pause work without implying fraud;
- model and anomaly outputs are labeled assistive;
- customer-approved deterministic rules are visually distinct from risk-routing
  signals;
- rule/model releases show gates and blockers before publish;
- audit can reconstruct actor, role, customer scope, event, and refs.

If a screen cannot answer "what should the reviewer do next and why?", it is
not finished.
