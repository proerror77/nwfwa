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
  FROM audit_events ae
  JOIN claims c ON c.id = ae.claim_id
  WHERE c.external_claim_id LIKE 'CLM-INBOX-%'
    AND ae.event_type = 'scoring.completed'
    AND ae.payload -> 'canonical_claim_context_trace' ->> 'input_mode' = 'canonical_claim_context'
    AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements_text(ae.evidence_refs) AS ref(value)
      WHERE ref.value LIKE 'invoice:INV-INBOX-%:fee_detail:LINE-INBOX-%'
    )
    AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements_text(ae.payload -> 'canonical_claim_context_trace' -> 'evidence_refs') AS ref(value)
      WHERE ref.value LIKE 'invoice:INV-INBOX-%:fee_detail:LINE-INBOX-%'
    )
    AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements_text(ae.payload -> 'canonical_claim_context_trace' -> 'source_refs') AS ref(value)
      WHERE ref.value = 'reportCase.policyList[0].invoiceList[0].feeList[0].feeDetailList[0]'
    )
    AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements_text(ae.payload -> 'canonical_claim_context_trace' -> 'source_refs') AS ref(value)
      WHERE ref.value LIKE 'medical_record:MR-INBOX-%'
    );
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected normalized inbox canonical scoring audit trace';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE run_id = demo_run_id
    AND claim_id = demo_claim_uuid
    AND event_type = 'scoring.completed'
    AND payload -> 'provider_profile' ->> 'provider_id' = 'PRV-0287'
    AND (payload -> 'provider_profile' ->> 'risk_score')::integer >= 70
    AND payload -> 'provider_profile' ->> 'risk_tier' = 'high'
    AND payload -> 'provider_profile' ->> 'review_route' = 'provider_review'
    AND (payload -> 'provider_profile' ->> 'review_required')::boolean = true
    AND payload -> 'provider_profile' -> 'evidence_refs' ? 'providers:PRV-0287';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected provider risk profile in scoring audit payload for %', demo_run_id;
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
  FROM qa_reviews
  WHERE claim_id = demo_claim_id
    AND qa_case_id = 'QA-DEMO-SMOKE'
    AND feedback_target = 'rules'
    AND feedback_status = 'resolved';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected resolved QA feedback for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE event_type = 'qa.feedback.status.updated'
    AND payload ->> 'feedback_id' = 'qa_feedback_QA-DEMO-SMOKE'
    AND payload ->> 'claim_id' = demo_claim_id
    AND payload ->> 'to_status' = 'resolved'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected QA feedback status audit for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_samples sample
  CROSS JOIN LATERAL jsonb_array_elements(sample.selected_leads_json) AS lead(value)
  WHERE sample.sample_mode = 'qa_calibration'
    AND sample.selection_method = 'reviewer_consistency_rotation'
    AND sample.reviewer = 'qa-sampling-demo'
    AND sample.assignment_queue = 'QA Review'
    AND sample.outcome_distribution_json ->> 'selected_count' = '1'
    AND lead.value ->> 'claim_id' = demo_claim_id
    AND (lead.value ->> 'risk_score')::integer >= 70
    AND jsonb_array_length(lead.value -> 'evidence_refs') >= 1;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected QA calibration audit sample for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_samples sample
  CROSS JOIN LATERAL jsonb_array_elements(sample.selected_leads_json) AS lead(value)
  JOIN qa_reviews review
    ON review.qa_case_id = 'qa_' || sample.sample_id || '_' || (lead.value ->> 'lead_id')
  WHERE sample.sample_mode = 'qa_calibration'
    AND sample.reviewer = 'qa-sampling-demo'
    AND lead.value ->> 'claim_id' = review.claim_id
    AND review.qa_conclusion = 'pass'
    AND review.issue_type = 'qa_review_completed'
    AND review.feedback_target = 'workflow'
    AND review.evidence_refs ? ('audit_samples:' || sample.sample_id);
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected reviewed QA calibration sample result';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events event
  JOIN audit_samples sample
    ON event.payload ->> 'sample_id' = sample.sample_id
  WHERE sample.sample_mode = 'qa_calibration'
    AND sample.reviewer = 'qa-sampling-demo'
    AND event.event_type = 'audit_sample.created'
    AND event.evidence_refs ? ('audit_samples:' || sample.sample_id);
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected audit_sample.created governance audit event';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM knowledge_cases
  WHERE case_id = 'KC-DEMO-SMOKE'
    AND scheme_family = 'diagnosis_procedure_mismatch'
    AND evidence_refs ? 'investigation_results:INV-DEMO-SMOKE'
    AND evidence_refs ? 'qa_reviews:QA-DEMO-SMOKE';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected published demo knowledge case';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE claim_id = demo_claim_uuid
    AND event_type = 'knowledge.case.published'
    AND payload ->> 'case_id' = 'KC-DEMO-SMOKE'
    AND evidence_refs ? 'investigation_results:INV-DEMO-SMOKE'
    AND evidence_refs ? 'qa_reviews:QA-DEMO-SMOKE';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected knowledge case publish audit for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM webhook_delivery_attempts wda
  JOIN audit_events ae
    ON wda.event_id = 'webhook_' || ae.audit_id
  WHERE ae.run_id = demo_run_id
    AND ae.claim_id = demo_claim_uuid
    AND ae.event_type = 'scoring.completed'
    AND wda.attempt_number = 1
    AND wda.delivery_status = 'failed'
    AND wda.response_status_code = 503
    AND wda.error_message = 'TPA webhook endpoint unavailable'
    AND wda.next_attempt_at IS NOT NULL;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected failed TPA webhook delivery attempt for %', demo_run_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM saving_attributions
  WHERE claim_id = demo_claim_id
    AND investigation_id = 'INV-DEMO-SMOKE'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected saving attribution rows for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM agent_runs
  WHERE claim_id = demo_claim_id
    AND status = 'succeeded'
    AND decision_boundary = 'assistive_only'
    AND output_json ? 'evidence_sufficiency'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected governed agent run for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM agent_runs ar
  JOIN agent_steps ast ON ast.agent_run_id = ar.agent_run_id
  WHERE ar.claim_id = demo_claim_id
    AND ast.step_name = 'evidence_finding'
    AND ast.evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected evidence-backed agent steps for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM agent_runs ar
  JOIN agent_context_snapshots acs ON acs.agent_run_id = ar.agent_run_id
  WHERE ar.claim_id = demo_claim_id
    AND acs.redaction_status = 'pii_masked'
    AND acs.checksum LIKE 'snapshot:%'
    AND acs.context_json ->> 'claim_id' LIKE 'masked:claim:%'
    AND acs.source_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected PII-masked agent context snapshot for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM agent_runs ar
  JOIN tool_calls tc ON tc.agent_run_id = ar.agent_run_id
  JOIN agent_policy_checks apc
    ON apc.agent_run_id = ar.agent_run_id
   AND apc.tool_call_id = tc.tool_call_id
  JOIN tool_results tr
    ON tr.agent_run_id = ar.agent_run_id
   AND tr.tool_call_id = tc.tool_call_id
  WHERE ar.claim_id = demo_claim_id
    AND tc.tool_name = 'knowledge.search_similar'
    AND tc.status = 'succeeded'
    AND tc.evidence_refs <> '[]'::jsonb
    AND apc.policy_name = 'demo-agent-policy'
    AND apc.decision = 'allowed'
    AND apc.evidence_refs ? 'policy:demo-agent-policy'
    AND tr.status = 'succeeded'
    AND (tr.output_json ->> 'result_count')::integer > 0
    AND tr.evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected audited agent tool call, policy check, and result for %',
      demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM agent_runs ar
  JOIN agent_approvals aa ON aa.agent_run_id = ar.agent_run_id
  WHERE ar.claim_id = demo_claim_id
    AND aa.proposed_action = 'manual_review_required'
    AND aa.decision = 'approved'
    AND aa.approver = 'agent-governance-demo'
    AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements_text(aa.evidence_refs) AS ref(value)
      WHERE ref.value = 'agent_run:' || ar.agent_run_id
    )
    AND EXISTS (
      SELECT 1
      FROM jsonb_array_elements_text(aa.evidence_refs) AS ref(value)
      WHERE ref.value = 'policy:demo-agent-policy'
    );
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected approved human agent approval for %', demo_claim_id;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM agent_runs ar
  JOIN audit_events ae
    ON ae.payload ->> 'agent_run_id' = ar.agent_run_id
  WHERE ar.claim_id = demo_claim_id
    AND ae.event_type IN ('agent.investigation.completed', 'agent.approval.decided')
    AND ae.evidence_refs <> '[]'::jsonb;
  IF row_count < 2 THEN
    RAISE EXCEPTION 'expected agent investigation and approval audit events for %, found %',
      demo_claim_id, row_count;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM rule_backtest_runs
  WHERE rule_id = 'rule_early_claim'
    AND rule_version = 1
    AND promotion_recommendation = 'eligible_for_review'
    AND precision_value >= 0.70
    AND recall_value >= 0.60
    AND false_positive_rate <= 0.30
    AND estimated_saving::numeric > 0
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected eligible rule_early_claim backtest evidence';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM rules
  WHERE rule_key = 'candidate_early_high_amount'
    AND status = 'draft'
    AND owner = 'rule-discovery-demo';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected saved rule discovery candidate';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM rule_backtest_runs
  WHERE rule_id = 'candidate_early_high_amount'
    AND rule_version = 1
    AND promotion_recommendation = 'eligible_for_review'
    AND precision_value >= 0.70
    AND recall_value >= 0.60
    AND false_positive_rate <= 0.30
    AND estimated_saving::numeric > 0
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected eligible discovered candidate backtest evidence';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE event_type IN ('rule.candidate.saved', 'rule.backtest.completed')
    AND payload ->> 'rule_id' = 'candidate_early_high_amount'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 2 THEN
    RAISE EXCEPTION 'expected rule discovery candidate audit events, found %',
      row_count;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM rule_promotion_reviews
  WHERE rule_id = 'rule_early_claim'
    AND rule_version = 1
    AND decision = 'approved'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected approved rule_early_claim promotion review';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM rules
  WHERE rule_key = 'rule_early_claim'
    AND status = 'active';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected rule_early_claim to be active after publish';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE event_type IN (
      'rule.backtest.completed',
      'rule.promotion.reviewed',
      'rule.status.changed'
    )
    AND payload ->> 'rule_id' = 'rule_early_claim'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 5 THEN
    RAISE EXCEPTION 'expected rule backtest, promotion, and lifecycle audit events, found %',
      row_count;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM routing_policies
  WHERE policy_key LIKE 'demo_strict_prepay_%'
    AND version = 1
    AND review_mode = 'pre_payment'
    AND status = 'active'
    AND owner = 'policy-ops-demo'
    AND policy_json -> 'risk_thresholds' ->> 'high_min' = '1';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected active demo strict pre-payment routing policy';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE event_type IN (
      'routing_policy.candidate.saved',
      'routing_policy.status.changed',
      'routing_policy.activation.completed'
    )
    AND payload ->> 'policy_id' LIKE 'demo_strict_prepay_%'
    AND payload ->> 'review_mode' = 'pre_payment'
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 4 THEN
    RAISE EXCEPTION 'expected routing policy governance audit events, found %',
      row_count;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM scoring_runs
  JOIN claims c ON c.id = scoring_runs.claim_id
  WHERE c.external_claim_id LIKE 'CLM-ROUTING-demo_strict_prepay_%'
    AND scoring_runs.routing_policy ->> 'policy_id' LIKE 'demo_strict_prepay_%'
    AND scoring_runs.routing_policy -> 'risk_thresholds' ->> 'high_min' = '1';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected scoring run controlled by demo routing policy';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM external_dataset_versions edv
  WHERE edv.dataset_key = 'demo_claims_fwa'
    AND edv.dataset_version = '2026-05-demo'
    AND edv.sample_grain = 'claim'
    AND edv.label_column = 'confirmed_fwa'
    AND edv.storage_format = 'parquet'
    AND edv.row_count = 25000
    AND edv.entity_keys ? 'claim_id'
    AND edv.entity_keys ? 'member_id'
    AND edv.entity_keys ? 'provider_id';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected demo_claims_fwa dataset version with claim-grain entity lineage';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM external_schema_fields esf
  WHERE esf.dataset_id = '71000000-0000-0000-0000-000000000001'
    AND esf.semantic_role = 'key'
    AND esf.field_name IN ('claim_id', 'member_id', 'provider_id')
    AND esf.profile_json ->> 'owner' = 'data-ops';
  IF row_count <> 3 THEN
    RAISE EXCEPTION 'expected claim/member/provider schema keys for demo_claims_fwa, found %', row_count;
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM model_dataset_versions mdv
  JOIN feature_set_versions fsv
    ON fsv.id = mdv.feature_set_id
  JOIN external_dataset_versions edv
    ON edv.id = fsv.dataset_id
  WHERE mdv.id = '75000000-0000-0000-0000-000000000001'
    AND mdv.task_type = 'binary_classification'
    AND mdv.label_name = 'confirmed_fwa'
    AND mdv.status = 'active'
    AND fsv.feature_set_key = 'fwa_demo_factor_set'
    AND fsv.version = '2026-05-demo'
    AND edv.dataset_key = 'demo_claims_fwa'
    AND edv.dataset_version = '2026-05-demo';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected active model dataset linked to demo_claims_fwa feature set';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM model_evaluation_runs mer
  WHERE mer.evaluation_run_id = 'eval-baseline-fwa-2026-05-demo'
    AND mer.model_key = 'baseline_fwa'
    AND mer.model_version = '0.1.0'
    AND mer.model_dataset_id = '75000000-0000-0000-0000-000000000001'
    AND mer.scheme_family = 'diagnosis_procedure_mismatch'
    AND mer.auc >= 0.8
    AND mer.precision_value >= 0.7
    AND mer.recall_value >= 0.6
    AND mer.threshold IS NOT NULL
    AND mer.feature_importance_uri IS NOT NULL
    AND (
      lower(split_part(split_part(mer.feature_importance_uri, '?', 1), '#', 1)) LIKE '%.parquet'
      OR lower(split_part(split_part(mer.feature_importance_uri, '?', 1), '#', 1)) LIKE '%/'
    )
    AND (mer.metrics_json ->> 'psi')::numeric <= 0.1
    AND mer.metrics_json ->> 'review_capacity_threshold_status' = 'passed'
    AND mer.metrics_json ->> 'leakage_check_status' = 'passed'
    AND mer.metrics_json ->> 'shadow_comparison_status' = 'passed'
    AND mer.metrics_json ->> 'serving_version_lock_status' = 'passed'
    AND mer.metrics_json ->> 'artifact_integrity_status' = 'passed'
    AND mer.metrics_json ->> 'feature_store_materialization_status' = 'passed'
    AND mer.metrics_json ->> 'segment_fairness_status' = 'passed'
    AND mer.metrics_json ->> 'label_provenance_status' = 'passed'
    AND mer.metrics_json ->> 'pilot_validation_status' = 'passed'
    AND mer.metrics_json ->> 'approval_status' = 'approved'
    AND mer.metrics_json ->> 'feature_reproducibility_hash' = 'sha256:demo-baseline-feature-reproducibility'
    AND mer.metrics_json ? 'out_of_time_auc';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected governed baseline_fwa evaluation linked to demo model dataset with parquet feature importance artifact';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM model_retraining_jobs
  WHERE model_key = 'baseline_fwa'
    AND model_version = '0.1.0'
    AND status = 'completed'
    AND readiness_recommendation = 'prepare_retraining'
    AND candidate_model_version IS NOT NULL
    AND candidate_artifact_uri IS NOT NULL
    AND validation_report_uri IS NOT NULL
    AND output_evaluation_id IS NOT NULL;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected completed baseline_fwa retraining job';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM model_versions mv
  JOIN model_retraining_jobs mrj
    ON mrj.model_key = mv.model_key
   AND mrj.candidate_model_version = mv.version
  WHERE mrj.model_key = 'baseline_fwa'
    AND mrj.status = 'completed'
    AND mv.status = 'active'
    AND mv.artifact_uri = mrj.candidate_artifact_uri;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected retraining candidate activated after governance approval';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM model_versions
  WHERE model_key = 'baseline_fwa'
    AND version = '0.1.0'
    AND status = 'approved';
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected previous baseline_fwa model version moved to approved';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM model_evaluation_runs mer
  JOIN model_retraining_jobs mrj
    ON mrj.output_evaluation_id = mer.evaluation_run_id
  WHERE mrj.model_key = 'baseline_fwa'
    AND mrj.status = 'completed'
    AND mer.model_version = mrj.candidate_model_version;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected candidate evaluation registered from retraining output';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM model_promotion_reviews mpr
  JOIN model_retraining_jobs mrj
    ON mrj.model_key = mpr.model_key
   AND mrj.candidate_model_version = mpr.model_version
  WHERE mrj.model_key = 'baseline_fwa'
    AND mrj.status = 'completed'
    AND mpr.decision = 'approved'
    AND mpr.evidence_refs <> '[]'::jsonb;
  IF row_count < 1 THEN
    RAISE EXCEPTION 'expected approved promotion review for retraining candidate';
  END IF;

  SELECT COUNT(*) INTO row_count
  FROM audit_events
  WHERE event_type IN (
      'model.retraining.queued',
      'model.retraining.claimed',
      'model.retraining.status_updated',
      'model.retraining.output_registered',
      'model.promotion.reviewed',
      'model.activation.completed'
    )
    AND evidence_refs <> '[]'::jsonb;
  IF row_count < 6 THEN
    RAISE EXCEPTION 'expected retraining governance audit events, found %', row_count;
  END IF;
END $$;
