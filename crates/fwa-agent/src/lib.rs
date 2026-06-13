use fwa_core::assess_evidence_sufficiency;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimilarCaseInput {
    pub case_id: String,
    pub similarity_score: f64,
    pub matched_signals: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvestigationRequest {
    pub claim_id: String,
    pub risk_score: u8,
    pub rag: String,
    pub scheme_family: String,
    pub top_reasons: Vec<String>,
    pub similar_cases: Vec<SimilarCaseInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvestigationFinding {
    pub finding: String,
    pub evidence_refs: Vec<String>,
}

pub use fwa_core::EvidenceSufficiency;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EvidenceReferenceBuckets {
    pub claim: Vec<String>,
    pub provider: Vec<String>,
    pub rule: Vec<String>,
    pub model: Vec<String>,
    pub anomaly: Vec<String>,
    pub document: Vec<String>,
    pub similar_case: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvestigationPackage {
    pub agent_run_id: String,
    pub decision_boundary: String,
    pub risk_summary: String,
    pub findings: Vec<InvestigationFinding>,
    pub investigation_checklist: Vec<String>,
    pub similar_cases: Vec<SimilarCaseInput>,
    pub qa_opinion_draft: String,
    pub evidence_sufficiency: EvidenceSufficiency,
    pub evidence_refs: Vec<String>,
    pub evidence_refs_by_type: EvidenceReferenceBuckets,
}

#[derive(Debug, Clone, Copy)]
pub struct DeterministicInvestigator;

impl DeterministicInvestigator {
    pub fn investigate(self, request: InvestigationRequest) -> InvestigationPackage {
        let agent_run_id = format!("agent_{}", Ulid::new());
        let findings = build_findings(&request);
        let mut evidence_refs = findings
            .iter()
            .flat_map(|finding| finding.evidence_refs.clone())
            .collect::<Vec<_>>();
        for similar_case in &request.similar_cases {
            evidence_refs.extend(similar_case.evidence_refs.clone());
            evidence_refs.extend(similar_case.provenance_refs.clone());
        }
        evidence_refs.sort();
        evidence_refs.dedup();
        let evidence_refs_by_type = bucket_evidence_refs(&evidence_refs);

        let evidence_sufficiency = build_evidence_sufficiency(&request);

        InvestigationPackage {
            agent_run_id,
            decision_boundary: "assistive_only".into(),
            risk_summary: format!(
                "案件 {} 当前风险分 {}，RAG 为 {}。建议围绕高贡献风险原因整理证据并进入人工复核。",
                request.claim_id, request.risk_score, request.rag
            ),
            findings,
            investigation_checklist: vec![
                "核对保单生效日期、等待期和理赔发生日期".into(),
                "复核诊断、项目、药品与病历材料的一致性".into(),
                "对照相似 FWA 案例检查 provider 与项目组合".into(),
            ],
            similar_cases: request.similar_cases,
            qa_opinion_draft:
                "建议 QA 复核告警处理是否完整，并确认所有结论均有理赔、规则、模型或知识库证据支持。"
                    .into(),
            evidence_sufficiency,
            evidence_refs,
            evidence_refs_by_type,
        }
    }
}

pub fn crate_ready() -> bool {
    true
}

fn build_findings(request: &InvestigationRequest) -> Vec<InvestigationFinding> {
    let mut findings = request
        .top_reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| InvestigationFinding {
            finding: reason.clone(),
            evidence_refs: vec![format!(
                "claim:{}:top_reason:{}",
                request.claim_id,
                index + 1
            )],
        })
        .collect::<Vec<_>>();

    if findings.is_empty() {
        findings.push(InvestigationFinding {
            finding: "未提供显著风险原因，建议先补齐评分证据。".into(),
            evidence_refs: vec![format!("claim:{}:risk_score", request.claim_id)],
        });
    }
    findings
}

fn build_evidence_sufficiency(request: &InvestigationRequest) -> EvidenceSufficiency {
    assess_evidence_sufficiency(&request.scheme_family, &evidence_text(request))
}

fn evidence_text(request: &InvestigationRequest) -> String {
    let mut parts = request.top_reasons.clone();
    for similar_case in &request.similar_cases {
        parts.extend(similar_case.matched_signals.clone());
        parts.extend(similar_case.provenance_refs.clone());
        parts.extend(similar_case.evidence_refs.clone());
    }
    parts.join(" ").to_ascii_lowercase()
}

fn bucket_evidence_refs(evidence_refs: &[String]) -> EvidenceReferenceBuckets {
    let mut claim = BTreeSet::new();
    let mut provider = BTreeSet::new();
    let mut rule = BTreeSet::new();
    let mut model = BTreeSet::new();
    let mut anomaly = BTreeSet::new();
    let mut document = BTreeSet::new();
    let mut similar_case = BTreeSet::new();

    for reference in evidence_refs {
        match evidence_ref_bucket(reference) {
            Some("claim") => {
                claim.insert(reference.clone());
            }
            Some("provider") => {
                provider.insert(reference.clone());
            }
            Some("rule") => {
                rule.insert(reference.clone());
            }
            Some("model") => {
                model.insert(reference.clone());
            }
            Some("anomaly") => {
                anomaly.insert(reference.clone());
            }
            Some("document") => {
                document.insert(reference.clone());
            }
            Some("similar_case") => {
                similar_case.insert(reference.clone());
            }
            _ => {}
        }
    }

    EvidenceReferenceBuckets {
        claim: claim.into_iter().collect(),
        provider: provider.into_iter().collect(),
        rule: rule.into_iter().collect(),
        model: model.into_iter().collect(),
        anomaly: anomaly.into_iter().collect(),
        document: document.into_iter().collect(),
        similar_case: similar_case.into_iter().collect(),
    }
}

fn evidence_ref_bucket(reference: &str) -> Option<&'static str> {
    if reference.starts_with("knowledge_cases:")
        || reference.starts_with("retrieval:")
        || reference.starts_with("matched_signal:")
        || reference.starts_with("query_claim:")
    {
        Some("similar_case")
    } else if reference.starts_with("rule_runs:") || reference.starts_with("rules:") {
        Some("rule")
    } else if reference.starts_with("model_scores:") || reference.starts_with("model_versions:") {
        Some("model")
    } else if reference.starts_with("documents:")
        || reference.starts_with("document_chunks:")
        || reference.starts_with("ocr:")
    {
        Some("document")
    } else if reference.starts_with("claim:")
        || reference.starts_with("claims:")
        || reference.starts_with("claim_items:")
    {
        Some("claim")
    } else if reference.starts_with("provider_sanctions:")
        || reference.starts_with("provider_profile:")
        || reference.starts_with("providers:")
    {
        Some("provider")
    } else if reference.starts_with("anomaly:")
        || (reference.starts_with("scoring_runs:") && reference.contains("anomaly"))
    {
        Some("anomaly")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_investigator_returns_evidence_backed_package() {
        let request = InvestigationRequest {
            claim_id: "CLM-0287".into(),
            risk_score: 87,
            rag: "RED".into(),
            scheme_family: "provider_peer_outlier".into(),
            top_reasons: vec![
                "金额高于同病种同地区 P99".into(),
                "诊断-项目匹配度偏低".into(),
            ],
            similar_cases: vec![SimilarCaseInput {
                case_id: "KC-1001".into(),
                similarity_score: 0.82,
                matched_signals: vec!["diagnosis:J10".into()],
                provenance_refs: vec!["retrieval:structured_signal_overlap".into()],
                evidence_refs: vec!["knowledge_cases:KC-1001".into()],
            }],
        };

        let package = DeterministicInvestigator.investigate(request);

        assert_eq!(package.decision_boundary, "assistive_only");
        assert!(package.agent_run_id.starts_with("agent_"));
        assert!(!package.agent_run_id.contains("CLM-0287"));
        assert!(package.risk_summary.contains("CLM-0287"));
        assert!(!package.investigation_checklist.is_empty());
        assert_eq!(package.similar_cases[0].case_id, "KC-1001");
        assert!(package
            .findings
            .iter()
            .all(|finding| !finding.evidence_refs.is_empty()));
        assert_eq!(
            package.evidence_sufficiency.scheme_family,
            "provider_peer_outlier"
        );
        assert!(package
            .evidence_sufficiency
            .minimum_evidence
            .contains(&"peer_group_definition".into()));
        assert!(package
            .evidence_sufficiency
            .missing_evidence
            .contains(&"specialty".into()));
        assert!(package.similar_cases[0]
            .provenance_refs
            .contains(&"retrieval:structured_signal_overlap".into()));
        assert!(package
            .evidence_refs
            .contains(&"retrieval:structured_signal_overlap".into()));
        assert!(!package.evidence_refs.is_empty());
        assert!(!package.qa_opinion_draft.contains("拒赔"));
    }

    #[test]
    fn deterministic_investigator_uses_unique_run_ids_for_repeated_claims() {
        let request = InvestigationRequest {
            claim_id: "CLM-0287".into(),
            risk_score: 87,
            rag: "RED".into(),
            scheme_family: "provider_peer_outlier".into(),
            top_reasons: vec!["金额高于同病种同地区 P99".into()],
            similar_cases: vec![],
        };

        let first = DeterministicInvestigator.investigate(request.clone());
        let second = DeterministicInvestigator.investigate(request);

        assert!(first.agent_run_id.starts_with("agent_"));
        assert!(second.agent_run_id.starts_with("agent_"));
        assert_ne!(first.agent_run_id, second.agent_run_id);
    }

    #[test]
    fn buckets_provider_sanctions_evidence_refs() {
        let request = InvestigationRequest {
            claim_id: "CLM-0287".into(),
            risk_score: 100,
            rag: "RED".into(),
            scheme_family: "provider_peer_outlier".into(),
            top_reasons: Vec::new(),
            similar_cases: vec![SimilarCaseInput {
                case_id: "KC-1001".into(),
                similarity_score: 0.82,
                matched_signals: vec![],
                provenance_refs: vec![],
                evidence_refs: vec!["provider_sanctions:PRV-1:oig".into()],
            }],
        };

        let package = DeterministicInvestigator.investigate(request);

        assert!(package
            .evidence_refs_by_type
            .provider
            .contains(&"provider_sanctions:PRV-1:oig".into()));
    }
}
