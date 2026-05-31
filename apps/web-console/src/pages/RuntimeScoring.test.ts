import { describe, expect, it } from "vitest";
import {
  buildClaimIdScorePayload,
  buildFeatureTraceRows,
  buildModelScoreSummary,
  buildRuntimeEvidenceRefRows,
  buildRoutingPolicySummary,
  buildTpaEmbeddedPanelSummary,
} from "./RuntimeScoring";
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

describe("buildRoutingPolicySummary", () => {
  it("summarizes the auditable routing thresholds", () => {
    expect(
      buildRoutingPolicySummary({
        policy_id: "fwa_risk_fusion_routing",
        version: 1,
        review_mode: "post_payment",
        risk_thresholds: {
          low_max: 39,
          medium_min: 40,
          high_min: 70,
          critical_min: 85,
        },
        confidence_thresholds: {
          low_confidence_below: 60,
          high_confidence_min: 80,
        },
        provider_review_threshold: 70,
      }),
    ).toEqual({
      policyLabel: "fwa_risk_fusion_routing v1",
      reviewModeLabel: "Post-payment",
      riskThresholdLabel: "Low <= 39, Medium >= 40, High >= 70, Critical >= 85",
      confidenceThresholdLabel: "Low < 60, High >= 80",
      providerThresholdLabel: "Provider review >= 70",
    });
  });
});

describe("buildTpaEmbeddedPanelSummary", () => {
  it("summarizes the scoring response fields needed by an embedded TPA panel", () => {
    expect(
      buildTpaEmbeddedPanelSummary({
        run_id: "run_CLM-1",
        audit_id: "audit_CLM-1",
        claim_id: "CLM-1",
        review_mode: "pre_payment",
        risk_score: 87,
        rag: "RED",
        risk_level: "High",
        recommended_action: "MANUAL_REVIEW",
        confidence_score: 76,
        confidence: "Medium",
        routing_reason: "High risk manual review.",
        routing_policy: {
          policy_id: "fwa_risk_fusion_routing",
          version: 1,
          review_mode: "pre_payment",
          risk_thresholds: {
            low_max: 39,
            medium_min: 40,
            high_min: 70,
            critical_min: 85,
          },
          confidence_thresholds: {
            low_confidence_below: 60,
            high_confidence_min: 80,
          },
          provider_review_threshold: 70,
        },
        scores: {
          peer_deviation_score: 90,
          rule_score: 80,
          anomaly_score: 70,
          ml_score: 75,
          medical_reasonableness_score: 65,
          provider_network_score: 60,
          similar_case_score: 55,
          final_score: 87,
        },
        model_score: {
          model_key: "baseline_fwa",
          model_version: "0.1.0",
          runtime_kind: "python_fastapi",
          execution_provider: "cpu",
          score: 75,
          label: "HIGH_RISK",
          explanations: [
            {
              feature: "claim_amount_to_limit_ratio",
              direction: "increases_risk",
              contribution: 0.8,
              reason: "理赔金额占保障额度比例较高",
            },
          ],
          metadata: {
            fraud_probability: 0.75,
            abuse_probability: 0.62,
            waste_probability: 0.48,
          },
          latency_ms: 12,
        },
        alerts: [
          {
            alert_code: "EARLY_HIGH_CLAIM",
            severity: "high",
            reason: "Early high claim.",
            rule_id: "rule_early_claim",
            rule_version: 1,
          },
        ],
        layers: [],
        top_reasons: ["金额高于同病种 P99", "保单生效后短期理赔"],
        similar_cases: [],
        feature_values: [],
        evidence_refs: ["rules:rule_early_claim:v1", "audit:audit_CLM-1"],
      }),
    ).toEqual({
      claimId: "CLM-1",
      riskScore: 87,
      rag: "RED",
      recommendedAction: "MANUAL_REVIEW",
      reviewModeLabel: "Pre-payment",
      confidenceLabel: "Medium (76)",
      alertCount: 1,
      topReasonCount: 2,
      evidenceCount: 2,
      auditId: "audit_CLM-1",
    });
    expect(buildTpaEmbeddedPanelSummary(null)).toBeNull();
  });
});

