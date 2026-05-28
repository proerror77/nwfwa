import { describe, expect, it } from "vitest";
import { buildClaimIdScorePayload } from "./RuntimeScoring";
import { buildProviderProfileInspection } from "./providerProfileInspection";
import {
  buildClinicalEvidenceInspection,
  buildProviderGraphInspection,
  buildSimilarCaseInspection,
} from "./runtimeEvidence";

describe("buildClaimIdScorePayload", () => {
  it("builds the stored-claim scoring request contract", () => {
    expect(buildClaimIdScorePayload("tpa-demo", " CLM-0287 ", "post_payment")).toEqual({
      source_system: "tpa-demo",
      claim_id: "CLM-0287",
      review_mode: "post_payment",
    });
  });
});

describe("buildProviderProfileInspection", () => {
  it("summarizes provider review route, outliers, and evidence", () => {
    const inspection = buildProviderProfileInspection({
      provider_id: "PRV-PROVIDER-1",
      risk_score: 86,
      risk_tier: "high",
      review_required: true,
      review_route: "provider_review",
      outlier_flags: ["peer_amount_p97", "high_cost_item_ratio_90d"],
      evidence_refs: ["provider_profile:PRV-PROVIDER-1:90d"],
      window_findings: [
        {
          window_days: 30,
          risk_score: 58,
          reason: "Provider 30d behavior is elevated.",
        },
        {
          window_days: 90,
          risk_score: 86,
          reason: "Provider 90d peer amount percentile is high.",
        },
      ],
    });

    expect(inspection).toEqual({
      providerId: "PRV-PROVIDER-1",
      routeLabel: "provider_review",
      reviewLabel: "Review required",
      maxWindowLabel: "90d / 86",
      outlierSummary: "peer_amount_p97, high_cost_item_ratio_90d",
      evidenceSummary: "provider_profile:PRV-PROVIDER-1:90d",
    });
  });

  it("does not build an inspection when provider profile is missing", () => {
    expect(buildProviderProfileInspection(undefined)).toBeNull();
  });
});

describe("runtime evidence inspections", () => {
  it("summarizes clinical evidence gaps for medical review", () => {
    expect(
      buildClinicalEvidenceInspection({
        review_required: true,
        review_route: "medical_review",
        evidence_status: "missing_required_evidence",
        minimum_evidence: ["clinical_order", "medical_record"],
        missing_evidence: ["medical_record"],
        item_findings: [
          {
            item_code: "IMG-900",
            issue_type: "medical_necessity_review_required",
            required_evidence: ["clinical_order", "medical_record"],
            missing_evidence: ["medical_record"],
            reason: "High value imaging needs support.",
            review_route: "medical_review",
            evidence_refs: ["claim_items:IMG-900"],
          },
        ],
        evidence_refs: ["claim_items:IMG-900"],
      }),
    ).toEqual({
      reviewLabel: "Medical review required",
      routeLabel: "medical_review",
      statusLabel: "missing_required_evidence",
      findingCount: 1,
      firstFindingLabel: "IMG-900 · medical_necessity_review_required",
      minimumEvidenceSummary: "clinical_order, medical_record",
      missingEvidenceSummary: "medical_record",
      evidenceSummary: "claim_items:IMG-900",
    });
  });

  it("summarizes provider graph risk and evidence refs", () => {
    expect(
      buildProviderGraphInspection({
        provider_id: "PRV-GRAPH-1",
        risk_score: 82,
        risk_tier: "high",
        review_required: true,
        review_route: "provider_graph_review",
        graph_reasons: ["Provider graph community is elevated."],
        findings: [
          {
            signal: "provider_patient_overlap_score",
            risk_score: 25,
            reason: "Overlap is high.",
            evidence_ref: "provider_graph:PRV-GRAPH-1:provider_patient_overlap_score",
          },
          {
            signal: "network_component_risk_score",
            risk_score: 82,
            reason: "Component risk is high.",
            evidence_ref: "provider_graph:PRV-GRAPH-1:network_component_risk_score",
          },
        ],
        evidence_refs: ["relationship_edges:PRV-GRAPH-1"],
      }),
    ).toEqual({
      providerId: "PRV-GRAPH-1",
      riskLabel: "high / 82",
      reviewLabel: "Graph review required",
      routeLabel: "provider_graph_review",
      topSignalLabel: "network_component_risk_score / 82",
      reasonSummary: "Provider graph community is elevated.",
      evidenceSummary: "relationship_edges:PRV-GRAPH-1",
    });
  });

  it("summarizes the top similar knowledge case", () => {
    expect(
      buildSimilarCaseInspection([
        {
          case_id: "KC-LOW",
          title: "Lower match",
          scheme_family: "provider_outlier",
          similarity_score: 0.62,
          matched_signals: ["provider_pattern"],
          retrieval_method: "hybrid",
          provenance_refs: ["knowledge_cases:KC-LOW"],
          summary: "Lower match.",
          outcome: "Closed.",
          evidence_refs: [],
        },
        {
          case_id: "KC-HIGH",
          title: "Higher match",
          scheme_family: "early_high_value_claim",
          similarity_score: 0.91,
          matched_signals: ["early_claim", "high_amount"],
          retrieval_method: "hybrid",
          provenance_refs: ["knowledge_cases:KC-HIGH"],
          summary: "Higher match.",
          outcome: "Confirmed.",
          evidence_refs: [],
        },
      ]),
    ).toEqual({
      caseCount: 2,
      topCaseLabel: "KC-HIGH · 91%",
      schemeLabel: "early_high_value_claim",
      matchedSignalsSummary: "early_claim, high_amount",
      provenanceSummary: "knowledge_cases:KC-HIGH",
    });
  });
});
