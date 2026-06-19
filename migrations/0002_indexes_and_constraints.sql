-- Migration 0002: Missing indexes and CHECK constraints
--
-- Adds performance-critical indexes for hot query paths and database-level
-- CHECK constraints to prevent invalid enum values from being persisted.
-- All statements use IF NOT EXISTS / IF NOT VALID guarding for idempotency.

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — claims
-- ──────────────────────────────────────────────────────────────────────────────

-- FK columns (member_id, policy_id, provider_id) are the join target for every
-- downstream table and have no index in migration 0001.
CREATE INDEX IF NOT EXISTS idx_claims_member_id    ON claims(member_id);
CREATE INDEX IF NOT EXISTS idx_claims_policy_id    ON claims(policy_id);
CREATE INDEX IF NOT EXISTS idx_claims_provider_id  ON claims(provider_id);

-- service_date range scans and status filters appear in every queue query.
CREATE INDEX IF NOT EXISTS idx_claims_service_date ON claims(service_date);
CREATE INDEX IF NOT EXISTS idx_claims_status       ON claims(status);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — policies
-- ──────────────────────────────────────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_policies_member_id ON policies(member_id);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — scoring_runs
-- ──────────────────────────────────────────────────────────────────────────────

-- claim_id is the primary lookup path from audit and lead joins.
CREATE INDEX IF NOT EXISTS idx_scoring_runs_claim_id ON scoring_runs(claim_id);
CREATE INDEX IF NOT EXISTS idx_scoring_runs_status   ON scoring_runs(status);
CREATE INDEX IF NOT EXISTS idx_scoring_runs_actor_id ON scoring_runs(actor_id);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — fwa_leads
-- ──────────────────────────────────────────────────────────────────────────────

-- claim_id is used to find all leads for a claim (TEXT, no FK, see review notes).
CREATE INDEX IF NOT EXISTS idx_fwa_leads_claim_id ON fwa_leads(claim_id);
-- status + rag drive the inbox queue view (filter + sort).
CREATE INDEX IF NOT EXISTS idx_fwa_leads_status   ON fwa_leads(status);
CREATE INDEX IF NOT EXISTS idx_fwa_leads_rag       ON fwa_leads(rag);
-- run_id FK lookup (already has CASCADE constraint, needs index for JOIN speed).
CREATE INDEX IF NOT EXISTS idx_fwa_leads_run_id   ON fwa_leads(run_id);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — investigation_cases
-- ──────────────────────────────────────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_investigation_cases_claim_id ON investigation_cases(claim_id);
CREATE INDEX IF NOT EXISTS idx_investigation_cases_status   ON investigation_cases(status);
CREATE INDEX IF NOT EXISTS idx_investigation_cases_assignee ON investigation_cases(assignee);
-- lead_id FK column has no index despite the REFERENCES constraint.
CREATE INDEX IF NOT EXISTS idx_investigation_cases_lead_id  ON investigation_cases(lead_id);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — audit_events
-- ──────────────────────────────────────────────────────────────────────────────

