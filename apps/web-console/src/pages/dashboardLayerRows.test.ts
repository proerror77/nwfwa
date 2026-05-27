import { describe, expect, it } from "vitest";
import { buildDashboardLayerRows } from "./dashboardLayerRows";

describe("buildDashboardLayerRows", () => {
  it("returns dashboard layer rows in layer id order", () => {
    const rows = buildDashboardLayerRows({
      L7_RISK_FUSION_ROUTING: {
        name: "Risk Fusion & Routing",
        scored_runs: 3,
        average_score: 81.6,
        high_risk_count: 2,
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
