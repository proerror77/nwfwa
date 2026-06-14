CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

INSERT INTO members (id, external_member_id, name_hash, dob, gender)
VALUES
  ('10000000-0000-0000-0000-000000000287', 'MBR-0287', 'demo-member-0287', '1985-03-14', 'F'),
  ('10000000-0000-0000-0000-000000000900', 'MBR-9100', 'demo-member-9100', '1978-09-02', 'M')
ON CONFLICT (external_member_id) DO UPDATE
SET name_hash = EXCLUDED.name_hash,
    dob = EXCLUDED.dob,
    gender = EXCLUDED.gender,
    updated_at = now();

INSERT INTO providers (id, external_provider_id, name, provider_type, region, risk_tier)
VALUES
  ('20000000-0000-0000-0000-000000000287', 'PRV-0287', 'Northwind Hospital', 'hospital', 'Shanghai', 'High'),
  ('20000000-0000-0000-0000-000000000900', 'PRV-9100', 'Harbor Specialty Clinic', 'clinic', 'Shanghai', 'Medium')
ON CONFLICT (external_provider_id) DO UPDATE
SET name = EXCLUDED.name,
    provider_type = EXCLUDED.provider_type,
    region = EXCLUDED.region,
    risk_tier = EXCLUDED.risk_tier,
    updated_at = now();

INSERT INTO policies (
  id,
  external_policy_id,
  member_id,
  product_code,
  coverage_start_date,
  coverage_end_date,
  coverage_limit_amount,
  currency
)
VALUES
  (
    '30000000-0000-0000-0000-000000000287',
    'POL-0287',
    (SELECT id FROM members WHERE external_member_id = 'MBR-0287'),
    'MED',
    '2026-01-01',
    '2026-12-31',
    10000,
    'CNY'
  ),
  (
    '30000000-0000-0000-0000-000000000900',
    'POL-9100',
    (SELECT id FROM members WHERE external_member_id = 'MBR-9100'),
    'MED-PREMIUM',
    '2026-01-01',
    '2026-12-31',
    50000,
    'CNY'
  )
ON CONFLICT (external_policy_id) DO UPDATE
SET member_id = EXCLUDED.member_id,
    product_code = EXCLUDED.product_code,
    coverage_start_date = EXCLUDED.coverage_start_date,
    coverage_end_date = EXCLUDED.coverage_end_date,
    coverage_limit_amount = EXCLUDED.coverage_limit_amount,
    currency = EXCLUDED.currency,
    updated_at = now();

INSERT INTO claims (
  id,
  external_claim_id,
  member_id,
  policy_id,
  provider_id,
  claim_type,
  diagnosis_code,
  service_date,
  claim_amount,
  currency,
  status,
  raw_payload
)
VALUES
  (
    '40000000-0000-0000-0000-000000000287',
    'CLM-0287',
    (SELECT id FROM members WHERE external_member_id = 'MBR-0287'),
    (SELECT id FROM policies WHERE external_policy_id = 'POL-0287'),
    (SELECT id FROM providers WHERE external_provider_id = 'PRV-0287'),
    'medical',
    'J10',
    '2026-01-06',
    8000,
    'CNY',
    'submitted',
    '{"demo": true, "source": "seed_demo"}'::jsonb
  ),
  (
    '40000000-0000-0000-0000-000000000900',
    'CLM-9100',
    (SELECT id FROM members WHERE external_member_id = 'MBR-9100'),
    (SELECT id FROM policies WHERE external_policy_id = 'POL-9100'),
    (SELECT id FROM providers WHERE external_provider_id = 'PRV-9100'),
    'medical',
    'M54',
    '2026-02-12',
    12600,
    'CNY',
    'qa_reviewed',
    '{"demo": true, "source": "seed_demo", "timeline": "historical"}'::jsonb
  )
ON CONFLICT (external_claim_id) DO UPDATE
SET member_id = EXCLUDED.member_id,
    policy_id = EXCLUDED.policy_id,
    provider_id = EXCLUDED.provider_id,
    claim_type = EXCLUDED.claim_type,
    diagnosis_code = EXCLUDED.diagnosis_code,
    service_date = EXCLUDED.service_date,
    claim_amount = EXCLUDED.claim_amount,
    currency = EXCLUDED.currency,
    status = EXCLUDED.status,
    raw_payload = EXCLUDED.raw_payload,
    updated_at = now();

DELETE FROM claim_items
WHERE claim_id IN (
  SELECT id FROM claims WHERE external_claim_id IN ('CLM-0287', 'CLM-9100')
);

INSERT INTO claim_items (
  claim_id,
  item_code,
  item_type,
  description,
  quantity,
  unit_amount,
  total_amount,
  currency
)
VALUES
  ((SELECT id FROM claims WHERE external_claim_id = 'CLM-0287'), 'PROC-001', 'procedure', 'Imaging package', 1, 8000, 8000, 'CNY'),
  ((SELECT id FROM claims WHERE external_claim_id = 'CLM-9100'), 'PROC-021', 'procedure', 'Pain management package', 1, 7600, 7600, 'CNY'),
  ((SELECT id FROM claims WHERE external_claim_id = 'CLM-9100'), 'DRUG-031', 'drug', 'Adjuvant medication bundle', 1, 5000, 5000, 'CNY');

