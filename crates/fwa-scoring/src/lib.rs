use fwa_core::{RecommendedAction, RiskLevel, RiskScore};
use fwa_ml_runtime::ModelScore;
use fwa_rules::RuleMatch;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoringDecision {
    pub risk_score: RiskScore,
    pub rag: RiskLevel,
    pub recommended_action: RecommendedAction,
    pub rule_score: u8,
    pub ml_score: u8,
    pub top_reasons: Vec<String>,
}

pub fn aggregate(rule_matches: &[RuleMatch], model_score: &ModelScore) -> ScoringDecision {
    let rule_score = rule_matches
        .iter()
        .map(|rule_match| rule_match.score_contribution)
        .sum::<u8>()
        .min(100);
    let final_score_value = ((rule_score as u16 + model_score.score as u16) / 2) as u8;
    let risk_score = RiskScore::new(final_score_value).expect("clamped score is valid");
    let rag = RiskLevel::from_score(risk_score);
    let recommended_action = match rag {
        RiskLevel::Green => RecommendedAction::AutoApprove,
        RiskLevel::Amber => RecommendedAction::ManualReview,
        RiskLevel::Red => RecommendedAction::EscalateInvestigation,
    };
    let mut top_reasons: Vec<String> = rule_matches
        .iter()
        .map(|rule_match| rule_match.reason.clone())
        .collect();
    top_reasons.extend(
        model_score
            .explanations
            .iter()
            .map(|explanation| explanation.reason.clone()),
    );
    top_reasons.truncate(5);

    ScoringDecision {
        risk_score,
        rag,
        recommended_action,
        rule_score,
        ml_score: model_score.score,
        top_reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_ml_runtime::ModelExplanation;

    #[test]
    fn aggregates_rule_and_model_scores() {
        let rules = vec![RuleMatch {
            rule_id: "rule_1".into(),
            rule_version: 1,
            score_contribution: 80,
            alert_code: "EARLY_HIGH_AMOUNT".into(),
            reason: "早期高额理赔".into(),
            recommended_action: RecommendedAction::ManualReview,
        }];
        let model = ModelScore {
            model_key: "baseline".into(),
            model_version: "0.1.0".into(),
            runtime_kind: "heuristic".into(),
            execution_provider: "cpu".into(),
            score: 90,
            label: "HIGH_RISK".into(),
            explanations: vec![ModelExplanation {
                feature: "claim_amount_to_limit_ratio".into(),
                direction: "increases_risk".into(),
                contribution: 0.8,
                reason: "金额比例高".into(),
            }],
            metadata: serde_json::json!({}),
            latency_ms: 0,
        };

        let decision = aggregate(&rules, &model);
        assert_eq!(decision.risk_score.value(), 85);
        assert_eq!(decision.rag, RiskLevel::Red);
        assert_eq!(
            decision.recommended_action,
            RecommendedAction::EscalateInvestigation
        );
    }
}
