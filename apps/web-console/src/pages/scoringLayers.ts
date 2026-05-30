export type ScoringLayer = {
  layer_id: string;
  name: string;
  score: number;
  status: string;
  reason: string;
};

export const SCORING_LAYER_ORDER = [
  "L1_PEER_BENCHMARK",
  "L2_RULE_DETECTION",
  "L3_UNSUPERVISED_ANOMALY",
  "L4_SUPERVISED_ML",
  "L5_MEDICAL_REASONABLENESS",
  "L6_PROVIDER_GRAPH_RISK",
  "L7_RISK_FUSION_ROUTING",
];

export function buildScoringLayerSummary(layers: ScoringLayer[]) {
  const highestLayer = layers.slice().sort((left, right) => right.score - left.score)[0];
  const presentLayerIds = new Set(layers.map((layer) => layer.layer_id));
  const missingLayerIds = SCORING_LAYER_ORDER.filter((layerId) => !presentLayerIds.has(layerId));

  return {
    layerCount: layers.length,
    expectedLayerCount: SCORING_LAYER_ORDER.length,
    coverageLabel: `${SCORING_LAYER_ORDER.length - missingLayerIds.length}/${SCORING_LAYER_ORDER.length}`,
    missingLayerLabel: missingLayerIds.length === 0 ? "none" : missingLayerIds.join(", "),
    activeCount: layers.filter((layer) => layer.status === "active").length,
    baselineCount: layers.filter((layer) => layer.status === "baseline").length,
    highestLayerLabel: highestLayer
      ? `${highestLayer.layer_id} / ${highestLayer.score}`
      : "No layer data",
  };
}
