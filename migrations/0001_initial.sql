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

CREATE TABLE IF NOT EXISTS inbox_claim_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL UNIQUE,
  audit_id TEXT NOT NULL UNIQUE,
  external_message_id TEXT,
  idempotency_key TEXT,
  external_message_fingerprint TEXT,
  raw_payload_checksum TEXT NOT NULL,
  raw_payload_ref TEXT,
  mapping_version TEXT NOT NULL,
  validation_result TEXT NOT NULL,
  scoring_ready BOOLEAN NOT NULL,
  claim_id TEXT NOT NULL,
  source_system TEXT NOT NULL,
  customer_scope_id TEXT NOT NULL,
  canonical_claim_context JSONB NOT NULL DEFAULT '{}'::jsonb,
  validation_errors JSONB NOT NULL DEFAULT '[]'::jsonb,
  data_quality_signals JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS external_message_id TEXT;
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS idempotency_key TEXT;
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS external_message_fingerprint TEXT;
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS raw_payload_checksum TEXT NOT NULL DEFAULT '';
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS raw_payload_ref TEXT;
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS mapping_version TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS validation_result TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS scoring_ready BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS claim_id TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS source_system TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS customer_scope_id TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS canonical_claim_context JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS validation_errors JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS data_quality_signals JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE inbox_claim_runs ADD COLUMN IF NOT EXISTS evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE UNIQUE INDEX IF NOT EXISTS inbox_claim_runs_run_id_idx
  ON inbox_claim_runs(run_id);
CREATE UNIQUE INDEX IF NOT EXISTS inbox_claim_runs_audit_id_idx
  ON inbox_claim_runs(audit_id);
CREATE UNIQUE INDEX IF NOT EXISTS inbox_claim_runs_idempotency_key_idx
  ON inbox_claim_runs(idempotency_key)
  WHERE idempotency_key IS NOT NULL;

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
  submitted_by_actor_id TEXT,
  hit_rate_7d FLOAT,
  hit_rate_90d FLOAT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE rules ADD COLUMN IF NOT EXISTS submitted_by_actor_id TEXT;
ALTER TABLE rules ADD COLUMN IF NOT EXISTS hit_rate_7d FLOAT;
ALTER TABLE rules ADD COLUMN IF NOT EXISTS hit_rate_90d FLOAT;

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