INSERT INTO rules (id, rule_key, name, status, owner)
VALUES
  ('50000000-0000-0000-0000-000000000001', 'rule_early_claim', 'Early claim', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000002', 'rule_high_amount_to_limit', 'High amount to policy limit', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000003', 'rule_early_high_amount', 'Early high amount', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000004', 'rule_duplicate_claim', 'Duplicate claim', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000005', 'rule_provider_profile_high', 'Provider profile high', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000006', 'rule_low_medical_match', 'Low medical match', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000007', 'rule_medically_unnecessary_service', 'Medically unnecessary service', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000008', 'rule_large_limit_usage', 'Large limit usage', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000009', 'rule_high_cost_single_item', 'High cost single item', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000010', 'rule_upcoding_complexity', 'Upcoding complexity', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000011', 'rule_unbundling_component_pattern', 'Unbundling component pattern', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000012', 'rule_same_member_repeated_service', 'Same member repeated service', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000013', 'rule_relationship_concentration', 'Relationship concentration', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000014', 'rule_many_claim_items', 'Many claim items', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000015', 'rule_peer_p95_amount', 'Peer P95 amount', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000016', 'rule_peer_p99_amount', 'Peer P99 amount', 'active', 'rules-ops'),
  ('50000000-0000-0000-0000-000000000017', 'rule_provider_high_risk_tier', 'Provider high risk tier', 'active', 'rules-ops')
ON CONFLICT (rule_key) DO UPDATE
SET name = EXCLUDED.name,
    status = EXCLUDED.status,
    owner = EXCLUDED.owner,
    updated_at = now();

INSERT INTO rule_versions (
  rule_id,
  version,
  dsl,
  score,
  recommended_action,
  created_by,
  approved_by,
  published_at
)
VALUES
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_early_claim'),
    1,
    '{"review_mode":"both","scheme_family":"early_high_value_claim","conditions":[{"field":"days_since_policy_start","operator":"<=","value":7}],"action":{"score":75,"alert_code":"EARLY_CLAIM","recommended_action":"ManualReview","reason":"Policy start within 7 days"}}'::jsonb,
    75,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_high_amount_to_limit'),
    1,
    '{"review_mode":"both","scheme_family":"early_high_value_claim","conditions":[{"field":"claim_amount_to_limit_ratio","operator":">=","value":0.75}],"action":{"score":30,"alert_code":"HIGH_AMOUNT_TO_LIMIT","recommended_action":"ManualReview","reason":"Claim amount consumes a high share of policy limit"}}'::jsonb,
    30,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_early_high_amount'),
    1,
    '{"review_mode":"both","scheme_family":"early_high_value_claim","conditions":[{"field":"days_since_policy_start","operator":"<=","value":10},{"field":"claim_amount_to_limit_ratio","operator":">=","value":0.7}],"action":{"score":45,"alert_code":"EARLY_HIGH_AMOUNT","recommended_action":"ManualReview","reason":"Policy start is recent and claim amount is high relative to limit"}}'::jsonb,
    45,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_duplicate_claim'),
    1,
    '{"review_mode":"both","scheme_family":"duplicate_billing","conditions":[{"field":"duplicate_claim_similarity_score","operator":">=","value":0.95}],"action":{"score":35,"alert_code":"DUPLICATE_CLAIM","recommended_action":"ManualReview","reason":"Similar member, provider, service date, item, and amount indicate possible duplicate billing"}}'::jsonb,
    35,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_provider_profile_high'),
    1,
    '{"review_mode":"both","scheme_family":"provider_peer_outlier","conditions":[{"field":"provider_profile_score","operator":">=","value":70}],"action":{"score":30,"alert_code":"PROVIDER_PROFILE_HIGH","recommended_action":"ManualReview","reason":"Provider risk profile score is high versus peer baseline"}}'::jsonb,
    30,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_low_medical_match'),
    1,
    '{"review_mode":"both","scheme_family":"diagnosis_procedure_mismatch","conditions":[{"field":"diagnosis_procedure_match_score","operator":"<=","value":0.4}],"action":{"score":30,"alert_code":"LOW_MEDICAL_MATCH","recommended_action":"ManualReview","reason":"Diagnosis and billed procedure have weak medical match"}}'::jsonb,
    30,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_medically_unnecessary_service'),
    1,
    '{"review_mode":"both","scheme_family":"medically_unnecessary_service","conditions":[{"field":"clinical_review_required","operator":"==","value":1}],"action":{"score":30,"alert_code":"MEDICALLY_UNNECESSARY_SERVICE","recommended_action":"ManualReview","reason":"Clinical evidence is missing or insufficient for medical necessity"}}'::jsonb,
    30,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_large_limit_usage'),
    1,
    '{"review_mode":"both","scheme_family":"early_high_value_claim","conditions":[{"field":"claim_amount_to_limit_ratio","operator":">=","value":0.8}],"action":{"score":35,"alert_code":"LARGE_LIMIT_USAGE","recommended_action":"ManualReview","reason":"Claim amount is close to policy limit"}}'::jsonb,
    35,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_high_cost_single_item'),
    1,
    '{"review_mode":"both","scheme_family":"high_risk_claim","conditions":[{"field":"high_cost_item_ratio","operator":">=","value":0.5}],"action":{"score":25,"alert_code":"HIGH_COST_SINGLE_ITEM","recommended_action":"ManualReview","reason":"Single high-cost item contributes a high share of claim amount"}}'::jsonb,
    25,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_upcoding_complexity'),
    1,
    '{"review_mode":"both","scheme_family":"upcoding","conditions":[{"field":"diagnosis_procedure_match_score","operator":"<=","value":0.45},{"field":"high_cost_item_ratio","operator":">=","value":0.5}],"action":{"score":35,"alert_code":"UPCODING_COMPLEXITY","recommended_action":"ManualReview","reason":"High-complexity or high-cost services have weak diagnosis support"}}'::jsonb,
    35,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_unbundling_component_pattern'),
    1,
    '{"review_mode":"both","scheme_family":"unbundling","conditions":[{"field":"claim_item_count","operator":">=","value":6}],"action":{"score":25,"alert_code":"UNBUNDLING_COMPONENT_PATTERN","recommended_action":"ManualReview","reason":"Claim contains unusually many line items and may require unbundling review"}}'::jsonb,
    25,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_same_member_repeated_service'),
    1,
    '{"review_mode":"both","scheme_family":"excessive_utilization","conditions":[{"field":"same_member_service_count_30d","operator":">=","value":3}],"action":{"score":25,"alert_code":"SAME_MEMBER_REPEATED_SERVICE","recommended_action":"ManualReview","reason":"Same member has repeated similar service usage within 30 days"}}'::jsonb,
    25,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_relationship_concentration'),
    1,
    '{"review_mode":"both","scheme_family":"relationship_concentration","conditions":[{"field":"provider_high_risk_neighbor_signal","operator":"==","value":true}],"action":{"score":35,"alert_code":"RELATIONSHIP_CONCENTRATION","recommended_action":"EscalateInvestigation","reason":"Provider relationship graph has high-risk neighbor or concentration signal"}}'::jsonb,
    35,
    'EscalateInvestigation',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_many_claim_items'),
    1,
    '{"review_mode":"both","scheme_family":"excessive_utilization","conditions":[{"field":"claim_item_count","operator":">=","value":5}],"action":{"score":20,"alert_code":"MANY_CLAIM_ITEMS","recommended_action":"ManualReview","reason":"Claim contains many billing line items"}}'::jsonb,
    20,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_peer_p95_amount'),
    1,
    '{"review_mode":"both","scheme_family":"provider_peer_outlier","conditions":[{"field":"claim_amount_peer_percentile","operator":">=","value":95}],"action":{"score":25,"alert_code":"PEER_P95_AMOUNT","recommended_action":"ManualReview","reason":"Claim amount is above peer P95"}}'::jsonb,
    25,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_peer_p99_amount'),
    1,
    '{"review_mode":"both","scheme_family":"provider_peer_outlier","conditions":[{"field":"claim_amount_peer_percentile","operator":">=","value":99}],"action":{"score":40,"alert_code":"PEER_P99_AMOUNT","recommended_action":"ManualReview","reason":"Claim amount is above peer P99"}}'::jsonb,
    40,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_provider_high_risk_tier'),
    1,
    '{"review_mode":"both","scheme_family":"provider_peer_outlier","conditions":[{"field":"provider_risk_tier","operator":"==","value":"HIGH"}],"action":{"score":30,"alert_code":"PROVIDER_HIGH_RISK_TIER","recommended_action":"ManualReview","reason":"Provider risk tier is high"}}'::jsonb,
    30,
    'ManualReview',
    'seed',
    'seed',
    now()
  )
