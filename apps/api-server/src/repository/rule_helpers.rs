use super::{
    normalize_scheme_family, scheme_family_from_alert_code, RoutingPolicyRecord,
    RuleApplicabilityScopeRecord, RuleBacktestRecord, RuleBacktestSummaryRecord,
    RuleConditionLibraryRecord, RuleDetailRecord, RuleFalsePositiveHistoryRecord,
    RuleSummaryRecord, RuleVersionRecord,
};
use fwa_core::{RecommendedAction, RuleActionClass};
use fwa_rules::{AdjudicationPolicy, Condition, RequiredEvidence, Rule, RuleAction};
use fwa_scoring::RoutingPolicy;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;

pub fn default_runtime_rules() -> Vec<Rule> {
    vec![
        Rule {
            rule_id: "rule_service_before_coverage".into(),
            version: 1,
            name: "Service before coverage start".into(),
            review_mode: "pre_payment".into(),
            scheme_family: Some("early_high_value_claim".into()),
            conditions: vec![Condition {
                field: "days_since_policy_start".into(),
                operator: "<=".into(),
                value: serde_json::json!(-1),
            }],
            action: RuleAction {
                score: 100,
                alert_code: "SERVICE_BEFORE_COVERAGE_START".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::HardDeny,
                required_evidence: vec![RequiredEvidence {
                    evidence_type: "coverage_eligibility".into(),
                    evidence_request_type: None,
                    blocking: true,
                    policy_authority_ref: Some("policy:coverage-eligibility:v1".into()),
                    exception_check: Some("no_retroactive_coverage_exception".into()),
                }],
                adjudication_policy: Some(AdjudicationPolicy {
                    customer_approval_ref: "customer-rule-list:demo:v1".into(),
                    appeal_or_override_route: "appeals:manual-review:v1".into(),
                    effective_date: "2026-01-01".into(),
                    rollback_plan_ref: "rollback:rules:v1".into(),
                    production_threshold_ref: "thresholds:prepay-hard-deny:v1".into(),
                    routing_impact_ref: "routing-impact:shadow-demo:v1".into(),
                }),
                reason: "服务日期早于保单生效日，且无追溯保障例外".into(),
            },
        },
        Rule {
            rule_id: "rule_early_claim".into(),
            version: 1,
            name: "Early claim".into(),
            review_mode: "both".into(),
            scheme_family: Some("early_high_value_claim".into()),
            conditions: vec![Condition {
                field: "days_since_policy_start".into(),
                operator: "<=".into(),
                value: serde_json::json!(7),
            }],
            action: RuleAction {
                score: 75,
                alert_code: "EARLY_CLAIM".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "保单生效后 7 天内发生理赔".into(),
            },
        },
        Rule {
            rule_id: "rule_early_high_amount".into(),
            version: 1,
            name: "Early high amount".into(),
            review_mode: "both".into(),
            scheme_family: Some("early_high_value_claim".into()),
            conditions: vec![
                Condition {
                    field: "days_since_policy_start".into(),
                    operator: "<=".into(),
                    value: serde_json::json!(10),
                },
                Condition {
                    field: "claim_amount_to_limit_ratio".into(),
                    operator: ">=".into(),
                    value: serde_json::json!(0.7),
                },
            ],
            action: RuleAction {
                score: 45,
                alert_code: "EARLY_HIGH_AMOUNT".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "保单生效早期发生高额理赔".into(),
            },
        },
        Rule {
            rule_id: "rule_high_cost_single_item".into(),
            version: 1,
            name: "High cost single item".into(),
            review_mode: "both".into(),
            scheme_family: Some("high_risk_claim".into()),
            conditions: vec![Condition {
                field: "high_cost_item_ratio".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.5),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "HIGH_COST_SINGLE_ITEM".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "单个高价项目占理赔金额比例偏高".into(),
            },
        },
        Rule {
            rule_id: "rule_large_limit_usage".into(),
            version: 1,
            name: "Large limit usage".into(),
            review_mode: "both".into(),
            scheme_family: Some("early_high_value_claim".into()),
            conditions: vec![Condition {
                field: "claim_amount_to_limit_ratio".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.8),
            }],
            action: RuleAction {
                score: 35,
                alert_code: "LARGE_LIMIT_USAGE".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "理赔金额接近保障额度".into(),
            },
        },
        Rule {
            rule_id: "rule_low_medical_match".into(),
            version: 1,
            name: "Low medical match".into(),
            review_mode: "both".into(),
            scheme_family: Some("diagnosis_procedure_mismatch".into()),
            conditions: vec![Condition {
                field: "diagnosis_procedure_match_score".into(),
                operator: "<=".into(),
                value: serde_json::json!(0.4),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "LOW_MEDICAL_MATCH".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "诊断与项目匹配度偏低".into(),
            },
        },
        Rule {
            rule_id: "rule_duplicate_claim".into(),
            version: 1,
            name: "Duplicate claim".into(),
            review_mode: "both".into(),
            scheme_family: Some("duplicate_billing".into()),
            conditions: vec![Condition {
                field: "duplicate_claim_similarity_score".into(),
                operator: ">=".into(),
                value: serde_json::json!(0.95),
            }],
            action: RuleAction {
                score: 35,
                alert_code: "DUPLICATE_CLAIM".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "同一投保人、Provider、服务日期、项目和金额疑似重复理赔".into(),
            },
        },
        Rule {
            rule_id: "rule_upcoding_complexity".into(),
            version: 1,
            name: "Upcoding complexity".into(),
            review_mode: "both".into(),
            scheme_family: Some("upcoding".into()),
            conditions: vec![
                Condition {
                    field: "diagnosis_procedure_match_score".into(),
                    operator: "<=".into(),
                    value: serde_json::json!(0.45),
                },
                Condition {
                    field: "high_cost_item_ratio".into(),
                    operator: ">=".into(),
                    value: serde_json::json!(0.5),
                },
            ],
            action: RuleAction {
                score: 35,
                alert_code: "UPCODING_COMPLEXITY".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "高复杂度或高价项目与诊断支持度偏低，疑似 upcoding".into(),
            },
        },
        Rule {
            rule_id: "rule_unbundling_component_pattern".into(),
            version: 1,
            name: "Unbundling component pattern".into(),
            review_mode: "both".into(),
            scheme_family: Some("unbundling".into()),
            conditions: vec![Condition {
                field: "claim_item_count".into(),
                operator: ">=".into(),
                value: serde_json::json!(6),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "UNBUNDLING_COMPONENT_PATTERN".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "同一案件明细项目数量异常偏多，需核查是否存在拆分计费".into(),
            },
        },
        Rule {
            rule_id: "rule_medically_unnecessary_service".into(),
            version: 1,
            name: "Medically unnecessary service".into(),
            review_mode: "both".into(),
            scheme_family: Some("medically_unnecessary_service".into()),
            conditions: vec![Condition {
                field: "clinical_review_required".into(),
                operator: "==".into(),
                value: serde_json::json!(1),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "MEDICALLY_UNNECESSARY_SERVICE".into(),
                recommended_action: RecommendedAction::RequestEvidence,
                action_class: RuleActionClass::PendingEvidence,
                required_evidence: vec![RequiredEvidence {
                    evidence_type: "clinical_missing_evidence".into(),
                    evidence_request_type: Some("clinical_document_request".into()),
                    blocking: true,
                    policy_authority_ref: Some("policy:clinical-evidence:v1".into()),
                    exception_check: Some("required_clinical_documents_not_present".into()),
                }],
                adjudication_policy: None,
                reason: "临床证据不足或存在缺失，需复核医疗必要性".into(),
            },
        },
        Rule {
            rule_id: "rule_same_member_repeated_service".into(),
            version: 1,
            name: "Same member repeated service".into(),
            review_mode: "both".into(),
            scheme_family: Some("excessive_utilization".into()),
            conditions: vec![Condition {
                field: "same_member_service_count_30d".into(),
                operator: ">=".into(),
                value: serde_json::json!(3),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "SAME_MEMBER_REPEATED_SERVICE".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "同一投保人短期内同类服务重复出现，需核查过度使用".into(),
            },
        },
        Rule {
            rule_id: "rule_relationship_concentration".into(),
            version: 1,
            name: "Relationship concentration".into(),
            review_mode: "both".into(),
            scheme_family: Some("relationship_concentration".into()),
            conditions: vec![Condition {
                field: "provider_high_risk_neighbor_signal".into(),
                operator: "==".into(),
                value: serde_json::json!(true),
            }],
            action: RuleAction {
                score: 35,
                alert_code: "RELATIONSHIP_CONCENTRATION".into(),
                recommended_action: RecommendedAction::EscalateInvestigation,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "Provider 关系网络存在高风险邻居或集中关联信号".into(),
            },
        },
        Rule {
            rule_id: "rule_many_claim_items".into(),
            version: 1,
            name: "Many claim items".into(),
            review_mode: "both".into(),
            scheme_family: Some("excessive_utilization".into()),
            conditions: vec![Condition {
                field: "claim_item_count".into(),
                operator: ">=".into(),
                value: serde_json::json!(5),
            }],
            action: RuleAction {
                score: 20,
                alert_code: "MANY_CLAIM_ITEMS".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "理赔明细项目数量偏多".into(),
            },
        },
        Rule {
            rule_id: "rule_peer_p95_amount".into(),
            version: 1,
            name: "Peer P95 amount".into(),
            review_mode: "both".into(),
            scheme_family: Some("provider_peer_outlier".into()),
            conditions: vec![Condition {
                field: "claim_amount_peer_percentile".into(),
                operator: ">=".into(),
                value: serde_json::json!(95),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "PEER_P95_AMOUNT".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "理赔金额高于同类样本 P95".into(),
            },
        },
        Rule {
            rule_id: "rule_peer_p99_amount".into(),
            version: 1,
            name: "Peer P99 amount".into(),
            review_mode: "both".into(),
            scheme_family: Some("provider_peer_outlier".into()),
            conditions: vec![Condition {
                field: "claim_amount_peer_percentile".into(),
                operator: ">=".into(),
                value: serde_json::json!(99),
            }],
            action: RuleAction {
                score: 40,
                alert_code: "PEER_P99_AMOUNT".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "理赔金额高于同类样本 P99".into(),
            },
        },
        Rule {
            rule_id: "rule_provider_high_risk_tier".into(),
            version: 1,
            name: "Provider high risk tier".into(),
            review_mode: "both".into(),
            scheme_family: Some("provider_peer_outlier".into()),
            conditions: vec![Condition {
                field: "provider_risk_tier".into(),
                operator: "==".into(),
                value: serde_json::json!("HIGH"),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "PROVIDER_HIGH_RISK_TIER".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "Provider 风险等级较高".into(),
            },
        },
        Rule {
            rule_id: "rule_provider_profile_high".into(),
            version: 1,
            name: "Provider profile high".into(),
            review_mode: "both".into(),
            scheme_family: Some("provider_peer_outlier".into()),
            conditions: vec![Condition {
                field: "provider_profile_score".into(),
                operator: ">=".into(),
                value: serde_json::json!(70),
            }],
            action: RuleAction {
                score: 30,
                alert_code: "PROVIDER_PROFILE_HIGH".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: "Provider 风险画像分偏高".into(),
            },
        },
    ]
}

pub(super) fn default_rule_details() -> Vec<RuleDetailRecord> {
    default_runtime_rules()
        .into_iter()
        .map(|rule| rule_detail_from_rule(rule, "active", "rules-ops".into()))
        .collect()
}

pub(super) fn rule_detail_from_rule(rule: Rule, status: &str, owner: String) -> RuleDetailRecord {
    let active_version = (status == "active").then_some(rule.version);
    let review_mode = normalize_review_mode(&rule.review_mode);
    let scheme_family = rule
        .scheme_family
        .as_deref()
        .map(normalize_scheme_family)
        .unwrap_or_else(|| scheme_family_from_alert_code(&rule.action.alert_code));
    let dsl = serde_json::json!({
        "review_mode": review_mode,
        "scheme_family": scheme_family,
        "conditions": rule.conditions,
        "action": rule.action
    });
    let summary = RuleSummaryRecord {
        rule_id: rule.rule_id.clone(),
        name: rule.name.clone(),
        status: status.into(),
        owner,
        active_version,
        latest_version: rule.version,
        review_mode: review_mode.clone(),
        scheme_family: scheme_family.clone(),
        score: rule.action.score,
        alert_code: rule.action.alert_code.clone(),
        recommended_action: rule.action.recommended_action,
        applicability_scope: rule_applicability_scope(&review_mode, &scheme_family),
        backtest_result: default_rule_backtest_summary(),
        estimated_saving: "0.00".into(),
        false_positive_history: default_rule_false_positive_history(),
        evidence_refs: rule_governance_evidence_refs(&rule.rule_id, rule.version),
    };
    let version = RuleVersionRecord {
        version: rule.version,
        status: status.into(),
        dsl,
        review_mode,
        scheme_family,
        score: rule.action.score,
        alert_code: rule.action.alert_code,
        recommended_action: rule.action.recommended_action,
        reason: rule.action.reason,
    };
    RuleDetailRecord {
        summary,
        versions: vec![version],
        audit_events: vec![],
    }
}

pub(super) fn rule_condition_records_from_detail(
    detail: &RuleDetailRecord,
) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
    let mut records = Vec::new();
    for version in &detail.versions {
        let conditions: Vec<Condition> = serde_json::from_value(version.dsl["conditions"].clone())?;
        for (index, condition) in conditions.into_iter().enumerate() {
            let condition_key = rule_condition_key(&detail.summary.rule_id, version.version, index);
            let mut evidence_refs = detail.summary.evidence_refs.clone();
            evidence_refs.push(format!("rule_conditions:{condition_key}"));
            records.push(RuleConditionLibraryRecord {
                condition_key,
                source_rule_key: detail.summary.rule_id.clone(),
                source_rule_version: version.version,
                condition_index: index as u32,
                field: condition.field,
                operator: condition.operator,
                value: condition.value,
                review_mode: version.review_mode.clone(),
                scheme_family: version.scheme_family.clone(),
                status: rule_condition_status(&detail.summary.status),
                owner: detail.summary.owner.clone(),
                evidence_refs,
                created_at: None,
                updated_at: None,
            });
        }
    }
    Ok(records)
}

fn rule_condition_key(rule_id: &str, version: u32, index: usize) -> String {
    format!(
        "{}_v{}_c{}",
        safe_condition_key_segment(rule_id),
        version,
        index + 1
    )
}

pub(super) fn rule_condition_status(rule_status: &str) -> String {
    match rule_status {
        "active" => "active",
        "draft" => "candidate",
        "submitted" | "approved" => "governance_review",
        _ => "retired",
    }
    .into()
}

fn safe_condition_key_segment(value: &str) -> String {
    let segment = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let segment = segment.trim_matches('_').to_string();
    if segment.is_empty() {
        "condition".into()
    } else {
        segment
    }
}

pub(super) fn rule_applicability_scope(
    review_mode: &str,
    scheme_family: &str,
) -> RuleApplicabilityScopeRecord {
    RuleApplicabilityScopeRecord {
        review_mode: review_mode.into(),
        scheme_family: scheme_family.into(),
        source: "rule_dsl".into(),
    }
}

pub(super) fn rule_governance_evidence_refs(rule_id: &str, version: u32) -> Vec<String> {
    vec![format!("rules:{rule_id}:v{version}")]
}

pub(super) fn default_rule_backtest_summary() -> RuleBacktestSummaryRecord {
    RuleBacktestSummaryRecord {
        status: "not_run".into(),
        sample_count: 0,
        matched_count: 0,
        precision: 0.0,
        recall: 0.0,
        lift: 0.0,
        false_positive_rate: 0.0,
        estimated_saving: "0.00".into(),
        evidence_refs: vec![],
        created_at: None,
    }
}

pub(super) fn default_rule_false_positive_history() -> RuleFalsePositiveHistoryRecord {
    RuleFalsePositiveHistoryRecord {
        status: "not_observed".into(),
        false_positive_count: 0,
        false_positive_rate: 0.0,
        evidence_refs: vec![],
    }
}

pub(super) fn rule_backtest_summary(backtest: &RuleBacktestRecord) -> RuleBacktestSummaryRecord {
    RuleBacktestSummaryRecord {
        status: "completed".into(),
        sample_count: backtest.sample_count,
        matched_count: backtest.matched_count,
        precision: backtest.precision,
        recall: backtest.recall,
        lift: backtest.lift,
        false_positive_rate: backtest.false_positive_rate,
        estimated_saving: backtest.estimated_saving.clone(),
        evidence_refs: backtest.evidence_refs.clone(),
        created_at: backtest.created_at.clone(),
    }
}

pub(super) fn rule_false_positive_history(
    backtest: &RuleBacktestRecord,
) -> RuleFalsePositiveHistoryRecord {
    RuleFalsePositiveHistoryRecord {
        status: if backtest.reviewed_count == 0 {
            "not_observed"
        } else {
            "observed"
        }
        .into(),
        false_positive_count: backtest.false_positive_count,
        false_positive_rate: backtest.false_positive_rate,
        evidence_refs: backtest.evidence_refs.clone(),
    }
}

pub(super) fn apply_rule_backtest_metadata(
    summary: &mut RuleSummaryRecord,
    backtest: Option<&RuleBacktestRecord>,
) {
    if let Some(backtest) = backtest {
        summary.estimated_saving = backtest.estimated_saving.clone();
        summary.backtest_result = rule_backtest_summary(backtest);
        summary.false_positive_history = rule_false_positive_history(backtest);
        for reference in &backtest.evidence_refs {
            if !summary.evidence_refs.contains(reference) {
                summary.evidence_refs.push(reference.clone());
            }
        }
    }
}

pub(super) fn latest_rule_backtest_for<'a>(
    backtests: &'a [RuleBacktestRecord],
    rule_id: &str,
    rule_version: u32,
) -> Option<&'a RuleBacktestRecord> {
    backtests
        .iter()
        .rev()
        .find(|record| record.rule_id == rule_id && record.rule_version == rule_version)
}

pub(super) fn apply_rule_status(detail: &mut RuleDetailRecord, statuses: &HashMap<String, String>) {
    if let Some(status) = statuses.get(&detail.summary.rule_id) {
        detail.summary.status = status.clone();
        detail.summary.active_version =
            (status == "active").then_some(detail.summary.latest_version);
        for version in &mut detail.versions {
            version.status = status.clone();
        }
    }
}

pub(super) fn parse_recommended_action(value: &str) -> RecommendedAction {
    match value {
        "AutoApprove" | "StandardProcessing" => RecommendedAction::StandardProcessing,
        "QaSample" => RecommendedAction::QaSample,
        "RequestEvidence" => RecommendedAction::RequestEvidence,
        "EscalateInvestigation" => RecommendedAction::EscalateInvestigation,
        "PostPaymentAudit" => RecommendedAction::PostPaymentAudit,
        "ProviderReview" => RecommendedAction::ProviderReview,
        "RecoveryReview" => RecommendedAction::RecoveryReview,
        _ => RecommendedAction::ManualReview,
    }
}

pub(super) fn review_mode_from_dsl(dsl: &Value) -> String {
    dsl.get("review_mode")
        .and_then(Value::as_str)
        .map(normalize_review_mode)
        .unwrap_or_else(|| "both".into())
}

pub(super) fn normalize_review_mode(value: &str) -> String {
    match value {
        "pre_payment" | "post_payment" | "both" => value.into(),
        _ => "both".into(),
    }
}

pub(super) fn routing_policy_review_mode_applies(
    policy_review_mode: &str,
    requested_review_mode: &str,
) -> bool {
    policy_review_mode == "both" || policy_review_mode == requested_review_mode
}

pub(super) fn default_routing_policies() -> Vec<RoutingPolicy> {
    ["pre_payment", "post_payment", "both"]
        .into_iter()
        .map(fwa_scoring::default_routing_policy)
        .collect()
}

pub(super) fn seed_default_routing_policy_records(policies: &mut Vec<RoutingPolicyRecord>) {
    if policies.is_empty() {
        policies.extend(
            default_routing_policies()
                .into_iter()
                .map(|policy| routing_policy_record(policy, "active", "system", None, None)),
        );
    }
}

pub(super) fn routing_policy_record(
    policy: RoutingPolicy,
    status: &str,
    owner: &str,
    activated_at: Option<String>,
    created_at: Option<String>,
) -> RoutingPolicyRecord {
    RoutingPolicyRecord {
        policy_id: policy.policy_id,
        version: policy.version,
        review_mode: policy.review_mode,
        status: status.into(),
        owner: owner.into(),
        risk_thresholds: policy.risk_thresholds,
        confidence_thresholds: policy.confidence_thresholds,
        provider_review_threshold: policy.provider_review_threshold,
        activated_at,
        created_at,
    }
}

pub(super) fn routing_policy_record_from_row(
    row: (Value, String, String, Option<String>, Option<String>),
) -> anyhow::Result<RoutingPolicyRecord> {
    let (policy_json, status, owner, activated_at, created_at) = row;
    let policy: RoutingPolicy = serde_json::from_value(policy_json)?;
    Ok(routing_policy_record(
        policy,
        &status,
        &owner,
        activated_at,
        created_at,
    ))
}

pub(super) fn routing_policy_from_record(record: &RoutingPolicyRecord) -> RoutingPolicy {
    RoutingPolicy {
        policy_id: record.policy_id.clone(),
        version: record.version,
        review_mode: record.review_mode.clone(),
        risk_thresholds: record.risk_thresholds.clone(),
        confidence_thresholds: record.confidence_thresholds.clone(),
        provider_review_threshold: record.provider_review_threshold,
    }
}

pub(super) fn runtime_rule_from_detail(detail: RuleDetailRecord) -> anyhow::Result<Rule> {
    let version = detail
        .versions
        .into_iter()
        .find(|version| Some(version.version) == detail.summary.active_version)
        .ok_or_else(|| {
            anyhow::anyhow!("active version missing for rule {}", detail.summary.rule_id)
        })?;
    runtime_rule_from_parts(
        detail.summary.rule_id,
        detail.summary.name,
        version.version,
        version.dsl,
    )
}

pub(super) fn runtime_rule_from_parts(
    rule_id: String,
    name: String,
    version: u32,
    dsl: Value,
) -> anyhow::Result<Rule> {
    Ok(Rule {
        rule_id,
        version,
        name,
        review_mode: review_mode_from_dsl(&dsl),
        scheme_family: dsl["scheme_family"].as_str().map(normalize_scheme_family),
        conditions: serde_json::from_value(dsl["conditions"].clone())?,
        action: serde_json::from_value(dsl["action"].clone())?,
    })
}

pub(super) async fn ensure_default_rules_seeded(pool: &PgPool) -> anyhow::Result<()> {
    ensure_rule_condition_library_table(pool).await?;
    for detail in default_rule_details() {
        let mut tx = pool.begin().await?;
        let row: (String,) = sqlx::query_as(
            "INSERT INTO rules (rule_key, name, status, owner)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (rule_key) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&detail.summary.rule_id)
        .bind(&detail.summary.name)
        .bind(&detail.summary.status)
        .bind(&detail.summary.owner)
        .fetch_one(&mut *tx)
        .await?;

        for version in &detail.versions {
            sqlx::query(
                "INSERT INTO rule_versions
                 (rule_id, version, dsl, score, recommended_action, created_by, approved_by, published_at)
                 VALUES ($1::uuid, $2, $3, $4, $5, 'system', 'system', now())
                 ON CONFLICT (rule_id, version) DO NOTHING",
            )
            .bind(&row.0)
            .bind(version.version as i32)
            .bind(&version.dsl)
            .bind(version.score as i32)
            .bind(format!("{:?}", version.recommended_action))
            .execute(&mut *tx)
            .await?;
        }

        upsert_rule_conditions_tx(&mut tx, &row.0, &detail).await?;

        tx.commit().await?;
    }
    Ok(())
}

pub(super) async fn ensure_rule_condition_library_table(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rule_condition_library (
           condition_key TEXT PRIMARY KEY,
           source_rule_id UUID NOT NULL REFERENCES rules(id),
           source_rule_key TEXT NOT NULL,
           source_rule_version INTEGER NOT NULL,
           condition_index INTEGER NOT NULL,
           field_name TEXT NOT NULL,
           operator TEXT NOT NULL,
           value JSONB NOT NULL,
           review_mode TEXT NOT NULL,
           scheme_family TEXT NOT NULL,
           status TEXT NOT NULL,
           owner TEXT NOT NULL,
           evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
           updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
           UNIQUE(source_rule_key, source_rule_version, condition_index)
         )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn upsert_rule_conditions_tx(
    tx: &mut Transaction<'_, Postgres>,
    source_rule_id: &str,
    detail: &RuleDetailRecord,
) -> anyhow::Result<()> {
    for condition in rule_condition_records_from_detail(detail)? {
        sqlx::query(
            "INSERT INTO rule_condition_library
             (condition_key, source_rule_id, source_rule_key, source_rule_version,
              condition_index, field_name, operator, value, review_mode, scheme_family,
              status, owner, evidence_refs)
             VALUES ($1, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (source_rule_key, source_rule_version, condition_index) DO UPDATE
             SET condition_key = EXCLUDED.condition_key,
                 field_name = EXCLUDED.field_name,
                 operator = EXCLUDED.operator,
                 value = EXCLUDED.value,
                 review_mode = EXCLUDED.review_mode,
                 scheme_family = EXCLUDED.scheme_family,
                 status = EXCLUDED.status,
                 owner = EXCLUDED.owner,
                 evidence_refs = EXCLUDED.evidence_refs,
                 updated_at = now()",
        )
        .bind(&condition.condition_key)
        .bind(source_rule_id)
        .bind(&condition.source_rule_key)
        .bind(condition.source_rule_version as i32)
        .bind(condition.condition_index as i32)
        .bind(&condition.field)
        .bind(&condition.operator)
        .bind(&condition.value)
        .bind(&condition.review_mode)
        .bind(&condition.scheme_family)
        .bind(&condition.status)
        .bind(&condition.owner)
        .bind(serde_json::json!(condition.evidence_refs))
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