CREATE TABLE IF NOT EXISTS rule_condition_library (
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
  routing_policy JSONB NOT NULL DEFAULT '{}'::jsonb,
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
ALTER TABLE scoring_runs ADD COLUMN IF NOT EXISTS routing_policy JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE scoring_runs ADD COLUMN IF NOT EXISTS score_breakdown JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE IF NOT EXISTS routing_policies (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  policy_key TEXT NOT NULL,
  version INTEGER NOT NULL,
  review_mode TEXT NOT NULL,
  status TEXT NOT NULL,
  owner TEXT NOT NULL,
  policy_json JSONB NOT NULL,
  activated_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(policy_key, version, review_mode)
);

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

CREATE TABLE IF NOT EXISTS webhook_delivery_attempts (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  event_id TEXT NOT NULL,
  attempt_number INTEGER NOT NULL,
  delivery_status TEXT NOT NULL,
  response_status_code INTEGER,
  error_message TEXT,
  next_attempt_at TIMESTAMPTZ,
  attempted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(event_id, attempt_number)
);

CREATE TABLE IF NOT EXISTS knowledge_cases (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  case_id TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  fwa_type TEXT NOT NULL,
  scheme_family TEXT NOT NULL DEFAULT 'high_risk_claim',
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

ALTER TABLE knowledge_cases
  ADD COLUMN IF NOT EXISTS scheme_family TEXT NOT NULL DEFAULT 'high_risk_claim';

CREATE TABLE IF NOT EXISTS evidence_documents (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  document_id TEXT NOT NULL UNIQUE,
  customer_scope_id TEXT NOT NULL,
  source_system TEXT NOT NULL,
  source_record_ref TEXT NOT NULL,
  claim_id UUID REFERENCES claims(id),
  external_document_id TEXT,
  document_type TEXT NOT NULL,
  storage_uri TEXT NOT NULL,
  content_checksum TEXT NOT NULL,
  ingestion_status TEXT NOT NULL,
  redaction_status TEXT NOT NULL,
  retention_policy_id TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS evidence_document_chunks (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  chunk_id TEXT NOT NULL UNIQUE,
  document_id TEXT NOT NULL REFERENCES evidence_documents(document_id) ON DELETE CASCADE,
  chunk_index INTEGER NOT NULL,
  chunking_version TEXT NOT NULL,
  redaction_status TEXT NOT NULL,
  text_checksum TEXT NOT NULL,
  token_count INTEGER NOT NULL,
  storage_uri TEXT NOT NULL,
  source_offsets_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(document_id, chunk_index, chunking_version)
);

CREATE TABLE IF NOT EXISTS evidence_ocr_outputs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  ocr_output_id TEXT NOT NULL UNIQUE,
  document_id TEXT NOT NULL REFERENCES evidence_documents(document_id) ON DELETE CASCADE,
  ocr_engine TEXT NOT NULL,
  ocr_engine_version TEXT NOT NULL,
  output_uri TEXT NOT NULL,
  output_checksum TEXT NOT NULL,
  confidence_score NUMERIC,
  quality_status TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS evidence_redaction_reviews (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  redaction_review_id TEXT NOT NULL UNIQUE,
  document_id TEXT REFERENCES evidence_documents(document_id) ON DELETE CASCADE,
  chunk_id TEXT REFERENCES evidence_document_chunks(chunk_id) ON DELETE CASCADE,
  redaction_policy_id TEXT NOT NULL,
  redaction_status TEXT NOT NULL,
  reviewer TEXT NOT NULL,
  review_notes TEXT NOT NULL,
  before_checksum TEXT,
  after_checksum TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (document_id IS NOT NULL OR chunk_id IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS evidence_embedding_jobs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  embedding_job_id TEXT NOT NULL UNIQUE,
  customer_scope_id TEXT NOT NULL,
  target_kind TEXT NOT NULL CHECK (target_kind IN ('document', 'document_chunk', 'knowledge_case')),
  target_ref TEXT NOT NULL,
  embedding_model TEXT NOT NULL,
  embedding_model_version TEXT NOT NULL,
  chunking_version TEXT NOT NULL,
  redaction_status TEXT NOT NULL,
  vector_store_kind TEXT NOT NULL,
  vector_store_ref TEXT NOT NULL,
  embedding_checksum TEXT NOT NULL,
  status TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS evidence_retrieval_audit_events (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  retrieval_id TEXT NOT NULL UNIQUE,
  customer_scope_id TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  actor_role TEXT NOT NULL,
  query_kind TEXT NOT NULL,
  query_checksum TEXT NOT NULL,
  retrieval_method TEXT NOT NULL,
  embedding_model_version TEXT,
  top_k INTEGER NOT NULL,
  source_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  result_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  redaction_status TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
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

CREATE TABLE IF NOT EXISTS agent_registry (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  agent_identity_id TEXT NOT NULL UNIQUE,
  agent_kind TEXT NOT NULL,
  agent_version INTEGER NOT NULL,
  capability_scope JSONB NOT NULL DEFAULT '[]'::jsonb,
  phi_fields_allowed JSONB NOT NULL DEFAULT '[]'::jsonb,
  status TEXT NOT NULL,
  registered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deprovisioned_at TIMESTAMPTZ,
  UNIQUE (agent_kind, agent_version)
);

CREATE TABLE IF NOT EXISTS investigations (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  investigation_id TEXT NOT NULL UNIQUE,
  claim_id TEXT NOT NULL,
  status TEXT NOT NULL,
  orchestrator_version TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  closed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS investigations_claim_id_idx
  ON investigations(claim_id);

INSERT INTO agent_registry
  (agent_identity_id, agent_kind, agent_version, capability_scope, phi_fields_allowed, status)
VALUES
  (
    'agent_identity:deterministic_investigator:v1',
    'deterministic_investigator',
    1,
    '["knowledge.search_similar", "agent.investigation.package"]'::jsonb,
    '["claim_id", "risk_score", "rag", "diagnosis_code", "provider_region"]'::jsonb,
    'active'
  )
ON CONFLICT (agent_identity_id) DO NOTHING;

CREATE TABLE IF NOT EXISTS agent_audit_events (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  audit_event_id TEXT NOT NULL UNIQUE,
  investigation_id TEXT NOT NULL,
  agent_run_id TEXT NOT NULL REFERENCES agent_runs(agent_run_id) ON DELETE CASCADE,
  agent_kind TEXT NOT NULL,
  agent_version INTEGER NOT NULL,
  actor_id TEXT NOT NULL,
  actor_role TEXT NOT NULL,
  action_type TEXT NOT NULL,
  input_digest TEXT NOT NULL,
  decision_boundary TEXT NOT NULL,
  findings_count INTEGER NOT NULL,
  evidence_sufficiency TEXT NOT NULL,
  tool_call_count INTEGER NOT NULL,
  human_review_required BOOLEAN NOT NULL,
  phi_fields_accessed JSONB NOT NULL DEFAULT '[]'::jsonb,
  payload JSONB NOT NULL DEFAULT '{}'::jsonb,
  previous_event_hash TEXT,
  event_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS agent_audit_events_run_idx
  ON agent_audit_events(agent_run_id, created_at);

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

CREATE TABLE IF NOT EXISTS agent_workspace_artifacts (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  workspace_artifact_id TEXT NOT NULL UNIQUE,
  agent_run_id TEXT NOT NULL REFERENCES agent_runs(agent_run_id) ON DELETE CASCADE,
  artifact_kind TEXT NOT NULL,
  artifact_uri TEXT NOT NULL,
  artifact_checksum TEXT NOT NULL,
  redaction_status TEXT NOT NULL,
  retention_policy_id TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_evidence_documents_customer_scope
  ON evidence_documents(customer_scope_id);
CREATE INDEX IF NOT EXISTS idx_evidence_documents_claim_id
  ON evidence_documents(claim_id);
CREATE INDEX IF NOT EXISTS idx_evidence_document_chunks_document_id
  ON evidence_document_chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_evidence_ocr_outputs_document_id
  ON evidence_ocr_outputs(document_id);
CREATE INDEX IF NOT EXISTS idx_evidence_embedding_jobs_target
  ON evidence_embedding_jobs(target_kind, target_ref);
CREATE INDEX IF NOT EXISTS idx_evidence_retrieval_audit_customer_scope
  ON evidence_retrieval_audit_events(customer_scope_id, created_at);
CREATE INDEX IF NOT EXISTS idx_agent_workspace_artifacts_run
  ON agent_workspace_artifacts(agent_run_id);

CREATE TABLE IF NOT EXISTS fwa_leads (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  lead_id TEXT NOT NULL UNIQUE,
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  claim_id TEXT NOT NULL,
  member_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  source_system TEXT NOT NULL,
  review_mode TEXT NOT NULL DEFAULT 'pre_payment',
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

ALTER TABLE fwa_leads
  ADD COLUMN IF NOT EXISTS review_mode TEXT NOT NULL DEFAULT 'pre_payment';

CREATE TABLE IF NOT EXISTS investigation_cases (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  case_id TEXT NOT NULL UNIQUE,
  lead_id TEXT NOT NULL REFERENCES fwa_leads(lead_id),
  claim_id TEXT NOT NULL,
  member_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  source_system TEXT NOT NULL,
  review_mode TEXT NOT NULL DEFAULT 'pre_payment',
  scheme_family TEXT NOT NULL,
  lead_source TEXT NOT NULL,
  status TEXT NOT NULL,
  assignee TEXT NOT NULL,
  reviewer TEXT NOT NULL,
  priority TEXT NOT NULL,
  routing_reason TEXT NOT NULL,
  evidence_package_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  final_outcome TEXT,
  reviewer_notes TEXT,
  investigation_result_id TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE investigation_cases
  ADD COLUMN IF NOT EXISTS review_mode TEXT NOT NULL DEFAULT 'pre_payment',
  ADD COLUMN IF NOT EXISTS final_outcome TEXT,
  ADD COLUMN IF NOT EXISTS reviewer_notes TEXT,
  ADD COLUMN IF NOT EXISTS investigation_result_id TEXT;

UPDATE investigation_cases AS c
SET review_mode = l.review_mode
FROM fwa_leads AS l
WHERE c.lead_id = l.lead_id
  AND c.review_mode IS DISTINCT FROM l.review_mode;

CREATE TABLE IF NOT EXISTS audit_samples (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  sample_id TEXT NOT NULL UNIQUE,
  customer_scope_id TEXT NOT NULL DEFAULT 'demo-customer',
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

ALTER TABLE audit_samples
  ADD COLUMN IF NOT EXISTS customer_scope_id TEXT NOT NULL DEFAULT 'demo-customer';

CREATE INDEX IF NOT EXISTS idx_audit_samples_customer_scope
  ON audit_samples(customer_scope_id);

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
  scheme_family TEXT NOT NULL DEFAULT 'high_risk_claim',
  auc NUMERIC,
  ks NUMERIC,
  precision_value NUMERIC,
  recall_value NUMERIC,
  f1 NUMERIC,
  accuracy NUMERIC,
  threshold NUMERIC,
  confusion_matrix_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  feature_importance_uri TEXT,
  permutation_importance_uri TEXT,
  metrics_json JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE model_evaluation_runs
  ADD COLUMN IF NOT EXISTS scheme_family TEXT NOT NULL DEFAULT 'high_risk_claim';
ALTER TABLE model_evaluation_runs
  ADD COLUMN IF NOT EXISTS permutation_importance_uri TEXT;

CREATE TABLE IF NOT EXISTS scoring_feature_context_materializations (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  materialization_id TEXT NOT NULL,
  customer_scope_id TEXT NOT NULL,
  as_of_date TEXT NOT NULL,
  report_uri TEXT NOT NULL,
  report_kind TEXT NOT NULL,
  source_uris JSONB NOT NULL DEFAULT '{}'::jsonb,
  claim_count INTEGER NOT NULL,
  context_count INTEGER NOT NULL,
  contexts_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  governance_boundary TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, materialization_id)
);

CREATE INDEX IF NOT EXISTS scoring_feature_context_materializations_scope_date_idx
  ON scoring_feature_context_materializations(customer_scope_id, as_of_date);
CREATE INDEX IF NOT EXISTS scoring_feature_context_materializations_contexts_gin_idx
  ON scoring_feature_context_materializations USING GIN (contexts_json);

CREATE TABLE IF NOT EXISTS worker_data_pipeline_readiness_reports (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  source_report_uri TEXT NOT NULL,
  report_kind TEXT NOT NULL,
  plan_uri TEXT NOT NULL,
  readiness_input_uri TEXT NOT NULL,
  readiness_status TEXT NOT NULL,
  job_count BIGINT NOT NULL,
  ready_job_count BIGINT NOT NULL,
  blocked_job_count BIGINT NOT NULL,
  review_task_count BIGINT NOT NULL,
  job_readiness_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  review_tasks_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  governance_boundary TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, source_report_uri)
);

CREATE INDEX IF NOT EXISTS worker_data_pipeline_readiness_reports_scope_status_idx
  ON worker_data_pipeline_readiness_reports(customer_scope_id, readiness_status);

CREATE TABLE IF NOT EXISTS worker_data_pipeline_execution_reports (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  source_report_uri TEXT NOT NULL,
  report_kind TEXT NOT NULL,
  plan_uri TEXT NOT NULL,
  run_status_uri TEXT NOT NULL,
  readiness_report_uri TEXT,
  readiness_gate_status TEXT,
  run_id TEXT NOT NULL,
  execution_date TEXT NOT NULL,
  job_count BIGINT NOT NULL,
  pending_or_failed_job_count BIGINT NOT NULL,
  review_task_count BIGINT NOT NULL,
  job_executions_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  review_tasks_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  governance_boundary TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, source_report_uri)
);

CREATE INDEX IF NOT EXISTS worker_data_pipeline_execution_reports_scope_run_idx
  ON worker_data_pipeline_execution_reports(customer_scope_id, run_id);

CREATE TABLE IF NOT EXISTS clinical_compatibility_references (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  compatibility_key TEXT NOT NULL,
  reference_version TEXT NOT NULL,
  effective_date TEXT NOT NULL,
  source_authority TEXT NOT NULL,
  diagnosis_code_prefix TEXT NOT NULL,
  procedure_code TEXT NOT NULL,
  diagnosis_procedure_match_score DOUBLE PRECISION NOT NULL,
  data_source TEXT NOT NULL,
  policy_authority_ref TEXT NOT NULL,
  rationale TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  source_report_uri TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, compatibility_key, reference_version)
);

CREATE INDEX IF NOT EXISTS clinical_compatibility_references_scope_codes_idx
  ON clinical_compatibility_references(customer_scope_id, diagnosis_code_prefix, procedure_code);

CREATE TABLE IF NOT EXISTS unbundling_comparator_candidates (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  candidate_id TEXT NOT NULL,
  as_of_date TEXT NOT NULL,
  rule_id TEXT NOT NULL,
  episode_key TEXT NOT NULL,
  member_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  window_days INTEGER NOT NULL,
  bundled_code TEXT NOT NULL,
  matched_component_codes JSONB NOT NULL DEFAULT '[]'::jsonb,
  claim_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
  policy_authority_ref TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  recommended_review TEXT NOT NULL,
  source_report_uri TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, candidate_id, as_of_date)
);

CREATE INDEX IF NOT EXISTS unbundling_comparator_candidates_scope_provider_idx
  ON unbundling_comparator_candidates(customer_scope_id, provider_id);

CREATE INDEX IF NOT EXISTS unbundling_comparator_candidates_scope_episode_idx
  ON unbundling_comparator_candidates(customer_scope_id, episode_key);

CREATE TABLE IF NOT EXISTS provider_sanctions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  sanction_key TEXT NOT NULL,
  list TEXT NOT NULL,
  provider_id TEXT,
  npi TEXT,
  provider_name TEXT NOT NULL,
  sanction_type TEXT,
  effective_date TEXT,
  source_ref TEXT,
  risk_feature TEXT NOT NULL,
  risk_score INTEGER NOT NULL,
  source_report_uri TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, sanction_key)
);

CREATE INDEX IF NOT EXISTS provider_sanctions_scope_provider_idx
  ON provider_sanctions(customer_scope_id, provider_id);

CREATE TABLE IF NOT EXISTS provider_profile_windows (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  specialty TEXT,
  network_status TEXT,
  as_of_date TEXT NOT NULL,
  windows JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  source_report_uri TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, provider_id, as_of_date)
);

CREATE INDEX IF NOT EXISTS provider_profile_windows_scope_provider_idx
  ON provider_profile_windows(customer_scope_id, provider_id);
CREATE INDEX IF NOT EXISTS provider_profile_windows_scope_provider_date_idx
  ON provider_profile_windows(customer_scope_id, provider_id, as_of_date);

CREATE TABLE IF NOT EXISTS provider_graph_signals (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  as_of_date TEXT NOT NULL,
  high_risk_neighbor_ratio DOUBLE PRECISION,
  provider_patient_overlap_score DOUBLE PRECISION,
  referral_concentration_score DOUBLE PRECISION,
  billing_ring_membership BOOLEAN NOT NULL DEFAULT false,
  temporal_co_billing_frequency_7d DOUBLE PRECISION NOT NULL DEFAULT 0,
  referral_concentration_entropy DOUBLE PRECISION,
  shared_member_provider_count INTEGER NOT NULL DEFAULT 0,
  connected_confirmed_fwa_count INTEGER,
  network_component_risk_score INTEGER,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  source_report_uri TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, provider_id, as_of_date)
);

CREATE INDEX IF NOT EXISTS provider_graph_signals_scope_provider_idx
  ON provider_graph_signals(customer_scope_id, provider_id);

CREATE TABLE IF NOT EXISTS peer_benchmark_groups (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  peer_group_key TEXT NOT NULL,
  specialty TEXT NOT NULL,
  region TEXT NOT NULL,
  service_segment TEXT NOT NULL,
  benchmark_month TEXT NOT NULL,
  claim_count INTEGER NOT NULL DEFAULT 0,
  p25 DOUBLE PRECISION NOT NULL DEFAULT 0,
  p50 DOUBLE PRECISION NOT NULL DEFAULT 0,
  p75 DOUBLE PRECISION NOT NULL DEFAULT 0,
  p90 DOUBLE PRECISION NOT NULL DEFAULT 0,
  p99 DOUBLE PRECISION NOT NULL DEFAULT 0,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  source_report_uri TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, peer_group_key, benchmark_month)
);

CREATE INDEX IF NOT EXISTS peer_benchmark_groups_scope_segment_idx
  ON peer_benchmark_groups(customer_scope_id, specialty, region, service_segment);

CREATE TABLE IF NOT EXISTS episode_rollups (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  customer_scope_id TEXT NOT NULL,
  episode_key TEXT NOT NULL,
  member_id TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  as_of_date TEXT NOT NULL,
  windows JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  source_report_uri TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(customer_scope_id, episode_key, as_of_date)
);

CREATE INDEX IF NOT EXISTS episode_rollups_scope_provider_idx
  ON episode_rollups(customer_scope_id, provider_id);

CREATE INDEX IF NOT EXISTS episode_rollups_scope_member_idx
  ON episode_rollups(customer_scope_id, member_id);

CREATE TABLE IF NOT EXISTS model_promotion_reviews (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  model_key TEXT NOT NULL,
  model_version TEXT NOT NULL,
  decision TEXT NOT NULL CHECK (decision IN ('approved', 'rejected')),
  reviewer TEXT NOT NULL,
  notes TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE model_promotion_reviews ADD COLUMN IF NOT EXISTS evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE TABLE IF NOT EXISTS probability_calibration_reports (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  model_key TEXT NOT NULL,
  model_version TEXT NOT NULL,
  report_uri TEXT NOT NULL,
  report_kind TEXT NOT NULL,
  as_of_date TEXT NOT NULL,
  row_count BIGINT NOT NULL,
  minimum_calibration_rows BIGINT NOT NULL,
  bin_count BIGINT NOT NULL,
  expected_calibration_error DOUBLE PRECISION NOT NULL,
  max_expected_calibration_error DOUBLE PRECISION NOT NULL,
  brier_score DOUBLE PRECISION NOT NULL,
  max_brier_score DOUBLE PRECISION NOT NULL,
  calibration_status TEXT NOT NULL,
  bins_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  review_tasks_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  governance_boundary TEXT NOT NULL,
  submitted_by TEXT NOT NULL,
  notes TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(model_key, model_version, report_uri)
);

CREATE INDEX IF NOT EXISTS probability_calibration_reports_model_idx
  ON probability_calibration_reports(model_key, model_version, as_of_date);

CREATE TABLE IF NOT EXISTS model_retraining_jobs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  model_key TEXT NOT NULL,
  model_version TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'validation', 'completed', 'failed', 'cancelled')),
  requested_by TEXT NOT NULL,
  request_notes TEXT NOT NULL,
  status_note TEXT NOT NULL,
  updated_by TEXT NOT NULL,
  readiness_recommendation TEXT NOT NULL,
  latest_evaluation_id TEXT NOT NULL,
  source_dataset_id TEXT NOT NULL,
  source_data_quality_score DOUBLE PRECISION,
  source_data_quality_status TEXT NOT NULL,
  trigger_summary_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  blocker_summary_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  candidate_model_version TEXT,
  candidate_artifact_uri TEXT,
  candidate_endpoint_url TEXT,
  validation_report_uri TEXT,
  output_evaluation_id TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE model_retraining_jobs ADD COLUMN IF NOT EXISTS candidate_model_version TEXT;
ALTER TABLE model_retraining_jobs ADD COLUMN IF NOT EXISTS candidate_artifact_uri TEXT;
ALTER TABLE model_retraining_jobs ADD COLUMN IF NOT EXISTS candidate_endpoint_url TEXT;
ALTER TABLE model_retraining_jobs ADD COLUMN IF NOT EXISTS validation_report_uri TEXT;
ALTER TABLE model_retraining_jobs ADD COLUMN IF NOT EXISTS output_evaluation_id TEXT;

CREATE TABLE IF NOT EXISTS rule_promotion_reviews (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  rule_id TEXT NOT NULL,
  rule_version INTEGER NOT NULL,
  decision TEXT NOT NULL CHECK (decision IN ('approved', 'rejected')),
  reviewer TEXT NOT NULL,
  notes TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE rule_promotion_reviews ADD COLUMN IF NOT EXISTS evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb;

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

CREATE TABLE IF NOT EXISTS rule_shadow_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  rule_id TEXT NOT NULL,
  rule_version INTEGER NOT NULL,
  report_uri TEXT NOT NULL,
  decision TEXT NOT NULL CHECK (decision IN ('shadow_passed', 'shadow_blocked')),
  reviewer TEXT NOT NULL,
  notes TEXT NOT NULL,
  reviewed_count INTEGER NOT NULL,
  matched_count INTEGER NOT NULL,
  false_positive_count INTEGER NOT NULL,
  false_positive_rate DOUBLE PRECISION NOT NULL,
  blockers JSONB NOT NULL DEFAULT '[]'::jsonb,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS investigation_results (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  investigation_id TEXT NOT NULL UNIQUE,
  case_id TEXT,
  claim_id TEXT NOT NULL,
  outcome TEXT NOT NULL,
  confirmed_fwa BOOLEAN NOT NULL,
  financial_impact_type TEXT NOT NULL DEFAULT 'prevented_payment',
  saving_amount NUMERIC,
  currency TEXT,
  notes TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE investigation_results
  ADD COLUMN IF NOT EXISTS financial_impact_type TEXT NOT NULL DEFAULT 'prevented_payment',
  ADD COLUMN IF NOT EXISTS case_id TEXT;

CREATE TABLE IF NOT EXISTS saving_attributions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  attribution_id TEXT NOT NULL UNIQUE,
  claim_id TEXT NOT NULL,
  investigation_id TEXT NOT NULL,
  source_type TEXT NOT NULL,
  source_id TEXT NOT NULL,
  financial_impact_type TEXT NOT NULL DEFAULT 'prevented_payment',
  action TEXT NOT NULL,
  saving_amount NUMERIC NOT NULL,
  currency TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE saving_attributions
  ADD COLUMN IF NOT EXISTS financial_impact_type TEXT NOT NULL DEFAULT 'prevented_payment';

CREATE TABLE IF NOT EXISTS qa_reviews (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  qa_case_id TEXT NOT NULL UNIQUE,
  claim_id TEXT NOT NULL,
  qa_conclusion TEXT NOT NULL,
  issue_type TEXT NOT NULL,
  feedback_target TEXT NOT NULL,
  feedback_status TEXT NOT NULL DEFAULT 'open',
  notes TEXT NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE qa_reviews
  ADD COLUMN IF NOT EXISTS feedback_status TEXT NOT NULL DEFAULT 'open';
