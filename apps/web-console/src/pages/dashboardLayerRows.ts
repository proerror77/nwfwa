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
    .sort((left, right) => left.layerId.localeCompare(right.layerId));
}
