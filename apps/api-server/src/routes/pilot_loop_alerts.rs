use crate::repository::{
    AgentApprovalRecord, AgentRunLogRecord, AuditHistoryEventRecord, CaseRecord, LeadRecord,
};

use super::pilot_loop_types::OpsAlertRecord;

pub(super) fn build_ops_alerts(
    leads: &[LeadRecord],
    cases: &[CaseRecord],
    scoring_events: &[AuditHistoryEventRecord],
    medical_review_events: &[AuditHistoryEventRecord],
    agent_runs: &[AgentRunLogRecord],
) -> Vec<OpsAlertRecord> {
    let mut alerts = leads
        .iter()
        .filter(|lead| lead.status != "triaged" && (lead.risk_score >= 70 || lead.rag == "RED"))
        .map(high_risk_routing_alert)
        .chain(
            cases
                .iter()
                .filter(|case| matches!(case.sla_status.as_str(), "breached" | "closed_breached"))
                .map(sla_breach_alert),
        )
        .chain(build_medical_review_alerts(
            scoring_events,
            medical_review_events,
        ))
        .chain(build_agent_approval_alerts(agent_runs))
        .collect::<Vec<_>>();
    alerts.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| left.alert_type.cmp(&right.alert_type))
            .then_with(|| left.alert_id.cmp(&right.alert_id))
    });
    alerts
}

fn build_agent_approval_alerts(agent_runs: &[AgentRunLogRecord]) -> Vec<OpsAlertRecord> {
    agent_runs
        .iter()
        .flat_map(|run| {
            run.approvals
                .iter()
                .filter(|approval| approval.decision == "pending")
                .map(move |approval| agent_approval_alert(run, approval))
        })
        .collect()
}

