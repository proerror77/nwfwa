\if :{?demo_claim_id}
\else
\set demo_claim_id 'CLM-0287'
\endif

DROP TABLE IF EXISTS demo_persistence_context;

CREATE TEMP TABLE demo_persistence_context AS
SELECT sr.run_id, sr.id AS scoring_run_uuid, c.id AS claim_uuid, c.external_claim_id
FROM scoring_runs sr
JOIN claims c ON c.id = sr.claim_id
WHERE c.external_claim_id = :'demo_claim_id'
  AND sr.status = 'succeeded'
  AND sr.risk_score IS NOT NULL
ORDER BY sr.completed_at DESC NULLS LAST, sr.started_at DESC, sr.run_id DESC
LIMIT 1;

DO $$
DECLARE
  demo_run_id TEXT;
  demo_claim_uuid UUID;
  demo_claim_id TEXT;
  row_count INTEGER;
BEGIN
  SELECT run_id, claim_uuid, external_claim_id
  INTO demo_run_id, demo_claim_uuid, demo_claim_id
  FROM demo_persistence_context;

  IF demo_run_id IS NULL THEN
    RAISE EXCEPTION 'demo scoring run was not persisted';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM feature_values
  WHERE run_id = demo_run_id
    AND claim_id = demo_claim_uuid;
  IF row_count < 8 THEN
    RAISE EXCEPTION 'expected at least 8 feature_values for %, found %', demo_run_id, row_count;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM rule_runs
  WHERE run_id = demo_run_id
    AND matched = true
    AND alert_code IS NOT NULL
    AND reason IS NOT NULL
    AND jsonb_array_length(evidence_json) >= 2;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected matched rule_runs with alert, reason, and evidence for %', demo_run_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM model_scores
  WHERE run_id = demo_run_id
    AND model_key = 'baseline_fwa'
    AND score BETWEEN 0 AND 100
    AND label IS NOT NULL;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected baseline_fwa model_score for %', demo_run_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE run_id = demo_run_id
    AND claim_id = demo_claim_uuid
    AND event_type = 'scoring.completed'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected scoring.completed audit event for %', demo_run_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM fwa_leads
  WHERE run_id = demo_run_id
    AND claim_id = demo_claim_id
    AND status = 'triaged'
    AND disposition = 'open_case'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected triaged FWA lead for %', demo_run_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM investigation_cases
  WHERE claim_id = demo_claim_id
    AND status = 'investigating'
    AND evidence_package_json -> 'evidence_sufficiency' -> 'minimum_evidence' <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected investigating case with evidence sufficiency for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE run_id = demo_run_id
    AND event_type IN ('lead.triaged', 'case.status.updated')
    AND evidence_refs <> '[]'::jsonb;
  IF row_count <> 2 THEN
    RAISE EXCEPTION 'expected lead.triaged and case.status.updated audit events for %, found %',
      demo_run_id, row_count;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE payload ->> 'claim_id' = demo_claim_id
    AND event_type = 'medical.review.recorded'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected medical.review.recorded audit event for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM investigation_results
  WHERE claim_id = demo_claim_id
    AND investigation_id = 'INV-DEMO-SMOKE'
    AND confirmed_fwa = true
    AND financial_impact_type = 'estimated_impact'
    AND saving_amount = 8200.00
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected investigation result writeback for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM qa_reviews
  WHERE claim_id = demo_claim_id
    AND qa_case_id = 'QA-DEMO-SMOKE'
    AND feedback_target = 'rules'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected QA review writeback for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM saving_attributions
  WHERE claim_id = demo_claim_id
    AND investigation_id = 'INV-DEMO-SMOKE'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected saving attribution rows for %', demo_claim_id;
  END IF;
END $$;
