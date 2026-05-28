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
                    "required": ["rule_id", "name", "status", "owner", "latest_version", "score", "alert_code", "recommended_action"],
                    "properties": {
                        "rule_id": { "type": "string" },
                        "name": { "type": "string" },
                        "status": { "type": "string", "enum": ["draft", "active", "submitted", "approved"] },
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
                "RuleDetailResponse": {
                    "type": "object",
                    "required": ["summary", "versions", "audit_events"],
                    "properties": {
                        "summary": { "$ref": "#/components/schemas/RuleSummary" },
                        "versions": {
                            "type": "array",
                            "items": { "type": "object" }
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
                            "enum": ["runtime", "approval", "evaluation", "labels", "metadata", "missing"]
                        }
                    }
                },
                "ModelPromotionGatesResponse": {
                    "type": "object",
                    "required": ["model_key", "model_version", "decision", "passed_count", "total_count", "latest_evaluation_id", "data_status", "scored_runs", "gates", "blockers"],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "decision": { "type": "string", "enum": ["routing_allowed", "routing_blocked"] },
                        "passed_count": { "type": "integer" },
                        "total_count": { "type": "integer" },
                        "latest_evaluation_id": { "type": "string" },
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
                "DashboardSummaryResponse": {
                    "type": "object",
                    "required": ["suspected_claims", "confirmed_fwa", "risk_amount", "saving_amount", "rag_distribution", "rule_hits", "model_scores", "layer_scores", "saving_attributions", "investigation_results", "qa_reviews"],
                    "properties": {
                        "suspected_claims": { "type": "integer" },
                        "confirmed_fwa": { "type": "integer" },
                        "risk_amount": { "type": "string", "format": "decimal" },
                        "saving_amount": { "type": "string", "format": "decimal" },
                        "rag_distribution": {
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
                        "investigation_results": { "type": "integer" },
                        "qa_reviews": { "type": "integer" }
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
                    "required": ["case_id", "lead_id", "claim_id", "member_id", "provider_id", "source_system", "scheme_family", "lead_source", "status", "assignee", "reviewer", "priority", "routing_reason", "evidence_package"],
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
                        "evidence_package": { "type": "object" }
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
                "PublishKnowledgeCaseRequest": {
                    "type": "object",
                    "required": ["case_id", "title", "fwa_type", "diagnosis_code", "provider_region", "provider_type", "summary", "outcome", "tags", "evidence_refs"],
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
                    "required": ["case_id", "title", "similarity_score", "matched_signals", "retrieval_method", "provenance_refs", "summary", "outcome", "evidence_refs"],
                    "properties": {
                        "case_id": { "type": "string" },
                        "title": { "type": "string" },
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
                },
                "InvestigationResultRequest": {
                    "type": "object",
                    "required": ["investigation_id", "claim_id", "outcome", "confirmed_fwa", "notes", "evidence_refs"],
                    "properties": {
                        "investigation_id": { "type": "string" },
                        "claim_id": { "type": "string" },
                        "outcome": { "type": "string" },
                        "confirmed_fwa": { "type": "boolean" },
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
