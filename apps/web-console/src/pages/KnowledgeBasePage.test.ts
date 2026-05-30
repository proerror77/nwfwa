import { describe, expect, it } from "vitest";
import {
  buildKnowledgeCaseDetailSummary,
  buildPublishedCaseSummary,
  buildSimilarCaseEvidenceRefs,
  buildSimilarSearchSummary,
  type SimilarCase,
} from "./KnowledgeBasePage";

describe("buildKnowledgeCaseDetailSummary", () => {
  it("summarizes case tags and confirmed evidence provenance", () => {
    expect(
      buildKnowledgeCaseDetailSummary({
        case_id: "KC-1001",
        title: "Confirmed provider overuse case",
        fwa_type: "Waste",
        scheme_family: "provider_peer_outlier",
        diagnosis_code: "J10",
        provider_region: "Shanghai",
        provider_type: "hospital",
        summary: "Provider pattern matched prior confirmed overuse.",
        outcome: "Provider education and post-payment audit opened.",
        tags: ["provider_outlier", "high_amount"],
        evidence_refs: ["knowledge_cases:KC-1001", "qa_reviews:QA-1001"],
      }),
    ).toEqual({
      caseId: "KC-1001",
      schemeFamily: "provider_peer_outlier",
      tagLabel: "provider_outlier, high_amount",
      tagCount: 2,
      evidenceCount: 2,
      confirmedEvidence: true,
    });
  });

  it("uses explicit empty labels before a case is selected", () => {
    expect(buildKnowledgeCaseDetailSummary(null)).toEqual({
      caseId: "none",
      schemeFamily: "none",
      tagLabel: "none",
      tagCount: 0,
      evidenceCount: 0,
      confirmedEvidence: false,
    });
  });
});

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

describe("buildSimilarSearchSummary", () => {
  it("summarizes search results for TPA and agent evidence packages", () => {
    expect(
      buildSimilarSearchSummary([
        {
          case_id: "KC-LOW",
          title: "Lower match",
          scheme_family: "provider_peer_outlier",
          similarity_score: 0.72,
          matched_signals: ["provider_region"],
          retrieval_method: "structured_similarity",
          provenance_refs: ["knowledge_cases:KC-LOW"],
          summary: "Lower match.",
          outcome: "Provider education.",
          evidence_refs: ["qa_reviews:QA-LOW"],
        },
        {
          case_id: "KC-HIGH",
          title: "Higher match",
          scheme_family: "early_high_value_claim",
          similarity_score: 0.91,
          matched_signals: ["provider_region", "early_claim"],
          retrieval_method: "hybrid",
          provenance_refs: ["knowledge_cases:KC-HIGH", "audit:knowledge.publish"],
          summary: "Higher match.",
          outcome: "Confirmed FWA.",
          evidence_refs: ["audit:knowledge.publish", "investigation_results:INV-HIGH"],
        },
      ]),
    ).toEqual({
      resultCount: 2,
      topCaseLabel: "KC-HIGH · 91%",
      topSchemeFamily: "early_high_value_claim",
      retrievalMethods: "structured_similarity, hybrid",
      evidenceRefCount: 5,
      matchedSignalCount: 2,
    });
  });

  it("returns empty search labels before a search is run", () => {
    expect(buildSimilarSearchSummary(null)).toEqual({
      resultCount: 0,
      topCaseLabel: "none",
      topSchemeFamily: "none",
      retrievalMethods: "none",
      evidenceRefCount: 0,
      matchedSignalCount: 0,
    });
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
