use super::{ModelExplanation, ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer};
use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct HeuristicModelScorer;

#[async_trait]
impl ModelScorer for HeuristicModelScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        let ratio = request
            .features
            .get("claim_amount_to_limit_ratio")
            .and_then(|feature| feature.value.as_f64())
            .unwrap_or(0.0);
        let score = (ratio * 100.0).round().clamp(0.0, 100.0) as u8;
        Ok(ModelScore {
            model_key: request.model_key,
            model_version: request.model_version,
            runtime_kind: "heuristic".into(),
            execution_provider: "cpu".into(),
            score,
            label: if score >= 70 { "HIGH_RISK" } else { "LOW_RISK" }.into(),
            explanations: vec![ModelExplanation {
                feature: "claim_amount_to_limit_ratio".into(),
                direction: "increases_risk".into(),
                contribution: ratio,
                reason: "理赔金额占保障额度比例影响模型分".into(),
            }],
            metadata: serde_json::json!({
                "source": "heuristic",
                "calibration": "none",
                "fraud_probability": score as f64 / 100.0,
                "abuse_probability": (ratio * 0.70).clamp(0.0, 1.0),
                "waste_probability": (ratio * 0.40).clamp(0.0, 1.0)
            }),
            latency_ms: 0,
        })
    }
}
