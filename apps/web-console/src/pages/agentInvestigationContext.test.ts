import { describe, expect, it } from "vitest";
import { buildAgentInvestigationContextFromScoring } from "./agentInvestigationContext";

describe("buildAgentInvestigationContextFromScoring", () => {
  it("prefers the API-provided agent investigation prefill contract", () => {
    expect(
      buildAgentInvestigationContextFromScoring({
        run_id: "run_CLM-0287",
        claim_id: "CLM-FALLBACK",
        risk_score: 1,
        rag: "Green",
        top_reasons: [],
        alerts: [],
        similar_cases: [],
        agent_investigation_prefill: {
          claim_id: "CLM-0287",
          risk_score: 87,
          rag: "RED",
          scheme_family: "diagnosis_procedure_mismatch",
          top_reasons: ["金额高于同病种 P99"],
          similar_case_query: {
            diagnosis_code: "J10",
            provider_region: "Shanghai",
            tags: ["early_claim", "high_amount"],
          },
          evidence_refs: ["audit_events:aud_1"],
        },
      }),
    ).toEqual({
      source: "runtime_scoring",
      sourceRunId: "run_CLM-0287",
      claimId: "CLM-0287",
      riskScore: 87,
      rag: "RED",
      schemeFamily: "diagnosis_procedure_mismatch",
      topReasons: ["金额高于同病种 P99"],
      diagnosisCode: "J10",
      providerRegion: "Shanghai",
      tags: ["early_claim", "high_amount"],
    });
  });

  it("prefills agent investigation inputs from runtime scoring and payload hints", () => {
    expect(
      buildAgentInvestigationContextFromScoring(
        {
          run_id: "run_CLM-0287",
          claim_id: "CLM-0287",
          risk_score: 87,
          rag: "Red",
          top_reasons: ["金额高于同病种 P99", "诊断-项目匹配度偏低"],
          alerts: [{ alert_code: "EARLY_HIGH_CLAIM" }],
          similar_cases: [
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
              provenance_refs: ["knowledge_cases:KC-HIGH"],
              summary: "Higher match.",
              outcome: "Confirmed FWA.",
              evidence_refs: ["investigation_results:INV-HIGH"],
            },
          ],
        },
        JSON.stringify({
          claim: {
            diagnosis_code: "J10",
            provider: { region: "Shanghai" },
          },
        }),
      ),
    ).toEqual({
      source: "runtime_scoring",
      sourceRunId: "run_CLM-0287",
      claimId: "CLM-0287",
      riskScore: 87,
      rag: "RED",
      schemeFamily: "early_high_value_claim",
      topReasons: ["金额高于同病种 P99", "诊断-项目匹配度偏低"],
      diagnosisCode: "J10",
      providerRegion: "Shanghai",
      tags: ["provider_region", "early_claim", "early_high_claim"],
    });
  });

  it("keeps claim scoring context usable when stored-claim mode has no payload hints", () => {
    expect(
      buildAgentInvestigationContextFromScoring({
        run_id: "run_CLM-1",
        claim_id: "CLM-1",
        risk_score: 72,
        rag: "AMBER",
        top_reasons: [],
        alerts: [],
        similar_cases: [],
      }),
    ).toMatchObject({
      sourceRunId: "run_CLM-1",
      claimId: "CLM-1",
      riskScore: 72,
      rag: "AMBER",
      tags: [],
    });
  });
});
