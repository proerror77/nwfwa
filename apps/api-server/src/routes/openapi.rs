use axum::Json;
use serde_json::{json, Value};

pub async fn openapi_schema() -> Json<Value> {
    Json(json!({
        "openapi": "3.1.0",
        "info": {
            "title": "FWA Core Runtime API",
            "version": "0.1.0",
            "description": "MVP API contract for claim scoring and runtime health checks."
        },
        "paths": {
            "/api/v1/health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": {
                            "description": "Service is healthy"
                        }
                    }
                }
            },
            "/api/v1/claims/score": {
                "post": {
                    "summary": "Score a health insurance claim for FWA risk",
                    "security": [
                        {
                            "ApiKeyAuth": []
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ScoreClaimRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Risk score and audit-backed recommendation",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ScoreClaimResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid or ambiguous scoring request",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ErrorResponse"
                                    }
                                }
                            }
                        },
                        "401": {
                            "description": "Missing or invalid API key",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ErrorResponse"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Claim id was not found",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ErrorResponse"
                                    }
                                }
                            }
                        },
                        "502": {
                            "description": "Model service failed or returned an invalid response",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ErrorResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules": {
                "get": {
                    "summary": "List rule library",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Rule summaries",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleListResponse" }
                                }
                            }
                        },
                        "401": {
                            "description": "Missing or invalid API key",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules/{rule_id}": {
                "get": {
                    "summary": "Get rule details and versions",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "rule_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Rule detail",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleDetailResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/datasets": {
                "get": {
                    "summary": "List registered external datasets",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Dataset catalog entries",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/DatasetListResponse" }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Register a governed Parquet dataset",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/DatasetRegistrationRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Registered dataset",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/DatasetRecord" }
                                }
                            }
                        },
                        "400": {
                            "description": "Only parquet datasets can be registered",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/datasets/{dataset_id}": {
                "get": {
                    "summary": "Get external dataset catalog detail",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "dataset_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Dataset catalog detail",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/DatasetRecord" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/datasets/{dataset_id}/mappings": {
                "post": {
                    "summary": "Add an external field mapping for a dataset",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "dataset_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/FieldMappingRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Created field mapping",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/FieldMappingResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules/backtest": {
                "post": {
                    "summary": "Backtest a candidate rule against sample claims",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/RuleBacktestRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Backtest metrics",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleBacktestResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models": {
                "get": {
                    "summary": "List model versions",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Model versions",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/performance": {
                "get": {
                    "summary": "Get model performance metrics",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Model performance metrics",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelPerformanceResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/knowledge/cases": {
                "get": {
                    "summary": "List FWA knowledge cases",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Knowledge case summaries",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/KnowledgeCaseListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/knowledge/search-similar": {
                "post": {
                    "summary": "Search similar FWA knowledge cases",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SimilarCaseSearchRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Similar knowledge cases",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/SimilarCaseSearchResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/agent/cases/investigate": {
                "post": {
                    "summary": "Generate an assistive agent investigation package",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/AgentInvestigationRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Evidence-backed assistive investigation package",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/AgentInvestigationResponse" }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "ApiKeyAuth": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "x-api-key"
                }
            },
            "schemas": {
                "ScoreClaimRequest": {
                    "oneOf": [
                        {
                            "$ref": "#/components/schemas/ClaimIdScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/FullPayloadScoreClaimRequest"
                        }
                    ]
                },
                "ClaimIdScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "claim_id"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "examples": ["tpa-demo"]
                        },
                        "claim_id": {
                            "type": "string",
                            "description": "Existing claim id to load from FWA storage."
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] }
                        ]
                    }
                },
                "FullPayloadScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "claim"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "examples": ["tpa-demo"]
                        },
                        "claim": {
                            "$ref": "#/components/schemas/FullClaimPayload"
                        },
                        "items": {
                            "type": "array",
                            "description": "Top-level claim items for spec-style full payload requests. Do not send the same entity both nested under claim and at the top level.",
                            "items": {
                                "$ref": "#/components/schemas/ClaimItemPayload"
                            }
                        },
                        "member": {
                            "$ref": "#/components/schemas/MemberPayload"
                        },
                        "policy": {
                            "$ref": "#/components/schemas/PolicyPayload"
                        },
                        "provider": {
                            "$ref": "#/components/schemas/ProviderPayload"
                        }
                    },
                    "not": {
                        "required": ["claim_id"]
                    }
                },
                "FullClaimPayload": {
                    "type": "object",
                    "required": ["external_claim_id", "claim_amount", "currency"],
                    "properties": {
                        "external_claim_id": {
                            "type": "string"
                        },
                        "claim_amount": {
                            "type": "string",
                            "format": "decimal"
                        },
                        "currency": {
                            "type": "string"
                        },
                        "service_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "diagnosis_code": {
                            "type": "string"
                        },
                        "items": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/ClaimItemPayload"
                            }
                        },
                        "member": {
                            "$ref": "#/components/schemas/MemberPayload"
                        },
                        "policy": {
                            "$ref": "#/components/schemas/PolicyPayload"
                        },
                        "provider": {
                            "$ref": "#/components/schemas/ProviderPayload"
                        }
                    }
                },
                "ClaimItemPayload": {
                    "type": "object",
                    "required": ["item_code", "item_type", "description", "quantity", "unit_amount", "total_amount"],
                    "properties": {
                        "item_code": {
                            "type": "string"
                        },
                        "item_type": {
                            "type": "string"
                        },
                        "description": {
                            "type": "string"
                        },
                        "quantity": {
                            "type": "integer",
                            "minimum": 0
                        },
                        "unit_amount": {
                            "type": "string",
                            "format": "decimal"
                        },
                        "total_amount": {
                            "type": "string",
                            "format": "decimal"
                        },
                        "currency": {
                            "type": "string"
                        }
                    }
                },
                "MemberPayload": {
                    "type": "object",
                    "required": ["external_member_id"],
                    "properties": {
                        "external_member_id": {
                            "type": "string"
                        },
                        "dob": {
                            "type": "string",
                            "format": "date"
                        },
                        "gender": {
                            "type": "string"
                        }
                    }
                },
                "PolicyPayload": {
                    "type": "object",
                    "required": ["external_policy_id", "coverage_start_date", "coverage_end_date", "coverage_limit"],
                    "properties": {
                        "external_policy_id": {
                            "type": "string"
                        },
                        "product_code": {
                            "type": "string"
                        },
                        "coverage_start_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "coverage_end_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "coverage_limit": {
                            "type": "string",
                            "format": "decimal"
                        },
                        "currency": {
                            "type": "string"
                        }
                    }
                },
                "ProviderPayload": {
                    "type": "object",
                    "required": ["external_provider_id", "name", "provider_type", "region"],
                    "properties": {
                        "external_provider_id": {
                            "type": "string"
                        },
                        "name": {
                            "type": "string"
                        },
                        "provider_type": {
                            "type": "string"
                        },
                        "region": {
                            "type": "string"
                        },
                        "risk_tier": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High"]
                        }
                    }
                },
                "ScoreClaimResponse": {
                    "type": "object",
                    "required": ["run_id", "audit_id", "claim_id", "risk_score", "rag", "recommended_action", "scores", "alerts", "top_reasons", "evidence_refs"],
                    "properties": {
                        "run_id": {
                            "type": "string"
                        },
                        "audit_id": {
                            "type": "string"
                        },
                        "claim_id": {
                            "type": "string"
                        },
                        "risk_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "rag": {
                            "type": "string",
                            "enum": ["Green", "Amber", "Red"]
                        },
                        "recommended_action": {
                            "type": "string",
                            "enum": ["AutoApprove", "ManualReview", "EscalateInvestigation"]
                        },
                        "scores": {
                            "$ref": "#/components/schemas/ScoreBreakdown"
                        },
                        "alerts": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/AlertResponse"
                            }
                        },
                        "top_reasons": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": {
                                "type": "object"
                            }
                        }
                    }
                },
                "ScoreBreakdown": {
                    "type": "object",
                    "required": ["rule_score", "ml_score", "final_score"],
                    "properties": {
                        "rule_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "ml_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "final_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        }
                    }
                },
                "AlertResponse": {
                    "type": "object",
                    "required": ["alert_code", "severity", "reason", "rule_id", "rule_version"],
                    "properties": {
                        "alert_code": {
                            "type": "string"
                        },
                        "severity": {
                            "type": "string"
                        },
                        "reason": {
                            "type": "string"
                        },
                        "rule_id": {
                            "type": "string"
                        },
                        "rule_version": {
                            "type": "integer"
                        }
                    }
                },
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
                "RuleSummary": {
                    "type": "object",
                    "required": ["rule_id", "name", "status", "owner", "latest_version", "score", "alert_code", "recommended_action"],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "name": { "type": "string" },
                        "status": { "type": "string", "enum": ["active", "submitted", "approved"] },
                        "owner": { "type": "string" },
                        "active_version": { "type": ["integer", "null"] },
                        "latest_version": { "type": "integer" },
                        "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "alert_code": { "type": "string" },
                        "recommended_action": { "type": "string" }
                    }
                },
                "RuleListResponse": {
                    "type": "object",
                    "required": ["rules"],
                    "properties": {
                        "rules": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RuleSummary" }
                        }
                    }
                },
                "RuleDetailResponse": {
                    "type": "object",
                    "required": ["summary", "versions"],
                    "properties": {
                        "summary": { "$ref": "#/components/schemas/RuleSummary" },
                        "versions": {
                            "type": "array",
                            "items": { "type": "object" }
                        }
                    }
                },
                "RuleBacktestRequest": {
                    "type": "object",
                    "required": ["rule", "samples"],
                    "properties": {
                        "rule": { "type": "object" },
                        "samples": {
                            "type": "array",
                            "items": { "type": "object" }
                        }
                    }
                },
                "RuleBacktestResponse": {
                    "type": "object",
                    "required": ["sample_count", "matched_count", "match_rate", "average_score_contribution", "estimated_saving", "matched_claim_ids"],
                    "properties": {
                        "sample_count": { "type": "integer" },
                        "matched_count": { "type": "integer" },
                        "match_rate": { "type": "number" },
                        "average_score_contribution": { "type": "number" },
                        "estimated_saving": { "type": "string", "format": "decimal" },
                        "matched_claim_ids": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "DatasetSplit": {
                    "type": "object",
                    "required": ["split_name", "data_uri", "row_count", "label_distribution_json"],
                    "properties": {
                        "split_name": { "type": "string" },
                        "data_uri": { "type": "string" },
                        "row_count": { "type": "integer" },
                        "positive_count": { "type": ["integer", "null"] },
                        "negative_count": { "type": ["integer", "null"] },
                        "label_distribution_json": { "type": "object" }
                    }
                },
                "SchemaField": {
                    "type": "object",
                    "required": ["field_name", "logical_type", "nullable", "semantic_role", "description", "profile_json"],
                    "properties": {
                        "field_name": { "type": "string" },
                        "logical_type": { "type": "string" },
                        "nullable": { "type": "boolean" },
                        "semantic_role": { "type": "string" },
                        "description": { "type": "string" },
                        "profile_json": { "type": "object" }
                    }
                },
                "FieldMapping": {
                    "type": "object",
                    "required": ["mapping_id", "dataset_id", "external_field", "canonical_target", "transform_kind", "transform_json", "status"],
                    "properties": {
                        "mapping_id": { "type": "string" },
                        "dataset_id": { "type": "string" },
                        "external_field": { "type": "string" },
                        "canonical_target": { "type": "string" },
                        "feature_name": { "type": ["string", "null"] },
                        "transform_kind": { "type": "string" },
                        "transform_json": { "type": "object" },
                        "status": { "type": "string" }
                    }
                },
                "DatasetRecord": {
                    "type": "object",
                    "required": ["dataset_id", "source_key", "display_name", "business_domain", "dataset_key", "dataset_version", "sample_grain", "label_column", "entity_keys", "manifest_uri", "schema_uri", "profile_uri", "storage_format", "schema_hash", "row_count", "status", "splits", "fields", "mappings"],
                    "properties": {
                        "dataset_id": { "type": "string" },
                        "source_key": { "type": "string" },
                        "display_name": { "type": "string" },
                        "business_domain": { "type": "string" },
                        "dataset_key": { "type": "string" },
                        "dataset_version": { "type": "string" },
                        "sample_grain": { "type": "string" },
                        "label_column": { "type": "string" },
                        "entity_keys": { "type": "array", "items": { "type": "string" } },
                        "manifest_uri": { "type": "string" },
                        "schema_uri": { "type": "string" },
                        "profile_uri": { "type": "string" },
                        "storage_format": { "type": "string", "const": "parquet" },
                        "schema_hash": { "type": "string" },
                        "row_count": { "type": "integer" },
                        "status": { "type": "string" },
                        "splits": { "type": "array", "items": { "$ref": "#/components/schemas/DatasetSplit" } },
                        "fields": { "type": "array", "items": { "$ref": "#/components/schemas/SchemaField" } },
                        "mappings": { "type": "array", "items": { "$ref": "#/components/schemas/FieldMapping" } }
                    }
                },
                "DatasetRegistrationRequest": {
                    "type": "object",
                    "required": ["source_key", "display_name", "business_domain", "owner", "description", "dataset_key", "dataset_version", "sample_grain", "label_column", "entity_keys", "manifest_uri", "schema_uri", "profile_uri", "storage_format", "schema_hash", "row_count", "status", "splits", "fields"],
                    "properties": {
                        "source_key": { "type": "string" },
                        "display_name": { "type": "string" },
                        "business_domain": { "type": "string" },
                        "owner": { "type": "string" },
                        "description": { "type": "string" },
                        "dataset_key": { "type": "string" },
                        "dataset_version": { "type": "string" },
                        "sample_grain": { "type": "string" },
                        "label_column": { "type": "string" },
                        "entity_keys": { "type": "array", "items": { "type": "string" } },
                        "manifest_uri": { "type": "string" },
                        "schema_uri": { "type": "string" },
                        "profile_uri": { "type": "string" },
                        "storage_format": { "type": "string", "const": "parquet" },
                        "schema_hash": { "type": "string" },
                        "row_count": { "type": "integer" },
                        "status": { "type": "string" },
                        "splits": { "type": "array", "items": { "$ref": "#/components/schemas/DatasetSplit" } },
                        "fields": { "type": "array", "items": { "$ref": "#/components/schemas/SchemaField" } }
                    }
                },
                "DatasetListResponse": {
                    "type": "object",
                    "required": ["datasets"],
                    "properties": {
                        "datasets": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DatasetRecord" }
                        }
                    }
                },
                "FieldMappingRequest": {
                    "type": "object",
                    "required": ["external_field", "canonical_target", "transform_kind", "transform_json", "status"],
                    "properties": {
                        "external_field": { "type": "string" },
                        "canonical_target": { "type": "string" },
                        "feature_name": { "type": ["string", "null"] },
                        "transform_kind": { "type": "string" },
                        "transform_json": { "type": "object" },
                        "status": { "type": "string" }
                    }
                },
                "FieldMappingResponse": {
                    "type": "object",
                    "required": ["mapping"],
                    "properties": {
                        "mapping": { "$ref": "#/components/schemas/FieldMapping" }
                    }
                },
                "ModelVersion": {
                    "type": "object",
                    "required": ["model_key", "version", "model_type", "runtime_kind", "execution_provider", "status"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "version": { "type": "string" },
                        "model_type": { "type": "string" },
                        "runtime_kind": { "type": "string" },
                        "execution_provider": { "type": "string" },
                        "status": { "type": "string" },
                        "artifact_uri": { "type": ["string", "null"] },
                        "endpoint_url": { "type": ["string", "null"] }
                    }
                },
                "ModelListResponse": {
                    "type": "object",
                    "required": ["models"],
                    "properties": {
                        "models": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelVersion" }
                        }
                    }
                },
                "ModelPerformanceResponse": {
                    "type": "object",
                    "required": ["model_key", "data_status", "scored_runs", "average_score", "high_risk_count"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "data_status": { "type": "string", "enum": ["empty", "ready"] },
                        "scored_runs": { "type": "integer" },
                        "average_score": { "type": "number" },
                        "high_risk_count": { "type": "integer" },
                        "latest_scored_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "KnowledgeCase": {
                    "type": "object",
                    "required": ["case_id", "title", "fwa_type", "diagnosis_code", "provider_region", "summary", "outcome", "tags", "evidence_refs"],
                    "properties": {
                        "case_id": { "type": "string" },
                        "title": { "type": "string" },
                        "fwa_type": { "type": "string" },
                        "diagnosis_code": { "type": "string" },
                        "provider_region": { "type": "string" },
                        "provider_type": { "type": "string" },
                        "summary": { "type": "string" },
                        "outcome": { "type": "string" },
                        "tags": { "type": "array", "items": { "type": "string" } },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "KnowledgeCaseListResponse": {
                    "type": "object",
                    "required": ["cases"],
                    "properties": {
                        "cases": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/KnowledgeCase" }
                        }
                    }
                },
                "SimilarCaseSearchRequest": {
                    "type": "object",
                    "required": ["diagnosis_code", "provider_region", "tags"],
                    "properties": {
                        "claim_id": { "type": ["string", "null"] },
                        "diagnosis_code": { "type": "string" },
                        "provider_region": { "type": "string" },
                        "tags": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "SimilarCaseSearchResponse": {
                    "type": "object",
                    "required": ["results"],
                    "properties": {
                        "results": {
                            "type": "array",
                            "items": { "type": "object" }
                        }
                    }
                },
                "AgentInvestigationRequest": {
                    "type": "object",
                    "required": ["claim_id", "risk_score", "rag", "top_reasons", "similar_case_query"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rag": { "type": "string" },
                        "top_reasons": { "type": "array", "items": { "type": "string" } },
                        "similar_case_query": { "$ref": "#/components/schemas/SimilarCaseSearchRequest" }
                    }
                },
                "AgentInvestigationResponse": {
                    "type": "object",
                    "required": ["agent_run_id", "decision_boundary", "risk_summary", "findings", "investigation_checklist", "similar_cases", "qa_opinion_draft", "evidence_refs"],
                    "properties": {
                        "agent_run_id": { "type": "string" },
                        "decision_boundary": { "type": "string", "const": "assistive_only" },
                        "risk_summary": { "type": "string" },
                        "findings": { "type": "array", "items": { "type": "object" } },
                        "investigation_checklist": { "type": "array", "items": { "type": "string" } },
                        "similar_cases": { "type": "array", "items": { "type": "object" } },
                        "qa_opinion_draft": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                }
            }
        }
    }))
}
