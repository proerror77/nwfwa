use futures::join;

use super::request_get_json;
use crate::types::*;

pub(crate) async fn get_rule_ops_snapshot(
    api_key: String,
    rule_id: String,
) -> Result<RuleOpsSnapshot, String> {
    let (rules_res, performance_res) = join!(
        request_get_json::<RuleListResponse>("/api/v1/ops/rules", api_key.clone()),
        request_get_json::<RulePerformanceResponse>("/api/v1/ops/rules/performance", api_key.clone()),
    );
    let rules = rules_res?.rules;
    let performance = performance_res?.rules;
    let selected_rule_id = rules
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .map(|rule| rule.rule_id.clone())
        .or_else(|| rules.first().map(|rule| rule.rule_id.clone()))
        .unwrap_or(rule_id);
    let gates = request_get_json::<RulePromotionGates>(
        &format!("/api/v1/ops/rules/{selected_rule_id}/promotion-gates"),
        api_key,
    )
    .await?;
    Ok(RuleOpsSnapshot {
        rules,
        performance,
        gates,
    })
}
