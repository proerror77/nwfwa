use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RiskScoreError {
    #[error("risk score must be between 0 and 100")]
    OutOfRange,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskScore(u8);

impl RiskScore {
    pub fn new(value: u8) -> Result<Self, RiskScoreError> {
        if value <= 100 {
            Ok(Self(value))
        } else {
            Err(RiskScoreError::OutOfRange)
        }
    }

    /// Constructs a RiskScore clamping the value to [0, 100].
    /// Use when the caller cannot guarantee the value is already in range.
    pub fn saturating(value: u8) -> Self {
        Self(value.min(100))
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Green,
    Amber,
    Red,
}

impl RiskLevel {
    pub fn from_score(score: RiskScore) -> Self {
        Self::from_thresholds(score, 40, 70)
    }

    pub fn from_thresholds(score: RiskScore, amber_min: u8, red_min: u8) -> Self {
        match score.value() {
            value if value >= red_min => Self::Red,
            value if value >= amber_min => Self::Amber,
            _ => Self::Green,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendedAction {
    StandardProcessing,
    QaSample,
    ManualReview,
    RequestEvidence,
    EscalateInvestigation,
    PostPaymentAudit,
    ProviderReview,
    RecoveryReview,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionOutcome {
    StraightThrough,
    AutoDeny,
    PendingEvidence,
    ManualReview,
    QaSample,
    PostPaymentAudit,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAuthority {
    CustomerPolicyRule,
    ClinicalPolicyRule,
    RiskRoutingPolicy,
    HumanReviewer,
    QaPolicy,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionConfidence {
    Deterministic,
    High,
    Medium,
    Low,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleActionClass {
    HardDeny,
    StraightThrough,
    PendingEvidence,
    ManualReview,
    ScoreOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_scores_to_rag_levels() {
        assert_eq!(
            RiskLevel::from_score(RiskScore::new(20).unwrap()),
            RiskLevel::Green
        );
        assert_eq!(
            RiskLevel::from_score(RiskScore::new(55).unwrap()),
            RiskLevel::Amber
        );
        assert_eq!(
            RiskLevel::from_score(RiskScore::new(87).unwrap()),
            RiskLevel::Red
        );
    }

    #[test]
    fn maps_scores_to_policy_threshold_rag_levels() {
        assert_eq!(
            RiskLevel::from_thresholds(RiskScore::new(49).unwrap(), 50, 80),
            RiskLevel::Green
        );
        assert_eq!(
            RiskLevel::from_thresholds(RiskScore::new(50).unwrap(), 50, 80),
            RiskLevel::Amber
        );
        assert_eq!(
            RiskLevel::from_thresholds(RiskScore::new(80).unwrap(), 50, 80),
            RiskLevel::Red
        );
    }
}
