CREATE DATABASE IF NOT EXISTS fwa_analytics;

CREATE TABLE IF NOT EXISTS fwa_analytics.analytics_scoring_events
(
    event_time DateTime64(3, 'UTC'),
    event_date Date DEFAULT toDate(event_time),
    customer_scope_id LowCardinality(String),
    source_system LowCardinality(String),
    scoring_run_id String,
    claim_hash String,
    member_hash String,
    provider_hash String,
    review_mode LowCardinality(String),
    risk_score Float64,
    rag_band LowCardinality(String),
    recommended_action LowCardinality(String),
    claim_amount Decimal(18, 2),
    model_key LowCardinality(String),
    model_version LowCardinality(String),
    rule_pack_version LowCardinality(String),
    evidence_refs Array(String),
    source_record_ref String
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(event_date)
ORDER BY (customer_scope_id, event_date, provider_hash, claim_hash, scoring_run_id);

CREATE TABLE IF NOT EXISTS fwa_analytics.analytics_rule_events
(
    event_time DateTime64(3, 'UTC'),
    event_date Date DEFAULT toDate(event_time),
    customer_scope_id LowCardinality(String),
    source_system LowCardinality(String),
    rule_run_id String,
    scoring_run_id String,
    claim_hash String,
    provider_hash String,
    rule_id LowCardinality(String),
    rule_version LowCardinality(String),
    scheme_code LowCardinality(String),
    rule_action LowCardinality(String),
    hit UInt8,
    severity LowCardinality(String),
    risk_delta Float64,
    false_positive_label Nullable(UInt8),
    confirmed_fwa_label Nullable(UInt8),
    saving_amount Decimal(18, 2),
    false_positive_cost Decimal(18, 2),
    evidence_refs Array(String),
    source_record_ref String
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(event_date)
ORDER BY (customer_scope_id, event_date, rule_id, provider_hash, scoring_run_id);

CREATE TABLE IF NOT EXISTS fwa_analytics.analytics_model_events
(
    event_time DateTime64(3, 'UTC'),
    event_date Date DEFAULT toDate(event_time),
    customer_scope_id LowCardinality(String),
    source_system LowCardinality(String),
    model_score_id String,
    scoring_run_id String,
    claim_hash String,
    provider_hash String,
    model_key LowCardinality(String),
    model_version LowCardinality(String),
    serving_version_lock LowCardinality(String),
    score Float64,
    threshold Float64,
    shadow_score Nullable(Float64),
    drift_population LowCardinality(String),
    feature_vector_hash String,
    feature_summary_json String,
    calibration_bucket LowCardinality(String),
    outcome_label Nullable(UInt8),
    evidence_refs Array(String),
    source_record_ref String
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(event_date)
ORDER BY (customer_scope_id, event_date, model_key, model_version, provider_hash, scoring_run_id);

CREATE TABLE IF NOT EXISTS fwa_analytics.analytics_case_sla_events
(
    event_time DateTime64(3, 'UTC'),
    event_date Date DEFAULT toDate(event_time),
    customer_scope_id LowCardinality(String),
    source_system LowCardinality(String),
    case_id String,
    lead_id String,
    claim_hash String,
    provider_hash String,
    reviewer_id_hash String,
    queue LowCardinality(String),
    case_status LowCardinality(String),
    priority LowCardinality(String),
    opened_at DateTime64(3, 'UTC'),
    triaged_at Nullable(DateTime64(3, 'UTC')),
    closed_at Nullable(DateTime64(3, 'UTC')),
    sla_minutes UInt32,
    minutes_to_triage Nullable(UInt32),
    minutes_to_close Nullable(UInt32),
    sla_breached UInt8,
    outcome LowCardinality(String),
    evidence_refs Array(String),
    source_record_ref String
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(event_date)
ORDER BY (customer_scope_id, event_date, queue, reviewer_id_hash, case_id);

CREATE TABLE IF NOT EXISTS fwa_analytics.analytics_value_events
(
    event_time DateTime64(3, 'UTC'),
    event_date Date DEFAULT toDate(event_time),
    customer_scope_id LowCardinality(String),
    source_system LowCardinality(String),
    attribution_id String,
    case_id String,
    claim_hash String,
    provider_hash String,
    rule_id LowCardinality(String),
    model_key LowCardinality(String),
    scheme_code LowCardinality(String),
    attribution_method LowCardinality(String),
    currency LowCardinality(String),
    prevented_amount Decimal(18, 2),
    recovered_amount Decimal(18, 2),
    avoided_exposure_amount Decimal(18, 2),
    saving_amount Decimal(18, 2),
    review_cost_amount Decimal(18, 2),
    false_positive_cost Decimal(18, 2),
    net_value_amount Decimal(18, 2),
    evidence_refs Array(String),
    source_record_ref String
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(event_date)
ORDER BY (customer_scope_id, event_date, scheme_code, rule_id, model_key, provider_hash);

CREATE TABLE IF NOT EXISTS fwa_analytics.analytics_reviewer_capacity_events
(
    event_time DateTime64(3, 'UTC'),
    event_date Date DEFAULT toDate(event_time),
    customer_scope_id LowCardinality(String),
    source_system LowCardinality(String),
    reviewer_id_hash String,
    queue LowCardinality(String),
    reviewer_role LowCardinality(String),
    assigned_case_count UInt32,
    closed_case_count UInt32,
    confirmed_fwa_count UInt32,
    false_positive_count UInt32,
    available_minutes UInt32,
    review_minutes UInt32,
    capacity_utilization Float64,
    precision_at_capacity Float64,
    evidence_refs Array(String),
    source_record_ref String
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(event_date)
ORDER BY (customer_scope_id, event_date, queue, reviewer_id_hash);

CREATE TABLE IF NOT EXISTS fwa_analytics.analytics_provider_graph_snapshots
(
    snapshot_time DateTime64(3, 'UTC'),
    snapshot_date Date DEFAULT toDate(snapshot_time),
    customer_scope_id LowCardinality(String),
    source_system LowCardinality(String),
    snapshot_id String,
    graph_version LowCardinality(String),
    provider_hash String,
    provider_segment LowCardinality(String),
    connected_provider_count UInt32,
    shared_member_count UInt32,
    referral_edge_count UInt32,
    claim_count UInt32,
    high_risk_claim_count UInt32,
    claim_concentration_ratio Float64,
    suspicious_cluster_score Float64,
    top_relationship_refs Array(String),
    evidence_refs Array(String),
    source_record_ref String
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(snapshot_date)
ORDER BY (customer_scope_id, snapshot_date, provider_segment, provider_hash, snapshot_id);
