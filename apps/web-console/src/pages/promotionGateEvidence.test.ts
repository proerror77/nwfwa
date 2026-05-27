import { describe, expect, it } from "vitest";
import { buildPromotionGateEvidenceRows } from "./promotionGateEvidence";

describe("buildPromotionGateEvidenceRows", () => {
  it("labels promotion gate evidence source for operators", () => {
    expect(
      buildPromotionGateEvidenceRows([
        {
          label: "Deterministic backtest evidence",
          passed: true,
          blocker: "backtest evidence missing",
          evidence_source: "backtest",
        },
        {
          label: "Shadow or limited rollout",
          passed: false,
          blocker: "shadow rollout missing",
          evidence_source: "missing",
        },
        {
          label: "Holdout metrics",
          passed: true,
          blocker: "holdout metrics missing",
          evidence_source: "evaluation",
        },
      ]),
    ).toEqual([
      {
        label: "Deterministic backtest evidence",
        status: "passed",
        evidenceSource: "Backtest",
        evidenceClassName: "source-backtest",
      },
      {
        label: "Shadow or limited rollout",
        status: "shadow rollout missing",
        evidenceSource: "Missing",
        evidenceClassName: "source-missing",
      },
      {
        label: "Holdout metrics",
        status: "passed",
        evidenceSource: "Evaluation",
        evidenceClassName: "source-evaluation",
      },
    ]);
  });
});
