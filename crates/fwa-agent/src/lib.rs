use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimilarCaseInput {
    pub case_id: String,
    pub similarity_score: f64,
    pub matched_signals: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceSufficiency {
    pub scheme_family: String,
    pub status: String,
    pub minimum_evidence: Vec<String>,
    pub present_evidence: Vec<String>,
    pub missing_evidence: Vec<String>,
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
}

#[derive(Debug, Clone, Copy)]
pub struct DeterministicInvestigator;

impl DeterministicInvestigator {
    pub fn investigate(self, request: InvestigationRequest) -> InvestigationPackage {
        let agent_run_id = format!("agent_{}", request.claim_id);
        let findings = build_findings(&request);
        let mut evidence_refs = findings
            .iter()
            .flat_map(|finding| finding.evidence_refs.clone())
            .collect::<Vec<_>>();
        for similar_case in &request.similar_cases {
            evidence_refs.extend(similar_case.evidence_refs.clone());
        }
        evidence_refs.sort();
        evidence_refs.dedup();

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
    let minimum_evidence = minimum_evidence_for_scheme(&request.scheme_family);
    let evidence_text = evidence_text(request);
    let present_evidence = minimum_evidence
        .iter()
        .filter(|item| evidence_item_present(item, &evidence_text))
        .cloned()
        .collect::<Vec<_>>();
    let missing_evidence = minimum_evidence
        .iter()
        .filter(|item| !present_evidence.contains(item))
        .cloned()
        .collect::<Vec<_>>();
    let status = if missing_evidence.is_empty() {
        "sufficient"
    } else {
        "needs_more_evidence"
    };

    EvidenceSufficiency {
        scheme_family: request.scheme_family.clone(),
        status: status.into(),
        minimum_evidence,
        present_evidence,
        missing_evidence,
    }
}

fn minimum_evidence_for_scheme(scheme_family: &str) -> Vec<String> {
    let items = match scheme_family {
        "duplicate_billing" => &[
            "same_member",
            "provider",
            "service_date",
            "procedure",
            "amount",
            "claim_lineage",
        ][..],
        "upcoding" => &[
            "diagnosis",
            "billed_code",
            "lower_complexity_comparator",
            "medical_record",
            "coding_rationale",
        ],
        "unbundling" => &[
            "component_codes",
            "bundled_code_comparator",
            "same_episode",
            "billing_timeline",
        ],
        "medically_unnecessary_service" | "medical_necessity" => &[
            "diagnosis",
            "order",
            "chart_note",
            "treatment_context",
            "reviewer_finding",
            "policy_rule",
        ],
        "laboratory_testing_abuse" | "lab_overuse" => &[
            "ordering_pattern",
            "diagnosis_match",
            "frequency",
            "peer_benchmark",
            "ordering_provider",
        ],
        "excessive_utilization" => &[
            "member_history",
            "service_frequency",
            "peer_benchmark",
            "time_window",
            "clinical_rationale",
        ],
        "provider_peer_outlier" => &[
            "peer_group_definition",
            "time_window",
            "specialty",
            "region",
            "statistical_deviation",
        ],
        "telehealth_abuse" => &[
            "visit_mode",
            "provider_member_location",
            "visit_frequency",
            "documentation",
            "policy_rule",
        ],
        "pharmacy_controlled_substance_abuse" | "pharmacy_or_opioid_abuse" => &[
            "prescription",
            "prescriber",
            "fill_pattern",
            "dosage",
            "member_history",
            "policy_rule",
        ],
        "genetic_testing_abuse" => &[
            "test_order",
            "diagnosis",
            "policy_rule",
            "medical_record",
            "lab_provider",
        ],
        "dme_home_health_hospice_rehab_risk" => &[
            "order",
            "supplier_provider",
            "medical_record",
            "delivery_or_service_proof",
            "policy_rule",
        ],
        "relationship_concentration" => &[
            "relationship_graph",
            "provider_member_link",
            "referral_pattern",
            "ownership_or_affiliation",
            "time_window",
        ],
        "early_high_value_claim" => &[
            "policy_start_date",
            "service_date",
            "claim_amount",
            "coverage_limit",
            "medical_record",
        ],
        "diagnosis_procedure_mismatch" => &[
            "diagnosis",
            "procedure",
            "medical_record",
            "clinical_rationale",
            "policy_rule",
        ],
        _ => &["claim_context", "risk_reason", "evidence_refs"],
    };
    items.iter().map(|item| (*item).to_string()).collect()
}

fn evidence_text(request: &InvestigationRequest) -> String {
    let mut parts = request.top_reasons.clone();
    for similar_case in &request.similar_cases {
        parts.extend(similar_case.matched_signals.clone());
        parts.extend(similar_case.evidence_refs.clone());
    }
    parts.join(" ").to_ascii_lowercase()
}

fn evidence_item_present(item: &str, evidence_text: &str) -> bool {
    match item {
        "amount" | "claim_amount" => {
            evidence_text.contains("amount") || evidence_text.contains("金额")
        }
        "billed_code" | "component_codes" | "procedure" => {
            evidence_text.contains("code")
                || evidence_text.contains("procedure")
                || evidence_text.contains("项目")
        }
        "diagnosis" | "diagnosis_match" => {
            evidence_text.contains("diagnosis") || evidence_text.contains("诊断")
        }
        "documentation" | "medical_record" | "chart_note" => {
            evidence_text.contains("document")
                || evidence_text.contains("medical_record")
                || evidence_text.contains("病历")
        }
        "evidence_refs" => evidence_text.contains(':'),
        "peer_benchmark" | "peer_group_definition" => {
            evidence_text.contains("peer")
                || evidence_text.contains("同病种")
                || evidence_text.contains("同地区")
        }
        "policy_rule" => evidence_text.contains("policy") || evidence_text.contains("rule"),
        "provider" | "ordering_provider" | "prescriber" | "supplier_provider" | "lab_provider" => {
            evidence_text.contains("provider")
        }
        "region" | "provider_member_location" => {
            evidence_text.contains("region:") || evidence_text.contains("地区")
        }
        "risk_reason" => !evidence_text.trim().is_empty(),
        "statistical_deviation" => {
            evidence_text.contains("p99")
                || evidence_text.contains("percentile")
                || evidence_text.contains("zscore")
                || evidence_text.contains("偏离")
                || evidence_text.contains("高于")
        }
        "time_window" | "billing_timeline" => {
            evidence_text.contains("30d")
                || evidence_text.contains("90d")
                || evidence_text.contains("window")
                || evidence_text.contains("近")
        }
        other => evidence_text.contains(other),
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
                evidence_refs: vec!["knowledge_cases:KC-1001".into()],
            }],
        };

        let package = DeterministicInvestigator.investigate(request);

        assert_eq!(package.decision_boundary, "assistive_only");
        assert!(package.agent_run_id.starts_with("agent_"));
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
        assert!(!package.evidence_refs.is_empty());
        assert!(!package.qa_opinion_draft.contains("拒赔"));
    }
}
