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
      },
      {
        source_type: "agent",
        source_id: "agent_CLM-0287",
        action: "investigation_confirmed",
        saving_amount: "4100.00",
        currency: "CNY",
        claim_count: 1,
      },
    ]);

    expect(rows).toEqual([
      {
        key: "agent:agent_CLM-0287:investigation_confirmed",
        sourceLabel: "agent / agent_CLM-0287",
        action: "investigation_confirmed",
        savingAmount: "4100.00",
        currency: "CNY",
        claimCount: 1,
      },
      {
        key: "rule:EARLY_CLAIM:investigation_confirmed",
        sourceLabel: "rule / EARLY_CLAIM",
        action: "investigation_confirmed",
        savingAmount: "4100.00",
        currency: "CNY",
        claimCount: 1,
      },
    ]);
  });
});
