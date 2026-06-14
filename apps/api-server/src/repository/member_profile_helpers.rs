use super::{format_decimal_cents, MemberProfileSummaryInput, MemberProfileSummaryRecord};
use fwa_core::ClaimContext;
use rust_decimal::Decimal;
use std::collections::BTreeSet;

pub(super) fn member_profile_from_contexts(
    member_id: &str,
    contexts: &[ClaimContext],
    runs: &[super::PersistedScoringRun],
) -> Option<MemberProfileSummaryRecord> {
    if contexts.is_empty() {
        return None;
    }

    let claim_ids = contexts
        .iter()
        .map(|context| context.claim.external_claim_id.clone())
        .collect::<BTreeSet<_>>();
    let policy_count = contexts
        .iter()
        .map(|context| context.policy.external_policy_id.clone())
        .collect::<BTreeSet<_>>()
        .len() as u32;
    let total_claim_amount = contexts
        .iter()
        .map(|context| context.claim.amount.amount)
        .sum::<Decimal>();
    let currency = contexts
        .first()
        .map(|context| context.claim.amount.currency.clone())
        .unwrap_or_else(|| "UNKNOWN".into());
    let high_risk_claim_count = runs
        .iter()
        .filter(|run| claim_ids.contains(&run.claim_id) && run.risk_score >= 70)
        .map(|run| run.claim_id.clone())
        .collect::<BTreeSet<_>>()
        .len() as u32;
    let latest_claim_id = contexts
        .iter()
        .max_by(|left, right| {
            left.claim
                .service_date
                .cmp(&right.claim.service_date)
                .then_with(|| {
                    left.claim
                        .external_claim_id
                        .cmp(&right.claim.external_claim_id)
                })
        })
        .map(|context| context.claim.external_claim_id.clone());
    let evidence_refs = std::iter::once(format!("members:{member_id}"))
        .chain(
            claim_ids
                .iter()
                .map(|claim_id| format!("claims:{claim_id}")),
        )
        .collect::<BTreeSet<_>>();

    Some(member_profile_summary_record(MemberProfileSummaryInput {
        member_id: member_id.into(),
        claim_count: contexts.len() as u32,
        policy_count,
        total_claim_amount,
        currency,
        high_risk_claim_count,
        latest_claim_id,
        evidence_refs,
    }))
}

pub(super) fn member_profile_summary_record(
    input: MemberProfileSummaryInput,
) -> MemberProfileSummaryRecord {
    let risk_level_summary = if input.high_risk_claim_count > 0 {
        "has_high_risk_history"
    } else {
        "no_high_risk_history"
    };
    let profile_summary = format!(
        "投保人共有 {} 张保单、{} 笔历史理赔，累计理赔金额 {} {}，其中 {} 笔为高风险评分记录。",
        input.policy_count,
        input.claim_count,
        format_decimal_cents(input.total_claim_amount),
        input.currency,
        input.high_risk_claim_count
    );

    MemberProfileSummaryRecord {
        member_id: input.member_id,
        claim_count: input.claim_count,
        policy_count: input.policy_count,
        total_claim_amount: input.total_claim_amount,
        currency: input.currency,
        high_risk_claim_count: input.high_risk_claim_count,
        latest_claim_id: input.latest_claim_id,
        risk_level_summary: risk_level_summary.into(),
        profile_summary,
        evidence_refs: input.evidence_refs.into_iter().collect(),
    }
}