ON CONFLICT (rule_id, version) DO UPDATE
SET dsl = EXCLUDED.dsl,
    score = EXCLUDED.score,
    recommended_action = EXCLUDED.recommended_action,
    approved_by = EXCLUDED.approved_by,
    published_at = EXCLUDED.published_at;

DELETE FROM model_scores
WHERE model_version_id IN (
  SELECT id
  FROM model_versions
  WHERE model_key = 'baseline_fwa'
    AND version <> '0.1.0'
);

DELETE FROM model_promotion_reviews
WHERE model_key = 'baseline_fwa'
  AND model_version <> '0.1.0';

DELETE FROM model_evaluation_runs
WHERE model_key = 'baseline_fwa'
  AND model_version <> '0.1.0';

DELETE FROM model_retraining_jobs
WHERE model_key = 'baseline_fwa';

DELETE FROM model_versions
WHERE model_key = 'baseline_fwa'
  AND version <> '0.1.0';

INSERT INTO model_versions (
  id,
  model_key,
  version,
  model_type,
  runtime_kind,
  artifact_uri,
  endpoint_url,
  execution_provider,
  status,
  metrics,
  activated_at
)
VALUES (
  '60000000-0000-0000-0000-000000000001',
  'baseline_fwa',
  '0.1.0',
  'baseline_classifier',
  'python_http',
  's3://fwa-demo/models/baseline_fwa/0.1.0/model.pkl',
  'http://127.0.0.1:8001/score',
  'cpu',
  'active',
  '{"auc":0.81,"precision":0.72,"recall":0.66}'::jsonb,
  now()
)
ON CONFLICT (model_key, version) DO UPDATE
SET model_type = EXCLUDED.model_type,
    runtime_kind = EXCLUDED.runtime_kind,
    artifact_uri = EXCLUDED.artifact_uri,
    endpoint_url = EXCLUDED.endpoint_url,
    execution_provider = EXCLUDED.execution_provider,
    status = EXCLUDED.status,
    metrics = EXCLUDED.metrics,
    activated_at = EXCLUDED.activated_at;

INSERT INTO external_data_sources (
  source_key,
  display_name,
  business_domain,
  owner,
  description,
  status
)
VALUES (
  'fwa_demo_claim_features',
  'FWA Demo Claim Feature Store',
  'health_fwa',
  'feature-ops',
  'Demo feature catalog for explainable health-insurance FWA factor cards.',
  'active'
)
ON CONFLICT (source_key) DO UPDATE
SET display_name = EXCLUDED.display_name,
    business_domain = EXCLUDED.business_domain,
    owner = EXCLUDED.owner,
    description = EXCLUDED.description,
    status = EXCLUDED.status,
    updated_at = now();

INSERT INTO external_dataset_versions (
  id,
  source_key,
  dataset_key,
  dataset_version,
  sample_grain,
  label_column,
  entity_keys,
  manifest_uri,
  schema_uri,
  profile_uri,
  storage_format,
  schema_hash,
  row_count,
  status
)
VALUES (
  '70000000-0000-0000-0000-000000000001',
  'fwa_demo_claim_features',
  'fwa_claim_feature_cards',
  'v1',
  'claim',
  'confirmed_fwa',
  '["claim_id","provider_id","member_id"]'::jsonb,
  'data/demo/fwa_claim_feature_cards/v1/manifest.json',
  'data/demo/fwa_claim_feature_cards/v1/schema.json',
  'data/demo/fwa_claim_feature_cards/v1/profile.json',
  'parquet',
  'sha256:fwa-demo-feature-cards-v1',
  2,
  'active'
)
ON CONFLICT (dataset_key, dataset_version) DO UPDATE
SET sample_grain = EXCLUDED.sample_grain,
    label_column = EXCLUDED.label_column,
    entity_keys = EXCLUDED.entity_keys,
    manifest_uri = EXCLUDED.manifest_uri,
    schema_uri = EXCLUDED.schema_uri,
    profile_uri = EXCLUDED.profile_uri,
    storage_format = EXCLUDED.storage_format,
    schema_hash = EXCLUDED.schema_hash,
    row_count = EXCLUDED.row_count,
    status = EXCLUDED.status;

INSERT INTO external_dataset_splits (
  dataset_id,
  split_name,
  data_uri,
  row_count,
  positive_count,
  negative_count,
  label_distribution_json
)
VALUES (
  '70000000-0000-0000-0000-000000000001',
  'demo',
  'data/demo/fwa_claim_feature_cards/v1/split=demo/',
  2,
  1,
  1,
  '{"confirmed":1,"not_confirmed":1}'::jsonb
)
ON CONFLICT (dataset_id, split_name) DO UPDATE
SET data_uri = EXCLUDED.data_uri,
    row_count = EXCLUDED.row_count,
    positive_count = EXCLUDED.positive_count,
    negative_count = EXCLUDED.negative_count,
    label_distribution_json = EXCLUDED.label_distribution_json;

