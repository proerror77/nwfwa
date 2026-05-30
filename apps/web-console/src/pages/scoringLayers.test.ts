import { describe, expect, it } from "vitest";
import { buildScoringLayerSummary } from "./scoringLayers";

const layers = [
  {
    layer_id: "L1_PEER_BENCHMARK",
    name: "Peer Benchmark",
    score: 95,
    status: "active",
    reason: "Peer amount percentile is high.",
  },
  {
    layer_id: "L3_UNSUPERVISED_ANOMALY",
    name: "Unsupervised Anomaly",
    score: 72,
    status: "baseline",
    reason: "Rare claim pattern.",
  },
  {
    layer_id: "L7_RISK_FUSION_ROUTING",
    name: "Risk Fusion & Routing",
    score: 85,
    status: "active",
    reason: "Escalate for review.",
  },
];

describe("buildScoringLayerSummary", () => {
  it("summarizes layer count, active count, and highest layer score", () => {
    expect(buildScoringLayerSummary(layers)).toEqual({
      layerCount: 3,
      expectedLayerCount: 7,
      coverageLabel: "3/7",
      missingLayerLabel:
        "L2_RULE_DETECTION, L4_SUPERVISED_ML, L5_MEDICAL_REASONABLENESS, L6_PROVIDER_GRAPH_RISK",
      activeCount: 2,
      baselineCount: 1,
      highestLayerLabel: "L1_PEER_BENCHMARK / 95",
    });
  });

  it("handles missing layer data without inventing scores", () => {
    expect(buildScoringLayerSummary([])).toEqual({
      layerCount: 0,
      expectedLayerCount: 7,
      coverageLabel: "0/7",
      missingLayerLabel:
        "L1_PEER_BENCHMARK, L2_RULE_DETECTION, L3_UNSUPERVISED_ANOMALY, L4_SUPERVISED_ML, L5_MEDICAL_REASONABLENESS, L6_PROVIDER_GRAPH_RISK, L7_RISK_FUSION_ROUTING",
      activeCount: 0,
      baselineCount: 0,
      highestLayerLabel: "No layer data",
    });
  });

  it("marks complete coverage when all seven layers are returned", () => {
    const allLayers = [
      "L1_PEER_BENCHMARK",
      "L2_RULE_DETECTION",
      "L3_UNSUPERVISED_ANOMALY",
      "L4_SUPERVISED_ML",
      "L5_MEDICAL_REASONABLENESS",
      "L6_PROVIDER_GRAPH_RISK",
      "L7_RISK_FUSION_ROUTING",
    ].map((layerId, index) => ({
      layer_id: layerId,
      name: layerId,
      score: index + 1,
      status: "active",
      reason: "available",
    }));

    expect(buildScoringLayerSummary(allLayers)).toMatchObject({
      layerCount: 7,
      expectedLayerCount: 7,
      coverageLabel: "7/7",
      missingLayerLabel: "none",
    });
  });
});
