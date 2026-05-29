import { describe, expect, it } from "vitest";
import { buildAgentSimilarCaseRows, buildEvidenceSufficiencyRows } from "./AgentInvestigatorPage";

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

describe("buildAgentSimilarCaseRows", () => {
  it("formats similar cases for the investigation evidence package", () => {
    expect(
      buildAgentSimilarCaseRows([
        {
          case_id: "KC-1001",
          similarity_score: 0.914,
          matched_signals: ["diagnosis_code", "provider_region"],
        },
        {
          case_id: "KC-1002",
          similarity_score: 0.72,
          matched_signals: [],
        },
      ]),
    ).toEqual([
      {
        caseId: "KC-1001",
        similarityLabel: "91%",
        matchedSignalLabel: "diagnosis_code, provider_region",
      },
      {
        caseId: "KC-1002",
        similarityLabel: "72%",
        matchedSignalLabel: "none",
      },
    ]);
  });
});
