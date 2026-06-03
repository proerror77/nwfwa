-- scoring_volume_daily
SELECT
    customer_scope_id,
    event_date,
    source_system,
    review_mode,
    rag_band,
    recommended_action,
    count() AS scored_count,
    avg(risk_score) AS avg_risk_score,
    quantile(0.95)(risk_score) AS p95_risk_score,
    sum(claim_amount) AS claim_amount,
    countIf(rag_band IN ('RED', 'CRITICAL')) AS high_risk_count,
    high_risk_count / nullIf(scored_count, 0) AS high_risk_rate
FROM fwa_analytics.analytics_scoring_events
WHERE event_date >= today() - 30
GROUP BY customer_scope_id, event_date, source_system, review_mode, rag_band, recommended_action
ORDER BY customer_scope_id, source_system, review_mode, rag_band, event_date;

-- rule_drift_daily
SELECT
    customer_scope_id,
    event_date,
    rule_id,
    count() AS evaluated_count,
    sum(hit) AS hit_count,
    hit_count / nullIf(evaluated_count, 0) AS hit_rate,
    avg(risk_delta) AS avg_risk_delta,
    sum(false_positive_label = 1) AS false_positive_count,
    false_positive_count / nullIf(hit_count, 0) AS false_positive_rate,
    sum(saving_amount) AS saving_amount,
    sum(false_positive_cost) AS false_positive_cost
FROM fwa_analytics.analytics_rule_events
WHERE event_date >= today() - 30
GROUP BY customer_scope_id, event_date, rule_id
ORDER BY customer_scope_id, rule_id, event_date;

-- model_drift_daily
SELECT
    customer_scope_id,
    event_date,
    model_key,
    model_version,
    drift_population,
    count() AS scored_count,
    avg(score) AS avg_score,
    quantile(0.5)(score) AS median_score,
    quantile(0.95)(score) AS p95_score,
    avg(abs(score - ifNull(shadow_score, score))) AS avg_shadow_delta,
    sum(outcome_label = 1) / nullIf(countIf(outcome_label IS NOT NULL), 0) AS positive_label_rate
FROM fwa_analytics.analytics_model_events
WHERE event_date >= today() - 30
GROUP BY customer_scope_id, event_date, model_key, model_version, drift_population
ORDER BY customer_scope_id, model_key, model_version, drift_population, event_date;

-- sla_reporting_daily
SELECT
    customer_scope_id,
    event_date,
    queue,
    priority,
    count() AS case_count,
    sum(sla_breached) AS sla_breached_count,
    sla_breached_count / nullIf(case_count, 0) AS sla_breach_rate,
    avg(minutes_to_triage) AS avg_minutes_to_triage,
    quantile(0.9)(minutes_to_close) AS p90_minutes_to_close
FROM fwa_analytics.analytics_case_sla_events
WHERE event_date >= today() - 30
GROUP BY customer_scope_id, event_date, queue, priority
ORDER BY customer_scope_id, queue, priority, event_date;

-- roi_reporting_daily
SELECT
    customer_scope_id,
    event_date,
    scheme_code,
    rule_id,
    model_key,
    currency,
    sum(prevented_amount) AS prevented_amount,
    sum(recovered_amount) AS recovered_amount,
    sum(avoided_exposure_amount) AS avoided_exposure_amount,
    sum(saving_amount) AS gross_saving_amount,
    sum(review_cost_amount) AS review_cost_amount,
    sum(false_positive_cost) AS false_positive_cost,
    sum(net_value_amount) AS net_value_amount,
    net_value_amount / nullIf(review_cost_amount + false_positive_cost, 0) AS roi_ratio
FROM fwa_analytics.analytics_value_events
WHERE event_date >= today() - 90
GROUP BY customer_scope_id, event_date, scheme_code, rule_id, model_key, currency
ORDER BY customer_scope_id, scheme_code, rule_id, model_key, event_date;

-- reviewer_capacity_daily
SELECT
    customer_scope_id,
    event_date,
    queue,
    reviewer_role,
    sum(assigned_case_count) AS assigned_case_count,
    sum(closed_case_count) AS closed_case_count,
    sum(confirmed_fwa_count) AS confirmed_fwa_count,
    sum(false_positive_count) AS false_positive_count,
    sum(review_minutes) AS review_minutes,
    sum(available_minutes) AS available_minutes,
    review_minutes / nullIf(available_minutes, 0) AS capacity_utilization,
    confirmed_fwa_count / nullIf(closed_case_count, 0) AS precision_at_capacity
FROM fwa_analytics.analytics_reviewer_capacity_events
WHERE event_date >= today() - 30
GROUP BY customer_scope_id, event_date, queue, reviewer_role
ORDER BY customer_scope_id, queue, reviewer_role, event_date;

-- false_positive_cost_daily
SELECT
    customer_scope_id,
    event_date,
    scheme_code,
    sum(false_positive_cost) AS false_positive_cost,
    sum(review_cost_amount) AS review_cost_amount,
    sum(saving_amount) AS saving_amount,
    false_positive_cost / nullIf(saving_amount + false_positive_cost, 0) AS false_positive_cost_rate
FROM fwa_analytics.analytics_value_events
WHERE event_date >= today() - 90
GROUP BY customer_scope_id, event_date, scheme_code
ORDER BY customer_scope_id, scheme_code, event_date;

-- provider_graph_snapshots
SELECT
    customer_scope_id,
    snapshot_date,
    provider_segment,
    count() AS provider_count,
    avg(connected_provider_count) AS avg_connected_provider_count,
    avg(shared_member_count) AS avg_shared_member_count,
    avg(referral_edge_count) AS avg_referral_edge_count,
    quantile(0.95)(claim_concentration_ratio) AS p95_claim_concentration_ratio,
    quantile(0.95)(suspicious_cluster_score) AS p95_suspicious_cluster_score,
    sum(high_risk_claim_count) AS high_risk_claim_count
FROM fwa_analytics.analytics_provider_graph_snapshots
WHERE snapshot_date >= today() - 90
GROUP BY customer_scope_id, snapshot_date, provider_segment
ORDER BY customer_scope_id, provider_segment, snapshot_date;
