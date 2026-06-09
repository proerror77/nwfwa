use api_server::config::AppConfig;

pub(crate) fn test_config() -> AppConfig {
    AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
        database_url: "postgres://unused".into(),
        model_service_url: "heuristic://local".into(),
        object_storage_uri: "local://demo-artifacts".into(),
        customer_scope_id: "demo-customer".into(),
        retention_policy_id: "demo-retention-policy".into(),
        backup_restore_plan_id: "demo-backup-restore-plan".into(),
        pii_masking_policy_id: "demo-pii-masking-policy".into(),
        key_rotation_policy_id: "demo-key-rotation-policy".into(),
        network_allowlist_id: "demo-network-allowlist".into(),
        alert_routing_policy_id: "demo-alert-routing-policy".into(),
        observability_exporter_endpoint: "local://demo-observability".into(),
        agent_policy_id: "demo-agent-policy".into(),
    }
}

pub(crate) fn assert_writeback_pii_contract(schema: &serde_json::Value, schema_name: &str) {
    let notes_description = schema["components"]["schemas"][schema_name]["properties"]["notes"]
        ["description"]
        .as_str()
        .unwrap_or_default();
    assert!(
        notes_description.contains("must not contain PII"),
        "missing {schema_name}.notes PII contract"
    );
    let evidence_description = schema["components"]["schemas"][schema_name]["properties"]
        ["evidence_refs"]["description"]
        .as_str()
        .unwrap_or_default();
    assert!(
        evidence_description.contains("must not contain PII"),
        "missing {schema_name}.evidence_refs PII contract"
    );
}
