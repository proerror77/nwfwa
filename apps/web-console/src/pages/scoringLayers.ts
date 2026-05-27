export type ScoringLayer = {
  layer_id: string;
  name: string;
  score: number;
  status: string;
  reason: string;
};

export function buildScoringLayerSummary(layers: ScoringLayer[]) {
  const highestLayer = layers.slice().sort((left, right) => right.score - left.score)[0];

  return {
    layerCount: layers.length,
    activeCount: layers.filter((layer) => layer.status === "active").length,
    baselineCount: layers.filter((layer) => layer.status === "baseline").length,
    highestLayerLabel: highestLayer
      ? `${highestLayer.layer_id} / ${highestLayer.score}`
      : "No layer data",
  };
}
