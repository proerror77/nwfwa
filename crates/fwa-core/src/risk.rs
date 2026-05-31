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
        match score.value() {
            0..=39 => Self::Green,
            40..=69 => Self::Amber,
            _ => Self::Red,
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
}
