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
  recommended_action TEXT,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  error_code TEXT,
  error_message TEXT
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
