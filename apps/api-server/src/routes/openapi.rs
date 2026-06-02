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
                            "description": "Service is healthy",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/HealthResponse" }
                                }
                            }
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
            "/api/v1/inbox/claims/normalize": {
                "post": {
                    "summary": "Normalize a raw TPA claim-system payload before scoring",
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
                                    "$ref": "#/components/schemas/InboxNormalizeRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Normalized inbox context with validation warnings and data-quality signals",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/InboxNormalizeResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Rejected inbox payload with structured field-level validation errors",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/InboxNormalizeResponse"
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
            "/api/v1/ops/rules/performance": {
                "get": {
                    "summary": "Get rule performance and ROI metrics",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Per-rule operational performance metrics",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RulePerformanceResponse" }
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
            "/api/v1/ops/rules/{rule_id}/promotion-gates": {
                "get": {
                    "summary": "Get rule promotion gates before routing impact",
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
                            "description": "Rule promotion gate summary",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RulePromotionGatesResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules/{rule_id}/promotion-reviews": {
                "post": {
                    "summary": "Record a rule promotion review decision",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "rule_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitRulePromotionReviewRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded rule promotion review",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RulePromotionReview" }
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
            "/api/v1/ops/rules/{rule_id}/submit": {
                "post": {
                    "summary": "Submit a draft rule for governance review",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": rule_lifecycle_parameters(),
                    "requestBody": rule_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Rule submitted for review",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleLifecycleResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules/{rule_id}/approve": {
                "post": {
                    "summary": "Approve a submitted rule",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": rule_lifecycle_parameters(),
                    "requestBody": rule_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Rule approved",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleLifecycleResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules/{rule_id}/publish": {
                "post": {
                    "summary": "Publish an approved rule into production routing",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": rule_lifecycle_parameters(),
                    "requestBody": rule_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Rule published",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleLifecycleResponse" }
                                }
                            }
                        },
                        "409": {
                            "description": "Rule approval or promotion gates block publication",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules/{rule_id}/rollback": {
                "post": {
                    "summary": "Rollback an active rule out of production routing",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": rule_lifecycle_parameters(),
                    "requestBody": rule_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Rule rolled back to approved status",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleLifecycleResponse" }
                                }
                            }
                        },
                        "409": {
                            "description": "Rule is not active and cannot be rolled back",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
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
            "/api/v1/ops/feature-sets": {
                "post": {
                    "summary": "Register a Parquet feature set version",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/FeatureSetRegistrationRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Registered feature set",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/FeatureSet" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/factors/readiness": {
                "get": {
                    "summary": "Summarize factor factory readiness across registered datasets",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Factor readiness summary",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/FactorReadinessResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/model-datasets": {
                "post": {
                    "summary": "Register a model dataset version",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelDatasetRegistrationRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Registered model dataset",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelDataset" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/model-evaluations": {
                "get": {
                    "summary": "List model evaluation metrics",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Model evaluation metric list",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelEvaluationListResponse" }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Register model evaluation metrics for a model dataset",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ModelEvaluationRegistrationRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Registered model evaluation",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelEvaluationResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/model-evaluations/{evaluation_run_id}": {
                "get": {
                    "summary": "Get model evaluation metrics",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "evaluation_run_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Model evaluation metrics"
                        }
                    }
                }
            },
            "/api/v1/ops/dashboard/summary": {
                "get": {
                    "summary": "Get management dashboard summary metrics",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Dashboard summary metrics for FWA operations",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/DashboardSummaryResponse" }
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
            "/api/v1/ops/webhook-events": {
                "get": {
                    "summary": "List webhook events for TPA and operations integrations",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Webhook event outbox",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/WebhookEventListResponse" }
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
            "/api/v1/ops/webhook-events/{event_id}/delivery-attempts": {
                "post": {
                    "summary": "Record a webhook delivery attempt",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "event_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitWebhookDeliveryAttemptRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded webhook delivery attempt",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/WebhookDeliveryAttempt" }
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
            "/api/v1/ops/alerts": {
                "get": {
                    "summary": "List operational alerts for high-risk routing and SLA follow-up",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Operational alert feed",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/OpsAlertListResponse" }
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
            "/api/v1/ops/leads": {
                "get": {
                    "summary": "List FWA leads generated from scoring signals",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Lead lifecycle records",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/LeadListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/leads/{lead_id}/triage": {
                "post": {
                    "summary": "Triage a lead into an investigation case or non-case disposition",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "lead_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/TriageLeadRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Updated lead disposition and optional investigation case",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/TriageLeadResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/cases": {
                "get": {
                    "summary": "List FWA investigation cases",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Investigation case records",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CaseListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/cases/{case_id}/status": {
                "post": {
                    "summary": "Update investigation case workflow status",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "case_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/UpdateCaseStatusRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Updated investigation case status with audit id",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/UpdateCaseStatusResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/audit-samples": {
                "get": {
                    "summary": "List governed audit sampling runs",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Audit sampling records",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/AuditSampleListResponse" }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create a deterministic audit sample from FWA leads",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateAuditSampleRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Created audit sampling record",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/AuditSampleRecord" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/audit-events": {
                "get": {
                    "summary": "List global audit events for governance review",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "limit",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "integer", "minimum": 1, "maximum": 200 }
                        },
                        {
                            "name": "event_group",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string", "enum": ["governance"] }
                        },
                        {
                            "name": "event_type",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "actor_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "run_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "claim_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "rule_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "rule_version",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "model_key",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "model_version",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "routing_policy_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "routing_policy_version",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "review_mode",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] }
                        },
                        {
                            "name": "feedback_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "qa_case_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "sample_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "agent_run_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "dataset_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "feature_set_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "model_dataset_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "evaluation_run_id",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "has_canonical_trace",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "boolean" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Recent audit events across claims, rules, models, routing policies, QA, and Agent runs",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/AuditEventListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/api-calls": {
                "get": {
                    "summary": "List audit-backed TPA API call records",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "limit",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "integer", "minimum": 1, "maximum": 200 }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "TPA API call records derived from audit events",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ApiCallListResponse" }
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
            "/api/v1/ops/agent-runs": {
                "get": {
                    "summary": "List agent run logs for governance review",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Agent run logs with evidence-backed steps",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/AgentRunLogListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/agent-runs/{agent_run_id}/approvals": {
                "post": {
                    "summary": "Submit a human approval decision for an agent run",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "agent_run_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitAgentApprovalRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Agent approval decision accepted and audited",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/SubmitAgentApprovalResponse" }
                                }
                            }
                        },
                        "409": {
                            "description": "Agent approval is not pending or has already been decided",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
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
            "/api/v1/ops/rules/candidates": {
                "post": {
                    "summary": "Save a discovered candidate rule into the rule library",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SaveRuleCandidateRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Saved draft rule candidate",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleDetailResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules/discover": {
                "post": {
                    "summary": "Discover candidate rules from labeled sample claims",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/RuleDiscoveryRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Candidate rules and backtest-style discovery metrics",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleDiscoveryResponse" }
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
            "/api/v1/ops/routing-policies": {
                "get": {
                    "summary": "List active routing policy versions",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Routing policy versions and thresholds",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RoutingPolicyListResponse" }
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
                },
                "post": {
                    "summary": "Save a draft routing policy candidate",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SaveRoutingPolicyCandidateRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Draft routing policy candidate",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RoutingPolicyRecord" }
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
            "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/submit": {
                "post": {
                    "summary": "Submit a draft routing policy for governance review",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": routing_policy_lifecycle_parameters(),
                    "requestBody": routing_policy_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Submitted routing policy",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RoutingPolicyRecord" }
                                }
                            }
                        },
                        "409": {
                            "description": "Routing policy is not in the required source status",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/promotion-gates": {
                "get": {
                    "summary": "Evaluate promotion gates for a routing policy",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": routing_policy_lifecycle_parameters(),
                    "responses": {
                        "200": {
                            "description": "Routing policy promotion gates",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RoutingPolicyPromotionGatesResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/approve": {
                "post": {
                    "summary": "Approve a submitted routing policy",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": routing_policy_lifecycle_parameters(),
                    "requestBody": routing_policy_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Approved routing policy",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RoutingPolicyRecord" }
                                }
                            }
                        },
                        "409": {
                            "description": "Routing policy is not in the required source status",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/activate": {
                "post": {
                    "summary": "Activate an approved routing policy for scoring",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": routing_policy_lifecycle_parameters(),
                    "requestBody": routing_policy_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Activated routing policy",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RoutingPolicyRecord" }
                                }
                            }
                        },
                        "409": {
                            "description": "Routing policy is not approved for activation",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/routing-policies/{policy_id}/{review_mode}/{version}/rollback": {
                "post": {
                    "summary": "Roll back an active routing policy to approved status",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": routing_policy_lifecycle_parameters(),
                    "requestBody": routing_policy_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Rolled back routing policy",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RoutingPolicyRecord" }
                                }
                            }
                        },
                        "409": {
                            "description": "Routing policy is not active",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/providers/risk-summary": {
                "get": {
                    "summary": "Summarize Provider profile and graph-risk review signals",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Provider risk summary",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ProviderRiskSummaryResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/medical-review/queue": {
                "get": {
                    "summary": "List claims that require medical review from clinical evidence audit events",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "limit",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "integer", "minimum": 1, "maximum": 200 }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Medical review queue",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/MedicalReviewQueueResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/medical-review/results": {
                "post": {
                    "summary": "Record a medical review result with evidence references",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitMedicalReviewResultRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Medical review result recorded",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/MedicalReviewResultResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/fwa-schemes": {
                "get": {
                    "summary": "List governed FWA scheme taxonomy and evidence requirements",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "FWA scheme taxonomy",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/FwaSchemeListResponse" }
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
            "/api/v1/ops/models/{model_key}/promotion-gates": {
                "get": {
                    "summary": "Get model promotion gates before routing impact",
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
                            "description": "Model promotion gate summary",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelPromotionGatesResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/retraining-readiness": {
                "get": {
                    "summary": "Get model retraining readiness from drift, labels, feedback, and source data quality",
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
                            "description": "Model retraining readiness summary",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelRetrainingReadinessResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/retraining-jobs": {
                "get": {
                    "summary": "List model retraining jobs",
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
                            "description": "Model retraining jobs",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelRetrainingJobListResponse" }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Queue a model retraining job from readiness",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateModelRetrainingJobRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Queued model retraining job",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelRetrainingJob" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/model-retraining-jobs/{job_id}/status": {
                "post": {
                    "summary": "Update model retraining job status",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "job_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/UpdateModelRetrainingJobStatusRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Updated model retraining job",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelRetrainingJob" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/model-retraining-jobs/claim-next": {
                "post": {
                    "summary": "Claim the next queued model retraining job for a worker",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ClaimModelRetrainingJobRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Claimed model retraining job",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelRetrainingJob" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/model-retraining-jobs/{job_id}/output": {
                "post": {
                    "summary": "Register model retraining output, candidate version, and validation evaluation",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "job_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CompleteModelRetrainingJobRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Completed model retraining job output",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CompleteModelRetrainingJobResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/promotion-reviews": {
                "post": {
                    "summary": "Record a model promotion review decision",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitModelPromotionReviewRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded model promotion review",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelPromotionReview" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/activate": {
                "post": {
                    "summary": "Activate the latest governed model version for production routing",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": model_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Model lifecycle status after activation",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelLifecycleResponse" }
                                }
                            }
                        },
                        "409": {
                            "description": "Model activation is blocked by governance gates",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/rollback": {
                "post": {
                    "summary": "Roll back an active model to the previous active version",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": model_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Model lifecycle status after rollback",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelLifecycleResponse" }
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
                },
                "post": {
                    "summary": "Publish confirmed FWA knowledge case",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/PublishKnowledgeCaseRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Published knowledge case",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/PublishKnowledgeCaseResponse" }
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
                        },
                        "400": {
                            "description": "Invalid similar case search query",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
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
            },
            "/api/v1/members/{member_id}/profile-summary": {
                "get": {
                    "summary": "Get member policy and claim profile summary",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "member_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Member profile summary",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/MemberProfileSummaryResponse" }
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
                        },
                        "404": {
                            "description": "Member was not found",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/investigations/results": {
                "post": {
                    "summary": "Write back a pilot investigation result",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/InvestigationResultRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Investigation result accepted and audited",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/PilotWritebackResponse" }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid investigation result writeback",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
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
                        },
                        "404": {
                            "description": "Linked investigation case was not found",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/qa/results": {
                "post": {
                    "summary": "Write back a pilot QA result",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/QaResultRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "QA result accepted and audited",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/PilotWritebackResponse" }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid QA result writeback",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
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
            "/api/v1/ops/qa/feedback-items": {
                "get": {
                    "summary": "List QA feedback items for rule and model operators",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "status",
                            "in": "query",
                            "required": false,
                            "schema": {
                                "type": "string",
                                "enum": ["open", "in_progress", "resolved", "dismissed"]
                            }
                        },
                        {
                            "name": "feedback_target",
                            "in": "query",
                            "required": false,
                            "schema": {
                                "type": "string",
                                "enum": ["rules", "model", "models", "features", "provider_profile", "workflow", "tpa"]
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "QA feedback items created from QA review writeback",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/QaFeedbackItemListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/qa/feedback-items/{feedback_id}/status": {
                "post": {
                    "summary": "Update QA feedback item status",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "feedback_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/UpdateQaFeedbackStatusRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "QA feedback item status updated and audited",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/UpdateQaFeedbackStatusResponse" }
                                }
                            }
                        },
                        "404": {
                            "description": "QA feedback item was not found",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/qa/queue": {
                "get": {
                    "summary": "List QA review queue items from audit samples",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "QA queue items selected by audit sampling",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/QaQueueListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/qa/queue-summary": {
                "get": {
                    "summary": "Summarize the open QA feedback queue",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "QA queue backlog and routing summary",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/QaQueueSummaryResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/labels": {
                "get": {
                    "summary": "List governed outcome labels from human review writeback",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Governed labels derived from human review outcomes",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/OutcomeLabelListResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/audit/claims/{claim_id}": {
                "get": {
                    "summary": "Get pilot audit history for a claim",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "claim_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Claim audit history",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ClaimAuditHistoryResponse" }
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
                "InboxNormalizeRequest": {
                    "type": "object",
                    "description": "Customer-specific raw claim intake payload. MVP supports the AiClaim Core reportCase envelope.",
                    "required": ["systemCode", "transNo", "reportCase"],
                    "properties": {
                        "systemCode": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Source system code bound to the authenticated API key."
                        },
                        "transNo": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Source transaction id used with reportNo for idempotency."
                        },
                        "transDate": {
                            "type": ["string", "null"],
                            "description": "Source transaction timestamp when present."
                        },
                        "reportCase": {
                            "type": "object",
                            "description": "Raw source claim case payload. It may contain medical records, policy, invoice, product, and liability lists."
                        }
                    },
                    "additionalProperties": true
                },
                "InboxNormalizeResponse": {
                    "type": "object",
                    "required": [
                        "run_id",
                        "audit_id",
                        "mapping_version",
                        "validation_result",
                        "scoring_ready",
                        "validation_errors",
                        "canonical_claim_context",
                        "data_quality_signals",
                        "evidence_refs"
                    ],
                    "properties": {
                        "run_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "external_message_id": { "type": ["string", "null"] },
                        "idempotency_key": { "type": ["string", "null"] },
                        "mapping_version": { "type": "string" },
                        "validation_result": {
                            "type": "string",
                            "enum": ["accepted", "accepted_with_warnings", "rejected"]
                        },
                        "scoring_ready": { "type": "boolean" },
                        "raw_payload_ref": { "type": ["string", "null"] },
                        "validation_errors": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxValidationError" }
                        },
                        "canonical_claim_context": {
                            "$ref": "#/components/schemas/InboxCanonicalClaimContext"
                        },
                        "data_quality_signals": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxCanonicalClaimContext": {
                    "type": "object",
                    "required": [
                        "claim_header",
                        "member_policy_snapshot",
                        "provider_snapshot",
                        "itemized_bill_lines",
                        "document_evidence"
                    ],
                    "properties": {
                        "claim_header": { "$ref": "#/components/schemas/InboxClaimHeader" },
                        "member_policy_snapshot": { "$ref": "#/components/schemas/InboxMemberPolicySnapshot" },
                        "provider_snapshot": { "$ref": "#/components/schemas/InboxProviderSnapshot" },
                        "itemized_bill_lines": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxBillLine" }
                        },
                        "document_evidence": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxDocumentEvidence" }
                        }
                    }
                },
                "InboxClaimHeader": {
                    "type": "object",
                    "properties": {
                        "external_claim_id": { "type": "string" },
                        "source_system": { "type": "string" },
                        "service_date": { "type": ["string", "null"], "format": "date" },
                        "receive_date": { "type": ["string", "null"], "format": "date" },
                        "accident_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "service_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "receive_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "accident_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "accident_reason": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "currency": { "type": "string" },
                        "total_amount": { "type": ["number", "null"] }
                    }
                },
                "InboxMemberPolicySnapshot": {
                    "type": "object",
                    "properties": {
                        "masked_member_id": { "type": ["string", "null"] },
                        "masked_certificate_id": { "type": ["string", "null"] },
                        "certificate_type": { "type": ["string", "null"] },
                        "member_gender": { "type": ["string", "null"] },
                        "member_birth_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "member_birth_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "policy_id": { "type": ["string", "null"] },
                        "product_code": { "type": ["string", "null"] },
                        "liability_code": { "type": ["string", "null"] },
                        "liability_name": { "type": ["string", "null"] },
                        "policy_type": { "type": ["string", "null"] },
                        "policy_first_apply_date": { "type": ["string", "null"], "format": "date" },
                        "policy_first_apply_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "insured_with_social_insurance": { "type": ["boolean", "null"] },
                        "coverage_limit": { "type": ["number", "null"] },
                        "coverage_start_date": { "type": ["string", "null"], "format": "date" },
                        "coverage_end_date": { "type": ["string", "null"], "format": "date" },
                        "coverage_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "coverage_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_start_date": { "type": ["string", "null"], "format": "date" },
                        "liability_claim_start_date": { "type": ["string", "null"], "format": "date" },
                        "waiting_period_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_claim_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "product_liabilities": {
                            "type": "array",
                            "description": "All product and claim-liability windows from the source policies, preserving coverage and waiting-period candidates before scoring.",
                            "items": { "$ref": "#/components/schemas/InboxProductLiability" }
                        }
                    }
                },
                "InboxProductLiability": {
                    "type": "object",
                    "properties": {
                        "policy_id": { "type": ["string", "null"] },
                        "product_id": { "type": ["string", "null"] },
                        "product_code": { "type": ["string", "null"] },
                        "product_name": { "type": ["string", "null"] },
                        "plan_code": { "type": ["string", "null"] },
                        "plan_version": { "type": ["string", "null"] },
                        "product_start_date": { "type": ["string", "null"], "format": "date" },
                        "product_end_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "product_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "product_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_id": { "type": ["string", "null"] },
                        "liability_code": { "type": ["string", "null"] },
                        "liability_name": { "type": ["string", "null"] },
                        "liability_start_date": { "type": ["string", "null"], "format": "date" },
                        "liability_claim_start_date": { "type": ["string", "null"], "format": "date" },
                        "waiting_period_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_claim_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "is_serious_disease_liability": { "type": ["boolean", "null"] },
                        "main_liability": { "type": ["boolean", "null"] },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxProviderSnapshot": {
                    "type": "object",
                    "properties": {
                        "provider_code": { "type": ["string", "null"] },
                        "name": { "type": ["string", "null"] },
                        "class": { "type": ["string", "null"] },
                        "type": { "type": ["string", "null"] },
                        "city": { "type": ["string", "null"] },
                        "province": { "type": ["string", "null"] },
                        "network_flags": { "$ref": "#/components/schemas/InboxProviderNetworkFlags" }
                    }
                },
                "InboxProviderNetworkFlags": {
                    "type": "object",
                    "properties": {
                        "is_hospital_institution": { "type": ["boolean", "null"] },
                        "primary_care": { "type": ["boolean", "null"] },
                        "red_flag": { "type": ["string", "null"] }
                    }
                },
                "InboxBillLine": {
                    "type": "object",
                    "properties": {
                        "invoice_id": { "type": ["string", "null"] },
                        "diagnosis_list": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxDiagnosis" }
                        },
                        "fee_category": { "type": ["string", "null"] },
                        "item_name": { "type": ["string", "null"] },
                        "amount": { "type": ["number", "null"] },
                        "self_pay": { "type": ["number", "null"] },
                        "own_expense": { "type": ["number", "null"] },
                        "social_insurance_amount": { "type": ["number", "null"] },
                        "medical_category": { "type": ["string", "null"] },
                        "invoice_bill_type": { "type": ["string", "null"] },
                        "invoice_document_type": { "type": ["string", "null"] },
                        "social_insurance_type": { "type": ["string", "null"] },
                        "department": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "invoice_claim_nature": { "type": ["string", "null"] },
                        "invoice_start_date": { "type": ["string", "null"], "format": "date" },
                        "invoice_end_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "invoice_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "invoice_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "invoice_social_insurance_amount": { "type": ["number", "null"] },
                        "invoice_self_pay_amount": { "type": ["number", "null"] },
                        "invoice_own_expense_amount": { "type": ["number", "null"] },
                        "invoice_other_amount": { "type": ["number", "null"] },
                        "invoice_provider_code": { "type": ["string", "null"] },
                        "invoice_provider_name": { "type": ["string", "null"] },
                        "invoice_provider_class": { "type": ["string", "null"] },
                        "invoice_provider_type": { "type": ["string", "null"] },
                        "invoice_provider_city": { "type": ["string", "null"] },
                        "invoice_provider_province": { "type": ["string", "null"] },
                        "invoice_is_hospital_institution": { "type": ["boolean", "null"] },
                        "invoice_primary_care": { "type": ["boolean", "null"] },
                        "invoice_red_flag": { "type": ["string", "null"] },
                        "fee_group_amount": { "type": ["number", "null"] },
                        "fee_group_other_amount": { "type": ["number", "null"] },
                        "medicare_prorated": { "type": ["string", "null"] },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxDiagnosis": {
                    "type": "object",
                    "properties": {
                        "code": { "type": ["string", "null"] },
                        "name": { "type": ["string", "null"] }
                    }
                },
                "InboxDocumentEvidence": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": ["string", "null"] },
                        "department": { "type": ["string", "null"] },
                        "diagnosis": { "type": ["string", "null"] },
                        "claim_nature": { "type": ["string", "null"] },
                        "medical_record_type": { "type": ["string", "null"] },
                        "chief_complaint": { "type": ["string", "null"] },
                        "current_medical_history": { "type": ["string", "null"] },
                        "past_history": { "type": ["string", "null"] },
                        "extracted_diagnosis": { "type": ["string", "null"] },
                        "extracted_procedure": { "type": ["string", "null"] },
                        "extracted_prescription": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "visit_date": { "type": ["string", "null"], "format": "date" },
                        "first_happen_date": { "type": ["string", "null"], "format": "date" },
                        "operation_start_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "visit_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "first_happen_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "operation_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "medical_record_text": { "type": ["string", "null"] },
                        "source_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxValidationError": {
                    "type": "object",
                    "required": ["field_path", "severity", "remediation"],
                    "properties": {
                        "field_path": { "type": "string" },
                        "severity": { "type": "string", "enum": ["error", "warning"] },
                        "remediation": { "type": "string" }
                    }
                },
                "ScoreClaimRequest": {
                    "oneOf": [
                        {
                            "$ref": "#/components/schemas/ClaimIdScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/FullPayloadScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/CanonicalContextScoreClaimRequest"
                        }
                    ]
                },
                "ClaimIdScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "claim_id"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "claim_id": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Existing claim id to load from FWA storage."
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] },
                            { "required": ["documents"] },
                            { "required": ["provider_profile"] },
                            { "required": ["provider_relationships"] },
                            { "required": ["canonical_claim_context"] }
                        ]
                    }
                },
                "CanonicalContextScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "canonical_claim_context"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        },
                        "canonical_claim_context": {
                            "$ref": "#/components/schemas/InboxCanonicalClaimContext"
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim_id"] },
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] },
                            { "required": ["documents"] },
                            { "required": ["provider_profile"] },
                            { "required": ["provider_relationships"] }
                        ]
                    }
                },
                "FullPayloadScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "claim"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "claim": {
                            "$ref": "#/components/schemas/FullClaimPayload"
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
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
                        },
                        "documents": {
                            "type": "array",
                            "description": "Clinical documents linked to claim items for evidence sufficiency review.",
                            "items": {
                                "$ref": "#/components/schemas/DocumentPayload"
                            }
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfilePayload"
                        },
                        "provider_relationships": {
                            "$ref": "#/components/schemas/ProviderRelationshipGraphPayload"
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim_id"] },
                            { "required": ["canonical_claim_context"] }
                        ]
                    }
                },
                "FullClaimPayload": {
                    "type": "object",
                    "required": ["external_claim_id", "claim_amount", "currency"],
                    "properties": {
                        "external_claim_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "claim_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Positive decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        },
                        "service_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "diagnosis_code": {
                            "type": "string",
                            "minLength": 1
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
                        },
                        "documents": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/DocumentPayload"
                            }
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfilePayload"
                        },
                        "provider_relationships": {
                            "$ref": "#/components/schemas/ProviderRelationshipGraphPayload"
                        }
                    }
                },
                "ClaimItemPayload": {
                    "type": "object",
                    "required": ["item_code", "item_type", "description", "quantity", "unit_amount", "total_amount"],
                    "properties": {
                        "item_code": {
                            "type": "string",
                            "minLength": 1
                        },
                        "item_type": {
                            "type": "string",
                            "minLength": 1
                        },
                        "description": {
                            "type": "string",
                            "minLength": 1
                        },
                        "quantity": {
                            "type": "integer",
                            "minimum": 1
                        },
                        "unit_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Non-negative decimal string."
                        },
                        "total_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Non-negative decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "MemberPayload": {
                    "type": "object",
                    "required": ["external_member_id"],
                    "properties": {
                        "external_member_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "dob": {
                            "type": "string",
                            "format": "date"
                        },
                        "gender": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "PolicyPayload": {
                    "type": "object",
                    "required": ["external_policy_id", "coverage_start_date", "coverage_end_date", "coverage_limit"],
                    "properties": {
                        "external_policy_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "product_code": {
                            "type": "string",
                            "minLength": 1
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
                            "format": "decimal",
                            "description": "Positive decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "ProviderPayload": {
                    "type": "object",
                    "required": ["external_provider_id", "name", "provider_type", "region"],
                    "properties": {
                        "external_provider_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "name": {
                            "type": "string",
                            "minLength": 1
                        },
                        "provider_type": {
                            "type": "string",
                            "minLength": 1
                        },
                        "region": {
                            "type": "string",
                            "minLength": 1
                        },
                        "risk_tier": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High"]
                        }
                    }
                },
                "DocumentPayload": {
                    "type": "object",
                    "required": ["external_document_id", "document_type"],
                    "properties": {
                        "external_document_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "document_type": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Examples: medical_record, clinical_order, radiology_report, prescription, lab_result"
                        },
                        "linked_item_codes": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "minLength": 1
                            }
                        }
                    }
                },
                "ProviderProfilePayload": {
                    "type": "object",
                    "required": ["windows"],
                    "properties": {
                        "specialty": { "type": "string" },
                        "network_status": { "type": "string" },
                        "windows": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "$ref": "#/components/schemas/ProviderProfileWindowPayload" }
                        }
                    }
                },
                "ProviderProfileWindowPayload": {
                    "type": "object",
                    "required": [
                        "window_days",
                        "claim_count",
                        "total_claim_amount",
                        "high_cost_item_ratio",
                        "diagnosis_procedure_mismatch_rate",
                        "peer_amount_percentile",
                        "peer_frequency_percentile",
                        "review_failure_count",
                        "confirmed_fwa_count",
                        "false_positive_count"
                    ],
                    "properties": {
                        "window_days": { "type": "integer", "enum": [30, 90, 180] },
                        "claim_count": { "type": "integer", "minimum": 0 },
                        "total_claim_amount": { "type": "string", "format": "decimal", "description": "Non-negative decimal string." },
                        "high_cost_item_ratio": { "type": "number", "minimum": 0, "maximum": 1 },
                        "diagnosis_procedure_mismatch_rate": { "type": "number", "minimum": 0, "maximum": 1 },
                        "peer_amount_percentile": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "peer_frequency_percentile": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "review_failure_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 }
                    }
                },
                "ProviderRelationshipGraphPayload": {
                    "type": "object",
                    "required": [
                        "high_risk_neighbor_ratio",
                        "provider_patient_overlap_score",
                        "connected_confirmed_fwa_count"
                    ],
                    "properties": {
                        "high_risk_neighbor_ratio": { "type": "number", "minimum": 0, "maximum": 1 },
                        "provider_patient_overlap_score": { "type": "number", "minimum": 0, "maximum": 1 },
                        "referral_concentration_score": { "type": ["number", "null"], "minimum": 0, "maximum": 1 },
                        "connected_confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "network_component_risk_score": { "type": ["integer", "null"], "minimum": 0, "maximum": 100 },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ScoreClaimResponse": {
                    "type": "object",
                    "required": [
                        "run_id",
                        "audit_id",
                        "claim_id",
                        "review_mode",
                        "risk_score",
                        "rag",
                        "risk_level",
                        "recommended_action",
                        "confidence_score",
                        "confidence",
                        "routing_reason",
                        "routing_policy",
                        "scores",
                        "model_score",
                        "alerts",
                        "top_reasons",
                        "layers",
                        "clinical_evidence",
                        "provider_profile",
                        "provider_relationships",
                        "similar_cases",
                        "feature_values",
                        "evidence_refs",
                        "agent_investigation_prefill"
                    ],
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
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"]
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
                        "risk_level": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High", "Critical"]
                        },
                        "recommended_action": {
                            "type": "string",
                            "enum": [
                                "StandardProcessing",
                                "QaSample",
                                "ManualReview",
                                "RequestEvidence",
                                "EscalateInvestigation",
                                "PostPaymentAudit",
                                "ProviderReview",
                                "RecoveryReview"
                            ]
                        },
                        "confidence_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "confidence": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High"]
                        },
                        "routing_reason": {
                            "type": "string"
                        },
                        "routing_policy": {
                            "$ref": "#/components/schemas/RoutingPolicy"
                        },
                        "scores": {
                            "$ref": "#/components/schemas/ScoreBreakdown"
                        },
                        "model_score": {
                            "$ref": "#/components/schemas/ModelScore"
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
                                "type": "string",
                                "minLength": 1
                            }
                        },
                        "layers": {
                            "type": "array",
                            "minItems": 7,
                            "maxItems": 7,
                            "items": {
                                "$ref": "#/components/schemas/DetectionLayerScore"
                            }
                        },
                        "clinical_evidence": {
                            "$ref": "#/components/schemas/ClinicalEvidenceAssessment"
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfileAssessment"
                        },
                        "provider_relationships": {
                            "$ref": "#/components/schemas/ProviderRelationshipGraphAssessment"
                        },
                        "similar_cases": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/SimilarCase"
                            }
                        },
                        "feature_values": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/FeatureValue"
                            }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "oneOf": [
                                    { "type": "object" },
                                    { "type": "string" }
                                ]
                            }
                        },
                        "agent_investigation_prefill": {
                            "$ref": "#/components/schemas/AgentInvestigationPrefill"
                        }
                    }
                },
                "ModelScore": {
                    "type": "object",
                    "required": [
                        "model_key",
                        "model_version",
                        "runtime_kind",
                        "execution_provider",
                        "score",
                        "label",
                        "explanations",
                        "metadata",
                        "latency_ms"
                    ],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "runtime_kind": { "type": "string" },
                        "execution_provider": { "type": "string" },
                        "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "label": { "type": "string" },
                        "explanations": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelExplanation" }
                        },
                        "metadata": {
                            "type": "object",
                            "properties": {
                                "fraud_probability": { "type": "number", "minimum": 0, "maximum": 1 },
                                "abuse_probability": { "type": "number", "minimum": 0, "maximum": 1 },
                                "waste_probability": { "type": "number", "minimum": 0, "maximum": 1 }
                            },
                            "additionalProperties": true
                        },
                        "latency_ms": { "type": "integer", "minimum": 0 }
                    }
                },
                "ModelExplanation": {
                    "type": "object",
                    "required": ["feature", "direction", "contribution", "reason"],
                    "properties": {
                        "feature": { "type": "string" },
                        "direction": { "type": "string" },
                        "contribution": { "type": "number" },
                        "reason": { "type": "string" }
                    }
                },
                "FeatureValue": {
                    "type": "object",
                    "required": ["name", "version", "value", "evidence_refs"],
                    "properties": {
                        "name": { "type": "string" },
                        "version": { "type": "integer", "minimum": 0 },
                        "value": {},
                        "evidence_refs": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/EvidenceRef" }
                        }
                    }
                },
                "EvidenceRef": {
                    "type": "object",
                    "required": ["entity_type", "entity_id", "field"],
                    "properties": {
                        "entity_type": { "type": "string" },
                        "entity_id": { "type": "string" },
                        "field": { "type": "string" }
                    }
                },
                "DetectionLayerScore": {
                    "type": "object",
                    "required": ["layer_id", "name", "score", "status", "reason"],
                    "properties": {
                        "layer_id": {
                            "type": "string",
                            "enum": [
                                "L1_PEER_BENCHMARK",
                                "L2_RULE_DETECTION",
                                "L3_UNSUPERVISED_ANOMALY",
                                "L4_SUPERVISED_ML",
                                "L5_MEDICAL_REASONABLENESS",
                                "L6_PROVIDER_GRAPH_RISK",
                                "L7_RISK_FUSION_ROUTING"
                            ]
                        },
                        "name": { "type": "string" },
                        "score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "status": {
                            "type": "string",
                            "enum": ["active", "baseline", "no_data"]
                        },
                        "reason": { "type": "string" }
                    }
                },
                "RoutingPolicy": {
                    "type": "object",
                    "required": ["policy_id", "version", "review_mode", "risk_thresholds", "confidence_thresholds", "provider_review_threshold"],
                    "properties": {
                        "policy_id": { "type": "string", "minLength": 1 },
                        "version": { "type": "integer", "minimum": 1 },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "risk_thresholds": { "$ref": "#/components/schemas/RiskThresholds" },
                        "confidence_thresholds": { "$ref": "#/components/schemas/ConfidenceThresholds" },
                        "provider_review_threshold": { "type": "integer", "minimum": 0, "maximum": 100 }
                    }
                },
                "RiskThresholds": {
                    "type": "object",
                    "required": ["low_max", "medium_min", "high_min", "critical_min"],
                    "properties": {
                        "low_max": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "medium_min": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "high_min": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "critical_min": { "type": "integer", "minimum": 0, "maximum": 100 }
                    }
                },
                "ConfidenceThresholds": {
                    "type": "object",
                    "required": ["low_confidence_below", "high_confidence_min"],
                    "properties": {
                        "low_confidence_below": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "high_confidence_min": { "type": "integer", "minimum": 0, "maximum": 100 }
                    }
                },
                "ProviderProfileAssessment": {
                    "type": "object",
                    "required": [
                        "provider_id",
                        "risk_score",
                        "risk_tier",
                        "review_required",
                        "review_route",
                        "review_failure_count",
                        "confirmed_fwa_count",
                        "false_positive_count",
                        "outlier_flags",
                        "window_findings",
                        "evidence_refs"
                    ],
                    "properties": {
                        "provider_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "risk_tier": { "type": "string", "enum": ["low", "medium", "high"] },
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string", "enum": ["none", "provider_review"] },
                        "specialty": { "type": ["string", "null"] },
                        "network_status": { "type": ["string", "null"] },
                        "review_failure_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 },
                        "outlier_flags": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "window_findings": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ProviderProfileWindowFinding" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ProviderProfileWindowFinding": {
                    "type": "object",
                    "required": [
                        "window_days",
                        "risk_score",
                        "outlier_flags",
                        "reason",
                        "evidence_ref"
                    ],
                    "properties": {
                        "window_days": { "type": "integer" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "outlier_flags": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "reason": { "type": "string" },
                        "evidence_ref": { "type": "string" }
                    }
                },
                "ProviderRelationshipGraphAssessment": {
                    "type": "object",
                    "required": [
                        "provider_id",
                        "risk_score",
                        "risk_tier",
                        "review_required",
                        "review_route",
                        "graph_reasons",
                        "findings",
                        "evidence_refs"
                    ],
                    "properties": {
                        "provider_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "risk_tier": { "type": "string", "enum": ["no_data", "low", "medium", "high"] },
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string", "enum": ["none", "provider_graph_review"] },
                        "graph_reasons": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "findings": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ProviderRelationshipGraphFinding" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ProviderRelationshipGraphFinding": {
                    "type": "object",
                    "required": ["signal", "risk_score", "reason", "evidence_ref"],
                    "properties": {
                        "signal": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "reason": { "type": "string" },
                        "evidence_ref": { "type": "string" }
                    }
                },
                "ProviderRiskSummaryItem": {
                    "type": "object",
                    "required": ["provider_id", "risk_score", "risk_tier", "review_required", "review_route", "claim_count", "specialty", "network_status", "review_failure_count", "confirmed_fwa_count", "false_positive_count", "network_risk_score", "outlier_flags", "graph_reasons", "evidence_refs"],
                    "properties": {
                        "provider_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "risk_tier": { "type": "string" },
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string" },
                        "claim_count": { "type": "integer" },
                        "specialty": { "type": ["string", "null"] },
                        "network_status": { "type": ["string", "null"] },
                        "review_failure_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 },
                        "network_risk_score": { "type": ["integer", "null"], "minimum": 0, "maximum": 100 },
                        "latest_claim_id": { "type": ["string", "null"] },
                        "outlier_flags": { "type": "array", "items": { "type": "string" } },
                        "graph_reasons": { "type": "array", "items": { "type": "string" } },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "ProviderRiskSummaryResponse": {
                    "type": "object",
                    "required": ["provider_count", "review_required_count", "high_risk_count", "providers"],
                    "properties": {
                        "provider_count": { "type": "integer" },
                        "review_required_count": { "type": "integer" },
                        "high_risk_count": { "type": "integer" },
                        "providers": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ProviderRiskSummaryItem" }
                        }
                    }
                },
                "SubmitMedicalReviewResultRequest": {
                    "type": "object",
                    "required": ["claim_id", "scoring_audit_id", "reviewer", "decision", "notes", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string", "minLength": 1 },
                        "scoring_audit_id": { "type": "string", "minLength": 1 },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "decision": {
                            "type": "string",
                            "enum": ["evidence_sufficient", "request_more_evidence", "medical_necessity_issue", "no_medical_issue"]
                        },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Medical review notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "description": "Structured evidence references must not contain PII. For claims with the referenced normalized scoring trace, canonical evidence refs from that trace are merged into the persisted medical review and response.",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "MedicalReviewResultResponse": {
                    "type": "object",
                    "required": ["claim_id", "event_type", "event_status", "audit_id", "run_id", "review_status", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "event_type": { "type": "string" },
                        "event_status": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "review_status": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "MedicalReviewQueueItem": {
                    "type": "object",
                    "required": ["claim_id", "run_id", "audit_id", "medical_reasonableness_score", "review_route", "evidence_status", "missing_evidence", "item_finding_count", "evidence_refs", "canonical_source_refs", "canonical_evidence_refs", "review_status"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "medical_reasonableness_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "review_route": { "type": "string" },
                        "evidence_status": { "type": "string" },
                        "missing_evidence": { "type": "array", "items": { "type": "string" } },
                        "item_finding_count": { "type": "integer" },
                        "first_item_code": { "type": ["string", "null"] },
                        "first_issue_type": { "type": ["string", "null"] },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "canonical_source_refs": { "type": "array", "items": { "type": "string" } },
                        "canonical_evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"], "format": "date-time" },
                        "review_status": { "type": "string" },
                        "review_audit_id": { "type": ["string", "null"] },
                        "review_decision": { "type": ["string", "null"] },
                        "reviewer": { "type": ["string", "null"] },
                        "reviewed_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "MedicalReviewQueueResponse": {
                    "type": "object",
                    "required": ["items"],
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/MedicalReviewQueueItem" }
                        }
                    }
                },
                "ClinicalEvidenceAssessment": {
                    "type": "object",
                    "required": [
                        "review_required",
                        "review_route",
                        "evidence_status",
                        "minimum_evidence",
                        "missing_evidence",
                        "item_findings",
                        "evidence_refs"
                    ],
                    "properties": {
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string", "enum": ["none", "medical_review"] },
                        "evidence_status": {
                            "type": "string",
                            "enum": [
                                "no_clinical_evidence_required",
                                "sufficient_for_basic_review",
                                "missing_required_evidence"
                            ]
                        },
                        "minimum_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "missing_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "item_findings": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ClinicalEvidenceFinding" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ClinicalEvidenceFinding": {
                    "type": "object",
                    "required": [
                        "item_code",
                        "issue_type",
                        "required_evidence",
                        "missing_evidence",
                        "reason",
                        "review_route",
                        "evidence_refs"
                    ],
                    "properties": {
                        "item_code": { "type": "string" },
                        "issue_type": { "type": "string" },
                        "required_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "missing_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "reason": { "type": "string" },
                        "review_route": { "type": "string" },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ScoreBreakdown": {
                    "type": "object",
                    "required": [
                        "peer_deviation_score",
                        "rule_score",
                        "anomaly_score",
                        "ml_score",
                        "medical_reasonableness_score",
                        "provider_network_score",
                        "similar_case_score",
                        "final_score"
                    ],
                    "properties": {
                        "peer_deviation_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "rule_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "anomaly_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "ml_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "medical_reasonableness_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "provider_network_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "similar_case_score": {
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
                "HealthCheck": {
                    "type": "object",
                    "required": ["name", "status"],
                    "properties": {
                        "name": { "type": "string" },
                        "status": {
                            "type": "string",
                            "enum": ["ok", "configured", "local_dev_key", "local_demo_source"],
                            "description": "Check status. local_dev_key indicates the API is using the local development key. local_demo_source indicates the API is using the local demo source system. Both must be reconfigured before customer pilot or production use."
                        },
                        "runtime_kind": {
                            "type": "string",
                            "enum": ["python_http", "heuristic"],
                            "description": "Model scorer runtime boundary when the check is model_scorer. Internal service URLs are intentionally not exposed."
                        }
                    }
                },
                "HealthResponse": {
                    "type": "object",
                    "required": ["status", "service", "version", "checks"],
                    "properties": {
                        "status": { "type": "string", "enum": ["ok"] },
                        "service": { "type": "string" },
                        "version": { "type": "string" },
                        "checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/HealthCheck" }
                        }
                    }
                },
                "RuleSummary": {
                    "type": "object",
                    "required": ["rule_id", "name", "status", "owner", "latest_version", "review_mode", "scheme_family", "score", "alert_code", "recommended_action", "applicability_scope", "backtest_result", "estimated_saving", "false_positive_history", "evidence_refs"],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "name": { "type": "string" },
                        "status": { "type": "string", "enum": ["draft", "active", "submitted", "approved"] },
                        "owner": { "type": "string" },
                        "active_version": { "type": ["integer", "null"] },
                        "latest_version": { "type": "integer" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "alert_code": { "type": "string" },
                        "recommended_action": { "type": "string" },
                        "applicability_scope": { "$ref": "#/components/schemas/RuleApplicabilityScope" },
                        "backtest_result": { "$ref": "#/components/schemas/RuleBacktestSummary" },
                        "estimated_saving": { "type": "string", "format": "decimal" },
                        "false_positive_history": { "$ref": "#/components/schemas/RuleFalsePositiveHistory" },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "RuleApplicabilityScope": {
                    "type": "object",
                    "required": ["review_mode", "scheme_family", "source"],
                    "properties": {
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "source": { "type": "string" }
                    }
                },
                "RuleBacktestSummary": {
                    "type": "object",
                    "required": ["status", "sample_count", "matched_count", "precision", "recall", "lift", "false_positive_rate", "estimated_saving", "evidence_refs", "created_at"],
                    "properties": {
                        "status": { "type": "string", "enum": ["not_run", "completed"] },
                        "sample_count": { "type": "integer", "minimum": 0 },
                        "matched_count": { "type": "integer", "minimum": 0 },
                        "precision": { "type": "number", "minimum": 0 },
                        "recall": { "type": "number", "minimum": 0 },
                        "lift": { "type": "number", "minimum": 0 },
                        "false_positive_rate": { "type": "number", "minimum": 0 },
                        "estimated_saving": { "type": "string", "format": "decimal" },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string", "minLength": 1 }
                        },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "RuleFalsePositiveHistory": {
                    "type": "object",
                    "required": ["status", "false_positive_count", "false_positive_rate", "evidence_refs"],
                    "properties": {
                        "status": { "type": "string", "enum": ["not_observed", "observed"] },
                        "false_positive_count": { "type": "integer", "minimum": 0 },
                        "false_positive_rate": { "type": "number", "minimum": 0 },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "FwaSchemeFamily": {
                    "type": "string",
                    "enum": [
                        "duplicate_billing",
                        "upcoding",
                        "unbundling",
                        "medically_unnecessary_service",
                        "excessive_utilization",
                        "diagnosis_procedure_mismatch",
                        "laboratory_testing_abuse",
                        "telehealth_abuse",
                        "genetic_testing_abuse",
                        "pharmacy_controlled_substance_abuse",
                        "dme_home_health_hospice_rehab_risk",
                        "provider_peer_outlier",
                        "relationship_concentration",
                        "early_high_value_claim",
                        "high_risk_claim"
                    ]
                },
                "FwaSchemeDefinition": {
                    "type": "object",
                    "required": ["scheme_family", "display_name", "risk_domain", "description", "minimum_evidence", "default_review_route", "primary_layers"],
                    "properties": {
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "display_name": { "type": "string" },
                        "risk_domain": { "type": "string" },
                        "description": { "type": "string" },
                        "minimum_evidence": { "type": "array", "items": { "type": "string" } },
                        "default_review_route": { "type": "string" },
                        "primary_layers": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "FwaSchemeListResponse": {
                    "type": "object",
                    "required": ["schemes", "scheme_count"],
                    "properties": {
                        "schemes": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/FwaSchemeDefinition" }
                        },
                        "scheme_count": { "type": "integer", "minimum": 0 }
                    }
                },
                "RuleVersion": {
                    "type": "object",
                    "required": ["version", "status", "dsl", "review_mode", "scheme_family", "score", "alert_code", "recommended_action", "reason"],
                    "properties": {
                        "version": { "type": "integer" },
                        "status": { "type": "string" },
                        "dsl": { "type": "object" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "alert_code": { "type": "string" },
                        "recommended_action": { "type": "string" },
                        "reason": { "type": "string" }
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
                "RuleLifecycleResponse": {
                    "type": "object",
                    "required": ["rule_id", "status", "active_version", "latest_version"],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "status": { "type": "string", "enum": ["draft", "active", "submitted", "approved"] },
                        "active_version": { "type": ["integer", "null"] },
                        "latest_version": { "type": "integer" }
                    }
                },
                "RuleLifecycleRequest": {
                    "type": "object",
                    "required": ["evidence_refs"],
                    "properties": {
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must not contain PII.",
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "RulePerformanceRecord": {
                    "type": "object",
                    "required": [
                        "rule_id",
                        "alert_code",
                        "trigger_count",
                        "reviewed_count",
                        "confirmed_fwa_count",
                        "false_positive_count",
                        "mark_rate",
                        "precision",
                        "false_positive_rate",
                        "saving_amount",
                        "roi"
                    ],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "alert_code": { "type": "string" },
                        "trigger_count": { "type": "integer", "minimum": 0 },
                        "reviewed_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 },
                        "mark_rate": { "type": "number", "minimum": 0 },
                        "precision": { "type": "number", "minimum": 0 },
                        "false_positive_rate": { "type": "number", "minimum": 0 },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "roi": { "type": "number" }
                    }
                },
                "RulePerformanceResponse": {
                    "type": "object",
                    "required": ["rules"],
                    "properties": {
                        "rules": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RulePerformanceRecord" }
                        }
                    }
                },
                "RulePromotionGate": {
                    "type": "object",
                    "required": ["label", "passed", "blocker", "evidence_source"],
                    "properties": {
                        "label": { "type": "string" },
                        "passed": { "type": "boolean" },
                        "blocker": { "type": "string" },
                        "evidence_source": {
                            "type": "string",
                            "enum": ["runtime", "backtest", "approval", "labels", "qa_feedback", "metadata", "missing"]
                        }
                    }
                },
                "RulePromotionGatesResponse": {
                    "type": "object",
                    "required": [
                        "rule_id",
                        "rule_version",
                        "review_mode",
                        "decision",
                        "status",
                        "passed_count",
                        "total_count",
                        "trigger_count",
                        "reviewed_count",
                        "false_positive_rate",
                        "saving_amount",
                        "open_rule_feedback_count",
                        "unresolved_rule_feedback_count",
                        "approved_label_count",
                        "needs_review_label_count",
                        "gates",
                        "blockers"
                    ],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "rule_version": { "type": "integer" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "decision": { "type": "string", "enum": ["routing_allowed", "routing_blocked"] },
                        "status": { "type": "string" },
                        "passed_count": { "type": "integer" },
                        "total_count": { "type": "integer" },
                        "trigger_count": { "type": "integer", "minimum": 0 },
                        "reviewed_count": { "type": "integer", "minimum": 0 },
                        "false_positive_rate": { "type": "number", "minimum": 0 },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "open_rule_feedback_count": { "type": "integer" },
                        "unresolved_rule_feedback_count": { "type": "integer" },
                        "approved_label_count": { "type": "integer" },
                        "needs_review_label_count": { "type": "integer" },
                        "gates": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RulePromotionGate" }
                        },
                        "blockers": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "SubmitRulePromotionReviewRequest": {
                    "type": "object",
                    "required": ["decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Promotion review notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must not contain PII.",
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "RulePromotionReview": {
                    "type": "object",
                    "required": ["rule_id", "rule_version", "decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "rule_version": { "type": "integer" },
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string" },
                        "notes": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "AuditHistoryEvent": {
                    "type": "object",
                    "required": ["audit_id", "run_id", "event_type", "event_status", "summary", "payload", "evidence_refs"],
                    "properties": {
                        "audit_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "event_type": { "type": "string" },
                        "event_status": { "type": "string" },
                        "summary": { "type": "string" },
                        "payload": { "type": "object" },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "AuditEventListResponse": {
                    "type": "object",
                    "required": ["events"],
                    "properties": {
                        "events": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AuditHistoryEvent" }
                        }
                    }
                },
                "ApiCallRecord": {
                    "type": "object",
                    "required": ["call_id", "endpoint", "method", "status_code", "result", "source_system", "claim_id", "run_id", "audit_id", "event_type", "idempotency_key", "evidence_refs", "observed_at"],
                    "properties": {
                        "call_id": { "type": "string" },
                        "endpoint": { "type": "string" },
                        "method": { "type": "string" },
                        "status_code": { "type": "integer" },
                        "result": { "type": "string" },
                        "source_system": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "event_type": { "type": "string" },
                        "idempotency_key": { "type": ["string", "null"] },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "observed_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "ApiCallListResponse": {
                    "type": "object",
                    "required": ["calls"],
                    "properties": {
                        "calls": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ApiCallRecord" }
                        }
                    }
                },
                "WebhookEvent": {
                    "type": "object",
                    "required": ["event_id", "event_type", "source_event_type", "source_audit_id", "claim_id", "run_id", "delivery_status", "retry_count", "max_attempts", "next_attempt_at", "last_attempt_at", "last_response_status_code", "last_error_message", "idempotency_key", "signature_key_id", "signature_algorithm", "signature_base_string", "payload", "evidence_refs", "occurred_at"],
                    "properties": {
                        "event_id": { "type": "string" },
                        "event_type": {
                            "type": "string",
                            "enum": ["fwa.score.completed", "fwa.case.routed", "fwa.investigation.closed", "fwa.qa.reviewed", "fwa.medical.reviewed"]
                        },
                        "source_event_type": { "type": "string" },
                        "source_audit_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "delivery_status": { "type": "string", "enum": ["pending", "retry_wait", "delivered", "failed"] },
                        "retry_count": { "type": "integer" },
                        "max_attempts": { "type": "integer" },
                        "next_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                        "last_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                        "last_response_status_code": { "type": ["integer", "null"] },
                        "last_error_message": { "type": ["string", "null"] },
                        "idempotency_key": { "type": "string" },
                        "signature_key_id": { "type": "string" },
                        "signature_algorithm": { "type": "string", "enum": ["hmac-sha256"] },
                        "signature_base_string": { "type": "string" },
                        "payload": { "type": "object" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "occurred_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "SubmitWebhookDeliveryAttemptRequest": {
                    "type": "object",
                    "required": ["delivery_status"],
                    "properties": {
                        "delivery_status": { "type": "string", "enum": ["delivered", "failed"] },
                        "response_status_code": { "type": ["integer", "null"] },
                        "error_message": {
                            "type": ["string", "null"],
                            "description": "Webhook delivery error message; must not contain PII."
                        }
                    }
                },
                "WebhookDeliveryAttempt": {
                    "type": "object",
                    "required": ["event_id", "attempt_number", "delivery_status", "response_status_code", "error_message", "next_attempt_at", "attempted_at"],
                    "properties": {
                        "event_id": { "type": "string" },
                        "attempt_number": { "type": "integer" },
                        "delivery_status": { "type": "string", "enum": ["delivered", "failed"] },
                        "response_status_code": { "type": ["integer", "null"] },
                        "error_message": { "type": ["string", "null"] },
                        "next_attempt_at": { "type": ["string", "null"], "format": "date-time" },
                        "attempted_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "WebhookEventListResponse": {
                    "type": "object",
                    "required": ["events"],
                    "properties": {
                        "events": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/WebhookEvent" }
                        }
                    }
                },
                "OpsAlert": {
                    "type": "object",
                    "required": ["alert_id", "alert_type", "severity", "status", "claim_id", "lead_id", "case_id", "scheme_family", "message", "recommended_action", "evidence_refs"],
                    "properties": {
                        "alert_id": { "type": "string" },
                        "alert_type": {
                            "type": "string",
                            "enum": ["high_risk_routing", "sla_breach", "medical_review_required", "agent_approval_pending"]
                        },
                        "severity": {
                            "type": "string",
                            "enum": ["critical", "high", "medium", "low"]
                        },
                        "status": {
                            "type": "string",
                            "enum": ["open", "closed"]
                        },
                        "claim_id": { "type": "string" },
                        "lead_id": { "type": ["string", "null"] },
                        "case_id": { "type": ["string", "null"] },
                        "scheme_family": { "type": "string" },
                        "message": { "type": "string" },
                        "recommended_action": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "OpsAlertListResponse": {
                    "type": "object",
                    "required": ["alerts"],
                    "properties": {
                        "alerts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/OpsAlert" }
                        }
                    }
                },
                "RuleDetailResponse": {
                    "type": "object",
                    "required": ["summary", "versions", "audit_events"],
                    "properties": {
                        "summary": { "$ref": "#/components/schemas/RuleSummary" },
                        "versions": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RuleVersion" }
                        },
                        "audit_events": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AuditHistoryEvent" }
                        }
                    }
                },
                "RuleBacktestRequest": {
                    "type": "object",
                    "required": ["rule", "samples"],
                    "properties": {
                        "rule": { "$ref": "#/components/schemas/RuleDefinition" },
                        "expected_review_capacity": { "type": "integer", "minimum": 0 },
                        "samples": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "confirmed_fwa": { "type": "boolean" }
                                }
                            }
                        }
                    }
                },
                "RuleBacktestResponse": {
                    "type": "object",
                    "required": ["sample_count", "matched_count", "reviewed_count", "confirmed_fwa_count", "false_positive_count", "match_rate", "precision", "recall", "lift", "false_positive_rate", "average_score_contribution", "estimated_saving", "promotion_recommendation", "blockers", "matched_claim_ids", "evidence_refs"],
                    "properties": {
                        "sample_count": { "type": "integer" },
                        "matched_count": { "type": "integer" },
                        "reviewed_count": { "type": "integer" },
                        "confirmed_fwa_count": { "type": "integer" },
                        "false_positive_count": { "type": "integer" },
                        "match_rate": { "type": "number" },
                        "precision": { "type": "number" },
                        "recall": { "type": "number" },
                        "lift": { "type": "number" },
                        "false_positive_rate": { "type": "number" },
                        "average_score_contribution": { "type": "number" },
                        "estimated_saving": { "type": "string", "format": "decimal" },
                        "promotion_recommendation": { "type": "string" },
                        "blockers": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "matched_claim_ids": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "SaveRuleCandidateRequest": {
                    "type": "object",
                    "required": ["rule"],
                    "properties": {
                        "owner": { "type": "string" },
                        "rule": { "$ref": "#/components/schemas/RuleDefinition" }
                    }
                },
                "RuleDefinition": {
                    "type": "object",
                    "required": ["rule_id", "version", "name", "review_mode", "scheme_family", "conditions", "action"],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "version": { "type": "integer", "minimum": 1 },
                        "name": { "type": "string" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "conditions": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RuleCondition" }
                        },
                        "action": { "$ref": "#/components/schemas/RuleAction" }
                    }
                },
                "RuleCondition": {
                    "type": "object",
                    "required": ["field", "operator", "value"],
                    "properties": {
                        "field": { "type": "string" },
                        "operator": { "type": "string", "enum": ["<=", ">=", "=="] },
                        "value": {}
                    }
                },
                "RuleAction": {
                    "type": "object",
                    "required": ["score", "alert_code", "recommended_action", "reason"],
                    "properties": {
                        "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "alert_code": { "type": "string" },
                        "recommended_action": {
                            "type": "string",
                            "enum": ["StandardProcessing", "QaSample", "ManualReview", "RequestEvidence", "EscalateInvestigation", "PostPaymentAudit", "ProviderReview", "RecoveryReview"]
                        },
                        "reason": { "type": "string" }
                    }
                },
                "RuleDiscoveryRequest": {
                    "type": "object",
                    "required": ["samples"],
                    "properties": {
                        "min_support": { "type": "integer", "minimum": 1 },
                        "samples": {
                            "type": "array",
                            "items": { "type": "object" }
                        }
                    }
                },
                "RuleDiscoveryCandidate": {
                    "type": "object",
                    "required": ["rule", "support", "precision", "recall", "lift", "estimated_saving", "false_positive_rate", "matched_claim_ids", "explanation"],
                    "properties": {
                        "rule": { "$ref": "#/components/schemas/RuleDefinition" },
                        "support": { "type": "integer" },
                        "precision": { "type": "number" },
                        "recall": { "type": "number" },
                        "lift": { "type": "number" },
                        "estimated_saving": { "type": "string", "format": "decimal" },
                        "false_positive_rate": { "type": "number" },
                        "matched_claim_ids": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "explanation": { "type": "string" }
                    }
                },
                "RuleDiscoveryResponse": {
                    "type": "object",
                    "required": ["sample_count", "positive_count", "candidates"],
                    "properties": {
                        "sample_count": { "type": "integer" },
                        "positive_count": { "type": "integer" },
                        "candidates": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RuleDiscoveryCandidate" }
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
                        "description": {
                            "type": "string",
                            "description": "Business description for the factor; must not contain PII."
                        },
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
                        "description": {
                            "type": "string",
                            "description": "Dataset business description; must not contain PII."
                        },
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
                    "required": ["datasets", "health"],
                    "properties": {
                        "datasets": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DatasetRecord" }
                        },
                        "health": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DatasetHealth" }
                        }
                    }
                },
                "DatasetHealth": {
                    "type": "object",
                    "required": ["dataset_id", "dataset_key", "dataset_version", "data_quality_score", "data_quality_status", "field_count", "label_count", "entity_key_count", "high_missing_count", "unstable_field_count", "unowned_field_count", "online_ready_count", "issue_count"],
                    "properties": {
                        "dataset_id": { "type": "string" },
                        "dataset_key": { "type": "string" },
                        "dataset_version": { "type": "string" },
                        "data_quality_score": { "type": "number" },
                        "data_quality_status": { "type": "string", "enum": ["empty", "ready", "watch", "blocked"] },
                        "field_count": { "type": "integer" },
                        "label_count": { "type": "integer" },
                        "entity_key_count": { "type": "integer" },
                        "high_missing_count": { "type": "integer" },
                        "unstable_field_count": { "type": "integer" },
                        "unowned_field_count": { "type": "integer" },
                        "online_ready_count": { "type": "integer" },
                        "issue_count": { "type": "integer" }
                    }
                },
                "FieldMappingRequest": {
                    "type": "object",
                    "required": ["external_field", "canonical_target", "transform_kind", "transform_json", "status"],
                    "properties": {
                        "external_field": { "type": "string", "minLength": 1 },
                        "canonical_target": { "type": "string", "minLength": 1 },
                        "feature_name": { "type": ["string", "null"], "minLength": 1 },
                        "transform_kind": { "type": "string", "enum": ["direct", "cast", "enum_map", "derived", "aggregate"] },
                        "transform_json": { "type": "object" },
                        "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
                    }
                },
                "FieldMappingResponse": {
                    "type": "object",
                    "required": ["mapping"],
                    "properties": {
                        "mapping": { "$ref": "#/components/schemas/FieldMapping" }
                    }
                },
                "FeatureSet": {
                    "type": "object",
                    "required": ["feature_set_id", "business_domain", "feature_set_key", "version", "dataset_id", "features_uri", "feature_list_json", "row_count", "label_column", "status"],
                    "properties": {
                        "feature_set_id": { "type": "string" },
                        "business_domain": { "type": "string" },
                        "feature_set_key": { "type": "string" },
                        "version": { "type": "string" },
                        "dataset_id": { "type": "string" },
                        "features_uri": { "type": "string" },
                        "feature_list_json": { "type": "array", "items": { "type": "string" } },
                        "row_count": { "type": "integer", "minimum": 1 },
                        "label_column": { "type": "string" },
                        "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
                    }
                },
                "FeatureSetRegistrationRequest": {
                    "type": "object",
                    "required": ["business_domain", "feature_set_key", "version", "dataset_id", "features_uri", "feature_list_json", "row_count", "label_column", "status"],
                    "properties": {
                        "business_domain": { "type": "string", "minLength": 1 },
                        "feature_set_key": { "type": "string", "minLength": 1 },
                        "version": { "type": "string", "minLength": 1 },
                        "dataset_id": { "type": "string", "minLength": 1 },
                        "features_uri": { "type": "string", "minLength": 1 },
                        "feature_list_json": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
                        "row_count": { "type": "integer", "minimum": 1 },
                        "label_column": { "type": "string", "minLength": 1 },
                        "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
                    }
                },
                "ModelDataset": {
                    "type": "object",
                    "required": ["model_dataset_id", "business_domain", "task_type", "label_name", "feature_set_id", "train_uri", "validation_uri", "row_counts_json", "label_distribution_json", "status"],
                    "properties": {
                        "model_dataset_id": { "type": "string" },
                        "business_domain": { "type": "string" },
                        "task_type": { "type": "string" },
                        "label_name": { "type": "string" },
                        "feature_set_id": { "type": "string" },
                        "train_uri": { "type": "string" },
                        "validation_uri": { "type": "string" },
                        "test_uri": { "type": ["string", "null"] },
                        "row_counts_json": { "type": "object", "additionalProperties": true },
                        "label_distribution_json": { "type": "object", "additionalProperties": true },
                        "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
                    }
                },
                "ModelDatasetRegistrationRequest": {
                    "type": "object",
                    "required": ["business_domain", "task_type", "label_name", "feature_set_id", "train_uri", "validation_uri", "row_counts_json", "label_distribution_json", "status"],
                    "properties": {
                        "business_domain": { "type": "string", "minLength": 1 },
                        "task_type": { "type": "string", "minLength": 1 },
                        "label_name": { "type": "string", "minLength": 1 },
                        "feature_set_id": { "type": "string", "minLength": 1 },
                        "train_uri": { "type": "string", "minLength": 1 },
                        "validation_uri": { "type": "string", "minLength": 1 },
                        "test_uri": { "type": ["string", "null"], "minLength": 1 },
                        "row_counts_json": { "type": "object", "minProperties": 1, "additionalProperties": true },
                        "label_distribution_json": { "type": "object", "minProperties": 1, "additionalProperties": true },
                        "status": { "type": "string", "enum": ["draft", "active", "deprecated"] }
                    }
                },
                "FactorReadinessResponse": {
                    "type": "object",
                    "required": ["dataset_count", "factor_count", "label_count", "entity_key_count", "data_quality_score", "data_quality_status", "online_ready_count", "rule_convertible_count", "mapped_factor_count", "high_missing_count", "unstable_factor_count", "unowned_factor_count", "ready_factor_count", "review_factor_count", "readiness_issue_counts", "scheme_readiness", "factor_cards"],
                    "properties": {
                        "dataset_count": { "type": "integer" },
                        "factor_count": { "type": "integer" },
                        "label_count": { "type": "integer" },
                        "entity_key_count": { "type": "integer" },
                        "data_quality_score": { "type": "number" },
                        "data_quality_status": { "type": "string", "enum": ["empty", "ready", "watch", "blocked"] },
                        "online_ready_count": { "type": "integer" },
                        "rule_convertible_count": { "type": "integer" },
                        "mapped_factor_count": { "type": "integer" },
                        "high_missing_count": { "type": "integer" },
                        "unstable_factor_count": { "type": "integer" },
                        "unowned_factor_count": { "type": "integer" },
                        "ready_factor_count": { "type": "integer" },
                        "review_factor_count": { "type": "integer" },
                        "readiness_issue_counts": {
                            "type": "object",
                            "additionalProperties": { "type": "integer" }
                        },
                        "scheme_readiness": { "type": "array", "items": { "$ref": "#/components/schemas/FactorSchemeReadiness" } },
                        "factor_cards": { "type": "array", "items": { "$ref": "#/components/schemas/FactorCard" } }
                    }
                },
                "FactorSchemeReadiness": {
                    "type": "object",
                    "required": ["scheme_family", "factor_count", "ready_factor_count", "review_factor_count", "online_ready_count", "rule_convertible_count", "readiness_issue_counts"],
                    "properties": {
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "factor_count": { "type": "integer" },
                        "ready_factor_count": { "type": "integer" },
                        "review_factor_count": { "type": "integer" },
                        "online_ready_count": { "type": "integer" },
                        "rule_convertible_count": { "type": "integer" },
                        "readiness_issue_counts": {
                            "type": "object",
                            "additionalProperties": { "type": "integer" }
                        }
                    }
                },
                "FactorCard": {
                    "type": "object",
                    "required": ["dataset_id", "dataset_key", "dataset_version", "factor_name", "scheme_family", "chinese_name", "entity_type", "semantic_role", "logical_type", "calculation_window", "calculation_logic", "source_table", "source_fields", "business_meaning", "risk_direction", "missing_rate", "iv", "auc_gain", "lift", "psi", "stability", "model_contribution", "rule_convertible", "online_available", "readiness_status", "readiness_issues", "version", "owner", "is_label", "is_entity_key", "evidence_refs"],
                    "properties": {
                        "dataset_id": { "type": "string" },
                        "dataset_key": { "type": "string" },
                        "dataset_version": { "type": "string" },
                        "factor_name": { "type": "string" },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "chinese_name": { "type": "string" },
                        "entity_type": { "type": "string" },
                        "semantic_role": { "type": "string" },
                        "logical_type": { "type": "string" },
                        "calculation_window": { "type": "string" },
                        "calculation_logic": { "type": "string" },
                        "source_table": { "type": "string" },
                        "source_fields": { "type": "array", "items": { "type": "string" } },
                        "business_meaning": { "type": "string" },
                        "risk_direction": { "type": "string" },
                        "missing_rate": { "type": ["number", "null"] },
                        "iv": { "type": ["number", "null"] },
                        "auc_gain": { "type": ["number", "null"] },
                        "lift": { "type": ["number", "null"] },
                        "psi": { "type": ["number", "null"] },
                        "stability": { "type": "string", "enum": ["unmeasured", "stable", "watch", "drift"] },
                        "model_contribution": { "type": ["number", "null"] },
                        "rule_convertible": { "type": "boolean" },
                        "online_available": { "type": "boolean" },
                        "readiness_status": { "type": "string", "enum": ["ready", "needs_review"] },
                        "readiness_issues": { "type": "array", "items": { "type": "string" } },
                        "version": { "type": "string" },
                        "owner": { "type": "string" },
                        "is_label": { "type": "boolean" },
                        "is_entity_key": { "type": "boolean" },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "ModelVersion": {
                    "type": "object",
                    "required": ["model_key", "version", "model_type", "runtime_kind", "execution_provider", "status", "review_mode"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "version": { "type": "string" },
                        "model_type": { "type": "string" },
                        "runtime_kind": { "type": "string" },
                        "execution_provider": { "type": "string" },
                        "status": { "type": "string" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "artifact_uri": { "type": ["string", "null"] },
                        "endpoint_url": { "type": ["string", "null"] }
                    }
                },
                "ModelEvaluation": {
                    "type": "object",
                    "required": ["evaluation_run_id", "model_key", "model_version", "model_dataset_id", "scheme_family", "confusion_matrix_json", "metrics_json"],
                    "properties": {
                        "evaluation_run_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "model_dataset_id": { "type": "string" },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "auc": { "type": ["string", "null"] },
                        "ks": { "type": ["string", "null"] },
                        "precision": { "type": ["string", "null"] },
                        "recall": { "type": ["string", "null"] },
                        "f1": { "type": ["string", "null"] },
                        "accuracy": { "type": ["string", "null"] },
                        "threshold": { "type": ["string", "null"] },
                        "confusion_matrix_json": { "type": "object" },
                        "feature_importance_uri": {
                            "type": ["string", "null"],
                            "description": "Feature importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "metrics_json": {
                            "type": "object",
                            "description": "Model governance metrics. Promotion-ready evaluations should include time_group_split_status, time_split_field, group_split_fields, leakage_check_status, shadow_comparison_status, and label_provenance_status."
                        }
                    }
                },
                "ModelEvaluationRegistrationRequest": {
                    "type": "object",
                    "required": ["evaluation_run_id", "model_key", "model_version", "model_dataset_id", "scheme_family", "confusion_matrix_json", "metrics_json"],
                    "properties": {
                        "evaluation_run_id": { "type": "string", "minLength": 1 },
                        "model_key": { "type": "string", "minLength": 1 },
                        "model_version": { "type": "string", "minLength": 1 },
                        "model_dataset_id": { "type": "string", "minLength": 1 },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "auc": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "ks": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "precision": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "recall": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "f1": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "accuracy": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "threshold": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "confusion_matrix_json": { "type": "object", "minProperties": 1 },
                        "feature_importance_uri": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Feature importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "metrics_json": {
                            "type": "object",
                            "minProperties": 1,
                            "description": "Model governance metrics. Promotion-ready evaluations should include time_group_split_status, time_split_field, group_split_fields, leakage_check_status, shadow_comparison_status, and label_provenance_status."
                        }
                    }
                },
                "ModelEvaluationLineage": {
                    "type": "object",
                    "required": ["evaluation_run_id", "model_key", "model_version", "model_dataset_id", "source_dataset_id", "source_dataset_key", "source_dataset_version", "source_data_quality_score", "source_data_quality_status"],
                    "properties": {
                        "evaluation_run_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "model_dataset_id": { "type": "string" },
                        "source_dataset_id": { "type": ["string", "null"] },
                        "source_dataset_key": { "type": ["string", "null"] },
                        "source_dataset_version": { "type": ["string", "null"] },
                        "source_data_quality_score": { "type": ["number", "null"] },
                        "source_data_quality_status": { "type": ["string", "null"], "enum": ["empty", "ready", "watch", "blocked", null] }
                    }
                },
                "ModelEvaluationListResponse": {
                    "type": "object",
                    "required": ["evaluations", "lineage"],
                    "properties": {
                        "evaluations": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelEvaluation" }
                        },
                        "lineage": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelEvaluationLineage" }
                        }
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
                "RoutingPolicyRecord": {
                    "type": "object",
                    "required": ["policy_id", "version", "review_mode", "status", "owner", "risk_thresholds", "confidence_thresholds", "provider_review_threshold", "activated_at", "created_at"],
                    "properties": {
                        "policy_id": { "type": "string" },
                        "version": { "type": "integer", "minimum": 1 },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "status": { "type": "string", "enum": ["draft", "submitted", "approved", "active", "retired"] },
                        "owner": { "type": "string" },
                        "risk_thresholds": { "$ref": "#/components/schemas/RiskThresholds" },
                        "confidence_thresholds": { "$ref": "#/components/schemas/ConfidenceThresholds" },
                        "provider_review_threshold": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "activated_at": { "type": ["string", "null"], "format": "date-time" },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "RoutingPolicyListResponse": {
                    "type": "object",
                    "required": ["policies"],
                    "properties": {
                        "policies": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RoutingPolicyRecord" }
                        }
                    }
                },
                "SaveRoutingPolicyCandidateRequest": {
                    "type": "object",
                    "required": ["policy"],
                    "properties": {
                        "policy": { "$ref": "#/components/schemas/RoutingPolicy" },
                        "owner": { "type": ["string", "null"], "minLength": 1 }
                    }
                },
                "RoutingPolicyLifecycleRequest": {
                    "type": "object",
                    "required": ["evidence_refs"],
                    "properties": {
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must not contain PII.",
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "RoutingPolicyPromotionGate": {
                    "type": "object",
                    "required": ["label", "passed", "blocker", "evidence_source"],
                    "properties": {
                        "label": { "type": "string" },
                        "passed": { "type": "boolean" },
                        "blocker": { "type": "string" },
                        "evidence_source": { "type": "string", "enum": ["metadata", "approval", "policy_json"] }
                    }
                },
                "RoutingPolicyPromotionGatesResponse": {
                    "type": "object",
                    "required": ["policy_id", "version", "review_mode", "status", "decision", "passed_count", "total_count", "gates", "blockers"],
                    "properties": {
                        "policy_id": { "type": "string" },
                        "version": { "type": "integer" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "status": { "type": "string" },
                        "decision": { "type": "string", "enum": ["activation_allowed", "activation_blocked"] },
                        "passed_count": { "type": "integer" },
                        "total_count": { "type": "integer" },
                        "gates": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RoutingPolicyPromotionGate" }
                        },
                        "blockers": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ModelLifecycleResponse": {
                    "type": "object",
                    "required": ["model_key", "version", "status"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "version": { "type": "string" },
                        "status": { "type": "string" }
                    }
                },
                "ModelLifecycleRequest": {
                    "type": "object",
                    "required": ["evidence_refs"],
                    "properties": {
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must not contain PII and must include model_versions:{model_key}:{model_version} for the activation target or rollback active version.",
                            "items": { "type": "string", "minLength": 1 },
                            "contains": { "type": "string", "pattern": "^model_versions:[^:]+:[^:]+$" }
                        }
                    }
                },
                "ModelPerformanceResponse": {
                    "type": "object",
                    "required": ["model_key", "data_status", "scored_runs", "average_score", "high_risk_count", "score_psi", "drift_status"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "data_status": { "type": "string", "enum": ["empty", "ready"] },
                        "scored_runs": { "type": "integer" },
                        "average_score": { "type": "number" },
                        "high_risk_count": { "type": "integer" },
                        "score_psi": { "type": ["number", "null"] },
                        "drift_status": { "type": "string", "enum": ["not_available", "stable", "watch", "drift"] },
                        "latest_scored_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "ModelPromotionGate": {
                    "type": "object",
                    "required": ["label", "passed", "blocker", "evidence_source"],
                    "properties": {
                        "label": { "type": "string" },
                        "passed": { "type": "boolean" },
                        "blocker": { "type": "string" },
                        "evidence_source": {
                            "type": "string",
                            "enum": ["runtime", "approval", "dataset", "evaluation", "labels", "qa_feedback", "metadata", "missing"]
                        }
                    }
                },
                "ModelPromotionGatesResponse": {
                    "type": "object",
                    "required": ["model_key", "model_version", "review_mode", "decision", "passed_count", "total_count", "latest_evaluation_id", "source_dataset_id", "source_data_quality_score", "source_data_quality_status", "data_status", "scored_runs", "open_model_feedback_count", "unresolved_model_feedback_count", "approved_label_count", "needs_review_label_count", "gates", "blockers"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "decision": { "type": "string", "enum": ["routing_allowed", "routing_blocked"] },
                        "passed_count": { "type": "integer" },
                        "total_count": { "type": "integer" },
                        "latest_evaluation_id": { "type": "string" },
                        "source_dataset_id": { "type": "string" },
                        "source_data_quality_score": { "type": ["number", "null"] },
                        "source_data_quality_status": { "type": "string", "enum": ["missing", "ready", "watch", "blocked"] },
                        "data_status": { "type": "string" },
                        "scored_runs": { "type": "integer" },
                        "open_model_feedback_count": { "type": "integer" },
                        "unresolved_model_feedback_count": { "type": "integer" },
                        "approved_label_count": { "type": "integer" },
                        "needs_review_label_count": { "type": "integer" },
                        "gates": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelPromotionGate" }
                        },
                        "blockers": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ModelRetrainingReadinessResponse": {
                    "type": "object",
                    "required": ["model_key", "model_version", "recommendation", "latest_evaluation_id", "drift_status", "source_dataset_id", "source_data_quality_score", "source_data_quality_status", "open_model_feedback_count", "approved_label_count", "needs_review_label_count", "retraining_triggers", "blockers"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "recommendation": { "type": "string", "enum": ["monitor", "prepare_retraining", "blocked"] },
                        "latest_evaluation_id": { "type": "string" },
                        "drift_status": { "type": "string", "enum": ["not_available", "stable", "watch", "drift"] },
                        "source_dataset_id": { "type": "string" },
                        "source_data_quality_score": { "type": ["number", "null"] },
                        "source_data_quality_status": { "type": "string", "enum": ["missing", "ready", "watch", "blocked"] },
                        "open_model_feedback_count": { "type": "integer" },
                        "approved_label_count": { "type": "integer" },
                        "needs_review_label_count": { "type": "integer" },
                        "retraining_triggers": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "blockers": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ModelRetrainingJob": {
                    "type": "object",
                    "required": ["job_id", "model_key", "model_version", "status", "requested_by", "request_notes", "status_note", "updated_by", "readiness_recommendation", "latest_evaluation_id", "source_dataset_id", "source_data_quality_score", "source_data_quality_status", "trigger_summary", "blocker_summary", "created_at", "updated_at"],
                    "properties": {
                        "job_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "status": { "type": "string", "enum": ["queued", "running", "validation", "completed", "failed", "cancelled"] },
                        "requested_by": { "type": "string" },
                        "request_notes": { "type": "string" },
                        "status_note": { "type": "string" },
                        "updated_by": { "type": "string" },
                        "readiness_recommendation": { "type": "string" },
                        "latest_evaluation_id": { "type": "string" },
                        "source_dataset_id": { "type": "string" },
                        "source_data_quality_score": { "type": ["number", "null"] },
                        "source_data_quality_status": { "type": "string" },
                        "trigger_summary": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "blocker_summary": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "candidate_model_version": { "type": ["string", "null"] },
                        "candidate_artifact_uri": { "type": ["string", "null"] },
                        "candidate_endpoint_url": { "type": ["string", "null"] },
                        "validation_report_uri": { "type": ["string", "null"] },
                        "output_evaluation_id": { "type": ["string", "null"] },
                        "created_at": { "type": ["string", "null"], "format": "date-time" },
                        "updated_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "ModelRetrainingJobListResponse": {
                    "type": "object",
                    "required": ["jobs"],
                    "properties": {
                        "jobs": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelRetrainingJob" }
                        }
                    }
                },
                "CreateModelRetrainingJobRequest": {
                    "type": "object",
                    "required": ["requested_by", "notes"],
                    "properties": {
                        "requested_by": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Model retraining notes must not contain PII."
                        }
                    }
                },
                "UpdateModelRetrainingJobStatusRequest": {
                    "type": "object",
                    "required": ["status", "actor", "notes"],
                    "properties": {
                        "status": { "type": "string", "enum": ["queued", "running", "validation", "completed", "failed", "cancelled"] },
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Model retraining notes must not contain PII."
                        }
                    }
                },
                "ClaimModelRetrainingJobRequest": {
                    "type": "object",
                    "required": ["actor", "notes"],
                    "properties": {
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Model retraining notes must not contain PII."
                        },
                        "model_key": { "type": ["string", "null"] }
                    }
                },
                "CompleteModelRetrainingJobRequest": {
                    "type": "object",
                    "required": ["actor", "notes", "candidate_model_version", "artifact_uri", "validation_report_uri", "evaluation_run_id", "evidence_refs", "confusion_matrix_json", "metrics_json"],
                    "properties": {
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Model retraining notes must not contain PII."
                        },
                        "candidate_model_version": { "type": "string", "minLength": 1 },
                        "artifact_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Supported model artifact formats: .onnx, .pkl, or .joblib."
                        },
                        "endpoint_url": { "type": ["string", "null"], "minLength": 1 },
                        "validation_report_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Validation report URI must point to a JSON report."
                        },
                        "evaluation_run_id": { "type": "string", "minLength": 1 },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Model retraining output evidence_refs must not contain PII and must include model_artifacts, model_validation_reports, and model_evaluations refs."
                        },
                        "auc": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "ks": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "precision": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "recall": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "f1": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "accuracy": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "threshold": { "type": ["string", "null"], "minimum": 0, "maximum": 1 },
                        "confusion_matrix_json": { "type": "object", "minProperties": 1 },
                        "feature_importance_uri": {
                            "type": ["string", "null"],
                            "minLength": 1,
                            "description": "Feature importance artifact must be a Parquet file or Parquet partition directory."
                        },
                        "metrics_json": {
                            "type": "object",
                            "minProperties": 1,
                            "description": "Model governance metrics. Promotion-ready retraining outputs should include time_group_split_status, time_split_field, group_split_fields, leakage_check_status, shadow_comparison_status, and label_provenance_status."
                        }
                    }
                },
                "CompleteModelRetrainingJobResponse": {
                    "type": "object",
                    "required": ["job", "candidate_model", "evaluation"],
                    "properties": {
                        "job": { "$ref": "#/components/schemas/ModelRetrainingJob" },
                        "candidate_model": { "$ref": "#/components/schemas/ModelVersion" },
                        "evaluation": { "$ref": "#/components/schemas/ModelEvaluation" }
                    }
                },
                "SubmitModelPromotionReviewRequest": {
                    "type": "object",
                    "required": ["decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Promotion review notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must not contain PII and must include model_versions:{model_key}:{model_version} for the exact model under review.",
                            "items": { "type": "string", "minLength": 1 },
                            "contains": { "type": "string", "pattern": "^model_versions:[^:]+:[^:]+$" }
                        }
                    }
                },
                "ModelPromotionReview": {
                    "type": "object",
                    "required": ["model_key", "model_version", "decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string" },
                        "notes": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "DashboardModelScore": {
                    "type": "object",
                    "required": ["scored_runs", "average_score", "high_risk_count"],
                    "properties": {
                        "scored_runs": { "type": "integer" },
                        "average_score": { "type": "number" },
                        "high_risk_count": { "type": "integer" }
                    }
                },
                "DashboardLayerScore": {
                    "type": "object",
                    "required": ["name", "scored_runs", "average_score", "high_risk_count"],
                    "properties": {
                        "name": { "type": "string" },
                        "scored_runs": { "type": "integer" },
                        "average_score": { "type": "number" },
                        "high_risk_count": { "type": "integer" }
                    }
                },
                "DashboardAuditCoverage": {
                    "type": "object",
                    "required": ["scoring_runs", "canonical_trace_runs", "canonical_trace_coverage"],
                    "properties": {
                        "scoring_runs": { "type": "integer" },
                        "canonical_trace_runs": { "type": "integer" },
                        "canonical_trace_coverage": { "type": "number" }
                    }
                },
                "SavingAttributionSummary": {
                    "type": "object",
                    "required": ["source_type", "source_id", "action", "saving_amount", "currency", "claim_count", "evidence_refs"],
                    "properties": {
                        "source_type": { "type": "string" },
                        "source_id": { "type": "string" },
                        "action": { "type": "string" },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "currency": { "type": "string" },
                        "claim_count": { "type": "integer" },
                        "evidence_refs": { "type": "array", "items": { "type": "string", "minLength": 1 } }
                    }
                },
                "SavingSegmentSummary": {
                    "type": "object",
                    "required": ["segment_type", "segment_id", "saving_amount", "currency", "claim_count", "attribution_count", "roi"],
                    "properties": {
                        "segment_type": { "type": "string", "enum": ["provider", "scheme", "campaign"] },
                        "segment_id": { "type": "string" },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "currency": { "type": "string" },
                        "claim_count": { "type": "integer" },
                        "attribution_count": { "type": "integer" },
                        "roi": { "type": "number" }
                    }
                },
                "DashboardSummaryResponse": {
                    "type": "object",
                    "required": ["suspected_claims", "confirmed_fwa", "risk_amount", "saving_amount", "rag_distribution", "scheme_distribution", "rule_hits", "model_scores", "layer_scores", "saving_attributions", "saving_segments", "value_measurement", "audit_coverage", "label_pool", "qa_queue", "case_sla", "agent_governance", "model_governance", "rule_governance", "investigation_results", "qa_reviews"],
                    "properties": {
                        "suspected_claims": { "type": "integer" },
                        "confirmed_fwa": { "type": "integer" },
                        "risk_amount": { "type": "string", "format": "decimal" },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "rag_distribution": {
                            "type": "object",
                            "additionalProperties": { "type": "integer" }
                        },
                        "scheme_distribution": {
                            "type": "object",
                            "additionalProperties": { "type": "integer" }
                        },
                        "rule_hits": { "type": "integer" },
                        "model_scores": {
                            "type": "object",
                            "additionalProperties": { "$ref": "#/components/schemas/DashboardModelScore" }
                        },
                        "layer_scores": {
                            "type": "object",
                            "additionalProperties": { "$ref": "#/components/schemas/DashboardLayerScore" }
                        },
                        "saving_attributions": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/SavingAttributionSummary" }
                        },
                        "saving_segments": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/SavingSegmentSummary" }
                        },
                        "value_measurement": { "$ref": "#/components/schemas/DashboardValueMeasurement" },
                        "audit_coverage": { "$ref": "#/components/schemas/DashboardAuditCoverage" },
                        "label_pool": { "$ref": "#/components/schemas/DashboardLabelPool" },
                        "qa_queue": { "$ref": "#/components/schemas/DashboardQaQueue" },
                        "case_sla": { "$ref": "#/components/schemas/DashboardCaseSla" },
                        "agent_governance": { "$ref": "#/components/schemas/DashboardAgentGovernance" },
                        "model_governance": { "$ref": "#/components/schemas/DashboardModelGovernance" },
                        "rule_governance": { "$ref": "#/components/schemas/DashboardRuleGovernance" },
                        "investigation_results": { "type": "integer" },
                        "qa_reviews": { "type": "integer" }
                    }
                },
                "DashboardValueMeasurement": {
                    "type": "object",
                    "required": ["prevented_payment", "recovered_amount", "avoided_future_exposure", "estimated_impact", "review_cost", "false_positive_operational_cost", "reviewer_capacity_hours", "net_value", "currency", "evidence_caveat"],
                    "properties": {
                        "prevented_payment": { "type": "string", "format": "decimal" },
                        "recovered_amount": { "type": "string", "format": "decimal" },
                        "avoided_future_exposure": { "type": "string", "format": "decimal" },
                        "estimated_impact": { "type": "string", "format": "decimal" },
                        "review_cost": { "type": "string", "format": "decimal" },
                        "false_positive_operational_cost": { "type": "string", "format": "decimal" },
                        "reviewer_capacity_hours": { "type": "string", "format": "decimal" },
                        "net_value": { "type": "string", "format": "decimal" },
                        "currency": { "type": "string" },
                        "evidence_caveat": { "type": "string" }
                    }
                },
                "DashboardLabelPool": {
                    "type": "object",
                    "required": ["total_labels", "approved_for_training", "needs_review", "rule_feedback", "model_feedback", "features_feedback", "provider_profile_feedback", "workflow_feedback", "case_status_labels", "medical_review_labels", "false_positive_labels", "evidence_backed_labels"],
                    "properties": {
                        "total_labels": { "type": "integer" },
                        "approved_for_training": { "type": "integer" },
                        "needs_review": { "type": "integer" },
                        "rule_feedback": { "type": "integer" },
                        "model_feedback": { "type": "integer" },
                        "features_feedback": { "type": "integer" },
                        "provider_profile_feedback": { "type": "integer" },
                        "workflow_feedback": { "type": "integer" },
                        "case_status_labels": { "type": "integer" },
                        "medical_review_labels": { "type": "integer" },
                        "false_positive_labels": { "type": "integer" },
                        "evidence_backed_labels": { "type": "integer" }
                    }
                },
                "DashboardQaQueue": {
                    "type": "object",
                    "required": ["sampled_cases", "open_cases", "reviewed_cases", "disagreement_cases", "disagreement_rate", "feedback_open_count", "feedback_in_progress_count", "feedback_resolved_count", "feedback_dismissed_count", "unresolved_feedback_count", "rules_unresolved_feedback_count", "models_unresolved_feedback_count", "features_unresolved_feedback_count", "provider_profile_unresolved_feedback_count", "workflow_unresolved_feedback_count", "tpa_unresolved_feedback_count"],
                    "properties": {
                        "sampled_cases": { "type": "integer" },
                        "open_cases": { "type": "integer" },
                        "reviewed_cases": { "type": "integer" },
                        "disagreement_cases": { "type": "integer" },
                        "disagreement_rate": { "type": "number" },
                        "feedback_open_count": { "type": "integer" },
                        "feedback_in_progress_count": { "type": "integer" },
                        "feedback_resolved_count": { "type": "integer" },
                        "feedback_dismissed_count": { "type": "integer" },
                        "unresolved_feedback_count": { "type": "integer" },
                        "rules_unresolved_feedback_count": { "type": "integer" },
                        "models_unresolved_feedback_count": { "type": "integer" },
                        "features_unresolved_feedback_count": { "type": "integer" },
                        "provider_profile_unresolved_feedback_count": { "type": "integer" },
                        "workflow_unresolved_feedback_count": { "type": "integer" },
                        "tpa_unresolved_feedback_count": { "type": "integer" }
                    }
                },
                "DashboardCaseSla": {
                    "type": "object",
                    "required": ["total_cases", "open_cases", "closed_cases", "breached_cases", "sla_breach_rate", "average_time_to_triage_hours", "average_time_to_closure_hours"],
                    "properties": {
                        "total_cases": { "type": "integer" },
                        "open_cases": { "type": "integer" },
                        "closed_cases": { "type": "integer" },
                        "breached_cases": { "type": "integer" },
                        "sla_breach_rate": { "type": "number" },
                        "average_time_to_triage_hours": { "type": "number" },
                        "average_time_to_closure_hours": { "type": "number" }
                    }
                },
                "DashboardAgentGovernance": {
                    "type": "object",
                    "required": ["total_runs", "successful_runs", "evidence_backed_runs", "tool_call_count", "policy_check_count", "denied_policy_check_count", "failed_tool_call_count", "pending_approvals", "approved_approvals", "rejected_approvals"],
                    "properties": {
                        "total_runs": { "type": "integer" },
                        "successful_runs": { "type": "integer" },
                        "evidence_backed_runs": { "type": "integer" },
                        "tool_call_count": { "type": "integer" },
                        "policy_check_count": { "type": "integer" },
                        "denied_policy_check_count": { "type": "integer" },
                        "failed_tool_call_count": { "type": "integer" },
                        "pending_approvals": { "type": "integer" },
                        "approved_approvals": { "type": "integer" },
                        "rejected_approvals": { "type": "integer" }
                    }
                },
                "DashboardModelGovernance": {
                    "type": "object",
                    "required": ["total_models", "evaluated_models", "drift_watch_count", "drift_detected_count", "average_precision", "average_recall"],
                    "properties": {
                        "total_models": { "type": "integer" },
                        "evaluated_models": { "type": "integer" },
                        "drift_watch_count": { "type": "integer" },
                        "drift_detected_count": { "type": "integer" },
                        "average_precision": { "type": ["number", "null"] },
                        "average_recall": { "type": ["number", "null"] }
                    }
                },
                "DashboardRuleGovernance": {
                    "type": "object",
                    "required": ["total_rules", "active_rules", "triggered_rules", "total_trigger_count", "reviewed_count", "confirmed_fwa_count", "false_positive_count", "precision", "false_positive_rate", "saving_amount", "roi"],
                    "properties": {
                        "total_rules": { "type": "integer" },
                        "active_rules": { "type": "integer" },
                        "triggered_rules": { "type": "integer" },
                        "total_trigger_count": { "type": "integer" },
                        "reviewed_count": { "type": "integer" },
                        "confirmed_fwa_count": { "type": "integer" },
                        "false_positive_count": { "type": "integer" },
                        "precision": { "type": "number" },
                        "false_positive_rate": { "type": "number" },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "roi": { "type": "number" }
                    }
                },
                "Lead": {
                    "type": "object",
                    "required": ["lead_id", "run_id", "claim_id", "member_id", "provider_id", "source_system", "review_mode", "scheme_family", "lead_source", "status", "disposition", "risk_score", "rag", "reason", "evidence_refs"],
                    "properties": {
                        "lead_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "member_id": { "type": "string" },
                        "provider_id": { "type": "string" },
                        "source_system": { "type": "string" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "scheme_family": { "type": "string" },
                        "lead_source": { "type": "string" },
                        "status": { "type": "string" },
                        "disposition": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rag": { "type": "string" },
                        "reason": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "LeadListResponse": {
                    "type": "object",
                    "required": ["leads"],
                    "properties": {
                        "leads": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/Lead" }
                        }
                    }
                },
                "Case": {
                    "type": "object",
                    "required": ["case_id", "lead_id", "claim_id", "member_id", "provider_id", "source_system", "review_mode", "scheme_family", "lead_source", "status", "assignee", "reviewer", "priority", "routing_reason", "evidence_package", "sla_target_hours", "sla_status", "time_to_triage_hours", "time_to_closure_hours", "final_outcome", "reviewer_notes", "investigation_result_id"],
                    "properties": {
                        "case_id": { "type": "string" },
                        "lead_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "member_id": { "type": "string" },
                        "provider_id": { "type": "string" },
                        "source_system": { "type": "string" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "scheme_family": { "type": "string" },
                        "lead_source": { "type": "string" },
                        "status": { "type": "string" },
                        "assignee": { "type": "string" },
                        "reviewer": { "type": "string" },
                        "priority": { "type": "string" },
                        "routing_reason": { "type": "string" },
                        "evidence_package": { "$ref": "#/components/schemas/CaseEvidencePackage" },
                        "sla_target_hours": { "type": "integer" },
                        "sla_status": { "type": "string" },
                        "time_to_triage_hours": { "type": "number" },
                        "time_to_closure_hours": { "type": ["number", "null"] },
                        "final_outcome": { "type": ["string", "null"] },
                        "reviewer_notes": { "type": ["string", "null"] },
                        "investigation_result_id": { "type": ["string", "null"] }
                    }
                },
                "CaseEvidencePackage": {
                    "type": "object",
                    "required": ["lead_id", "claim_id", "risk_score", "rag", "reason", "triage_notes", "evidence_sufficiency", "evidence_refs", "evidence_refs_by_type"],
                    "properties": {
                        "lead_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rag": { "type": "string" },
                        "reason": { "type": "string" },
                        "triage_notes": { "type": "string" },
                        "evidence_sufficiency": { "$ref": "#/components/schemas/EvidenceSufficiency" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "evidence_refs_by_type": { "$ref": "#/components/schemas/EvidenceReferenceBuckets" }
                    }
                },
                "CaseListResponse": {
                    "type": "object",
                    "required": ["cases"],
                    "properties": {
                        "cases": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/Case" }
                        }
                    }
                },
                "TriageLeadRequest": {
                    "type": "object",
                    "required": ["decision", "assignee", "reviewer", "priority", "notes", "evidence_refs"],
                    "properties": {
                        "decision": {
                            "type": "string",
                            "enum": ["open_case", "reject_lead", "request_evidence", "merge_lead"]
                        },
                        "merge_target_lead_id": { "type": ["string", "null"] },
                        "assignee": { "type": "string", "minLength": 1 },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "priority": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Triage notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured triage decision evidence references must not contain PII.",
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "TriageLeadResponse": {
                    "type": "object",
                    "required": ["lead", "audit_id"],
                    "properties": {
                        "lead": { "$ref": "#/components/schemas/Lead" },
                        "case": {
                            "oneOf": [
                                { "$ref": "#/components/schemas/Case" },
                                { "type": "null" }
                            ]
                        },
                        "audit_id": { "type": "string" }
                    }
                },
                "UpdateCaseStatusRequest": {
                    "type": "object",
                    "required": ["status", "actor_id", "notes", "evidence_refs"],
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["triage", "investigating", "pending_evidence", "confirmed", "rejected", "closed"]
                        },
                        "actor_id": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Case status notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must not contain PII.",
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "UpdateCaseStatusResponse": {
                    "type": "object",
                    "required": ["case", "audit_id"],
                    "properties": {
                        "case": { "$ref": "#/components/schemas/Case" },
                        "audit_id": { "type": "string" }
                    }
                },
                "CreateAuditSampleRequest": {
                    "type": "object",
                    "required": ["sample_mode", "population_definition", "inclusion_criteria", "sample_size", "reviewer", "assignment_queue"],
                    "properties": {
                        "sample_mode": {
                            "type": "string",
                            "enum": ["risk_ranked", "random_control", "stratified", "post_payment_audit", "qa_calibration"]
                        },
                        "population_definition": { "type": "string", "minLength": 1 },
                        "inclusion_criteria": {
                            "type": "object",
                            "properties": {
                                "min_risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                                "scheme_family": { "type": "string" },
                                "rag": { "type": "string", "enum": ["GREEN", "AMBER", "RED"] },
                                "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                                "provider_type": { "type": "string" },
                                "provider_region": { "type": "string" },
                                "policy_type": { "type": "string" },
                                "risk_band": { "type": "string", "enum": ["low", "medium", "high", "critical"] }
                            }
                        },
                        "deterministic_seed": { "type": ["string", "null"] },
                        "sample_size": { "type": "integer", "minimum": 1 },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "assignment_queue": { "type": "string", "minLength": 1 }
                    }
                },
                "AuditSampleLeadRecord": {
                    "type": "object",
                    "required": ["lead_id", "claim_id", "scheme_family", "review_mode", "provider_id", "provider_type", "provider_region", "policy_type", "risk_band", "strata_key", "prior_reviewer_sample_count", "risk_score", "rag", "evidence_refs"],
                    "properties": {
                        "lead_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "scheme_family": { "type": "string" },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "provider_id": { "type": "string" },
                        "provider_type": { "type": "string" },
                        "provider_region": { "type": "string" },
                        "policy_type": { "type": "string" },
                        "risk_band": { "type": "string", "enum": ["low", "medium", "high", "critical"] },
                        "strata_key": { "type": "string" },
                        "prior_reviewer_sample_count": { "type": "integer", "minimum": 0 },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rag": { "type": "string", "enum": ["GREEN", "AMBER", "RED"] },
                        "evidence_refs": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } }
                    }
                },
                "AuditSampleRecord": {
                    "type": "object",
                    "required": ["sample_id", "sample_mode", "population_definition", "inclusion_criteria", "selection_method", "sample_size", "reviewer", "assignment_queue", "selected_leads", "outcome_distribution"],
                    "properties": {
                        "sample_id": { "type": "string" },
                        "sample_mode": { "type": "string" },
                        "population_definition": { "type": "string" },
                        "inclusion_criteria": { "type": "object" },
                        "deterministic_seed": { "type": ["string", "null"] },
                        "selection_method": { "type": "string" },
                        "sample_size": { "type": "integer" },
                        "reviewer": { "type": "string" },
                        "assignment_queue": { "type": "string" },
                        "selected_leads": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AuditSampleLeadRecord" }
                        },
                        "outcome_distribution": {
                            "type": "object",
                            "properties": {
                                "selected_count": { "type": "integer", "minimum": 0 },
                                "reviewed_count": { "type": "integer", "minimum": 0 },
                                "open_count": { "type": "integer", "minimum": 0 },
                                "qa_conclusions": { "type": "object" },
                                "issue_types": { "type": "object" },
                                "feedback_targets": { "type": "object" },
                                "strata_distribution": { "type": "object" },
                                "review_mode_distribution": { "type": "object" },
                                "reviewer_history_distribution": { "type": "object" },
                                "baseline_measurement": {
                                    "type": "object",
                                    "properties": {
                                        "control_cohort": { "type": "boolean" },
                                        "measurement_goal": { "type": "string", "enum": ["false_positive_and_missed_risk_baseline"] },
                                        "missed_risk_review_targets": { "type": "integer", "minimum": 0 },
                                        "false_positive_review_targets": { "type": "integer", "minimum": 0 }
                                    }
                                }
                            }
                        },
                        "created_at": { "type": ["string", "null"] }
                    }
                },
                "AuditSampleListResponse": {
                    "type": "object",
                    "required": ["samples"],
                    "properties": {
                        "samples": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AuditSampleRecord" }
                        }
                    }
                },
                "AgentRunLogRecord": {
                    "type": "object",
                    "required": ["agent_run_id", "claim_id", "status", "decision_boundary", "output_json", "evidence_refs", "steps", "context_snapshots", "policy_checks", "tool_calls", "tool_results", "approvals"],
                    "properties": {
                        "agent_run_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "status": { "type": "string" },
                        "decision_boundary": { "type": "string" },
                        "output_json": { "type": "object" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "steps": { "type": "array", "items": { "type": "object" } },
                        "context_snapshots": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentContextSnapshotRecord" }
                        },
                        "policy_checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentPolicyCheckRecord" }
                        },
                        "tool_calls": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentToolCallRecord" }
                        },
                        "tool_results": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentToolResultRecord" }
                        },
                        "approvals": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentApprovalRecord" }
                        },
                        "created_at": { "type": ["string", "null"] },
                        "completed_at": { "type": ["string", "null"] }
                    }
                },
                "AgentContextSnapshotRecord": {
                    "type": "object",
                    "required": ["snapshot_id", "redaction_status", "context_json", "source_refs", "checksum"],
                    "properties": {
                        "snapshot_id": { "type": "string" },
                        "redaction_status": { "type": "string" },
                        "context_json": { "type": "object" },
                        "source_refs": { "type": "array", "items": { "type": "string" } },
                        "checksum": { "type": "string" }
                    }
                },
                "AgentToolCallRecord": {
                    "type": "object",
                    "required": ["tool_call_id", "tool_name", "status", "input_json", "evidence_refs"],
                    "properties": {
                        "tool_call_id": { "type": "string" },
                        "tool_name": { "type": "string" },
                        "status": { "type": "string" },
                        "input_json": { "type": "object" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "AgentPolicyCheckRecord": {
                    "type": "object",
                    "required": ["policy_check_id", "agent_run_id", "tool_call_id", "tool_name", "policy_name", "decision", "reason", "evidence_refs"],
                    "properties": {
                        "policy_check_id": { "type": "string" },
                        "agent_run_id": { "type": "string" },
                        "tool_call_id": { "type": "string" },
                        "tool_name": { "type": "string" },
                        "policy_name": { "type": "string" },
                        "decision": { "type": "string", "enum": ["allowed", "denied"] },
                        "reason": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"] }
                    }
                },
                "AgentToolResultRecord": {
                    "type": "object",
                    "required": ["tool_result_id", "tool_call_id", "tool_name", "status", "output_json", "evidence_refs"],
                    "properties": {
                        "tool_result_id": { "type": "string" },
                        "tool_call_id": { "type": "string" },
                        "tool_name": { "type": "string" },
                        "status": { "type": "string" },
                        "output_json": { "type": "object" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "AgentApprovalRecord": {
                    "type": "object",
                    "required": ["approval_id", "agent_run_id", "proposed_action", "decision", "approver", "reason", "evidence_refs"],
                    "properties": {
                        "approval_id": { "type": "string" },
                        "agent_run_id": { "type": "string" },
                        "proposed_action": { "type": "string" },
                        "decision": { "type": "string", "enum": ["pending", "approved", "rejected"] },
                        "approver": { "type": "string" },
                        "reason": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"] }
                    }
                },
                "SubmitAgentApprovalRequest": {
                    "type": "object",
                    "required": ["decision", "approver", "reason", "evidence_refs"],
                    "properties": {
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "approver": { "type": "string", "minLength": 1 },
                        "reason": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Agent approval reason must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Must include agent_run:{agent_run_id} for the approved or rejected run and must not contain PII.",
                            "items": { "type": "string", "minLength": 1 },
                            "contains": {
                                "type": "string",
                                "pattern": "^agent_run:"
                            }
                        }
                    }
                },
                "SubmitAgentApprovalResponse": {
                    "type": "object",
                    "required": ["approval", "audit_id"],
                    "properties": {
                        "approval": { "$ref": "#/components/schemas/AgentApprovalRecord" },
                        "audit_id": { "type": "string" }
                    }
                },
                "AgentRunLogListResponse": {
                    "type": "object",
                    "required": ["runs"],
                    "properties": {
                        "runs": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AgentRunLogRecord" }
                        }
                    }
                },
                "KnowledgeCase": {
                    "type": "object",
                    "required": ["case_id", "title", "fwa_type", "scheme_family", "diagnosis_code", "provider_region", "summary", "outcome", "tags", "evidence_refs"],
                    "properties": {
                        "case_id": { "type": "string" },
                        "title": { "type": "string" },
                        "fwa_type": { "type": "string" },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
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
                "PublishKnowledgeCaseRequest": {
                    "type": "object",
                    "required": ["case_id", "title", "fwa_type", "diagnosis_code", "provider_region", "provider_type", "summary", "outcome", "tags", "evidence_refs"],
                    "properties": {
                        "case_id": { "type": "string", "minLength": 1 },
                        "title": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Knowledge case title must not contain PII."
                        },
                        "fwa_type": { "type": "string", "minLength": 1 },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "diagnosis_code": { "type": "string", "minLength": 1 },
                        "provider_region": { "type": "string", "minLength": 1 },
                        "provider_type": { "type": "string", "minLength": 1 },
                        "summary": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Knowledge case summary must not contain PII."
                        },
                        "outcome": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Knowledge case outcome must not contain PII."
                        },
                        "tags": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Knowledge case tags must not contain PII.",
                            "items": { "type": "string", "minLength": 1 }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Must include at least one confirmed review source: investigation_results:* or qa_reviews:* and must not contain PII. When source_claim_id has a prior canonical_claim_context_trace, publish automatically preserves canonical evidence_refs from the scoring audit.",
                            "items": { "type": "string", "minLength": 1 },
                            "contains": {
                                "type": "string",
                                "pattern": "^(investigation_results|qa_reviews):"
                            }
                        },
                        "source_claim_id": { "type": ["string", "null"] }
                    }
                },
                "PublishKnowledgeCaseResponse": {
                    "type": "object",
                    "required": ["case", "audit_id"],
                    "properties": {
                        "case": { "$ref": "#/components/schemas/KnowledgeCase" },
                        "audit_id": { "type": "string" }
                    }
                },
                "SimilarCaseSearchRequest": {
                    "type": "object",
                    "required": ["diagnosis_code", "provider_region", "tags"],
                    "properties": {
                        "claim_id": { "type": ["string", "null"] },
                        "diagnosis_code": { "type": "string", "minLength": 1 },
                        "provider_region": { "type": "string", "minLength": 1 },
                        "tags": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } }
                    }
                },
                "AgentInvestigationPrefill": {
                    "type": "object",
                    "required": ["claim_id", "risk_score", "rag", "scheme_family", "top_reasons", "similar_case_query", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string", "minLength": 1 },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rag": { "type": "string", "enum": ["GREEN", "AMBER", "RED"] },
                        "scheme_family": {
                            "oneOf": [
                                { "$ref": "#/components/schemas/FwaSchemeFamily" },
                                { "type": "null" }
                            ]
                        },
                        "top_reasons": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
                        "similar_case_query": { "$ref": "#/components/schemas/SimilarCaseSearchRequest" },
                        "evidence_refs": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } }
                    }
                },
                "SimilarCase": {
                    "type": "object",
                    "required": ["case_id", "title", "scheme_family", "similarity_score", "matched_signals", "retrieval_method", "provenance_refs", "summary", "outcome", "evidence_refs"],
                    "properties": {
                        "case_id": { "type": "string" },
                        "title": { "type": "string" },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "similarity_score": { "type": "number" },
                        "matched_signals": { "type": "array", "items": { "type": "string" } },
                        "retrieval_method": { "type": "string" },
                        "provenance_refs": { "type": "array", "items": { "type": "string" } },
                        "summary": { "type": "string" },
                        "outcome": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "SimilarCaseSearchResponse": {
                    "type": "object",
                    "required": ["results"],
                    "properties": {
                        "results": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/SimilarCase" }
                        }
                    }
                },
                "AgentSimilarCase": {
                    "type": "object",
                    "required": ["case_id", "similarity_score", "matched_signals", "provenance_refs", "evidence_refs"],
                    "properties": {
                        "case_id": { "type": "string" },
                        "similarity_score": { "type": "number" },
                        "matched_signals": { "type": "array", "items": { "type": "string" } },
                        "provenance_refs": { "type": "array", "items": { "type": "string" } },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "AgentInvestigationRequest": {
                    "type": "object",
                    "required": ["claim_id", "risk_score", "rag", "top_reasons", "similar_case_query"],
                    "properties": {
                        "claim_id": { "type": "string", "minLength": 1 },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rag": { "type": "string", "enum": ["GREEN", "AMBER", "RED"] },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "top_reasons": { "type": "array", "minItems": 1, "items": { "type": "string", "minLength": 1 } },
                        "similar_case_query": { "$ref": "#/components/schemas/SimilarCaseSearchRequest" }
                    }
                },
                "AgentInvestigationResponse": {
                    "type": "object",
                    "required": ["agent_run_id", "decision_boundary", "risk_summary", "findings", "investigation_checklist", "similar_cases", "qa_opinion_draft", "evidence_sufficiency", "evidence_refs", "evidence_refs_by_type"],
                    "properties": {
                        "agent_run_id": { "type": "string" },
                        "decision_boundary": { "type": "string", "const": "assistive_only" },
                        "risk_summary": { "type": "string" },
                        "findings": { "type": "array", "items": { "type": "object" } },
                        "investigation_checklist": { "type": "array", "items": { "type": "string" } },
                        "similar_cases": { "type": "array", "items": { "$ref": "#/components/schemas/AgentSimilarCase" } },
                        "qa_opinion_draft": { "type": "string" },
                        "evidence_sufficiency": { "$ref": "#/components/schemas/EvidenceSufficiency" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "evidence_refs_by_type": { "$ref": "#/components/schemas/EvidenceReferenceBuckets" }
                    }
                },
                "EvidenceReferenceBuckets": {
                    "type": "object",
                    "required": ["claim", "rule", "model", "anomaly", "document", "similar_case"],
                    "properties": {
                        "claim": { "type": "array", "items": { "type": "string" } },
                        "rule": { "type": "array", "items": { "type": "string" } },
                        "model": { "type": "array", "items": { "type": "string" } },
                        "anomaly": { "type": "array", "items": { "type": "string" } },
                        "document": { "type": "array", "items": { "type": "string" } },
                        "similar_case": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "EvidenceSufficiency": {
                    "type": "object",
                    "required": ["scheme_family", "status", "minimum_evidence", "present_evidence", "missing_evidence"],
                    "properties": {
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "status": { "type": "string", "enum": ["sufficient", "needs_more_evidence"] },
                        "minimum_evidence": { "type": "array", "items": { "type": "string" } },
                        "present_evidence": { "type": "array", "items": { "type": "string" } },
                        "missing_evidence": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "InvestigationResultRequest": {
                    "type": "object",
                    "required": ["investigation_id", "claim_id", "outcome", "confirmed_fwa", "notes", "evidence_refs"],
                    "properties": {
                        "investigation_id": { "type": "string", "minLength": 1 },
                        "case_id": { "type": ["string", "null"], "minLength": 1 },
                        "claim_id": { "type": "string", "minLength": 1 },
                        "outcome": { "type": "string", "minLength": 1 },
                        "confirmed_fwa": { "type": "boolean" },
                        "financial_impact_type": {
                            "type": ["string", "null"],
                            "enum": ["prevented_payment", "recovered_amount", "avoided_future_exposure", "deterrence_estimate", "estimated_impact", null]
                        },
                        "saving_amount": {
                            "type": ["string", "null"],
                            "format": "decimal",
                            "description": "Non-negative decimal string."
                        },
                        "currency": { "type": ["string", "null"] },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Investigation writeback notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "description": "Structured evidence references must not contain PII. For claims with a prior normalized scoring trace, canonical evidence refs from that trace are merged into the persisted investigation result and response.",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "QaResultRequest": {
                    "type": "object",
                    "required": ["qa_case_id", "claim_id", "qa_conclusion", "issue_type", "feedback_target", "notes", "evidence_refs"],
                    "properties": {
                        "qa_case_id": { "type": "string", "minLength": 1 },
                        "claim_id": { "type": "string", "minLength": 1 },
                        "qa_conclusion": {
                            "type": "string",
                            "enum": ["pass", "issue_found_return", "issue_found_escalate"]
                        },
                        "issue_type": {
                            "type": "string",
                            "enum": ["none", "confirmed_fwa", "false_positive", "improper_payment", "insufficient_evidence", "abuse_not_fraud", "documentation_issue", "medical_necessity_issue", "policy_exclusion", "qa_review_completed", "alert_handling_incomplete", "medical_reasonableness", "provider_pattern", "model_under_scored_confirmed_issue", "workflow_missing_evidence"]
                        },
                        "feedback_target": {
                            "type": "string",
                            "enum": ["rules", "model", "models", "features", "provider_profile", "workflow", "tpa"]
                        },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "QA writeback notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "description": "Structured evidence references must not contain PII.",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "PilotWritebackResponse": {
                    "type": "object",
                    "required": ["claim_id", "event_type", "event_status", "audit_id", "run_id", "idempotency_key", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "event_type": { "type": "string" },
                        "event_status": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "idempotency_key": {
                            "type": "string",
                            "description": "Stable key for retry-safe TPA writeback processing."
                        },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "MemberProfileSummaryResponse": {
                    "type": "object",
                    "required": ["member_id", "claim_count", "policy_count", "total_claim_amount", "currency", "high_risk_claim_count", "risk_level_summary", "profile_summary", "evidence_refs"],
                    "properties": {
                        "member_id": { "type": "string" },
                        "claim_count": { "type": "integer" },
                        "policy_count": { "type": "integer" },
                        "total_claim_amount": { "type": "string", "format": "decimal" },
                        "currency": { "type": "string" },
                        "high_risk_claim_count": { "type": "integer" },
                        "latest_claim_id": { "type": ["string", "null"] },
                        "risk_level_summary": { "type": "string", "enum": ["no_high_risk_history", "has_high_risk_history"] },
                        "profile_summary": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "QaFeedbackItem": {
                    "type": "object",
                    "required": ["feedback_id", "qa_case_id", "claim_id", "feedback_target", "issue_type", "qa_conclusion", "source", "status", "priority", "summary", "note_present", "evidence_refs", "status_evidence_refs"],
                    "properties": {
                        "feedback_id": { "type": "string" },
                        "qa_case_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "feedback_target": {
                            "type": "string",
                            "enum": ["rules", "model", "features", "provider_profile", "workflow", "tpa"]
                        },
                        "issue_type": {
                            "type": "string",
                            "enum": ["none", "confirmed_fwa", "false_positive", "improper_payment", "insufficient_evidence", "abuse_not_fraud", "documentation_issue", "medical_necessity_issue", "policy_exclusion", "qa_review_completed", "alert_handling_incomplete", "medical_reasonableness", "provider_pattern", "model_under_scored_confirmed_issue", "workflow_missing_evidence"]
                        },
                        "qa_conclusion": {
                            "type": "string",
                            "enum": ["pass", "issue_found_return", "issue_found_escalate"]
                        },
                        "source": { "type": "string", "const": "qa_review" },
                        "status": { "type": "string" },
                        "priority": { "type": "string" },
                        "summary": { "type": "string" },
                        "note_present": { "type": "boolean" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"], "format": "date-time" },
                        "status_updated_by": { "type": ["string", "null"] },
                        "status_audit_id": { "type": ["string", "null"] },
                        "status_updated_at": { "type": ["string", "null"], "format": "date-time" },
                        "status_evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "QaFeedbackItemListResponse": {
                    "type": "object",
                    "required": ["items"],
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/QaFeedbackItem" }
                        }
                    }
                },
                "UpdateQaFeedbackStatusRequest": {
                    "type": "object",
                    "required": ["status", "actor_id", "notes", "evidence_refs"],
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["open", "in_progress", "resolved", "dismissed"]
                        },
                        "actor_id": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "QA feedback status notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "description": "Structured evidence references must include qa_feedback:{feedback_id} for the updated feedback item and must not contain PII.",
                            "items": { "type": "string", "minLength": 1 },
                            "contains": { "type": "string", "pattern": "^qa_feedback:" }
                        }
                    }
                },
                "UpdateQaFeedbackStatusResponse": {
                    "type": "object",
                    "required": ["item", "audit_id"],
                    "properties": {
                        "item": { "$ref": "#/components/schemas/QaFeedbackItem" },
                        "audit_id": { "type": "string" }
                    }
                },
                "QaQueueItem": {
                    "type": "object",
                    "required": ["qa_case_id", "sample_id", "lead_id", "claim_id", "scheme_family", "rag", "risk_score", "reviewer", "assignment_queue", "status", "evidence_refs", "canonical_source_refs", "canonical_evidence_refs"],
                    "properties": {
                        "qa_case_id": { "type": "string" },
                        "sample_id": { "type": "string" },
                        "lead_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "scheme_family": { "type": "string" },
                        "rag": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "reviewer": { "type": "string" },
                        "assignment_queue": { "type": "string" },
                        "status": { "type": "string", "enum": ["open", "reviewed"] },
                        "qa_conclusion": {
                            "type": ["string", "null"],
                            "enum": ["pass", "issue_found_return", "issue_found_escalate", null]
                        },
                        "issue_type": {
                            "type": ["string", "null"],
                            "enum": ["none", "confirmed_fwa", "false_positive", "improper_payment", "insufficient_evidence", "abuse_not_fraud", "documentation_issue", "medical_necessity_issue", "policy_exclusion", "qa_review_completed", "alert_handling_incomplete", "medical_reasonableness", "provider_pattern", "model_under_scored_confirmed_issue", "workflow_missing_evidence", null]
                        },
                        "feedback_target": {
                            "type": ["string", "null"],
                            "enum": ["rules", "model", "features", "provider_profile", "workflow", "tpa", null]
                        },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "canonical_source_refs": { "type": "array", "items": { "type": "string" } },
                        "canonical_evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "QaQueueListResponse": {
                    "type": "object",
                    "required": ["items"],
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/QaQueueItem" }
                        }
                    }
                },
                "QaQueueSummaryResponse": {
                    "type": "object",
                    "required": ["open_count", "in_progress_count", "resolved_count", "dismissed_count", "unresolved_count", "rules_feedback_count", "models_feedback_count", "features_feedback_count", "provider_profile_feedback_count", "workflow_feedback_count", "tpa_feedback_count", "high_priority_count", "evidence_backed_count", "highest_priority"],
                    "properties": {
                        "open_count": { "type": "integer" },
                        "in_progress_count": { "type": "integer" },
                        "resolved_count": { "type": "integer" },
                        "dismissed_count": { "type": "integer" },
                        "unresolved_count": { "type": "integer" },
                        "rules_feedback_count": { "type": "integer" },
                        "models_feedback_count": { "type": "integer" },
                        "features_feedback_count": { "type": "integer" },
                        "provider_profile_feedback_count": { "type": "integer" },
                        "workflow_feedback_count": { "type": "integer" },
                        "tpa_feedback_count": { "type": "integer" },
                        "high_priority_count": { "type": "integer" },
                        "evidence_backed_count": { "type": "integer" },
                        "highest_priority": { "type": "string", "enum": ["none", "low", "medium", "high"] }
                    }
                },
                "OutcomeLabel": {
                    "type": "object",
                    "required": ["label_id", "claim_id", "label_name", "label_value", "source_type", "source_id", "governance_status", "feedback_target", "evidence_refs"],
                    "properties": {
                        "label_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "label_name": { "type": "string" },
                        "label_value": { "type": "string" },
                        "source_type": { "type": "string", "enum": ["investigation_result", "qa_review", "case_status", "medical_review", "lead_triage"] },
                        "source_id": { "type": "string" },
                        "governance_status": { "type": "string", "enum": ["approved_for_training", "needs_review"] },
                        "feedback_target": {
                            "type": "string",
                            "enum": ["rules", "model", "features", "provider_profile", "workflow", "tpa"]
                        },
                        "currency": { "type": ["string", "null"] },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "OutcomeLabelListResponse": {
                    "type": "object",
                    "required": ["labels"],
                    "properties": {
                        "labels": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/OutcomeLabel" }
                        }
                    }
                },
                "ClaimAuditHistoryResponse": {
                    "type": "object",
                    "required": ["claim_id", "events"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "events": { "type": "array", "items": { "type": "object" } }
                    }
                }
            }
        }
    }))
}

fn routing_policy_lifecycle_parameters() -> Value {
    json!([
        {
            "name": "policy_id",
            "in": "path",
            "required": true,
            "schema": { "type": "string" }
        },
        {
            "name": "review_mode",
            "in": "path",
            "required": true,
            "schema": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] }
        },
        {
            "name": "version",
            "in": "path",
            "required": true,
            "schema": { "type": "integer", "minimum": 1 }
        }
    ])
}

fn rule_lifecycle_parameters() -> Value {
    json!([
        {
            "name": "rule_id",
            "in": "path",
            "required": true,
            "schema": { "type": "string" }
        }
    ])
}

fn rule_lifecycle_request_body() -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/RuleLifecycleRequest" }
            }
        }
    })
}

fn routing_policy_lifecycle_request_body() -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/RoutingPolicyLifecycleRequest" }
            }
        }
    })
}

fn model_lifecycle_request_body() -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/ModelLifecycleRequest" }
            }
        }
    })
}
