import { describe, expect, it } from "vitest";
import { buildEvidenceSufficiencyRows } from "./AgentInvestigatorPage";

describe("buildEvidenceSufficiencyRows", () => {
  it("marks minimum evidence items as present or missing", () => {
    expect(
      buildEvidenceSufficiencyRows({
        scheme_family: "provider_peer_outlier",
        status: "needs_more_evidence",
        minimum_evidence: ["peer_group_definition", "specialty", "statistical_deviation"],
        present_evidence: ["peer_group_definition"],
        missing_evidence: ["specialty", "statistical_deviation"],
      }),
    ).toEqual([
      { item: "peer_group_definition", status: "present" },
      { item: "specialty", status: "missing" },
      { item: "statistical_deviation", status: "missing" },
    ]);
  });
});