fn agent_approval_alert(run: &AgentRunLogRecord, approval: &AgentApprovalRecord) -> OpsAlertRecord {
    let mut evidence_refs = approval.evidence_refs.clone();
    evidence_refs.extend(run.evidence_refs.clone());
    evidence_refs.push(format!("agent_run:{}", run.agent_run_id));
    OpsAlertRecord {
        alert_id: format!("alert_agent_approval_{}", approval.approval_id),
        alert_type: "agent_approval_pending".into(),
        severity: "high".into(),
        status: "open".into(),
        claim_id: run.claim_id.clone(),
        lead_id: None,
        case_id: None,
        scheme_family: run.output_json["evidence_sufficiency"]["scheme_family"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        message: format!(
            "Agent output {} for claim {} is waiting for human approval.",
            run.agent_run_id, run.claim_id
        ),
        recommended_action: "Review the evidence package and approve or reject the Agent output."
            .into(),
        evidence_refs: dedupe_strings(evidence_refs),
    }
}

fn build_medical_review_alerts(
    scoring_events: &[AuditHistoryEventRecord],
    medical_review_events: &[AuditHistoryEventRecord],
) -> Vec<OpsAlertRecord> {
    let reviewed_scoring_audit_ids = medical_review_events
        .iter()
        .filter_map(|event| event.payload["scoring_audit_id"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    scoring_events
        .iter()
        .filter(|event| !reviewed_scoring_audit_ids.contains(event.audit_id.as_str()))
        .filter_map(medical_review_alert_from_scoring_event)
        .collect()
}

fn medical_review_alert_from_scoring_event(
    event: &AuditHistoryEventRecord,
) -> Option<OpsAlertRecord> {
    let clinical = &event.payload["clinical_evidence"];
    let review_required = clinical["review_required"].as_bool().unwrap_or(false);
    let review_route = clinical["review_route"].as_str().unwrap_or_default();
    if !review_required && review_route != "medical_review" {
        return None;
    }
    let claim_id = event.payload["claim_id"].as_str()?.to_string();
    let medical_score = event.payload["scores"]["medical_reasonableness_score"]
        .as_u64()
        .unwrap_or_default();
    let mut evidence_refs = vec![format!("audit:{}", event.audit_id)];
    evidence_refs.extend(json_string_array(&clinical["evidence_refs"]));
    Some(OpsAlertRecord {
        alert_id: format!("alert_medical_review_{}", event.audit_id),
        alert_type: "medical_review_required".into(),
        severity: if medical_score >= 80 {
            "high"
        } else {
            "medium"
        }
        .into(),
        status: "open".into(),
        claim_id: claim_id.clone(),
        lead_id: None,
        case_id: None,
        scheme_family: "medically_unnecessary_service".into(),
        message: format!("Claim {claim_id} has clinical evidence gaps requiring medical review."),
        recommended_action:
            "Assign a medical reviewer and record an evidence-backed review result.".into(),
        evidence_refs: dedupe_strings(evidence_refs),
    })
}

fn high_risk_routing_alert(lead: &LeadRecord) -> OpsAlertRecord {
    OpsAlertRecord {
        alert_id: format!("alert_high_risk_{}", lead.lead_id),
        alert_type: "high_risk_routing".into(),
        severity: if lead.risk_score >= 90 || lead.rag == "RED" {
            "critical".into()
        } else {
            "high".into()
        },
        status: "open".into(),
        claim_id: lead.claim_id.clone(),
        lead_id: Some(lead.lead_id.clone()),
        case_id: None,
        scheme_family: lead.scheme_family.clone(),
        message: format!(
            "High-risk FWA lead {} for claim {} is pending triage.",
            lead.lead_id, lead.claim_id
        ),
        recommended_action: "Open an investigation case and assign reviewer ownership.".into(),
        evidence_refs: lead.evidence_refs.clone(),
    }
}

fn sla_breach_alert(case: &CaseRecord) -> OpsAlertRecord {
    OpsAlertRecord {
        alert_id: format!("alert_sla_{}", case.case_id),
        alert_type: "sla_breach".into(),
        severity: match case.priority.as_str() {
            "critical" | "high" => "critical",
            "medium" => "high",
            _ => "medium",
        }
        .into(),
        status: if case.sla_status == "closed_breached" {
            "closed".into()
        } else {
            "open".into()
        },
        claim_id: case.claim_id.clone(),
        lead_id: Some(case.lead_id.clone()),
        case_id: Some(case.case_id.clone()),
        scheme_family: case.scheme_family.clone(),
        message: format!(
            "Case {} for claim {} breached the {}h SLA target.",
            case.case_id, case.claim_id, case.sla_target_hours
        ),
        recommended_action: "Escalate the overdue case and record owner follow-up.".into(),
        evidence_refs: case_evidence_refs(case),
    }
}

fn case_evidence_refs(case: &CaseRecord) -> Vec<String> {
    let refs = case
        .evidence_package
        .get("evidence_refs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if refs.is_empty() {
        vec![format!("investigation_cases:{}", case.case_id)]
    } else {
        refs
    }
}

fn json_string_array(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ops_alerts_includes_sla_breach_alerts() {
        let case = CaseRecord {
            case_id: "case_CLM-SLA-1".into(),
            lead_id: "lead_CLM-SLA-1".into(),
            claim_id: "CLM-SLA-1".into(),
            member_id: "MBR-SLA-1".into(),
            provider_id: "PRV-SLA-1".into(),
            source_system: "tpa-demo".into(),
            review_mode: "pre_payment".into(),
            scheme_family: "provider_peer_outlier".into(),
            lead_source: "scoring_run".into(),
            status: "investigating".into(),
            assignee: "siu-owner".into(),
            reviewer: "medical-owner".into(),
            priority: "high".into(),
            routing_reason: "Provider peer outlier".into(),
            evidence_package: serde_json::json!({
                "evidence_refs": ["rule_runs:PROVIDER_PROFILE_HIGH", "case_workflow:overdue"]
            }),
            sla_target_hours: 24,
            sla_status: "breached".into(),
            time_to_triage_hours: 0.0,
            time_to_closure_hours: None,
            final_outcome: None,
            reviewer_notes: None,
            investigation_result_id: None,
        };

        let alerts = build_ops_alerts(&[], &[case], &[], &[], &[]);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, "sla_breach");
        assert_eq!(alerts[0].severity, "critical");
        assert_eq!(alerts[0].status, "open");
        assert_eq!(alerts[0].claim_id, "CLM-SLA-1");
        assert_eq!(alerts[0].case_id.as_deref(), Some("case_CLM-SLA-1"));
        assert_eq!(
            alerts[0].evidence_refs,
            vec![
                "rule_runs:PROVIDER_PROFILE_HIGH".to_string(),
                "case_workflow:overdue".to_string()
            ]
        );
    }

    #[test]
    fn build_ops_alerts_includes_open_medical_review_alerts() {
        let scoring_event = AuditHistoryEventRecord {
            audit_id: "audit_scoring_medical_1".into(),
            run_id: "run_medical_1".into(),
            actor_role: "tpa_system".into(),
            event_type: "scoring.completed".into(),
            event_status: "succeeded".into(),
            summary: "FWA scoring completed".into(),
            payload: serde_json::json!({
                "claim_id": "CLM-MED-ALERT-1",
                "scores": {
                    "medical_reasonableness_score": 88
                },
                "clinical_evidence": {
                    "review_required": true,
                    "review_route": "medical_review",
                    "evidence_refs": ["claim_items:IMG-900"]
                }
            }),
            evidence_refs: vec!["claim_items:IMG-900".into()],
            created_at: None,
        };

        let alerts = build_ops_alerts(&[], &[], &[scoring_event], &[], &[]);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, "medical_review_required");
        assert_eq!(alerts[0].severity, "high");
        assert_eq!(alerts[0].claim_id, "CLM-MED-ALERT-1");
        assert!(alerts[0]
            .evidence_refs
            .contains(&"audit:audit_scoring_medical_1".to_string()));
    }

    #[test]
    fn build_ops_alerts_includes_pending_agent_approval_alerts() {
        let run = AgentRunLogRecord {
            agent_run_id: "agent_CLM-AGENT-ALERT-1".into(),
            claim_id: "CLM-AGENT-ALERT-1".into(),
            status: "succeeded".into(),
            decision_boundary: "assistive_only".into(),
            output_json: serde_json::json!({
                "evidence_sufficiency": {
                    "scheme_family": "provider_peer_outlier"
                }
            }),
            evidence_refs: vec!["knowledge_cases:KC-1001".into()],
            steps: vec![],
            context_snapshots: vec![],
            policy_checks: vec![],
            tool_calls: vec![],
            tool_results: vec![],
            approvals: vec![AgentApprovalRecord {
                approval_id: "approval_agent_CLM-AGENT-ALERT-1".into(),
                agent_run_id: "agent_CLM-AGENT-ALERT-1".into(),
                proposed_action: "manual_review_required".into(),
                decision: "pending".into(),
                approver: "unassigned".into(),
                reason: "Agent output requires human approval before downstream action.".into(),
                evidence_refs: vec!["agent_run:agent_CLM-AGENT-ALERT-1".into()],
                created_at: None,
            }],
            created_at: None,
            completed_at: None,
        };

        let alerts = build_ops_alerts(&[], &[], &[], &[], &[run]);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, "agent_approval_pending");
        assert_eq!(alerts[0].severity, "high");
        assert_eq!(alerts[0].claim_id, "CLM-AGENT-ALERT-1");
        assert_eq!(alerts[0].scheme_family, "provider_peer_outlier");
        assert!(alerts[0]
            .evidence_refs
            .contains(&"agent_run:agent_CLM-AGENT-ALERT-1".to_string()));
        assert!(alerts[0]
            .evidence_refs
            .contains(&"knowledge_cases:KC-1001".to_string()));
    }
}
