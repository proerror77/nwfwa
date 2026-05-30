import { describe, expect, it } from "vitest";
import {
  buildDashboardLayerCoverageSummary,
  buildDashboardLayerRows,
} from "./dashboardLayerRows";

describe("buildDashboardLayerRows", () => {
  it("returns dashboard layer rows in seven-layer order", () => {
    const rows = buildDashboardLayerRows({
      L7_RISK_FUSION_ROUTING: {
        name: "Risk Fusion & Routing",
        scored_runs: 3,
        average_score: 81.6,
        high_risk_count: 2,
      },
      L3_UNSUPERVISED_ANOMALY: {
        name: "Unsupervised Anomaly",
        scored_runs: 3,
        average_score: 78.5,
        high_risk_count: 1,
      },
      L1_PEER_BENCHMARK: {
        name: "Peer Benchmark",
        scored_runs: 3,
        average_score: 74.2,
        high_risk_count: 1,
      },
    });

    expect(rows).toEqual([
      {
        layerId: "L1_PEER_BENCHMARK",
        name: "Peer Benchmark",
        scoredRuns: 3,
        averageScore: 74.2,
        highRiskCount: 1,
      },
      {
        layerId: "L3_UNSUPERVISED_ANOMALY",
        name: "Unsupervised Anomaly",
        scoredRuns: 3,
        averageScore: 78.5,
        highRiskCount: 1,
      },
      {
        layerId: "L7_RISK_FUSION_ROUTING",
        name: "Risk Fusion & Routing",
        scoredRuns: 3,
        averageScore: 81.6,
        highRiskCount: 2,
      },
    ]);
  });

  it("handles empty dashboard layer scores", () => {
    expect(buildDashboardLayerRows({})).toEqual([]);
  });
});

describe("buildDashboardLayerCoverageSummary", () => {
  it("summarizes seven-layer score coverage", () => {
    expect(
      buildDashboardLayerCoverageSummary({
        L1_PEER_BENCHMARK: {
          name: "Peer Benchmark",
          scored_runs: 3,
          average_score: 74.2,
          high_risk_count: 1,
        },
        L7_RISK_FUSION_ROUTING: {
          name: "Risk Fusion & Routing",
          scored_runs: 3,
          average_score: 81.6,
          high_risk_count: 2,
        },
      }),
    ).toEqual({
      expectedCount: 7,
      presentCount: 2,
      coverageLabel: "2/7",
      missingLayerIds: [
        "L2_RULE_DETECTION",
        "L3_UNSUPERVISED_ANOMALY",
        "L4_SUPERVISED_ML",
        "L5_MEDICAL_REASONABLENESS",
        "L6_PROVIDER_GRAPH_RISK",
      ],
      missingLayerLabel:
        "L2_RULE_DETECTION, L3_UNSUPERVISED_ANOMALY, L4_SUPERVISED_ML, L5_MEDICAL_REASONABLENESS, L6_PROVIDER_GRAPH_RISK",
    });
  });

  it("marks full seven-layer coverage when every layer is present", () => {
    const layerScores = Object.fromEntries(
      [
        "L1_PEER_BENCHMARK",
        "L2_RULE_DETECTION",
        "L3_UNSUPERVISED_ANOMALY",
        "L4_SUPERVISED_ML",
        "L5_MEDICAL_REASONABLENESS",
        "L6_PROVIDER_GRAPH_RISK",
        "L7_RISK_FUSION_ROUTING",
      ].map((layerId) => [
        layerId,
        {
          name: layerId,
          scored_runs: 1,
          average_score: 50,
          high_risk_count: 0,
        },
      ]),
    );

    expect(buildDashboardLayerCoverageSummary(layerScores)).toEqual({
      expectedCount: 7,
      presentCount: 7,
      coverageLabel: "7/7",
      missingLayerIds: [],
      missingLayerLabel: "none",
    });
  });
});