INSERT INTO external_schema_fields (
  dataset_id,
  field_name,
  logical_type,
  nullable,
  semantic_role,
  description,
  profile_json
)
VALUES
  (
    '70000000-0000-0000-0000-000000000001',
    'claim_id',
    'string',
    false,
    'key',
    'External claim identifier used for evidence traceability.',
    '{"missing_rate":0.0,"business_meaning":"理赔案件追踪键","risk_direction":"identifier","calculation_window":"claim","calculation_logic":"source claim id","source_table":"claims","source_fields":["external_claim_id"],"owner":"data-ops","version":1,"online_available":true,"convertible_to_rule":false}'::jsonb
  ),
  (
    '70000000-0000-0000-0000-000000000001',
    'provider_id',
    'string',
    false,
    'key',
    'External provider identifier used for provider profile and network risk joins.',
    '{"missing_rate":0.0,"business_meaning":"医疗服务方关联键","risk_direction":"identifier","calculation_window":"claim","calculation_logic":"source provider id","source_table":"providers","source_fields":["external_provider_id"],"owner":"data-ops","version":1,"online_available":true,"convertible_to_rule":false}'::jsonb
  ),
  (
    '70000000-0000-0000-0000-000000000001',
    'member_id',
    'string',
    false,
    'key',
    'External member identifier used for member utilization features.',
    '{"missing_rate":0.0,"business_meaning":"被保人关联键","risk_direction":"identifier","calculation_window":"claim","calculation_logic":"source member id","source_table":"members","source_fields":["external_member_id"],"owner":"data-ops","version":1,"online_available":true,"convertible_to_rule":false}'::jsonb
  ),
  (
    '70000000-0000-0000-0000-000000000001',
    'claim_amount_to_limit_ratio',
    'decimal',
    false,
    'feature',
    'Claim amount divided by the policy coverage limit.',
    '{"missing_rate":0.0,"business_meaning":"理赔金额占保障额度比例","risk_direction":"higher_is_riskier","calculation_window":"claim","calculation_logic":"claim_amount / coverage_limit_amount","source_table":"claims, policies","source_fields":["claim_amount","coverage_limit_amount"],"owner":"feature-ops","version":1,"iv":0.21,"auc_gain":0.03,"lift":2.4,"psi":0.04,"model_contribution":0.18,"online_available":true,"convertible_to_rule":true}'::jsonb
  ),
  (
    '70000000-0000-0000-0000-000000000001',
    'days_since_policy_start',
    'int32',
    false,
    'feature',
    'Days between policy start date and claim service date.',
    '{"missing_rate":0.0,"business_meaning":"保单生效后短期理赔","risk_direction":"lower_is_riskier","calculation_window":"claim","calculation_logic":"service_date - coverage_start_date","source_table":"claims, policies","source_fields":["service_date","coverage_start_date"],"owner":"feature-ops","version":1,"iv":0.17,"auc_gain":0.02,"lift":1.9,"psi":0.06,"model_contribution":0.12,"online_available":true,"convertible_to_rule":true}'::jsonb
  ),
  (
    '70000000-0000-0000-0000-000000000001',
    'diagnosis_procedure_match_score',
    'decimal',
    false,
    'feature',
    'Clinical consistency score between diagnosis and billed procedures.',
    '{"missing_rate":0.02,"business_meaning":"诊断与诊疗项目医学匹配度","risk_direction":"lower_is_riskier","calculation_window":"claim","calculation_logic":"clinical rule baseline score for diagnosis and procedure pair","source_table":"claims, claim_items, medical_codes","source_fields":["diagnosis_code","item_code","item_type"],"owner":"clinical-ops","version":1,"iv":0.24,"auc_gain":0.04,"lift":2.8,"psi":0.08,"model_contribution":0.22,"online_available":true,"convertible_to_rule":true}'::jsonb
  ),
  (
    '70000000-0000-0000-0000-000000000001',
    'provider_high_cost_item_ratio_30d',
    'decimal',
    true,
    'feature',
    'Provider 30-day ratio of high-cost claim items.',
    '{"missing_rate":0.04,"business_meaning":"Provider 近 30 天高价项目占比","risk_direction":"higher_is_riskier","calculation_window":"30d","calculation_logic":"high_cost_item_count_30d / item_count_30d","source_table":"claim_items, providers","source_fields":["provider_id","item_code","total_amount"],"owner":"provider-risk","version":1,"iv":0.19,"auc_gain":0.03,"lift":2.2,"psi":0.05,"model_contribution":0.16,"online_available":true,"convertible_to_rule":true}'::jsonb
  ),
  (
    '70000000-0000-0000-0000-000000000001',
    'confirmed_fwa',
    'boolean',
    false,
    'label',
    'Confirmed FWA outcome from investigation or QA.',
    '{"missing_rate":0.0,"business_meaning":"调查或 QA 确认的 FWA 标签","risk_direction":"label","calculation_window":"outcome","calculation_logic":"human-confirmed investigation or QA result","source_table":"investigation_results, qa_reviews","source_fields":["confirmed_fwa","qa_conclusion"],"owner":"qa-ops","version":1,"online_available":false,"convertible_to_rule":false}'::jsonb
  )
ON CONFLICT (dataset_id, field_name) DO UPDATE
SET logical_type = EXCLUDED.logical_type,
    nullable = EXCLUDED.nullable,
    semantic_role = EXCLUDED.semantic_role,
    description = EXCLUDED.description,
    profile_json = EXCLUDED.profile_json;

INSERT INTO knowledge_cases (
  case_id,
  title,
  fwa_type,
  diagnosis_code,
  provider_region,
  provider_type,
  summary,
  outcome,
  tags,
  evidence_refs
)
VALUES
  (
    'KC-1001',
    'Early high-amount respiratory claim',
    'Abuse',
    'J10',
    'Shanghai',
    'hospital',
    'Policy-start-window respiratory claim with high billed amount and low supporting evidence.',
    'Manual review confirmed over-treatment pattern',
    '["early_claim","high_amount","medical_mismatch"]'::jsonb,
    '["knowledge_cases:KC-1001","rule_runs:EARLY_CLAIM"]'::jsonb
  ),
  (
    'KC-1002',
    'Provider repeated high-cost package pattern',
    'Waste',
    'M54',
    'Shanghai',
    'clinic',
    'Provider repeatedly bills high-cost pain management packages above peer distribution.',
    'Provider education and pre-payment review added',
    '["provider_pattern","high_amount","peer_deviation"]'::jsonb,
    '["knowledge_cases:KC-1002","feature_values:provider_high_cost_item_ratio_30d"]'::jsonb
  )
