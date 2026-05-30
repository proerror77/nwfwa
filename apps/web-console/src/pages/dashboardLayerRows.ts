export type DashboardLayerScore = {
  name: string;
  scored_runs: number;
  average_score: number;
  high_risk_count: number;
};

export type DashboardLayerRow = {
  layerId: string;
  name: string;
  scoredRuns: number;
  averageScore: number;
  highRiskCount: number;
};

export const DASHBOARD_LAYER_ORDER = [
  "L1_PEER_BENCHMARK",
  "L2_RULE_DETECTION",
  "L3_UNSUPERVISED_ANOMALY",
  "L4_SUPERVISED_ML",
  "L5_MEDICAL_REASONABLENESS",
  "L6_PROVIDER_GRAPH_RISK",
  "L7_RISK_FUSION_ROUTING",
];

function layerRank(layerId: string) {
  const index = DASHBOARD_LAYER_ORDER.indexOf(layerId);
  return index === -1 ? DASHBOARD_LAYER_ORDER.length : index;
}

export function buildDashboardLayerRows(
  layerScores: Record<string, DashboardLayerScore>,
): DashboardLayerRow[] {
  return Object.entries(layerScores)
    .map(([layerId, layer]) => ({
      layerId,
      name: layer.name,
      scoredRuns: layer.scored_runs,
      averageScore: layer.average_score,
      highRiskCount: layer.high_risk_count,
    }))
    .sort((left, right) => {
      const rankDifference = layerRank(left.layerId) - layerRank(right.layerId);
      return rankDifference || left.layerId.localeCompare(right.layerId);
    });
}

export function buildDashboardLayerCoverageSummary(
  layerScores: Record<string, DashboardLayerScore> = {},
) {
  const missingLayerIds = DASHBOARD_LAYER_ORDER.filter(
    (layerId) => !Object.prototype.hasOwnProperty.call(layerScores, layerId),
  );
  const presentCount = DASHBOARD_LAYER_ORDER.length - missingLayerIds.length;
  return {
    expectedCount: DASHBOARD_LAYER_ORDER.length,
    presentCount,
    coverageLabel: `${presentCount}/${DASHBOARD_LAYER_ORDER.length}`,
    missingLayerIds,
    missingLayerLabel: missingLayerIds.length === 0 ? "none" : missingLayerIds.join(", "),
  };
}
