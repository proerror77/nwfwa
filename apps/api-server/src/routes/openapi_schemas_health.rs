use serde_json::{json, Value};

pub(super) fn health_schemas() -> Value {
    json!({
                "ErrorResponse": {
                    "type": "object",
                    "required": ["code", "message"],
                    "properties": {
                        "code": {
                            "type": "string"
                        },
                        "message": {
                            "type": "string"
                        }
                    }
                },
                "HealthCheck": {
                    "type": "object",
                    "required": ["name", "status"],
                    "properties": {
                        "name": { "type": "string" },
                        "status": {
                            "type": "string",
                            "enum": ["ok", "configured", "local_dev_key", "local_demo_source", "local_dev_database", "local_dev_model_service", "heuristic_model_scorer", "local_demo_object_storage", "local_demo_customer_scope", "local_demo_retention_policy", "local_demo_backup_restore", "local_demo_pii_masking", "local_demo_key_rotation", "local_demo_network_allowlist", "local_demo_alert_routing", "local_demo_observability_exporter", "local_demo_agent_policy"],
                            "description": "Check status. local_dev_key indicates the API is using the local development key. local_demo_source indicates the API is using the local demo source system. local_dev_database indicates the API is using the local development database URL. local_dev_model_service indicates the API is using the local development model service URL. heuristic_model_scorer indicates the API is using the heuristic fallback scorer. local_demo_object_storage indicates the API is using the local demo object storage URI. local_demo_customer_scope indicates the API is using the local demo customer scope id. local_demo_retention_policy indicates the API is using the local demo retention policy id. local_demo_backup_restore indicates the API is using the local demo backup and restore plan id. local_demo_pii_masking indicates the API is using the local demo PII masking policy id. local_demo_key_rotation indicates the API is using the local demo key rotation policy id. local_demo_network_allowlist indicates the API is using the local demo network allowlist id. local_demo_alert_routing indicates the API is using the local demo alert routing policy id. local_demo_observability_exporter indicates the API is using the local demo observability exporter endpoint. local_demo_agent_policy indicates the API is using the local demo Agent tool policy id. These must be reconfigured before customer pilot or production use."
                        },
                        "runtime_kind": {
                            "type": "string",
                            "enum": ["python_http", "heuristic", "rust_artifact", "rust_serving_manifest"],
                            "description": "Model scorer runtime boundary when the check is model_scorer. Internal service URLs are intentionally not exposed."
                        },
                        "remediation": {
                            "type": "string",
                            "description": "Non-secret remediation hint returned for configuration checks that are not yet customer-pilot ready. Secret values and internal endpoint values are intentionally not exposed."
                        }
                    }
                },
                "PilotReadiness": {
                    "type": "object",
                    "required": ["status", "ready_for_customer_pilot", "required_check_names", "required_check_count", "ready_check_count", "blocking_check_count", "blocking_check_names", "remediation_summary", "ready_checks", "blocking_checks"],
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["ready", "not_ready"],
                            "description": "Aggregate customer pilot readiness derived from non-secret configuration checks."
                        },
                        "ready_for_customer_pilot": {
                            "type": "boolean",
                            "description": "True only when no required pilot configuration checks are blocking customer pilot traffic."
                        },
                        "required_check_names": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Configuration check names that must be configured before customer pilot traffic."
                        },
                        "required_check_count": {
                            "type": "integer",
                            "description": "Total number of required pilot configuration checks."
                        },
                        "ready_check_count": {
                            "type": "integer",
                            "description": "Number of required pilot configuration checks already configured."
                        },
                        "blocking_check_count": {
                            "type": "integer",
                            "description": "Number of required pilot configuration checks still blocking customer pilot readiness."
                        },
                        "blocking_check_names": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Compact list of blocking configuration check names for scripts and dashboards."
                        },
                        "remediation_summary": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Compact non-secret remediation hints for blocking readiness checks."
                        },
                        "ready_checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/HealthCheck" },
                            "description": "Configuration checks that are ready for customer pilot traffic."
                        },
                        "blocking_checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/HealthCheck" },
                            "description": "Configuration checks that must become configured before customer pilot traffic."
                        }
                    }
                },
                "HealthResponse": {
                    "type": "object",
                    "required": ["status", "service", "version", "pilot_readiness", "checks"],
                    "properties": {
                        "status": { "type": "string", "enum": ["ok"] },
                        "service": { "type": "string" },
                        "version": { "type": "string" },
                        "pilot_readiness": { "$ref": "#/components/schemas/PilotReadiness" },
                        "checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/HealthCheck" }
                        }
                    }
                },
    })
}
