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
      activeCount: 2,
      baselineCount: 1,
      highestLayerLabel: "L1_PEER_BENCHMARK / 95",
    });
  });

  it("handles missing layer data without inventing scores", () => {
    expect(buildScoringLayerSummary([])).toEqual({
      layerCount: 0,
      activeCount: 0,
      baselineCount: 0,
      highestLayerLabel: "No layer data",
    });
  });
});