ON CONFLICT (case_id) DO UPDATE
SET title = EXCLUDED.title,
    fwa_type = EXCLUDED.fwa_type,
    diagnosis_code = EXCLUDED.diagnosis_code,
    provider_region = EXCLUDED.provider_region,
    provider_type = EXCLUDED.provider_type,
    summary = EXCLUDED.summary,
    outcome = EXCLUDED.outcome,
    tags = EXCLUDED.tags,
    evidence_refs = EXCLUDED.evidence_refs,
    updated_at = now();

INSERT INTO external_data_sources (
  id,
  source_key,
  display_name,
  business_domain,
  owner,
  description,
  status
)
VALUES (
  '70000000-0000-0000-0000-000000000001',
  'demo_claims_lake',
  'Demo Claims Lake',
  'health_claims_fwa',
  'data-ops',
  'Curated demo Parquet dataset for FWA scoring, factor cards, and model evaluation.',
  'active'
)
ON CONFLICT (source_key) DO UPDATE
SET display_name = EXCLUDED.display_name,
    business_domain = EXCLUDED.business_domain,
    owner = EXCLUDED.owner,
    description = EXCLUDED.description,
    status = EXCLUDED.status,
    updated_at = now();

INSERT INTO external_dataset_versions (
  id,
  source_key,
  dataset_key,
  dataset_version,
  sample_grain,
  label_column,
  entity_keys,
  manifest_uri,
  schema_uri,
  profile_uri,
  storage_format,
  schema_hash,
  row_count,
  status
)
VALUES (
  '71000000-0000-0000-0000-000000000001',
  'demo_claims_lake',
  'demo_claims_fwa',
  '2026-05-demo',
  'claim',
  'confirmed_fwa',
  '["claim_id","member_id","provider_id"]'::jsonb,
  's3://fwa-demo/datasets/demo_claims_fwa/2026-05-demo/manifest.json',
  's3://fwa-demo/datasets/demo_claims_fwa/2026-05-demo/schema.json',
  's3://fwa-demo/datasets/demo_claims_fwa/2026-05-demo/profile.json',
  'parquet',
  'demo-schema-hash-202605',
  25000,
  'profiled'
)
ON CONFLICT (dataset_key, dataset_version) DO UPDATE
SET source_key = EXCLUDED.source_key,
    sample_grain = EXCLUDED.sample_grain,
    label_column = EXCLUDED.label_column,
    entity_keys = EXCLUDED.entity_keys,
    manifest_uri = EXCLUDED.manifest_uri,
    schema_uri = EXCLUDED.schema_uri,
    profile_uri = EXCLUDED.profile_uri,
    storage_format = EXCLUDED.storage_format,
    schema_hash = EXCLUDED.schema_hash,
    row_count = EXCLUDED.row_count,
    status = EXCLUDED.status;

