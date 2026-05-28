CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS members (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  external_member_id TEXT NOT NULL UNIQUE,
  name_hash TEXT,
  dob DATE,
  gender TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS policies (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  external_policy_id TEXT NOT NULL UNIQUE,
  member_id UUID NOT NULL REFERENCES members(id),
  product_code TEXT NOT NULL,
  coverage_start_date DATE NOT NULL,
  coverage_end_date DATE NOT NULL,
  coverage_limit_amount NUMERIC NOT NULL,
  currency TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS providers (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  external_provider_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  provider_type TEXT NOT NULL,
  region TEXT NOT NULL,
  risk_tier TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS claims (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  external_claim_id TEXT NOT NULL UNIQUE,
  member_id UUID NOT NULL REFERENCES members(id),
  policy_id UUID NOT NULL REFERENCES policies(id),
  provider_id UUID NOT NULL REFERENCES providers(id),
  claim_type TEXT NOT NULL,
  diagnosis_code TEXT NOT NULL,
  service_date DATE NOT NULL,
  claim_amount NUMERIC NOT NULL,
  currency TEXT NOT NULL,
  status TEXT NOT NULL,
  raw_payload JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS claim_items (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  claim_id UUID NOT NULL REFERENCES claims(id) ON DELETE CASCADE,
  item_code TEXT NOT NULL,
  item_type TEXT NOT NULL,
  description TEXT NOT NULL,
  quantity INTEGER NOT NULL,
  unit_amount NUMERIC NOT NULL,
  total_amount NUMERIC NOT NULL,
  currency TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS rules (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  rule_key TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  owner TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS rule_versions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  rule_id UUID NOT NULL REFERENCES rules(id),
  version INTEGER NOT NULL,
  dsl JSONB NOT NULL,
  score INTEGER NOT NULL,
  recommended_action TEXT NOT NULL,
  created_by TEXT NOT NULL,
  approved_by TEXT,
  published_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(rule_id, version)
);

CREATE TABLE IF NOT EXISTS model_versions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  model_key TEXT NOT NULL,
  version TEXT NOT NULL,
  model_type TEXT NOT NULL,
  runtime_kind TEXT NOT NULL,
  artifact_uri TEXT,
  endpoint_url TEXT,
  execution_provider TEXT NOT NULL,
  status TEXT NOT NULL,
  metrics JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  activated_at TIMESTAMPTZ,
  UNIQUE(model_key, version)
);

CREATE TABLE IF NOT EXISTS scoring_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL UNIQUE,
  claim_id UUID REFERENCES claims(id),
  source_system TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  status TEXT NOT NULL,
  risk_score INTEGER,
  rag TEXT,
  risk_level TEXT,
  recommended_action TEXT,
  confidence_score INTEGER,
  confidence TEXT,
  routing_reason TEXT,
  score_breakdown JSONB NOT NULL DEFAULT '{}'::jsonb,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  error_code TEXT,
  error_message TEXT
);

ALTER TABLE scoring_runs ADD COLUMN IF NOT EXISTS risk_level TEXT;
ALTER TABLE scoring_runs ADD COLUMN IF NOT EXISTS confidence_score INTEGER;
ALTER TABLE scoring_runs ADD COLUMN IF NOT EXISTS confidence TEXT;
ALTER TABLE scoring_runs ADD COLUMN IF NOT EXISTS routing_reason TEXT;
ALTER TABLE scoring_runs ADD COLUMN IF NOT EXISTS score_breakdown JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE IF NOT EXISTS feature_values (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  claim_id UUID REFERENCES claims(id),
  feature_name TEXT NOT NULL,
  feature_version INTEGER NOT NULL,
  value_json JSONB NOT NULL,
  evidence_json JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS rule_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  rule_id UUID REFERENCES rules(id),
  rule_version_id UUID REFERENCES rule_versions(id),
  matched BOOLEAN NOT NULL,
  score_contribution INTEGER NOT NULL,
  alert_code TEXT,
  reason TEXT,
  evidence_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS model_scores (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  model_version_id UUID REFERENCES model_versions(id),
  model_key TEXT NOT NULL,
  runtime_kind TEXT NOT NULL,
  execution_provider TEXT NOT NULL,
  score INTEGER NOT NULL,
  label TEXT NOT NULL,
  explanation_json JSONB NOT NULL,
  latency_ms INTEGER NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit_events (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  audit_id TEXT NOT NULL UNIQUE,
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  claim_id UUID REFERENCES claims(id),
  actor_id TEXT NOT NULL,
  actor_role TEXT NOT NULL,
  source_system TEXT NOT NULL,
  event_type TEXT NOT NULL,
  event_status TEXT NOT NULL,
  summary TEXT NOT NULL,
  payload JSONB NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS knowledge_cases (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  case_id TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  fwa_type TEXT NOT NULL,
  diagnosis_code TEXT NOT NULL,
  provider_region TEXT NOT NULL,
  provider_type TEXT NOT NULL,
  summary TEXT NOT NULL,
  outcome TEXT NOT NULL,
  tags JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS agent_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  agent_run_id TEXT NOT NULL UNIQUE,
  claim_id TEXT NOT NULL,
  status TEXT NOT NULL,
  decision_boundary TEXT NOT NULL,
  output_json JSONB NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS agent_steps (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  agent_run_id TEXT NOT NULL REFERENCES agent_runs(agent_run_id) ON DELETE CASCADE,
  step_name TEXT NOT NULL,
  status TEXT NOT NULL,
  output_json JSONB NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS agent_context_snapshots (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  snapshot_id TEXT NOT NULL UNIQUE,
  agent_run_id TEXT NOT NULL REFERENCES agent_runs(agent_run_id) ON DELETE CASCADE,
  redaction_status TEXT NOT NULL,
  context_json JSONB NOT NULL,
  source_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  checksum TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS tool_calls (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tool_call_id TEXT NOT NULL UNIQUE,
  agent_run_id TEXT NOT NULL REFERENCES agent_runs(agent_run_id) ON DELETE CASCADE,
  tool_name TEXT NOT NULL,
  status TEXT NOT NULL,
  input_json JSONB NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS agent_policy_checks (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  policy_check_id TEXT NOT NULL UNIQUE,
  agent_run_id TEXT NOT NULL REFERENCES agent_runs(agent_run_id) ON DELETE CASCADE,
  tool_call_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  policy_name TEXT NOT NULL,
  decision TEXT NOT NULL,
  reason TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS tool_results (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tool_result_id TEXT NOT NULL UNIQUE,
  tool_call_id TEXT NOT NULL REFERENCES tool_calls(tool_call_id) ON DELETE CASCADE,
  agent_run_id TEXT NOT NULL REFERENCES agent_runs(agent_run_id) ON DELETE CASCADE,
  tool_name TEXT NOT NULL,
  status TEXT NOT NULL,
  output_json JSONB NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS agent_approvals (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  approval_id TEXT NOT NULL UNIQUE,
  agent_run_id TEXT NOT NULL REFERENCES agent_runs(agent_run_id) ON DELETE CASCADE,
  proposed_action TEXT NOT NULL,
  decision TEXT NOT NULL,
  approver TEXT NOT NULL,
  reason TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS fwa_leads (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  lead_id TEXT NOT NULL UNIQUE,
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  claim_id TEXT NOT NULL,
  member_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  source_system TEXT NOT NULL,
  scheme_family TEXT NOT NULL,
  lead_source TEXT NOT NULL,
  status TEXT NOT NULL,
  disposition TEXT NOT NULL,
  risk_score INTEGER NOT NULL,
  rag TEXT NOT NULL,
  reason TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS investigation_cases (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  case_id TEXT NOT NULL UNIQUE,
  lead_id TEXT NOT NULL REFERENCES fwa_leads(lead_id),
  claim_id TEXT NOT NULL,
  member_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  source_system TEXT NOT NULL,
  scheme_family TEXT NOT NULL,
  lead_source TEXT NOT NULL,
  status TEXT NOT NULL,
  assignee TEXT NOT NULL,
  reviewer TEXT NOT NULL,
  priority TEXT NOT NULL,
  routing_reason TEXT NOT NULL,
  evidence_package_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit_samples (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  sample_id TEXT NOT NULL UNIQUE,
  sample_mode TEXT NOT NULL,
  population_definition TEXT NOT NULL,
  inclusion_criteria_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  deterministic_seed TEXT,
  selection_method TEXT NOT NULL,
  sample_size INTEGER NOT NULL,
  reviewer TEXT NOT NULL,
  assignment_queue TEXT NOT NULL,
  selected_leads_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  outcome_distribution_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS external_data_sources (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  source_key TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  business_domain TEXT NOT NULL,
  owner TEXT NOT NULL,
  description TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS external_dataset_versions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  source_key TEXT NOT NULL REFERENCES external_data_sources(source_key),
  dataset_key TEXT NOT NULL,
  dataset_version TEXT NOT NULL,
  sample_grain TEXT NOT NULL,
  label_column TEXT NOT NULL,
  entity_keys JSONB NOT NULL DEFAULT '[]'::jsonb,
  manifest_uri TEXT NOT NULL,
  schema_uri TEXT NOT NULL,
  profile_uri TEXT NOT NULL,
  storage_format TEXT NOT NULL CHECK (storage_format = 'parquet'),
  schema_hash TEXT NOT NULL,
  row_count BIGINT NOT NULL,
  status TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(dataset_key, dataset_version)
);

CREATE TABLE IF NOT EXISTS external_dataset_splits (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  dataset_id UUID NOT NULL REFERENCES external_dataset_versions(id) ON DELETE CASCADE,
  split_name TEXT NOT NULL,
  data_uri TEXT NOT NULL,
  row_count BIGINT NOT NULL,
  positive_count BIGINT,
  negative_count BIGINT,
  label_distribution_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  UNIQUE(dataset_id, split_name)
);

CREATE TABLE IF NOT EXISTS external_schema_fields (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  dataset_id UUID NOT NULL REFERENCES external_dataset_versions(id) ON DELETE CASCADE,
  field_name TEXT NOT NULL,
  logical_type TEXT NOT NULL,
  nullable BOOLEAN NOT NULL,
  semantic_role TEXT NOT NULL,
  description TEXT NOT NULL,
  profile_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  UNIQUE(dataset_id, field_name)
);

CREATE TABLE IF NOT EXISTS external_field_mappings (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  dataset_id UUID NOT NULL REFERENCES external_dataset_versions(id) ON DELETE CASCADE,
  external_field TEXT NOT NULL,
  canonical_target TEXT NOT NULL,
  feature_name TEXT,
  transform_kind TEXT NOT NULL,
  transform_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  status TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS feature_definitions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  feature_name TEXT NOT NULL,
  business_domain TEXT NOT NULL,
  version INTEGER NOT NULL,
  value_type TEXT NOT NULL,
  source_fields_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  calculation_kind TEXT NOT NULL,
  description TEXT NOT NULL,
  owner TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(feature_name, business_domain, version)
);

CREATE TABLE IF NOT EXISTS feature_set_versions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  feature_set_key TEXT NOT NULL,
  business_domain TEXT NOT NULL,
  version TEXT NOT NULL,
  dataset_id UUID NOT NULL REFERENCES external_dataset_versions(id),
  features_uri TEXT NOT NULL,
  feature_list_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  row_count BIGINT NOT NULL,
  label_column TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(feature_set_key, version)
);

CREATE TABLE IF NOT EXISTS model_dataset_versions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  business_domain TEXT NOT NULL,
  task_type TEXT NOT NULL,
  label_name TEXT NOT NULL,
  feature_set_id UUID NOT NULL REFERENCES feature_set_versions(id),
  train_uri TEXT NOT NULL,
  validation_uri TEXT NOT NULL,
  test_uri TEXT,
  row_counts_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  label_distribution_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  status TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS model_evaluation_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  evaluation_run_id TEXT NOT NULL UNIQUE,
  model_key TEXT NOT NULL,
  model_version TEXT NOT NULL,
  model_dataset_id UUID NOT NULL REFERENCES model_dataset_versions(id),
  auc NUMERIC,
  ks NUMERIC,
  precision_value NUMERIC,
  recall_value NUMERIC,
  f1 NUMERIC,
  accuracy NUMERIC,
  threshold NUMERIC,
  confusion_matrix_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  feature_importance_uri TEXT,
  metrics_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS model_promotion_reviews (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  model_key TEXT NOT NULL,
  model_version TEXT NOT NULL,
  decision TEXT NOT NULL CHECK (decision IN ('approved', 'rejected')),
  reviewer TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS rule_promotion_reviews (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  rule_id TEXT NOT NULL,
  rule_version INTEGER NOT NULL,
  decision TEXT NOT NULL CHECK (decision IN ('approved', 'rejected')),
  reviewer TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS rule_backtest_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  rule_id TEXT NOT NULL,
  rule_version INTEGER NOT NULL,
  sample_count INTEGER NOT NULL,
  matched_count INTEGER NOT NULL,
  reviewed_count INTEGER NOT NULL,
  confirmed_fwa_count INTEGER NOT NULL,
  false_positive_count INTEGER NOT NULL,
  precision_value DOUBLE PRECISION NOT NULL,
  recall_value DOUBLE PRECISION NOT NULL,
  lift DOUBLE PRECISION NOT NULL,
  false_positive_rate DOUBLE PRECISION NOT NULL,
  estimated_saving TEXT NOT NULL,
  promotion_recommendation TEXT NOT NULL,
  blockers JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS investigation_results (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  investigation_id TEXT NOT NULL UNIQUE,
  claim_id TEXT NOT NULL,
  outcome TEXT NOT NULL,
  confirmed_fwa BOOLEAN NOT NULL,
  saving_amount NUMERIC,
  currency TEXT,
  notes TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS saving_attributions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  attribution_id TEXT NOT NULL UNIQUE,
  claim_id TEXT NOT NULL,
  investigation_id TEXT NOT NULL,
  source_type TEXT NOT NULL,
  source_id TEXT NOT NULL,
  action TEXT NOT NULL,
  saving_amount NUMERIC NOT NULL,
  currency TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS qa_reviews (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  qa_case_id TEXT NOT NULL UNIQUE,
  claim_id TEXT NOT NULL,
  qa_conclusion TEXT NOT NULL,
  issue_type TEXT NOT NULL,
  feedback_target TEXT NOT NULL,
  notes TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
