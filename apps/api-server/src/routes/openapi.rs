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
            "/api/v1/ops/rules/{rule_id}/rollback": {
                "post": {
                    "summary": "Rollback an active rule out of production routing",
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
                                "schema": { "type": "object" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Registered feature set"
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
                                "schema": { "type": "object" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Registered model dataset"
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
                            "description": "Model evaluation metric list"
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
                                "schema": { "type": "object" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Registered model evaluation"
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
                    "summary": "Triage a lead into an investigation case",
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
                            "description": "Created or updated investigation case",
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
            "/api/v1/ops/models/{model_key}/rollback": {
                "post": {
                    "summary": "Roll back an active model to approved status",
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
                        }
                    }
                }
            },
            "/api/v1/ops/qa/feedback-items": {
                "get": {
                    "summary": "List QA feedback items for rule and model operators",
                    "security": [{ "ApiKeyAuth": [] }],
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
                    "summary": "List governed outcome labels from investigation and QA writeback",
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
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment", "both"],
                            "default": "pre_payment",
                            "description": "Routing policy context for pre-payment, post-payment, or shared review."
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
                            { "required": ["provider_profile"] }
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
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment", "both"],
                            "default": "pre_payment",
                            "description": "Routing policy context for pre-payment, post-payment, or shared review."
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
                        },
                        "documents": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/DocumentPayload"
                            }
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfilePayload"
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
                "DocumentPayload": {
                    "type": "object",
                    "required": ["external_document_id", "document_type"],
                    "properties": {
                        "external_document_id": {
                            "type": "string"
                        },
                        "document_type": {
                            "type": "string",
                            "description": "Examples: medical_record, clinical_order, radiology_report, prescription, lab_result"
                        },
                        "linked_item_codes": {
                            "type": "array",
                            "items": {
                                "type": "string"
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
                        "confirmed_fwa_count",
                        "false_positive_count"
                    ],
                    "properties": {
                        "window_days": { "type": "integer", "enum": [30, 90, 180] },
                        "claim_count": { "type": "integer", "minimum": 0 },
                        "total_claim_amount": { "type": "string", "format": "decimal" },
                        "high_cost_item_ratio": { "type": "number", "minimum": 0, "maximum": 1 },
                        "diagnosis_procedure_mismatch_rate": { "type": "number", "minimum": 0, "maximum": 1 },
                        "peer_amount_percentile": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "peer_frequency_percentile": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 }
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
                        "scores",
                        "alerts",
                        "top_reasons",
                        "layers",
                        "clinical_evidence",
                        "provider_profile",
                        "similar_cases",
                        "evidence_refs"
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
                            "enum": ["pre_payment", "post_payment", "both"]
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
                            "enum": ["AutoApprove", "ManualReview", "EscalateInvestigation"]
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
                        "layers": {
                            "type": "array",
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
                        "similar_cases": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/SimilarCase"
                            }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": {
                                "oneOf": [
                                    { "type": "object" },
                                    { "type": "string" }
                                ]
                            }
                        }
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
                "ProviderProfileAssessment": {
                    "type": "object",
                    "required": [
                        "provider_id",
                        "risk_score",
                        "risk_tier",
                        "review_required",
                        "review_route",
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
                "ProviderRiskSummaryItem": {
                    "type": "object",
                    "required": ["provider_id", "risk_score", "risk_tier", "review_required", "review_route", "claim_count", "outlier_flags", "evidence_refs"],
                    "properties": {
                        "provider_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "risk_tier": { "type": "string" },
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string" },
                        "claim_count": { "type": "integer" },
                        "latest_claim_id": { "type": ["string", "null"] },
                        "outlier_flags": { "type": "array", "items": { "type": "string" } },
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
                "RuleSummary": {
                    "type": "object",
                    "required": ["rule_id", "name", "status", "owner", "latest_version", "review_mode", "scheme_family", "score", "alert_code", "recommended_action"],
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
                        "recommended_action": { "type": "string" }
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
                            "enum": ["runtime", "backtest", "approval", "labels", "metadata", "missing"]
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
                    "required": ["decision", "reviewer", "notes"],
                    "properties": {
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                },
                "RulePromotionReview": {
                    "type": "object",
                    "required": ["rule_id", "rule_version", "decision", "reviewer", "notes"],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "rule_version": { "type": "integer" },
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string" },
                        "notes": { "type": "string" },
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
                "WebhookEvent": {
                    "type": "object",
                    "required": ["event_id", "event_type", "source_event_type", "source_audit_id", "claim_id", "run_id", "delivery_status", "retry_count", "max_attempts", "next_attempt_at", "last_attempt_at", "last_response_status_code", "last_error_message", "idempotency_key", "signature_key_id", "signature_algorithm", "signature_base_string", "payload", "evidence_refs", "occurred_at"],
                    "properties": {
                        "event_id": { "type": "string" },
                        "event_type": {
                            "type": "string",
                            "enum": ["fwa.score.completed", "fwa.case.routed", "fwa.investigation.closed", "fwa.qa.reviewed"]
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
                        "error_message": { "type": ["string", "null"] }
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
                            "enum": ["high_risk_routing", "sla_breach"]
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
                        "rule": { "type": "object" },
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
                        "rule": { "type": "object" }
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
                        "rule": { "type": "object" },
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
                "FactorReadinessResponse": {
                    "type": "object",
                    "required": ["dataset_count", "factor_count", "label_count", "entity_key_count", "data_quality_score", "data_quality_status", "online_ready_count", "rule_convertible_count", "mapped_factor_count", "high_missing_count", "unstable_factor_count", "unowned_factor_count"],
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
                        "unowned_factor_count": { "type": "integer" }
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
                    "required": ["evaluation_run_id", "model_key", "model_version", "model_dataset_id", "confusion_matrix_json", "metrics_json"],
                    "properties": {
                        "evaluation_run_id": { "type": "string" },
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "model_dataset_id": { "type": "string" },
                        "auc": { "type": ["string", "null"] },
                        "ks": { "type": ["string", "null"] },
                        "precision": { "type": ["string", "null"] },
                        "recall": { "type": ["string", "null"] },
                        "f1": { "type": ["string", "null"] },
                        "accuracy": { "type": ["string", "null"] },
                        "threshold": { "type": ["string", "null"] },
                        "confusion_matrix_json": { "type": "object" },
                        "feature_importance_uri": { "type": ["string", "null"] },
                        "metrics_json": { "type": "object" }
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
                "ModelLifecycleResponse": {
                    "type": "object",
                    "required": ["model_key", "version", "status"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "version": { "type": "string" },
                        "status": { "type": "string" }
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
                            "enum": ["runtime", "approval", "dataset", "evaluation", "labels", "metadata", "missing"]
                        }
                    }
                },
                "ModelPromotionGatesResponse": {
                    "type": "object",
                    "required": ["model_key", "model_version", "review_mode", "decision", "passed_count", "total_count", "latest_evaluation_id", "source_dataset_id", "source_data_quality_score", "source_data_quality_status", "data_status", "scored_runs", "gates", "blockers"],
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
                        "requested_by": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                },
                "UpdateModelRetrainingJobStatusRequest": {
                    "type": "object",
                    "required": ["status", "actor", "notes"],
                    "properties": {
                        "status": { "type": "string", "enum": ["queued", "running", "validation", "completed", "failed", "cancelled"] },
                        "actor": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                },
                "ClaimModelRetrainingJobRequest": {
                    "type": "object",
                    "required": ["actor", "notes"],
                    "properties": {
                        "actor": { "type": "string" },
                        "notes": { "type": "string" },
                        "model_key": { "type": ["string", "null"] }
                    }
                },
                "CompleteModelRetrainingJobRequest": {
                    "type": "object",
                    "required": ["actor", "notes", "candidate_model_version", "artifact_uri", "validation_report_uri", "evaluation_run_id", "confusion_matrix_json", "metrics_json"],
                    "properties": {
                        "actor": { "type": "string" },
                        "notes": { "type": "string" },
                        "candidate_model_version": { "type": "string" },
                        "artifact_uri": { "type": "string" },
                        "endpoint_url": { "type": ["string", "null"] },
                        "validation_report_uri": { "type": "string" },
                        "evaluation_run_id": { "type": "string" },
                        "auc": { "type": ["string", "null"] },
                        "ks": { "type": ["string", "null"] },
                        "precision": { "type": ["string", "null"] },
                        "recall": { "type": ["string", "null"] },
                        "f1": { "type": ["string", "null"] },
                        "accuracy": { "type": ["string", "null"] },
                        "threshold": { "type": ["string", "null"] },
                        "confusion_matrix_json": { "type": "object" },
                        "feature_importance_uri": { "type": ["string", "null"] },
                        "metrics_json": { "type": "object" }
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
                    "required": ["decision", "reviewer", "notes"],
                    "properties": {
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                },
                "ModelPromotionReview": {
                    "type": "object",
                    "required": ["model_key", "model_version", "decision", "reviewer", "notes"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "decision": { "type": "string", "enum": ["approved", "rejected"] },
                        "reviewer": { "type": "string" },
                        "notes": { "type": "string" },
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
                "SavingAttributionSummary": {
                    "type": "object",
                    "required": ["source_type", "source_id", "action", "saving_amount", "currency", "claim_count"],
                    "properties": {
                        "source_type": { "type": "string" },
                        "source_id": { "type": "string" },
                        "action": { "type": "string" },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "currency": { "type": "string" },
                        "claim_count": { "type": "integer" }
                    }
                },
                "SavingSegmentSummary": {
                    "type": "object",
                    "required": ["segment_type", "segment_id", "saving_amount", "currency", "claim_count", "attribution_count", "roi"],
                    "properties": {
                        "segment_type": { "type": "string", "enum": ["provider", "scheme"] },
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
                    "required": ["suspected_claims", "confirmed_fwa", "risk_amount", "saving_amount", "rag_distribution", "scheme_distribution", "rule_hits", "model_scores", "layer_scores", "saving_attributions", "saving_segments", "value_measurement", "label_pool", "qa_queue", "case_sla", "agent_governance", "model_governance", "rule_governance", "investigation_results", "qa_reviews"],
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
                    "required": ["total_labels", "approved_for_training", "needs_review", "rule_feedback", "model_feedback", "workflow_feedback"],
                    "properties": {
                        "total_labels": { "type": "integer" },
                        "approved_for_training": { "type": "integer" },
                        "needs_review": { "type": "integer" },
                        "rule_feedback": { "type": "integer" },
                        "model_feedback": { "type": "integer" },
                        "workflow_feedback": { "type": "integer" }
                    }
                },
                "DashboardQaQueue": {
                    "type": "object",
                    "required": ["sampled_cases", "open_cases", "reviewed_cases", "disagreement_cases", "disagreement_rate"],
                    "properties": {
                        "sampled_cases": { "type": "integer" },
                        "open_cases": { "type": "integer" },
                        "reviewed_cases": { "type": "integer" },
                        "disagreement_cases": { "type": "integer" },
                        "disagreement_rate": { "type": "number" }
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
                    "required": ["total_runs", "successful_runs", "pending_approvals", "approved_approvals", "rejected_approvals"],
                    "properties": {
                        "total_runs": { "type": "integer" },
                        "successful_runs": { "type": "integer" },
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
                    "required": ["lead_id", "run_id", "claim_id", "member_id", "provider_id", "source_system", "scheme_family", "lead_source", "status", "disposition", "risk_score", "rag", "reason", "evidence_refs"],
                    "properties": {
                        "lead_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "member_id": { "type": "string" },
                        "provider_id": { "type": "string" },
                        "source_system": { "type": "string" },
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
                    "required": ["case_id", "lead_id", "claim_id", "member_id", "provider_id", "source_system", "scheme_family", "lead_source", "status", "assignee", "reviewer", "priority", "routing_reason", "evidence_package", "sla_target_hours", "sla_status", "time_to_triage_hours", "time_to_closure_hours"],
                    "properties": {
                        "case_id": { "type": "string" },
                        "lead_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "member_id": { "type": "string" },
                        "provider_id": { "type": "string" },
                        "source_system": { "type": "string" },
                        "scheme_family": { "type": "string" },
                        "lead_source": { "type": "string" },
                        "status": { "type": "string" },
                        "assignee": { "type": "string" },
                        "reviewer": { "type": "string" },
                        "priority": { "type": "string" },
                        "routing_reason": { "type": "string" },
                        "evidence_package": { "type": "object" },
                        "sla_target_hours": { "type": "integer" },
                        "sla_status": { "type": "string" },
                        "time_to_triage_hours": { "type": "number" },
                        "time_to_closure_hours": { "type": ["number", "null"] }
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
                    "required": ["decision", "assignee", "reviewer", "priority", "notes"],
                    "properties": {
                        "decision": { "type": "string" },
                        "assignee": { "type": "string" },
                        "reviewer": { "type": "string" },
                        "priority": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                },
                "TriageLeadResponse": {
                    "type": "object",
                    "required": ["case", "audit_id"],
                    "properties": {
                        "case": { "$ref": "#/components/schemas/Case" },
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
                        "actor_id": { "type": "string" },
                        "notes": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
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
                        "population_definition": { "type": "string" },
                        "inclusion_criteria": { "type": "object" },
                        "deterministic_seed": { "type": ["string", "null"] },
                        "sample_size": { "type": "integer", "minimum": 1 },
                        "reviewer": { "type": "string" },
                        "assignment_queue": { "type": "string" }
                    }
                },
                "AuditSampleLeadRecord": {
                    "type": "object",
                    "required": ["lead_id", "claim_id", "scheme_family", "risk_score", "rag", "evidence_refs"],
                    "properties": {
                        "lead_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "scheme_family": { "type": "string" },
                        "risk_score": { "type": "integer" },
                        "rag": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
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
                        "outcome_distribution": { "type": "object" },
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
                        "approver": { "type": "string" },
                        "reason": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
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
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
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
                        "diagnosis_code": { "type": "string" },
                        "provider_region": { "type": "string" },
                        "tags": { "type": "array", "items": { "type": "string" } }
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
                "AgentInvestigationRequest": {
                    "type": "object",
                    "required": ["claim_id", "risk_score", "rag", "top_reasons", "similar_case_query"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "rag": { "type": "string" },
                        "scheme_family": { "$ref": "#/components/schemas/FwaSchemeFamily" },
                        "top_reasons": { "type": "array", "items": { "type": "string" } },
                        "similar_case_query": { "$ref": "#/components/schemas/SimilarCaseSearchRequest" }
                    }
                },
                "AgentInvestigationResponse": {
                    "type": "object",
                    "required": ["agent_run_id", "decision_boundary", "risk_summary", "findings", "investigation_checklist", "similar_cases", "qa_opinion_draft", "evidence_sufficiency", "evidence_refs"],
                    "properties": {
                        "agent_run_id": { "type": "string" },
                        "decision_boundary": { "type": "string", "const": "assistive_only" },
                        "risk_summary": { "type": "string" },
                        "findings": { "type": "array", "items": { "type": "object" } },
                        "investigation_checklist": { "type": "array", "items": { "type": "string" } },
                        "similar_cases": { "type": "array", "items": { "type": "object" } },
                        "qa_opinion_draft": { "type": "string" },
                        "evidence_sufficiency": { "$ref": "#/components/schemas/EvidenceSufficiency" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
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
                        "investigation_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "outcome": { "type": "string" },
                        "confirmed_fwa": { "type": "boolean" },
                        "financial_impact_type": {
                            "type": ["string", "null"],
                            "enum": ["prevented_payment", "recovered_amount", "avoided_future_exposure", "deterrence_estimate", "estimated_impact", null]
                        },
                        "saving_amount": { "type": ["string", "null"], "format": "decimal" },
                        "currency": { "type": ["string", "null"] },
                        "notes": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "QaResultRequest": {
                    "type": "object",
                    "required": ["qa_case_id", "claim_id", "qa_conclusion", "issue_type", "feedback_target", "notes", "evidence_refs"],
                    "properties": {
                        "qa_case_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "qa_conclusion": { "type": "string" },
                        "issue_type": { "type": "string" },
                        "feedback_target": { "type": "string" },
                        "notes": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "PilotWritebackResponse": {
                    "type": "object",
                    "required": ["claim_id", "event_type", "event_status", "audit_id", "run_id", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "event_type": { "type": "string" },
                        "event_status": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "run_id": { "type": "string" },
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
                    "required": ["feedback_id", "qa_case_id", "claim_id", "feedback_target", "issue_type", "qa_conclusion", "source", "status", "priority", "summary", "note_present", "evidence_refs"],
                    "properties": {
                        "feedback_id": { "type": "string" },
                        "qa_case_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "feedback_target": { "type": "string" },
                        "issue_type": { "type": "string" },
                        "qa_conclusion": { "type": "string" },
                        "source": { "type": "string", "const": "qa_review" },
                        "status": { "type": "string" },
                        "priority": { "type": "string" },
                        "summary": { "type": "string" },
                        "note_present": { "type": "boolean" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"], "format": "date-time" }
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
                "QaQueueItem": {
                    "type": "object",
                    "required": ["qa_case_id", "sample_id", "lead_id", "claim_id", "scheme_family", "rag", "risk_score", "reviewer", "assignment_queue", "status", "evidence_refs"],
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
                        "qa_conclusion": { "type": ["string", "null"] },
                        "issue_type": { "type": ["string", "null"] },
                        "feedback_target": { "type": ["string", "null"] },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
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
                    "required": ["open_count", "rules_feedback_count", "models_feedback_count", "tpa_feedback_count", "high_priority_count", "evidence_backed_count", "highest_priority"],
                    "properties": {
                        "open_count": { "type": "integer" },
                        "rules_feedback_count": { "type": "integer" },
                        "models_feedback_count": { "type": "integer" },
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
                        "source_type": { "type": "string", "enum": ["investigation_result", "qa_review"] },
                        "source_id": { "type": "string" },
                        "governance_status": { "type": "string", "enum": ["approved_for_training", "needs_review"] },
                        "feedback_target": { "type": "string" },
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
