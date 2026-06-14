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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvestigationCancellationError {
    pub checkpoint: String,
    pub reason: String,
}

pub trait InvestigationCancellation {
    fn is_cancelled(&self, checkpoint: &str) -> bool;

    fn cancellation_reason(&self) -> Option<&str> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoopInvestigationCancellation;

impl InvestigationCancellation for NoopInvestigationCancellation {
    fn is_cancelled(&self, _checkpoint: &str) -> bool {
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecialistAgentTask {
    pub agent_kind: String,
    pub responsibility: String,
    pub input_scope: Vec<String>,
    pub phi_fields_allowed: Vec<String>,
    pub decision_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediatedToolCall {
    pub tool_name: String,
    pub purpose: String,
    pub input_scope: Vec<String>,
    pub policy_check: String,
    pub cancellation_checkpoint: String,
    pub execution_mode: String,
    pub decision_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecialistAgentExecution {
    pub agent_kind: String,
    pub status: String,
    pub responsibility: String,
    pub decision_boundary: String,
    pub phi_fields_allowed: Vec<String>,
    pub tool_calls: Vec<MediatedToolCall>,
    pub evidence_refs: Vec<String>,
    pub summary: String,
}

pub trait InvestigationOrchestrator {
    fn orchestrator_version(&self) -> &'static str;

    fn specialist_plan(&self, request: &InvestigationRequest) -> Vec<SpecialistAgentTask>;

    fn dispatch_specialists_with_cancellation(
        &self,
        request: &InvestigationRequest,
        cancellation: &(dyn InvestigationCancellation + Sync),
    ) -> Result<Vec<SpecialistAgentExecution>, InvestigationCancellationError>;

    fn orchestrate(&self, request: InvestigationRequest) -> InvestigationPackage;

    fn orchestrate_with_cancellation(
        &self,
        request: InvestigationRequest,
        cancellation: &(dyn InvestigationCancellation + Sync),
    ) -> Result<InvestigationPackage, InvestigationCancellationError>;
}

#[derive(Debug, Clone, Copy)]
pub struct DeterministicInvestigator;

impl DeterministicInvestigator {
    pub fn investigate(self, request: InvestigationRequest) -> InvestigationPackage {
        self.investigate_with_cancellation(request, &NoopInvestigationCancellation)
            .expect("noop cancellation cannot cancel")
    }

    pub fn investigate_with_cancellation(
        self,
        request: InvestigationRequest,
        cancellation: &(dyn InvestigationCancellation + Sync),
    ) -> Result<InvestigationPackage, InvestigationCancellationError> {
        check_cancellation(cancellation, "investigation.start")?;
        let agent_run_id = format!("agent_{}", Ulid::new());
        let findings = build_findings(&request);
        check_cancellation(cancellation, "investigation.findings_built")?;
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
        check_cancellation(cancellation, "investigation.evidence_refs_bucketed")?;

        let evidence_sufficiency = build_evidence_sufficiency(&request);
        check_cancellation(cancellation, "investigation.evidence_sufficiency_built")?;

        Ok(InvestigationPackage {
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
        })
    }
}

impl InvestigationOrchestrator for DeterministicInvestigator {
    fn orchestrator_version(&self) -> &'static str {
        "deterministic_orchestrator_v1"
    }

    fn specialist_plan(&self, request: &InvestigationRequest) -> Vec<SpecialistAgentTask> {
        build_specialist_plan(request)
    }

    fn dispatch_specialists_with_cancellation(
        &self,
        request: &InvestigationRequest,
        cancellation: &(dyn InvestigationCancellation + Sync),
    ) -> Result<Vec<SpecialistAgentExecution>, InvestigationCancellationError> {
        dispatch_specialists(request, cancellation)
    }

    fn orchestrate(&self, request: InvestigationRequest) -> InvestigationPackage {
        self.investigate(request)
    }

    fn orchestrate_with_cancellation(
        &self,
        request: InvestigationRequest,
        cancellation: &(dyn InvestigationCancellation + Sync),
    ) -> Result<InvestigationPackage, InvestigationCancellationError> {
        self.investigate_with_cancellation(request, cancellation)
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

fn check_cancellation(
    cancellation: &(dyn InvestigationCancellation + Sync),
    checkpoint: &'static str,
) -> Result<(), InvestigationCancellationError> {
    if cancellation.is_cancelled(checkpoint) {
        Err(InvestigationCancellationError {
            checkpoint: checkpoint.into(),
            reason: cancellation
                .cancellation_reason()
                .unwrap_or("investigation cancelled")
                .into(),
        })
    } else {
        Ok(())
    }
}

fn build_evidence_sufficiency(request: &InvestigationRequest) -> EvidenceSufficiency {
    assess_evidence_sufficiency(&request.scheme_family, &evidence_text(request))
}

fn build_specialist_plan(request: &InvestigationRequest) -> Vec<SpecialistAgentTask> {
    let mut tasks = vec![
        SpecialistAgentTask {
            agent_kind: "intake_triage".into(),
            responsibility: "Normalize claim context and preserve the assistive-only boundary."
                .into(),
            input_scope: vec![
                "claim_id".into(),
                "risk_score".into(),
                "rag".into(),
                "top_reasons".into(),
            ],
            phi_fields_allowed: vec!["claim_id".into(), "risk_score".into(), "rag".into()],
            decision_boundary: "assistive_only".into(),
        },
        SpecialistAgentTask {
            agent_kind: "evidence_review".into(),
            responsibility:
                "Check whether findings have claim, rule, model, document, or knowledge evidence."
                    .into(),
            input_scope: vec![
                "scheme_family".into(),
                "top_reasons".into(),
                "similar_cases.evidence_refs".into(),
            ],
            phi_fields_allowed: vec!["claim_id".into(), "diagnosis_code".into()],
            decision_boundary: "assistive_only".into(),
        },
    ];

    if request.scheme_family.contains("provider") || !request.similar_cases.is_empty() {
        tasks.push(SpecialistAgentTask {
            agent_kind: "network_analysis".into(),
            responsibility:
                "Review provider relationship, peer outlier, and similar-case network signals."
                    .into(),
            input_scope: vec![
                "scheme_family".into(),
                "similar_cases.matched_signals".into(),
                "similar_cases.provenance_refs".into(),
            ],
            phi_fields_allowed: vec!["claim_id".into(), "provider_region".into()],
            decision_boundary: "assistive_only".into(),
        });
    }

    tasks
}

fn dispatch_specialists(
    request: &InvestigationRequest,
    cancellation: &(dyn InvestigationCancellation + Sync),
) -> Result<Vec<SpecialistAgentExecution>, InvestigationCancellationError> {
    build_specialist_plan(request)
        .into_iter()
        .map(|task| dispatch_specialist(request, task, cancellation))
        .collect()
}

fn dispatch_specialist(
    request: &InvestigationRequest,
    task: SpecialistAgentTask,
    cancellation: &(dyn InvestigationCancellation + Sync),
) -> Result<SpecialistAgentExecution, InvestigationCancellationError> {
    check_cancellation(
        cancellation,
        specialist_checkpoint(&task.agent_kind, "start"),
    )?;
    let tool_calls = mediated_tool_calls_for_task(&task);
    let evidence_refs = specialist_evidence_refs(request, &task);
    let summary = specialist_summary(request, &task.agent_kind);
    check_cancellation(
        cancellation,
        specialist_checkpoint(&task.agent_kind, "complete"),
    )?;
    Ok(SpecialistAgentExecution {
        agent_kind: task.agent_kind.clone(),
        status: "completed".into(),
        responsibility: task.responsibility,
        decision_boundary: task.decision_boundary,
        phi_fields_allowed: task.phi_fields_allowed,
        tool_calls,
        evidence_refs,
        summary,
    })
}

fn mediated_tool_calls_for_task(task: &SpecialistAgentTask) -> Vec<MediatedToolCall> {
    match task.agent_kind.as_str() {
        "evidence_review" => vec![MediatedToolCall {
            tool_name: "knowledge.search_similar".into(),
            purpose: "Retrieve similar governed FWA cases for human evidence review.".into(),
            input_scope: vec![
                "scheme_family".into(),
                "top_reasons".into(),
                "similar_cases.evidence_refs".into(),
            ],
            policy_check: "agent_registry.capability:knowledge.search_similar".into(),
            cancellation_checkpoint: "specialist.evidence_review.start".into(),
            execution_mode: "contract_only_not_executed".into(),
            decision_boundary: "assistive_only".into(),
        }],
        "network_analysis" => vec![MediatedToolCall {
            tool_name: "provider.graph.review".into(),
            purpose: "Review provider relationship and peer outlier signals before escalation."
                .into(),
            input_scope: vec![
                "scheme_family".into(),
                "similar_cases.matched_signals".into(),
                "similar_cases.provenance_refs".into(),
            ],
            policy_check: "agent_registry.capability:provider.graph.review".into(),
            cancellation_checkpoint: "specialist.network_analysis.start".into(),
            execution_mode: "contract_only_not_executed".into(),
            decision_boundary: "assistive_only".into(),
        }],
        _ => Vec::new(),
    }
}

fn specialist_evidence_refs(
    request: &InvestigationRequest,
    task: &SpecialistAgentTask,
) -> Vec<String> {
    let mut evidence_refs = BTreeSet::from([format!(
        "agent_specialists:{}:{}",
        request.claim_id, task.agent_kind
    )]);
    if task.agent_kind == "evidence_review" || task.agent_kind == "network_analysis" {
        for similar_case in &request.similar_cases {
            evidence_refs.extend(similar_case.evidence_refs.clone());
            evidence_refs.extend(similar_case.provenance_refs.clone());
        }
    }
    evidence_refs.into_iter().collect()
}

fn specialist_summary(request: &InvestigationRequest, agent_kind: &str) -> String {
    match agent_kind {
        "intake_triage" => format!(
            "Intake triage prepared claim {} for assistive investigation at risk score {}.",
            request.claim_id, request.risk_score
        ),
        "evidence_review" => format!(
            "Evidence review prepared {} top reasons and {} similar cases for human review.",
            request.top_reasons.len(),
            request.similar_cases.len()
        ),
        "network_analysis" => format!(
            "Network analysis prepared {} similar-case signals for provider relationship review.",
            request
                .similar_cases
                .iter()
                .map(|case| case.matched_signals.len())
                .sum::<usize>()
        ),
        _ => "Specialist execution completed under assistive-only boundary.".into(),
    }
}

fn specialist_checkpoint(agent_kind: &str, phase: &str) -> &'static str {
    match (agent_kind, phase) {
        ("intake_triage", "start") => "specialist.intake_triage.start",
        ("intake_triage", "complete") => "specialist.intake_triage.complete",
        ("evidence_review", "start") => "specialist.evidence_review.start",
        ("evidence_review", "complete") => "specialist.evidence_review.complete",
        ("network_analysis", "start") => "specialist.network_analysis.start",
        ("network_analysis", "complete") => "specialist.network_analysis.complete",
        _ => "specialist.unknown",
    }
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
    fn deterministic_orchestrator_exposes_specialist_agent_plan() {
        let request = InvestigationRequest {
            claim_id: "CLM-0287".into(),
            risk_score: 87,
            rag: "RED".into(),
            scheme_family: "provider_peer_outlier".into(),
            top_reasons: vec!["Provider peer outlier".into()],
            similar_cases: vec![SimilarCaseInput {
                case_id: "KC-1001".into(),
                similarity_score: 0.82,
                matched_signals: vec!["provider_network_signal".into()],
                provenance_refs: vec!["retrieval:structured_signal_overlap".into()],
                evidence_refs: vec!["knowledge_cases:KC-1001".into()],
            }],
        };

        let orchestrator: &dyn InvestigationOrchestrator = &DeterministicInvestigator;
        let plan = orchestrator.specialist_plan(&request);
        let package = orchestrator.orchestrate(request);

        assert_eq!(
            orchestrator.orchestrator_version(),
            "deterministic_orchestrator_v1"
        );
        assert!(plan.iter().any(|task| task.agent_kind == "intake_triage"));
        assert!(plan.iter().any(|task| task.agent_kind == "evidence_review"));
        let network_task = plan
            .iter()
            .find(|task| task.agent_kind == "network_analysis")
            .expect("provider investigations should include a network analysis specialist slot");
        assert_eq!(network_task.decision_boundary, "assistive_only");
        assert!(network_task
            .input_scope
            .contains(&"similar_cases.matched_signals".into()));
        assert!(plan.iter().all(|task| {
            task.decision_boundary == "assistive_only"
                && task
                    .phi_fields_allowed
                    .iter()
                    .all(|field| !field.contains("name") && !field.contains("certificate"))
        }));
        assert_eq!(package.decision_boundary, "assistive_only");
        assert!(package.agent_run_id.starts_with("agent_"));
    }

    #[test]
    fn deterministic_orchestrator_supports_noop_cancellation_signal() {
        let request = InvestigationRequest {
            claim_id: "CLM-0287".into(),
            risk_score: 87,
            rag: "RED".into(),
            scheme_family: "provider_peer_outlier".into(),
            top_reasons: vec!["Provider peer outlier".into()],
            similar_cases: vec![],
        };

        let orchestrator: &dyn InvestigationOrchestrator = &DeterministicInvestigator;
        let package = orchestrator
            .orchestrate_with_cancellation(request, &NoopInvestigationCancellation)
            .expect("noop cancellation should not stop investigation");

        assert_eq!(package.decision_boundary, "assistive_only");
        assert!(package.agent_run_id.starts_with("agent_"));
    }

    #[test]
    fn deterministic_orchestrator_dispatches_specialists_with_tool_mediation_contract() {
        let request = InvestigationRequest {
            claim_id: "CLM-0287".into(),
            risk_score: 87,
            rag: "RED".into(),
            scheme_family: "provider_peer_outlier".into(),
            top_reasons: vec!["Provider peer outlier".into()],
            similar_cases: vec![SimilarCaseInput {
                case_id: "KC-1001".into(),
                similarity_score: 0.82,
                matched_signals: vec!["provider_network_signal".into()],
                provenance_refs: vec!["retrieval:structured_signal_overlap".into()],
                evidence_refs: vec!["knowledge_cases:KC-1001".into()],
            }],
        };

        let orchestrator: &dyn InvestigationOrchestrator = &DeterministicInvestigator;
        let executions = orchestrator
            .dispatch_specialists_with_cancellation(&request, &NoopInvestigationCancellation)
            .expect("noop cancellation should not stop specialist dispatch");

        assert_eq!(executions.len(), 3);
        assert!(executions.iter().all(|execution| {
            execution.status == "completed"
                && execution.decision_boundary == "assistive_only"
                && execution
                    .phi_fields_allowed
                    .iter()
                    .all(|field| !field.contains("name") && !field.contains("certificate"))
        }));
        let evidence_review = executions
            .iter()
            .find(|execution| execution.agent_kind == "evidence_review")
            .expect("evidence review execution");
        assert!(evidence_review
            .evidence_refs
            .contains(&"knowledge_cases:KC-1001".into()));
        assert_eq!(evidence_review.tool_calls.len(), 1);
        assert_eq!(
            evidence_review.tool_calls[0].tool_name,
            "knowledge.search_similar"
        );
        assert_eq!(
            evidence_review.tool_calls[0].execution_mode,
            "contract_only_not_executed"
        );
        assert_eq!(
            evidence_review.tool_calls[0].policy_check,
            "agent_registry.capability:knowledge.search_similar"
        );
        assert_eq!(
            evidence_review.tool_calls[0].cancellation_checkpoint,
            "specialist.evidence_review.start"
        );
        let network_analysis = executions
            .iter()
            .find(|execution| execution.agent_kind == "network_analysis")
            .expect("network analysis execution");
        assert_eq!(
            network_analysis.tool_calls[0].tool_name,
            "provider.graph.review"
        );
        assert_eq!(
            network_analysis.tool_calls[0].cancellation_checkpoint,
            "specialist.network_analysis.start"
        );
    }

    #[test]
    fn deterministic_specialist_dispatch_stops_at_cancelled_checkpoint() {
        struct CancelAt(&'static str);

        impl InvestigationCancellation for CancelAt {
            fn is_cancelled(&self, checkpoint: &str) -> bool {
                checkpoint == self.0
            }

            fn cancellation_reason(&self) -> Option<&str> {
                Some("operator cancelled specialist dispatch")
            }
        }

        let request = InvestigationRequest {
            claim_id: "CLM-0287".into(),
            risk_score: 87,
            rag: "RED".into(),
            scheme_family: "provider_peer_outlier".into(),
            top_reasons: vec!["Provider peer outlier".into()],
            similar_cases: vec![],
        };

        let orchestrator: &dyn InvestigationOrchestrator = &DeterministicInvestigator;
        let error = orchestrator
            .dispatch_specialists_with_cancellation(
                &request,
                &CancelAt("specialist.evidence_review.start"),
            )
            .expect_err("cancelled specialist checkpoint should stop dispatch");

        assert_eq!(error.checkpoint, "specialist.evidence_review.start");
        assert_eq!(error.reason, "operator cancelled specialist dispatch");
    }

    #[test]
    fn deterministic_orchestrator_stops_at_cancelled_checkpoint() {
        struct CancelAt(&'static str);

        impl InvestigationCancellation for CancelAt {
            fn is_cancelled(&self, checkpoint: &str) -> bool {
                checkpoint == self.0
            }

            fn cancellation_reason(&self) -> Option<&str> {
                Some("operator kill-switch requested")
            }
        }

        let request = InvestigationRequest {
            claim_id: "CLM-0287".into(),
            risk_score: 87,
            rag: "RED".into(),
            scheme_family: "provider_peer_outlier".into(),
            top_reasons: vec!["Provider peer outlier".into()],
            similar_cases: vec![],
        };

        let orchestrator: &dyn InvestigationOrchestrator = &DeterministicInvestigator;
        let error = orchestrator
            .orchestrate_with_cancellation(
                request,
                &CancelAt("investigation.evidence_refs_bucketed"),
            )
            .expect_err("cancelled checkpoint should stop investigation");

        assert_eq!(error.checkpoint, "investigation.evidence_refs_bucketed");
        assert_eq!(error.reason, "operator kill-switch requested");
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
