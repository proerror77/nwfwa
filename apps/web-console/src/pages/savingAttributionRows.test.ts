import { describe, expect, it } from "vitest";
import { buildSavingAttributionRows } from "./savingAttributionRows";

describe("buildSavingAttributionRows", () => {
  it("returns saving attribution rows in source order", () => {
    const rows = buildSavingAttributionRows([
      {
        source_type: "rule",
        source_id: "EARLY_CLAIM",
        action: "investigation_confirmed",
        saving_amount: "4100.00",
        currency: "CNY",
        claim_count: 1,
        evidence_refs: ["rule_runs:EARLY_CLAIM"],
      },
      {
        source_type: "agent",
        source_id: "agent_CLM-0287",
        action: "investigation_confirmed",
        saving_amount: "4100.00",
        currency: "CNY",
        claim_count: 1,
        evidence_refs: ["agent_run:agent_CLM-0287"],
      },
    ]);

    expect(rows).toEqual([
      {
        key: "agent:agent_CLM-0287:investigation_confirmed",
        sourceLabel: "agent / agent_CLM-0287",
        lineageLabel: "agent:agent_CLM-0287 -> investigation_confirmed -> CNY 4100.00",
        lineageStatus: "lineage_evidence_present",
        action: "investigation_confirmed",
        savingAmount: "4100.00",
        currency: "CNY",
        claimCount: 1,
        averageSavingPerClaim: "4100.00",
        evidenceRefs: ["agent_run:agent_CLM-0287"],
      },
      {
        key: "rule:EARLY_CLAIM:investigation_confirmed",
        sourceLabel: "rule / EARLY_CLAIM",
        lineageLabel: "rule:EARLY_CLAIM -> investigation_confirmed -> CNY 4100.00",
        lineageStatus: "lineage_evidence_present",
        action: "investigation_confirmed",
        savingAmount: "4100.00",
        currency: "CNY",
        claimCount: 1,
        averageSavingPerClaim: "4100.00",
        evidenceRefs: ["rule_runs:EARLY_CLAIM"],
      },
    ]);
  });

  it("computes average saving per confirmed claim", () => {
    expect(
      buildSavingAttributionRows([
        {
          source_type: "model",
          source_id: "baseline_fwa",
          action: "investigation_confirmed",
          saving_amount: "9000.00",
          currency: "CNY",
          claim_count: 3,
          evidence_refs: ["model_scores:baseline_fwa"],
        },
      ])[0].averageSavingPerClaim,
    ).toBe("3000.00");
  });

  it("sorts evidence references for stable dashboard rendering", () => {
    expect(
      buildSavingAttributionRows([
        {
          source_type: "rule",
          source_id: "EARLY_CLAIM",
          action: "investigation_confirmed",
          saving_amount: "1000.00",
          currency: "CNY",
          claim_count: 1,
          evidence_refs: ["rules:rule_early_claim:v1", "audit:audit_1"],
        },
      ])[0].evidenceRefs,
    ).toEqual(["audit:audit_1", "rules:rule_early_claim:v1"]);
  });

  it("flags attribution rows that lack source-specific lineage evidence", () => {
    expect(
      buildSavingAttributionRows([
        {
          source_type: "agent",
          source_id: "agent_CLM-0287",
          action: "investigation_confirmed",
          saving_amount: "4100.00",
          currency: "CNY",
          claim_count: 1,
          evidence_refs: ["audit:audit_1"],
        },
      ])[0].lineageStatus,
    ).toBe("lineage_evidence_missing");
  });
});
