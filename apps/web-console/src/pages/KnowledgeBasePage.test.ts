import { describe, expect, it } from "vitest";
import {
  buildPublishedCaseSummary,
  buildSimilarCaseEvidenceRefs,
  type SimilarCase,
} from "./KnowledgeBasePage";

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

describe("buildPublishedCaseSummary", () => {
  it("summarizes published knowledge case audit evidence", () => {
    expect(
      buildPublishedCaseSummary({
        audit_id: "aud_knowledge_publish",
        case: {
          case_id: "KC-PUBLISHED-1",
          title: "Published provider lab overuse case",
          fwa_type: "Waste",
          scheme_family: "laboratory_testing_abuse",
          diagnosis_code: "E11",
          provider_region: "Guangzhou",
          provider_type: "lab",
          summary: "Confirmed repeated lab testing overuse pattern.",
          outcome: "Confirmed waste; provider education opened.",
          tags: ["lab_overuse"],
          evidence_refs: ["investigation_results:INV-KB-1", "qa_reviews:QA-KB-1"],
        },
      }),
    ).toEqual({
      caseId: "KC-PUBLISHED-1",
      title: "Published provider lab overuse case",
      schemeFamily: "laboratory_testing_abuse",
      auditId: "aud_knowledge_publish",
      evidenceCount: 2,
      evidenceRefs: ["investigation_results:INV-KB-1", "qa_reviews:QA-KB-1"],
    });
  });
});
