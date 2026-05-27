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
  ('50000000-0000-0000-0000-000000000002', 'rule_high_amount_to_limit', 'High amount to policy limit', 'active', 'rules-ops')
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
    '{"conditions":[{"field":"days_since_policy_start","operator":"<=","value":7}],"action":{"score":75,"alert_code":"EARLY_CLAIM","recommended_action":"ManualReview","reason":"Policy start within 7 days"}}'::jsonb,
    75,
    'ManualReview',
    'seed',
    'seed',
    now()
  ),
  (
    (SELECT id FROM rules WHERE rule_key = 'rule_high_amount_to_limit'),
    1,
    '{"conditions":[{"field":"claim_amount_to_limit_ratio","operator":">=","value":0.75}],"action":{"score":30,"alert_code":"HIGH_AMOUNT_TO_LIMIT","recommended_action":"ManualReview","reason":"Claim amount consumes a high share of policy limit"}}'::jsonb,
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
  ('71000000-0000-0000-0000-000000000001', 'claim_amount', 'decimal', false, 'measure', 'Submitted claim amount.', '{"missing_rate":0.0,"p50":420,"p95":6800,"p99":15800,"top_values":[]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'days_since_policy_start', 'integer', false, 'feature', 'Days between policy start and service date.', '{"missing_rate":0.0,"p50":141,"p95":320,"p99":365,"top_values":[{"value":"0-7","count":920}]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'claim_amount_to_limit_ratio', 'decimal', false, 'feature', 'Claim amount divided by policy limit.', '{"missing_rate":0.0,"p50":0.08,"p95":0.62,"p99":0.93,"top_values":[]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'provider_high_cost_item_ratio_30d', 'decimal', true, 'feature', 'Provider share of high-cost items over 30 days.', '{"missing_rate":0.03,"p50":0.12,"p95":0.47,"p99":0.71,"top_values":[]}'::jsonb),
  ('71000000-0000-0000-0000-000000000001', 'confirmed_fwa', 'boolean', false, 'label', 'Confirmed FWA label from investigation or QA.', '{"missing_rate":0.0,"top_values":[{"value":false,"count":23000},{"value":true,"count":2000}]}'::jsonb)
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
  0.812,
  0.421,
  0.72,
  0.66,
  0.689,
  0.914,
  0.70,
  '{"tp":181,"fp":70,"tn":3155,"fn":94}'::jsonb,
  's3://fwa-demo/model-evaluations/baseline_fwa/2026-05-demo/feature_importance.json',
  '{"psi":0.08,"lift_at_5_pct":4.8,"false_positive_rate":0.14}'::jsonb
)
ON CONFLICT (evaluation_run_id) DO UPDATE
SET model_key = EXCLUDED.model_key,
    model_version = EXCLUDED.model_version,
    model_dataset_id = EXCLUDED.model_dataset_id,
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
VALUES (
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
    'audit-demo-score-9100',
    'run-demo-historical-9100',
    '40000000-0000-0000-0000-000000000900',
    'seed',
    'system',
    'tpa-demo',
    'claim.scored',
    'completed',
    'Seeded historical claim scored as Red.',
    '{"risk_score":82,"rag":"Red","recommended_action":"ManualReview"}'::jsonb,
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
    '{"qa_case_id":"QA-9100","feedback_target":"rules"}'::jsonb,
    '["qa_reviews:QA-9100","rule_runs:HIGH_AMOUNT_TO_LIMIT"]'::jsonb
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