describe("buildModelScoreSummary", () => {
  it("summarizes model version, runtime, probabilities, and top explanation", () => {
    expect(
      buildModelScoreSummary({
        run_id: "run_CLM-1",
        audit_id: "audit_CLM-1",
        claim_id: "CLM-1",
        review_mode: "pre_payment",
        risk_score: 87,
        rag: "RED",
        risk_level: "High",
        recommended_action: "MANUAL_REVIEW",
        confidence_score: 76,
        confidence: "Medium",
        routing_reason: "High risk manual review.",
        routing_policy: {
          policy_id: "fwa_risk_fusion_routing",
          version: 1,
          review_mode: "pre_payment",
          risk_thresholds: {
            low_max: 39,
            medium_min: 40,
            high_min: 70,
            critical_min: 85,
          },
          confidence_thresholds: {
            low_confidence_below: 60,
            high_confidence_min: 80,
          },
          provider_review_threshold: 70,
        },
        scores: {
          peer_deviation_score: 90,
          rule_score: 80,
          anomaly_score: 70,
          ml_score: 75,
          medical_reasonableness_score: 65,
          provider_network_score: 60,
          similar_case_score: 55,
          final_score: 87,
        },
        model_score: {
          model_key: "baseline_fwa",
          model_version: "0.1.0",
          runtime_kind: "python_fastapi",
          execution_provider: "cpu",
          score: 75,
          label: "HIGH_RISK",
          explanations: [
            {
              feature: "claim_amount_to_limit_ratio",
              direction: "increases_risk",
              contribution: 0.8,
              reason: "理赔金额占保障额度比例较高",
            },
          ],
          metadata: {
            fraud_probability: 0.75,
            abuse_probability: 0.62,
            waste_probability: 0.48,
          },
          latency_ms: 12,
        },
        alerts: [],
        layers: [],
        top_reasons: [],
        similar_cases: [],
        feature_values: [],
        evidence_refs: [],
      }),
    ).toEqual({
      modelLabel: "baseline_fwa:0.1.0",
      runtimeLabel: "python_fastapi / cpu",
      scoreLabel: "75 · HIGH_RISK",
      fraudProbabilityLabel: "75.0%",
      abuseProbabilityLabel: "62.0%",
      wasteProbabilityLabel: "48.0%",
      explanationCount: 1,
      topExplanation: "理赔金额占保障额度比例较高",
    });

    expect(buildModelScoreSummary(null)).toBeNull();
  });
});

describe("buildFeatureTraceRows", () => {
  it("formats feature values with evidence refs for score traceability", () => {
    expect(
      buildFeatureTraceRows([
        {
          name: "claim_amount_to_limit_ratio",
          version: 1,
          value: 0.8,
          evidence_refs: [
            {
              entity_type: "claim",
              entity_id: "CLM-0287",
              field: "claim_amount",
            },
          ],
        },
      ]),
    ).toEqual([
      {
        key: "claim_amount_to_limit_ratio:1",
        name: "claim_amount_to_limit_ratio",
        versionLabel: "v1",
        valueLabel: "0.8",
        evidenceLabel: "claim:CLM-0287.claim_amount",
      },
    ]);
  });
});

describe("buildRuntimeEvidenceRefRows", () => {
  it("formats mixed runtime evidence refs for audit display", () => {
    expect(
      buildRuntimeEvidenceRefRows([
        {
          entity_type: "claim",
          entity_id: "CLM-0287",
          field: "claim_amount",
        },
        "rule_runs:EARLY_CLAIM",
        { source: "knowledge_cases", id: "KC-1001" },
      ]),
    ).toEqual([
      {
        key: "0:claim:CLM-0287.claim_amount",
        label: "claim:CLM-0287.claim_amount",
        kind: "feature",
      },
      {
        key: "1:rule_runs:EARLY_CLAIM",
        label: "rule_runs:EARLY_CLAIM",
        kind: "reference",
      },
      {
        key: '2:{"source":"knowledge_cases","id":"KC-1001"}',
        label: '{"source":"knowledge_cases","id":"KC-1001"}',
        kind: "reference",
      },
    ]);
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

  it("uses taxonomy labels for similar knowledge case schemes", () => {
    expect(
      buildSimilarCaseInspection(
        [
          {
            case_id: "KC-HIGH",
            title: "Higher match",
            scheme_family: "early_high_value_claim",
            similarity_score: 0.91,
            matched_signals: ["early_claim"],
            retrieval_method: "hybrid",
            provenance_refs: ["knowledge_cases:KC-HIGH"],
            summary: "Higher match.",
            outcome: "Confirmed.",
            evidence_refs: [],
          },
        ],
        { early_high_value_claim: "Early high-value claim (early_high_value_claim)" },
      ).schemeLabel,
    ).toBe("Early high-value claim (early_high_value_claim)");
  });
});
