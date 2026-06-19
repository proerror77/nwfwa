use super::PersistedInboxClaimRun;
use chrono::NaiveDate;
use fwa_core::{
    Claim, ClaimContext, ClaimId, ClaimItem, Member, MemberId, Money, Policy, PolicyId, Provider,
    ProviderId, ProviderRiskTier,
};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};

#[derive(sqlx::FromRow)]
pub(super) struct ClaimContextRow {
    external_claim_id: String,
    diagnosis_code: String,
    service_date: NaiveDate,
    claim_amount: Decimal,
    claim_currency: String,
    external_member_id: String,
    dob: Option<NaiveDate>,
    gender: Option<String>,
    external_policy_id: String,
    product_code: String,
    coverage_start_date: NaiveDate,
    coverage_end_date: NaiveDate,
    coverage_limit_amount: Decimal,
    policy_currency: String,
    external_provider_id: String,
    provider_name: String,
    provider_type: String,
    provider_region: String,
    provider_risk_tier: String,
}

pub(super) type ClaimItemRow = (String, String, String, i32, Decimal, Decimal, String);

pub(super) type LeadRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i32,
    String,
    String,
    Value,
);

#[derive(sqlx::FromRow)]
pub(super) struct CaseRow {
    pub(super) case_id: String,
    pub(super) lead_id: String,
    pub(super) claim_id: String,
    pub(super) member_id: String,
    pub(super) provider_id: String,
    pub(super) source_system: String,
    pub(super) review_mode: String,
    pub(super) scheme_family: String,
    pub(super) lead_source: String,
    pub(super) status: String,
    pub(super) assignee: String,
    pub(super) reviewer: String,
    pub(super) priority: String,
    pub(super) routing_reason: String,
    pub(super) evidence_package_json: Value,
    pub(super) final_outcome: Option<String>,
    pub(super) reviewer_notes: Option<String>,
    pub(super) investigation_result_id: Option<String>,
    pub(super) lead_created_at: chrono::DateTime<chrono::Utc>,
    pub(super) case_created_at: chrono::DateTime<chrono::Utc>,
    pub(super) case_updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
pub(super) struct AgentApprovalRow {
    pub(super) approval_id: String,
    pub(super) proposed_action: String,
    pub(super) decision: String,
    pub(super) approver: String,
    pub(super) reason: String,
    pub(super) evidence_refs: Value,
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
pub(super) struct AgentPolicyCheckRow {
    pub(super) policy_check_id: String,
    pub(super) tool_call_id: String,
    pub(super) tool_name: String,
    pub(super) policy_name: String,
    pub(super) decision: String,
    pub(super) reason: String,
    pub(super) evidence_refs: Value,
    pub(super) created_at: chrono::DateTime<chrono::Utc>,
}

pub(super) trait IntoClaimContext {
    fn into_context(self, items: Vec<ClaimItemRow>) -> ClaimContext;
}

impl IntoClaimContext for ClaimContextRow {
    fn into_context(self, items: Vec<ClaimItemRow>) -> ClaimContext {
        let member_id = MemberId::from_external(self.external_member_id.clone());
        let policy_id = PolicyId::from_external(self.external_policy_id.clone());
        let provider_id = ProviderId::from_external(self.external_provider_id.clone());

        ClaimContext {
            claim: Claim {
                id: ClaimId::from_external(self.external_claim_id.clone()),
                external_claim_id: self.external_claim_id,
                member_id: member_id.clone(),
                policy_id: policy_id.clone(),
                provider_id: provider_id.clone(),
                diagnosis_code: self.diagnosis_code,
                diagnosis_codes: vec![],
                service_date: self.service_date,
                amount: Money::new(self.claim_amount, self.claim_currency),
            },
            items: items
                .into_iter()
                .map(
                    |(
                        item_code,
                        item_type,
                        description,
                        quantity,
                        unit_amount,
                        total_amount,
                        currency,
                    )| ClaimItem {
                        item_code,
                        item_type,
                        description,
                        quantity: quantity.max(0) as u32,
                        unit_amount: Money::new(unit_amount, currency.clone()),
                        total_amount: Money::new(total_amount, currency),
                    },
                )
                .collect(),
            member: Member {
                id: member_id.clone(),
                external_member_id: self.external_member_id,
                dob: self.dob,
                gender: self.gender,
            },
            policy: Policy {
                id: policy_id,
                external_policy_id: self.external_policy_id,
                member_id,
                product_code: self.product_code,
                coverage_start_date: self.coverage_start_date,
                coverage_end_date: self.coverage_end_date,
                coverage_limit: Money::new(self.coverage_limit_amount, self.policy_currency),
            },
            provider: Provider {
                id: provider_id,
                external_provider_id: self.external_provider_id,
                name: self.provider_name,
                provider_type: self.provider_type,
                region: self.provider_region,
                risk_tier: provider_risk_tier_from_text(&self.provider_risk_tier),
            },
        }
    }
}

fn provider_risk_tier_from_text(value: &str) -> ProviderRiskTier {
    match value {
        "Low" => ProviderRiskTier::Low,
        "High" => ProviderRiskTier::High,
        _ => ProviderRiskTier::Medium,
    }
}

pub(super) fn inbox_claim_run_from_row(row: PgRow) -> PersistedInboxClaimRun {
    PersistedInboxClaimRun {
        run_id: row.try_get("run_id").unwrap_or_default(),
        audit_id: row.try_get("audit_id").unwrap_or_default(),
        external_message_id: row
            .try_get::<Option<String>, _>("external_message_id")
            .unwrap_or(None),
        idempotency_key: row
            .try_get::<Option<String>, _>("idempotency_key")
            .unwrap_or(None),
        external_message_fingerprint: row
            .try_get::<Option<String>, _>("external_message_fingerprint")
            .unwrap_or(None),
        raw_payload_checksum: row.try_get("raw_payload_checksum").unwrap_or_default(),
        raw_payload_ref: row
            .try_get::<Option<String>, _>("raw_payload_ref")
            .unwrap_or(None),
        mapping_version: row.try_get("mapping_version").unwrap_or_default(),
        validation_result: row.try_get("validation_result").unwrap_or_default(),
        scoring_ready: row.try_get("scoring_ready").unwrap_or(false),
        claim_id: row.try_get("claim_id").unwrap_or_default(),
        source_system: row.try_get("source_system").unwrap_or_default(),
        customer_scope_id: row.try_get("customer_scope_id").unwrap_or_default(),
        canonical_claim_context: row
            .try_get("canonical_claim_context")
            .unwrap_or_else(|_| serde_json::json!({})),
        validation_errors: row
            .try_get("validation_errors")
            .unwrap_or_else(|_| serde_json::json!([])),
        data_quality_signals: row
            .try_get("data_quality_signals")
            .unwrap_or_else(|_| serde_json::json!([])),
        evidence_refs: row
            .try_get("evidence_refs")
            .unwrap_or_else(|_| serde_json::json!([])),
    }
}
