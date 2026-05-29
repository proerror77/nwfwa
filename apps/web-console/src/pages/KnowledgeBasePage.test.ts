import { describe, expect, it } from "vitest";
import { buildSimilarCaseEvidenceRefs, type SimilarCase } from "./KnowledgeBasePage";

describe("buildSimilarCaseEvidenceRefs", () => {
  it("deduplicates provenance and evidence refs for similar case audit display", () => {
    const similarCase: SimilarCase = {
      case_id: "KC-1001",
      title: "Confirmed provider overuse case",
      scheme_family: "provider_peer_outlier",
      similarity_score: 0.91,
      matched_signals: ["provider_region", "tags"],
      retrieval_method: "structured_similarity",
      provenance_refs: ["knowledge_cases:KC-1001", "audit:knowledge.publish"],
      summary: "Provider pattern matched prior confirmed overuse.",
      outcome: "Provider education and post-payment audit opened.",
      evidence_refs: ["audit:knowledge.publish", "qa_reviews:QA-1001"],
    };

    expect(buildSimilarCaseEvidenceRefs(similarCase)).toEqual([
      "knowledge_cases:KC-1001",
      "audit:knowledge.publish",
      "qa_reviews:QA-1001",
    ]);
  });
});
