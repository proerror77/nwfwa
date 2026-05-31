use fwa_core::{Provider, ProviderRiskTier};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderProfileWindow {
    pub window_days: u16,
    pub claim_count: u32,
    pub total_claim_amount: Decimal,
    pub high_cost_item_ratio: f64,
    pub diagnosis_procedure_mismatch_rate: f64,
    pub peer_amount_percentile: u8,
    pub peer_frequency_percentile: u8,
    pub review_failure_count: u32,
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderProfileInput {
    pub specialty: Option<String>,
    pub network_status: Option<String>,
    pub windows: Vec<ProviderProfileWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfileWindowFinding {
    pub window_days: u16,
    pub risk_score: u8,
    pub outlier_flags: Vec<String>,
    pub reason: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfileAssessment {
    pub provider_id: String,
    pub risk_score: u8,
    pub risk_tier: String,
    pub review_required: bool,
    pub review_route: String,
    pub specialty: Option<String>,
    pub network_status: Option<String>,
    pub review_failure_count: u32,
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
    pub outlier_flags: Vec<String>,
    pub window_findings: Vec<ProviderProfileWindowFinding>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderRelationshipGraphInput {
    pub high_risk_neighbor_ratio: f64,
    pub provider_patient_overlap_score: f64,
    pub referral_concentration_score: Option<f64>,
    pub connected_confirmed_fwa_count: u32,
    pub network_component_risk_score: Option<u8>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRelationshipGraphFinding {
    pub signal: String,
    pub risk_score: u8,
    pub reason: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRelationshipGraphAssessment {
    pub provider_id: String,
    pub risk_score: u8,
    pub risk_tier: String,
    pub review_required: bool,
    pub review_route: String,
    pub graph_reasons: Vec<String>,
    pub findings: Vec<ProviderRelationshipGraphFinding>,
    pub evidence_refs: Vec<String>,
}

pub fn assess_provider_profile(
    provider: &Provider,
    profile: Option<&ProviderProfileInput>,
) -> ProviderProfileAssessment {
    let Some(profile) = profile else {
        let risk_score = tier_score(provider.risk_tier);
        return ProviderProfileAssessment {
            provider_id: provider.external_provider_id.clone(),
            risk_score,
            risk_tier: risk_tier(risk_score).into(),
            review_required: risk_score >= 70,
            review_route: if risk_score >= 70 {
                "provider_review".into()
            } else {
                "none".into()
            },
            specialty: None,
            network_status: None,
            review_failure_count: 0,
            confirmed_fwa_count: 0,
            false_positive_count: 0,
            outlier_flags: Vec::new(),
            window_findings: Vec::new(),
            evidence_refs: vec![format!("providers:{}", provider.external_provider_id)],
        };
    };

    let window_findings = profile
        .windows
        .iter()
        .map(|window| assess_window(&provider.external_provider_id, window))
        .collect::<Vec<_>>();
    let risk_score = window_findings
        .iter()
        .map(|finding| finding.risk_score)
        .max()
        .unwrap_or_else(|| tier_score(provider.risk_tier));
    let outlier_flags = window_findings
        .iter()
        .flat_map(|finding| finding.outlier_flags.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let evidence_refs = window_findings
        .iter()
        .map(|finding| finding.evidence_ref.clone())
        .collect::<Vec<_>>();
    let review_failure_count = profile
        .windows
        .iter()
        .map(|window| window.review_failure_count)
        .max()
        .unwrap_or(0);
    let confirmed_fwa_count = profile
        .windows
        .iter()
        .map(|window| window.confirmed_fwa_count)
        .max()
        .unwrap_or(0);
    let false_positive_count = profile
        .windows
        .iter()
        .map(|window| window.false_positive_count)
        .max()
        .unwrap_or(0);

    ProviderProfileAssessment {
        provider_id: provider.external_provider_id.clone(),
        risk_score,
        risk_tier: risk_tier(risk_score).into(),
        review_required: risk_score >= 70,
        review_route: if risk_score >= 70 {
            "provider_review".into()
        } else {
            "none".into()
        },
        specialty: profile.specialty.clone(),
        network_status: profile.network_status.clone(),
        review_failure_count,
        confirmed_fwa_count,
        false_positive_count,
        outlier_flags,
        window_findings,
        evidence_refs,
    }
}

pub fn assess_provider_relationship_graph(
    provider: &Provider,
    graph: Option<&ProviderRelationshipGraphInput>,
) -> ProviderRelationshipGraphAssessment {
    let Some(graph) = graph else {
        return ProviderRelationshipGraphAssessment {
            provider_id: provider.external_provider_id.clone(),
            risk_score: 0,
            risk_tier: "no_data".into(),
            review_required: false,
            review_route: "none".into(),
            graph_reasons: Vec::new(),
            findings: Vec::new(),
            evidence_refs: Vec::new(),
        };
    };

    let provider_id = provider.external_provider_id.as_str();
    let mut score = 0_u16;
    let mut findings = Vec::new();

    if graph.high_risk_neighbor_ratio >= 0.30 {
        score += 35;
        findings.push(graph_finding(
            provider_id,
            "high_risk_neighbor_ratio",
            35,
            "Provider 关系邻居中高风险节点占比偏高",
        ));
    } else if graph.high_risk_neighbor_ratio >= 0.15 {
        score += 15;
        findings.push(graph_finding(
            provider_id,
            "high_risk_neighbor_ratio",
            15,
            "Provider 关系邻居中存在高风险节点集中信号",
        ));
    }

    if graph.provider_patient_overlap_score >= 0.65 {
        score += 25;
        findings.push(graph_finding(
            provider_id,
            "provider_patient_overlap_score",
            25,
            "Provider 与患者群体重叠度异常偏高",
        ));
    } else if graph.provider_patient_overlap_score >= 0.40 {
        score += 10;
        findings.push(graph_finding(
            provider_id,
            "provider_patient_overlap_score",
            10,
            "Provider 与患者群体存在重叠集中信号",
        ));
    }

    if graph.referral_concentration_score.unwrap_or(0.0) >= 0.70 {
        score += 20;
        findings.push(graph_finding(
            provider_id,
            "referral_concentration_score",
            20,
            "转诊或关联服务路径集中度偏高",
        ));
    }

    if graph.connected_confirmed_fwa_count > 0 {
        let contribution = (graph.connected_confirmed_fwa_count * 10).min(30) as u8;
        score += contribution as u16;
        findings.push(graph_finding(
            provider_id,
            "connected_confirmed_fwa_count",
            contribution,
            "Provider 关系网络连接到历史确认 FWA 案例",
        ));
    }

    if let Some(component_score) = graph.network_component_risk_score {
        score = score.max(component_score as u16);
        if component_score >= 70 {
            findings.push(graph_finding(
                provider_id,
                "network_component_risk_score",
                component_score,
                "Provider 所在关系社区整体风险偏高",
            ));
        }
    }

    let risk_score = score.min(100) as u8;
    let mut evidence_refs = graph.evidence_refs.clone();
    evidence_refs.extend(findings.iter().map(|finding| finding.evidence_ref.clone()));
    evidence_refs.sort();
    evidence_refs.dedup();

    ProviderRelationshipGraphAssessment {
        provider_id: provider.external_provider_id.clone(),
        risk_score,
        risk_tier: risk_tier(risk_score).into(),
        review_required: risk_score >= 70,
        review_route: if risk_score >= 70 {
            "provider_graph_review".into()
        } else {
            "none".into()
        },
        graph_reasons: findings
            .iter()
            .map(|finding| finding.reason.clone())
            .collect(),
        findings,
        evidence_refs,
    }
}

fn assess_window(
    provider_id: &str,
    window: &ProviderProfileWindow,
) -> ProviderProfileWindowFinding {
    let mut score = 0_u16;
    let mut flags = Vec::new();

    if window.peer_amount_percentile >= 95 {
        score += 30;
        flags.push(format!("peer_amount_p{}", window.peer_amount_percentile));
    }
    if window.peer_frequency_percentile >= 95 {
        score += 25;
        flags.push(format!(
            "peer_frequency_p{}",
            window.peer_frequency_percentile
        ));
    }
    if window.high_cost_item_ratio >= 0.60 {
        score += 20;
        flags.push("high_cost_item_ratio".into());
    }
    if window.diagnosis_procedure_mismatch_rate >= 0.30 {
        score += 15;
        flags.push("diagnosis_procedure_mismatch_rate".into());
    }
    if window.confirmed_fwa_count > 0 {
        score += 20;
        flags.push("confirmed_fwa_history".into());
    }
    score = score.saturating_sub((window.false_positive_count * 5).min(15) as u16);

    ProviderProfileWindowFinding {
        window_days: window.window_days,
        risk_score: score.min(100) as u8,
        outlier_flags: flags,
        reason: format!(
            "{} 天窗口内 Provider 同侪分位、费用结构和历史确认结果提示风险",
            window.window_days
        ),
        evidence_ref: format!("provider_profile:{provider_id}:{}d", window.window_days),
    }
}

fn tier_score(tier: ProviderRiskTier) -> u8 {
    match tier {
        ProviderRiskTier::Low => 10,
        ProviderRiskTier::Medium => 45,
        ProviderRiskTier::High => 80,
    }
}

fn risk_tier(score: u8) -> &'static str {
    match score {
        0..=39 => "low",
        40..=69 => "medium",
        _ => "high",
    }
}

fn graph_finding(
    provider_id: &str,
    signal: &str,
    risk_score: u8,
    reason: &str,
) -> ProviderRelationshipGraphFinding {
    ProviderRelationshipGraphFinding {
        signal: signal.into(),
        risk_score,
        reason: reason.into(),
        evidence_ref: format!("provider_graph:{provider_id}:{signal}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_core::{Provider, ProviderId};

    #[test]
    fn detects_peer_outlier_provider_profile() {
        let provider = Provider {
            id: ProviderId::from_external("PRV-1"),
            external_provider_id: "PRV-1".into(),
            name: "Demo Hospital".into(),
            provider_type: "hospital".into(),
            region: "SH".into(),
            risk_tier: ProviderRiskTier::Medium,
        };
        let profile = ProviderProfileInput {
            specialty: Some("imaging".into()),
            network_status: Some("in_network".into()),
            windows: vec![ProviderProfileWindow {
                window_days: 90,
                claim_count: 126,
                total_claim_amount: Decimal::new(420000, 0),
                high_cost_item_ratio: 0.72,
                diagnosis_procedure_mismatch_rate: 0.38,
                peer_amount_percentile: 97,
                peer_frequency_percentile: 96,
                review_failure_count: 3,
                confirmed_fwa_count: 4,
                false_positive_count: 1,
            }],
        };

        let assessment = assess_provider_profile(&provider, Some(&profile));

        assert!(assessment.review_required);
        assert_eq!(assessment.review_route, "provider_review");
        assert!(assessment.risk_score >= 80);
        assert!(assessment
            .outlier_flags
            .contains(&"peer_amount_p97".to_string()));
        assert_eq!(assessment.review_failure_count, 3);
        assert_eq!(assessment.confirmed_fwa_count, 4);
        assert_eq!(assessment.false_positive_count, 1);
        assert_eq!(assessment.evidence_refs[0], "provider_profile:PRV-1:90d");
    }

    #[test]
    fn detects_relationship_graph_risk_from_network_signals() {
        let provider = Provider {
            id: ProviderId::from_external("PRV-1"),
            external_provider_id: "PRV-1".into(),
            name: "Demo Hospital".into(),
            provider_type: "hospital".into(),
            region: "SH".into(),
            risk_tier: ProviderRiskTier::Low,
        };
        let graph = ProviderRelationshipGraphInput {
            high_risk_neighbor_ratio: 0.34,
            provider_patient_overlap_score: 0.68,
            referral_concentration_score: Some(0.72),
            connected_confirmed_fwa_count: 2,
            network_component_risk_score: Some(82),
            evidence_refs: vec!["relationship_edges:PRV-1".into()],
        };

        let assessment = assess_provider_relationship_graph(&provider, Some(&graph));

        assert_eq!(assessment.provider_id, "PRV-1");
        assert!(assessment.risk_score >= 90);
        assert_eq!(assessment.risk_tier, "high");
        assert!(assessment.review_required);
        assert_eq!(assessment.review_route, "provider_graph_review");
        assert!(assessment
            .graph_reasons
            .iter()
            .any(|reason| reason.contains("关系邻居")));
        assert!(assessment
            .evidence_refs
            .contains(&"relationship_edges:PRV-1".into()));
        assert!(assessment
            .evidence_refs
            .contains(&"provider_graph:PRV-1:network_component_risk_score".into()));
    }
}