-- All three columns appear in compliance queries and claim audit history API.
CREATE INDEX IF NOT EXISTS idx_audit_events_claim_id   ON audit_events(claim_id);
CREATE INDEX IF NOT EXISTS idx_audit_events_actor_id   ON audit_events(actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_events_event_type ON audit_events(event_type);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — feature_values / rule_runs / model_scores
-- ──────────────────────────────────────────────────────────────────────────────

-- Composite indexes for score reconstruction queries.
CREATE INDEX IF NOT EXISTS idx_feature_values_run_feature ON feature_values(run_id, feature_name);
CREATE INDEX IF NOT EXISTS idx_rule_runs_run_rule         ON rule_runs(run_id, rule_id);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — agent_steps
-- ──────────────────────────────────────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_agent_steps_agent_run_id ON agent_steps(agent_run_id);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — investigation_results / saving_attributions / qa_reviews
-- ──────────────────────────────────────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_investigation_results_claim_id ON investigation_results(claim_id);
CREATE INDEX IF NOT EXISTS idx_saving_attributions_claim_id   ON saving_attributions(claim_id);
CREATE INDEX IF NOT EXISTS idx_saving_attributions_investigation_id ON saving_attributions(investigation_id);
CREATE INDEX IF NOT EXISTS idx_qa_reviews_claim_id            ON qa_reviews(claim_id);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — model_retraining_jobs
-- ──────────────────────────────────────────────────────────────────────────────

-- Job queue polling pattern: filter by model_key and status.
CREATE INDEX IF NOT EXISTS idx_model_retraining_jobs_model_status
  ON model_retraining_jobs(model_key, status);

-- ──────────────────────────────────────────────────────────────────────────────
-- CHECK CONSTRAINTS — enum / status columns
--
-- Use DO blocks with IF NOT EXISTS logic so this migration is idempotent.
-- Values are verified against the Rust repository layer before being added.
-- ──────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
  -- claims.status
  -- Actual value written by postgres_claims.rs INSERT: 'submitted'.
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'claims' AND constraint_name = 'chk_claims_status'
  ) THEN
    ALTER TABLE claims
      ADD CONSTRAINT chk_claims_status
      CHECK (status IN (
        'submitted', 'received', 'pending', 'scoring', 'scored',
        'adjudicated', 'paid', 'denied', 'void', 'under_review'
      ));
  END IF;

  -- scoring_runs.rag
  -- Stored via format!("{:?}", RagBand::Green) → PascalCase.
  -- RagBand variants: Green, Amber, Red (no Critical in the enum).
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'scoring_runs' AND constraint_name = 'chk_scoring_runs_rag'
  ) THEN
    ALTER TABLE scoring_runs
      ADD CONSTRAINT chk_scoring_runs_rag
      CHECK (rag IS NULL OR rag IN ('Green', 'Amber', 'Red'));
  END IF;

  -- scoring_runs.status
  -- Values from postgres_scoring.rs: 'succeeded' on success path.
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'scoring_runs' AND constraint_name = 'chk_scoring_runs_status'
  ) THEN
    ALTER TABLE scoring_runs
      ADD CONSTRAINT chk_scoring_runs_status
      CHECK (status IN ('succeeded', 'failed', 'started', 'running'));
  END IF;

  -- fwa_leads.disposition
  -- Values from triage_helpers.rs triage_disposition_for_decision:
  -- 'open_case', 'rejected', 'pending_evidence', 'merged', 'pending_triage'.
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'fwa_leads' AND constraint_name = 'chk_fwa_leads_disposition'
  ) THEN
    ALTER TABLE fwa_leads
      ADD CONSTRAINT chk_fwa_leads_disposition
      CHECK (disposition IN (
        'pending_triage', 'open_case', 'rejected', 'request_evidence',
        'pending_evidence', 'merged', 'closed'
      ));
  END IF;

  -- fwa_leads.rag
  -- Same format!("{:?}", RagBand) as scoring_runs.rag → PascalCase.
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'fwa_leads' AND constraint_name = 'chk_fwa_leads_rag'
  ) THEN
    ALTER TABLE fwa_leads
      ADD CONSTRAINT chk_fwa_leads_rag
      CHECK (rag IN ('Green', 'Amber', 'Red'));
  END IF;

  -- investigation_cases.status
  -- Values from triage_helpers.rs: 'triage' (initial), then
  -- 'investigating', 'pending_evidence', 'confirmed', 'rejected', 'closed'.
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'investigation_cases' AND constraint_name = 'chk_investigation_cases_status'
  ) THEN
    ALTER TABLE investigation_cases
      ADD CONSTRAINT chk_investigation_cases_status
      CHECK (status IN (
        'triage', 'investigating', 'pending_evidence',
        'confirmed', 'rejected', 'closed'
      ));
  END IF;

  -- investigation_cases.priority
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'investigation_cases' AND constraint_name = 'chk_investigation_cases_priority'
  ) THEN
    ALTER TABLE investigation_cases
      ADD CONSTRAINT chk_investigation_cases_priority
      CHECK (priority IN ('low', 'medium', 'high', 'critical'));
  END IF;

  -- providers.risk_tier
  -- Stored via format!("{:?}", ProviderRiskTier::High) → PascalCase.
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'providers' AND constraint_name = 'chk_providers_risk_tier'
  ) THEN
    ALTER TABLE providers
      ADD CONSTRAINT chk_providers_risk_tier
      CHECK (risk_tier IN ('Low', 'Medium', 'High'));
  END IF;

  -- qa_reviews.feedback_status
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'qa_reviews' AND constraint_name = 'chk_qa_reviews_feedback_status'
  ) THEN
    ALTER TABLE qa_reviews
      ADD CONSTRAINT chk_qa_reviews_feedback_status
      CHECK (feedback_status IN ('open', 'resolved', 'escalated', 'dismissed'));
  END IF;

  -- qa_reviews.qa_conclusion
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'qa_reviews' AND constraint_name = 'chk_qa_reviews_qa_conclusion'
  ) THEN
    ALTER TABLE qa_reviews
      ADD CONSTRAINT chk_qa_reviews_qa_conclusion
      CHECK (qa_conclusion IN (
        'confirmed_correct', 'false_positive', 'false_negative',
        'insufficient_evidence', 'escalated', 'needs_review'
      ));
  END IF;

  -- audit_events.event_status
  -- Values used throughout the codebase: 'succeeded', 'failed'.
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
    WHERE table_name = 'audit_events' AND constraint_name = 'chk_audit_events_event_status'
  ) THEN
    ALTER TABLE audit_events
      ADD CONSTRAINT chk_audit_events_event_status
      CHECK (event_status IN ('succeeded', 'failed', 'pending', 'cancelled'));
  END IF;
END
$$;

