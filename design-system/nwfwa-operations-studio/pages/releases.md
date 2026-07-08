# Releases Page Override

Use with `../MASTER.md`.

## Role

Rule/model owner and customer approver.

## First Screen

Show release readiness before any publish action:

1. Candidate version.
2. Source reviewer outcomes.
3. Backtest or evaluation evidence.
4. QA label readiness.
5. Drift or false-positive blocker.
6. Customer authority and rollback target.

## Acceptance

- Model, anomaly, similar-case, and agent signals are labeled assistive.
- Customer-approved deterministic rules are visually distinct from risk-routing
  signals.
- Publish actions stay secondary until gates pass.