INSERT INTO external_dataset_splits (
  dataset_id,
  split_name,
  data_uri,
  row_count,
  positive_count,
  negative_count,
  label_distribution_json
)
VALUES
  ('71000000-0000-0000-0000-000000000001', 'train', 's3://fwa-demo/datasets/demo_claims_fwa/2026-05-demo/train.parquet', 18000, 1450, 16550, '{"positive":1450,"negative":16550}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'validation', 's3://fwa-demo/datasets/demo_claims_fwa/2026-05-demo/validation.parquet', 3500, 280, 3220, '{"positive":280,"negative":3220}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'test', 's3://fwa-demo/datasets/demo_claims_fwa/2026-05-demo/test.parquet', 3500, 275, 3225, '{"positive":275,"negative":3225}'::jsonb)
ON CONFLICT (dataset_id, split_name) DO UPDATE
SET data_uri = EXCLUDED.data_uri,
    row_count = EXCLUDED.row_count,
    positive_count = EXCLUDED.positive_count,
    negative_count = EXCLUDED.negative_count,
    label_distribution_json = EXCLUDED.label_distribution_json;

INSERT INTO external_schema_fields (
  dataset_id,
  field_name,
  logical_type,
  nullable,
  semantic_role,
  description,
  profile_json
)
VALUES
  ('71000000-0000-0000-0000-000000000001', 'claim_id', 'string', false, 'key', 'External claim identifier for claim-grain lineage.', '{"owner":"data-ops","missing_rate":0.0,"evidence_refs":["datasets:demo_claims_fwa:2026-05-demo:claim_id"]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'member_id', 'string', false, 'key', 'External member identifier for member-level feature joins.', '{"owner":"data-ops","missing_rate":0.0,"evidence_refs":["datasets:demo_claims_fwa:2026-05-demo:member_id"]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'provider_id', 'string', false, 'key', 'External provider identifier for provider risk joins.', '{"owner":"data-ops","missing_rate":0.0,"evidence_refs":["datasets:demo_claims_fwa:2026-05-demo:provider_id"]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'claim_amount', 'decimal', false, 'measure', 'Submitted claim amount.', '{"owner":"data-ops","missing_rate":0.0,"p50":420,"p95":6800,"p99":15800,"top_values":[]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'days_since_policy_start', 'integer', false, 'feature', 'Days between policy start and service date.', '{"owner":"feature-ops","missing_rate":0.0,"p50":141,"p95":320,"p99":365,"top_values":[{"value":"0-7","count":920}]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'claim_amount_to_limit_ratio', 'decimal', false, 'feature', 'Claim amount divided by policy limit.', '{"owner":"feature-ops","missing_rate":0.0,"p50":0.08,"p95":0.62,"p99":0.93,"top_values":[]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'provider_high_cost_item_ratio_30d', 'decimal', true, 'feature', 'Provider share of high-cost items over 30 days.', '{"owner":"provider-ops","missing_rate":0.03,"p50":0.12,"p95":0.47,"p99":0.71,"top_values":[]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'confirmed_fwa', 'boolean', false, 'label', 'Confirmed FWA label from investigation or QA.', '{"owner":"model-ops","missing_rate":0.0,"top_values":[{"value":false,"count":23000},{"value":true,"count":2000}]}'::jsonb)
ON CONFLICT (dataset_id, field_name) DO UPDATE
SET logical_type = EXCLUDED.logical_type,
    nullable = EXCLUDED.nullable,
    semantic_role = EXCLUDED.semantic_role,
    description = EXCLUDED.description,
    profile_json = EXCLUDED.profile_json;

INSERT INTO external_field_mappings (
  id,
  dataset_id,
  external_field,
  canonical_target,
  feature_name,
  transform_kind,
  transform_json,
  status
)
VALUES
  ('72000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000001', 'claim_amount', 'claims.claim_amount', 'claim_amount', 'identity', '{}'::jsonb, 'active'),
  ('72000000-0000-0000-0000-000000000002', '71000000-0000-0000-0000-000000000001', 'days_since_policy_start', 'features.days_since_policy_start', 'days_since_policy_start', 'identity', '{}'::jsonb, 'active'),
  ('72000000-0000-0000-0000-000000000003', '71000000-0000-0000-0000-000000000001', 'claim_amount_to_limit_ratio', 'features.claim_amount_to_limit_ratio', 'claim_amount_to_limit_ratio', 'identity', '{}'::jsonb, 'active')
ON CONFLICT (id) DO UPDATE
SET external_field = EXCLUDED.external_field,
    canonical_target = EXCLUDED.canonical_target,
    feature_name = EXCLUDED.feature_name,
    transform_kind = EXCLUDED.transform_kind,
    transform_json = EXCLUDED.transform_json,
    status = EXCLUDED.status;

INSERT INTO feature_definitions (
  id,
  feature_name,
  business_domain,
  version,
  value_type,
  source_fields_json,
  calculation_kind,
  description,
  owner,
  status
)
VALUES
  ('73000000-0000-0000-0000-000000000001', 'days_since_policy_start', 'health_claims_fwa', 1, 'integer', '["coverage_start_date","service_date"]'::jsonb, 'runtime', 'Early policy-window risk factor.', 'feature-ops', 'active'),
  ('73000000-0000-0000-0000-000000000002', 'claim_amount_to_limit_ratio', 'health_claims_fwa', 1, 'decimal', '["claim_amount","coverage_limit_amount"]'::jsonb, 'runtime', 'Claim severity against covered limit.', 'feature-ops', 'active'),
  ('73000000-0000-0000-0000-000000000003', 'provider_high_cost_item_ratio_30d', 'health_claims_fwa', 1, 'decimal', '["provider_id","claim_items"]'::jsonb, 'batch_profile', 'Provider high-cost behavior profile.', 'feature-ops', 'candidate')
ON CONFLICT (feature_name, business_domain, version) DO UPDATE
SET value_type = EXCLUDED.value_type,
    source_fields_json = EXCLUDED.source_fields_json,
    calculation_kind = EXCLUDED.calculation_kind,
    description = EXCLUDED.description,
    owner = EXCLUDED.owner,
    status = EXCLUDED.status;

INSERT INTO feature_set_versions (
  id,
  feature_set_key,
  business_domain,
  version,
  dataset_id,
  features_uri,
  feature_list_json,
  row_count,
  label_column,
  status
)
VALUES (
  '74000000-0000-0000-0000-000000000001',
  'fwa_demo_factor_set',
  'health_claims_fwa',
  '2026-05-demo',
  '71000000-0000-0000-0000-000000000001',
  's3://fwa-demo/features/fwa_demo_factor_set/2026-05-demo/features.parquet',
  '["days_since_policy_start","claim_amount_to_limit_ratio","provider_high_cost_item_ratio_30d"]'::jsonb,
  25000,
  'confirmed_fwa',
  'active'
)
ON CONFLICT (feature_set_key, version) DO UPDATE
SET business_domain = EXCLUDED.business_domain,
    dataset_id = EXCLUDED.dataset_id,
    features_uri = EXCLUDED.features_uri,
    feature_list_json = EXCLUDED.feature_list_json,
    row_count = EXCLUDED.row_count,
    label_column = EXCLUDED.label_column,
    status = EXCLUDED.status;

INSERT INTO model_dataset_versions (
  id,
  business_domain,
  task_type,
  label_name,
  feature_set_id,
  train_uri,
  validation_uri,
  test_uri,
  row_counts_json,
  label_distribution_json,
  status
)
VALUES (
  '75000000-0000-0000-0000-000000000001',
  'health_claims_fwa',
  'binary_classification',
  'confirmed_fwa',
  '74000000-0000-0000-0000-000000000001',
  's3://fwa-demo/model-datasets/baseline_fwa/2026-05-demo/train.parquet',
  's3://fwa-demo/model-datasets/baseline_fwa/2026-05-demo/validation.parquet',
  's3://fwa-demo/model-datasets/baseline_fwa/2026-05-demo/test.parquet',
  '{"train":18000,"validation":3500,"test":3500}'::jsonb,
  '{"positive":2005,"negative":22995}'::jsonb,
  'active'
)
ON CONFLICT (id) DO UPDATE
SET business_domain = EXCLUDED.business_domain,
    task_type = EXCLUDED.task_type,
    label_name = EXCLUDED.label_name,
    feature_set_id = EXCLUDED.feature_set_id,
    train_uri = EXCLUDED.train_uri,
    validation_uri = EXCLUDED.validation_uri,
    test_uri = EXCLUDED.test_uri,
    row_counts_json = EXCLUDED.row_counts_json,
    label_distribution_json = EXCLUDED.label_distribution_json,
    status = EXCLUDED.status;

INSERT INTO model_evaluation_runs (
  evaluation_run_id,
  model_key,
  model_version,
  model_dataset_id,
  scheme_family,
  auc,
  ks,
  precision_value,
  recall_value,
  f1,
  accuracy,
  threshold,
  confusion_matrix_json,
  feature_importance_uri,
  metrics_json
)
VALUES (
  'eval-baseline-fwa-2026-05-demo',
  'baseline_fwa',
  '0.1.0',
  '75000000-0000-0000-0000-000000000001',
  'diagnosis_procedure_mismatch',
  0.812,
  0.421,
  0.72,
  0.66,
  0.689,
  0.914,
  0.70,
  '{"tp":181,"fp":70,"tn":3155,"fn":94}'::jsonb,
  's3://fwa-demo/model-evaluations/baseline_fwa/2026-05-demo/feature_importance.parquet',
  '{
    "psi": 0.08,
    "score_psi": 0.08,
    "lift_at_5_pct": 4.8,
    "false_positive_rate": 0.14,
    "out_of_time_auc": 0.804,
    "out_of_time_precision": 0.70,
    "review_capacity_threshold_status": "passed",
    "leakage_check_status": "passed",
    "time_group_split_status": "passed",
    "time_split_field": "service_date",
    "group_split_fields": ["member_id", "policy_id", "provider_id"],
    "shadow_comparison_status": "passed",
    "serving_version_lock_status": "passed",
    "artifact_integrity_status": "passed",
    "feature_store_materialization_status": "passed",
    "rust_feature_set_status": "passed",
    "rust_feature_set_manifest_uri": "s3://fwa-demo/features/fwa_demo_factor_set/2026-05-demo/rust_feature_set_manifest.json",
    "segment_fairness_status": "passed",
    "serving_manifest_uri": "s3://fwa-demo/model-evaluations/baseline_fwa/2026-05-demo/serving_manifest.json",
    "model_artifact_evaluation_status": "passed",
    "model_artifact_evaluation_report_uri": "s3://fwa-demo/model-evaluations/baseline_fwa/2026-05-demo/rust_serving_artifact_evaluation.json",
    "rust_serving_status": "passed",
    "rust_serving_latency_status": "passed",
    "rust_serving_p95_latency_ms": 18,
    "rust_serving_latency_measurement_kind": "benchmark",
    "rust_serving_latency_sample_count": 1000,
    "feature_reproducibility_hash": "sha256:demo-baseline-feature-reproducibility",
    "label_provenance_status": "passed",
    "label_reviewer_source": "investigation_results",
    "pilot_validation_status": "passed",
    "approval_status": "approved"
  }'::jsonb
)
ON CONFLICT (evaluation_run_id) DO UPDATE
SET model_key = EXCLUDED.model_key,
    model_version = EXCLUDED.model_version,
    model_dataset_id = EXCLUDED.model_dataset_id,
    scheme_family = EXCLUDED.scheme_family,
    auc = EXCLUDED.auc,
    ks = EXCLUDED.ks,
    precision_value = EXCLUDED.precision_value,
    recall_value = EXCLUDED.recall_value,
    f1 = EXCLUDED.f1,
    accuracy = EXCLUDED.accuracy,
    threshold = EXCLUDED.threshold,
    confusion_matrix_json = EXCLUDED.confusion_matrix_json,
    feature_importance_uri = EXCLUDED.feature_importance_uri,
    metrics_json = EXCLUDED.metrics_json;

INSERT INTO scoring_runs (
  id,
  run_id,
  claim_id,
  source_system,
  actor_id,
  status,
  risk_score,
  rag,
  recommended_action,
  completed_at
)
VALUES
  (
    '80000000-0000-0000-0000-000000000001',
    'run-demo-historical-9100',
    '40000000-0000-0000-0000-000000000900',
    'tpa-demo',
    'seed',
    'completed',
    82,
    'Red',
    'ManualReview',
    now()
  ),
  (
    '80000000-0000-0000-0000-000000000287',
    'run-demo-scope-0287',
    '40000000-0000-0000-0000-000000000287',
    'tpa-demo',
    'seed',
    'seeded_scope',
    NULL,
    NULL,
    NULL,
    now()
  ),
  (
    '80000000-0000-0000-0000-000000001001',
    'run-demo-mlops-monitoring',
    '40000000-0000-0000-0000-000000000287',
    'tpa-demo',
    'mlops-worker',
    'completed',
    NULL,
    NULL,
    NULL,
    now()
  ),
  (
    '80000000-0000-0000-0000-000000001002',
    'run-demo-mlops-alert-delivery',
    '40000000-0000-0000-0000-000000000287',
    'tpa-demo',
    'mlops-worker',
    'completed',
    NULL,
    NULL,
    NULL,
    now()
  )
ON CONFLICT (run_id) DO UPDATE
SET claim_id = EXCLUDED.claim_id,
    source_system = EXCLUDED.source_system,
    actor_id = EXCLUDED.actor_id,
    status = EXCLUDED.status,
    risk_score = EXCLUDED.risk_score,
    rag = EXCLUDED.rag,
    recommended_action = EXCLUDED.recommended_action,
    completed_at = EXCLUDED.completed_at;

DELETE FROM feature_values WHERE run_id = 'run-demo-historical-9100';
DELETE FROM rule_runs WHERE run_id = 'run-demo-historical-9100';
DELETE FROM model_scores WHERE run_id = 'run-demo-historical-9100';

INSERT INTO feature_values (
  run_id,
  claim_id,
  feature_name,
  feature_version,
  value_json,
  evidence_json
)
VALUES
  ('run-demo-historical-9100', '40000000-0000-0000-0000-000000000900', 'days_since_policy_start', 1, '{"value":42}'::jsonb, '{"source":"seed_demo"}'::jsonb),
  ('run-demo-historical-9100', '40000000-0000-0000-0000-000000000900', 'provider_high_cost_item_ratio_30d', 1, '{"value":0.71}'::jsonb, '{"source":"seed_demo"}'::jsonb);

INSERT INTO rule_runs (
  run_id,
  rule_id,
  rule_version_id,
  matched,
  score_contribution,
  alert_code,
  reason,
  evidence_json
)
VALUES (
  'run-demo-historical-9100',
  (SELECT id FROM rules WHERE rule_key = 'rule_high_amount_to_limit'),
  (SELECT rv.id FROM rule_versions rv JOIN rules r ON r.id = rv.rule_id WHERE r.rule_key = 'rule_high_amount_to_limit' AND rv.version = 1),
  true,
  30,
  'HIGH_AMOUNT_TO_LIMIT',
  'Claim amount consumes a high share of policy limit',
  '["feature_values:claim_amount_to_limit_ratio"]'::jsonb
);

INSERT INTO model_scores (
  run_id,
  model_version_id,
  model_key,
  runtime_kind,
  execution_provider,
  score,
  label,
  explanation_json,
  latency_ms
)
VALUES (
  'run-demo-historical-9100',
  (SELECT id FROM model_versions WHERE model_key = 'baseline_fwa' AND version = '0.1.0'),
  'baseline_fwa',
  'python_http',
  'cpu',
  79,
  'high_risk',
  '{"top_features":["provider_high_cost_item_ratio_30d","claim_amount_to_limit_ratio"]}'::jsonb,
  18
);

INSERT INTO audit_events (
  audit_id,
  run_id,
  claim_id,
  actor_id,
  actor_role,
  source_system,
  event_type,
  event_status,
  summary,
  payload,
  evidence_refs
)
VALUES
  (
    'audit-demo-scope-0287',
    'run-demo-scope-0287',
    '40000000-0000-0000-0000-000000000287',
    'seed',
    'system',
    'tpa-demo',
    'claim.seeded',
    'completed',
    'Seeded demo claim assigned to demo customer scope.',
    '{"customer_scope_id":"demo-customer","seed_scope":true}'::jsonb,
    '["claims:CLM-0287"]'::jsonb
  ),
  (
    'audit-demo-score-9100',
    'run-demo-historical-9100',
    '40000000-0000-0000-0000-000000000900',
    'seed',
    'system',
    'tpa-demo',
    'claim.scored',
    'completed',
    'Seeded historical claim scored as Red.',
    '{"customer_scope_id":"demo-customer","risk_score":82,"rag":"Red","recommended_action":"ManualReview"}'::jsonb,
    '["scoring_runs:run-demo-historical-9100"]'::jsonb
  ),
  (
    'audit-demo-qa-9100',
    'run-demo-historical-9100',
    '40000000-0000-0000-0000-000000000900',
    'seed',
    'qa',
    'tpa-demo',
    'qa.result.received',
    'completed',
    'Seeded QA review found provider-pattern issue.',
    '{"customer_scope_id":"demo-customer","qa_case_id":"QA-9100","feedback_target":"rules"}'::jsonb,
    '["qa_reviews:QA-9100","rule_runs:HIGH_AMOUNT_TO_LIMIT"]'::jsonb
  ),
  (
    'audit-demo-mlops-monitoring',
    'run-demo-mlops-monitoring',
    '40000000-0000-0000-0000-000000000287',
    'seed',
    'system',
    'tpa-demo',
    'model.mlops_monitoring.report_submitted',
    'succeeded',
    'Seeded MLOps monitoring report opened drift review.',
    '{
      "customer_scope_id":"demo-customer",
      "model_key":"baseline_fwa",
      "model_version":"0.1.0",
      "report_uri":"data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json",
      "report_kind":"mlops_monitoring_report",
      "monitoring_status":"watch",
      "retraining_recommendation":"prepare_retraining",
      "triggers":["model_drift_detected"],
      "trigger_count":1,
      "review_tasks":[{"task_kind":"mlops_monitoring_review","trigger":"model_drift_detected","review_status":"open"}],
      "review_task_count":1,
      "next_actions":["prepare_retraining_job_after_human_approval","review_monitoring_report"],
      "submitted_by":"mlops-worker",
      "note_present":true,
      "governance_boundary":"seeded monitoring report opens review only; it must not auto-create retraining jobs, activate models, or rollback models"
    }'::jsonb,
    '["model_versions:baseline_fwa:0.1.0","model_monitoring_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/mlops_monitoring_report.json"]'::jsonb
  ),
  (
    'audit-demo-mlops-alert-delivery',
    'run-demo-mlops-alert-delivery',
    '40000000-0000-0000-0000-000000000287',
    'seed',
    'system',
    'tpa-demo',
    'model.mlops_alert_delivery.submitted',
    'succeeded',
    'Seeded MLOps alert delivery queued customer alert-router confirmation.',
    '{
      "customer_scope_id":"demo-customer",
      "model_key":"baseline_fwa",
      "model_version":"0.1.0",
      "scheduler_execution_report_uri":"data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json",
      "report_kind":"mlops_scheduler_execution_report",
      "alert_delivery_status":"queued_for_external_alert_router",
      "alert_delivery_tasks":[{"task_kind":"mlops_alert_delivery","trigger":"model_drift_detected","route_key":"mlops_retraining_readiness","delivery_status":"queued_for_external_alert_router"}],
      "alert_delivery_task_count":1,
      "alert_routing_policy_configured":true,
      "alert_routing_policy_ref":"configured_alert_routing_policy",
      "submitted_by":"mlops-worker",
      "note_present":true,
      "governance_boundary":"seeded alert delivery records handoff only; it must not create retraining jobs, activate models, rollback models, or assign fraud labels"
    }'::jsonb,
    '["model_versions:baseline_fwa:0.1.0","mlops_scheduler_execution_reports:data/model-artifacts/baseline_fwa/0.1.0/mlops-monitoring/scheduler/mlops_scheduler_execution_report.json"]'::jsonb
  )
ON CONFLICT (audit_id) DO UPDATE
SET run_id = EXCLUDED.run_id,
    claim_id = EXCLUDED.claim_id,
    actor_id = EXCLUDED.actor_id,
    actor_role = EXCLUDED.actor_role,
    source_system = EXCLUDED.source_system,
    event_type = EXCLUDED.event_type,
    event_status = EXCLUDED.event_status,
    summary = EXCLUDED.summary,
    payload = EXCLUDED.payload,
    evidence_refs = EXCLUDED.evidence_refs;

INSERT INTO investigation_results (
  investigation_id,
  claim_id,
  outcome,
  confirmed_fwa,
  saving_amount,
  currency,
  notes,
  evidence_refs
)
VALUES (
  'INV-9100',
  'CLM-9100',
  'confirmed_fwa',
  true,
  12600,
  'CNY',
  'Seeded historical investigation confirmed provider high-cost package pattern.',
  '["audit:audit-demo-score-9100","knowledge_cases:KC-1002"]'::jsonb
)
ON CONFLICT (investigation_id) DO UPDATE
SET claim_id = EXCLUDED.claim_id,
    outcome = EXCLUDED.outcome,
    confirmed_fwa = EXCLUDED.confirmed_fwa,
    saving_amount = EXCLUDED.saving_amount,
    currency = EXCLUDED.currency,
    notes = EXCLUDED.notes,
    evidence_refs = EXCLUDED.evidence_refs;

INSERT INTO qa_reviews (
  qa_case_id,
  claim_id,
  qa_conclusion,
  issue_type,
  feedback_target,
  notes,
  evidence_refs
)
VALUES (
  'QA-9100',
  'CLM-9100',
  'issue_found_escalate',
  'provider_pattern',
  'rules',
  'Seeded QA review asks rule ops to monitor provider high-cost packages.',
  '["audit:audit-demo-qa-9100","rule_runs:HIGH_AMOUNT_TO_LIMIT"]'::jsonb
)
ON CONFLICT (qa_case_id) DO UPDATE
SET claim_id = EXCLUDED.claim_id,
    qa_conclusion = EXCLUDED.qa_conclusion,
    issue_type = EXCLUDED.issue_type,
    feedback_target = EXCLUDED.feedback_target,
    notes = EXCLUDED.notes,
    evidence_refs = EXCLUDED.evidence_refs;
