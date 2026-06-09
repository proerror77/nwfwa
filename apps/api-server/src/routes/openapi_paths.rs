use serde_json::{json, Value};

pub(super) fn openapi_paths() -> Value {
    json!({
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
            "/api/v1/ops/backfills": {
                "get": {
                    "summary": "List historical replay backfill jobs",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Historical replay jobs",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create a historical replay backfill job from governed candidates",
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
                            "description": "Created historical replay job",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/backfills/{job_id}/leads": {
                "get": {
                    "summary": "List candidate leads captured by a historical replay job",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "job_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Backfill candidate leads",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/evidence-requests": {
                "get": {
                    "summary": "List generated evidence requests",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Evidence request queue",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/evidence-requests/generate": {
                "post": {
                    "summary": "Generate evidence requests from scoring gaps",
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
                            "description": "Generated evidence requests",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/evidence-requests/{request_id}/status": {
                "post": {
                    "summary": "Update evidence request collection status",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "request_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
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
                            "description": "Updated evidence request",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/label-bootstrap/queue": {
                "get": {
                    "summary": "List label bootstrap items awaiting governance review",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Label bootstrap queue",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/label-bootstrap/items/{item_id}/review": {
                "post": {
                    "summary": "Record a governed review for a bootstrap label candidate",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "item_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
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
                            "description": "Recorded label bootstrap review",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
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
            "/api/v1/ops/rules/conditions": {
                "get": {
                    "summary": "List reusable rule conditions mined or curated from rule versions",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Rule condition library entries",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleConditionLibraryResponse" }
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
            "/api/v1/ops/rules/{rule_id}/shadow-runs": {
                "post": {
                    "summary": "Record reviewed shadow-run evidence for a rule version",
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
                                "schema": { "$ref": "#/components/schemas/SubmitRuleShadowRunRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded rule shadow-run evidence",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/RuleShadowRun" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/rules/candidate-reviews": {
                "post": {
                    "summary": "Record accept or reject review for a discovered rule candidate",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ReviewRuleCandidateRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded rule candidate review",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ReviewRuleCandidateResponse" }
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
            "/api/v1/ops/evidence/documents": {
                "get": {
                    "summary": "List governed evidence document metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": { "200": { "description": "Evidence documents scoped to the authenticated customer" } }
                },
                "post": {
                    "summary": "Register governed evidence document metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/EvidenceDocumentRegistrationRequest" }
                            }
                        }
                    },
                    "responses": { "200": { "description": "Registered evidence document metadata" } }
                }
            },
            "/api/v1/ops/evidence/documents/{document_id}": {
                "get": {
                    "summary": "Get governed evidence document metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        { "name": "document_id", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": { "200": { "description": "Evidence document metadata" }, "404": { "description": "Document not found in customer scope" } }
                }
            },
            "/api/v1/ops/evidence/documents/{document_id}/chunks": {
                "get": {
                    "summary": "List governed document chunk metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        { "name": "document_id", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": { "200": { "description": "Document chunk metadata" } }
                },
                "post": {
                    "summary": "Register governed document chunk metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        { "name": "document_id", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/EvidenceDocumentChunkRegistrationRequest" }
                            }
                        }
                    },
                    "responses": { "200": { "description": "Registered document chunk metadata" }, "404": { "description": "Document not found in customer scope" } }
                }
            },
            "/api/v1/ops/evidence/documents/{document_id}/ocr-outputs": {
                "get": {
                    "summary": "List governed OCR output metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        { "name": "document_id", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": { "200": { "description": "OCR output metadata" } }
                },
                "post": {
                    "summary": "Register governed OCR output metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        { "name": "document_id", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/EvidenceOcrOutputRegistrationRequest" }
                            }
                        }
                    },
                    "responses": { "200": { "description": "Registered OCR output metadata" }, "404": { "description": "Document not found in customer scope" } }
                }
            },
            "/api/v1/ops/evidence/embedding-jobs": {
                "get": {
                    "summary": "List governed evidence embedding jobs",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": { "200": { "description": "Embedding jobs scoped to the authenticated customer" } }
                },
                "post": {
                    "summary": "Register governed evidence embedding job metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/EvidenceEmbeddingJobRegistrationRequest" }
                            }
                        }
                    },
                    "responses": { "200": { "description": "Registered evidence embedding job metadata" } }
                }
            },
            "/api/v1/ops/evidence/retrieval-audit-events": {
                "get": {
                    "summary": "List governed evidence retrieval audit events",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": { "200": { "description": "Retrieval audit events scoped to the authenticated customer" } }
                },
                "post": {
                    "summary": "Record governed evidence retrieval audit metadata",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/EvidenceRetrievalAuditRegistrationRequest" }
                            }
                        }
                    },
                    "responses": { "200": { "description": "Recorded retrieval audit metadata" } }
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
                    "summary": "Save a discovered rule as a draft candidate for governance review",
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
            "/api/v1/ops/providers/anomaly-clustering-reports": {
                "post": {
                    "summary": "Submit an unsupervised anomaly clustering report into the human review queue",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitAnomalyClusteringReportRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Accepted clustering report for anomaly review queue only",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/SubmitAnomalyClusteringReportResponse" }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid clustering report submission or missing anomaly_clustering_reports evidence",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/providers/anomaly-review-queue": {
                "get": {
                    "summary": "List anomaly candidates derived from submitted clustering reports",
                    "security": [{ "ApiKeyAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Anomaly review queue",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/AnomalyReviewQueueResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/providers/anomaly-candidate-reviews": {
                "post": {
                    "summary": "Record a human review decision for an unsupervised anomaly candidate",
                    "security": [{ "ApiKeyAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ReviewAnomalyCandidateRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded anomaly candidate review decision",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ReviewAnomalyCandidateResponse" }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid anomaly candidate review or missing clustering report evidence",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
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
            "/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-gates": {
                "get": {
                    "summary": "Get promotion gates for an explicit model version before routing impact",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "model_version",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Version-scoped model promotion gate summary",
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
            "/api/v1/ops/models/{model_key}/mlops-monitoring-review-queue": {
                "get": {
                    "summary": "List human review tasks opened by submitted MLOps monitoring reports",
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
                            "description": "MLOps monitoring review queue",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelMonitoringReviewQueueResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/mlops-monitoring-review-tasks/{task_id}/reviews": {
                "post": {
                    "summary": "Record a human decision for an MLOps monitoring review task",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "task_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitModelMonitoringReviewTaskReviewRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded monitoring review task decision",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ModelMonitoringReviewTaskReviewResponse" }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid decision or missing evidence refs",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        },
                        "404": {
                            "description": "Monitoring review task not found",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/mlops-monitoring-reports": {
                "post": {
                    "summary": "Submit a Rust MLOps monitoring report into governance audit",
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
                                "schema": { "$ref": "#/components/schemas/SubmitMlopsMonitoringReportRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded MLOps monitoring report governance event",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/SubmitMlopsMonitoringReportResponse" }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid monitoring report submission",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/mlops-alert-deliveries": {
                "post": {
                    "summary": "Submit Rust MLOps alert-router delivery evidence into governance audit",
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
                                "schema": { "$ref": "#/components/schemas/SubmitMlopsAlertDeliveryRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded MLOps alert delivery governance event",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/SubmitMlopsAlertDeliveryResponse" }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid alert delivery submission",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/mlops-alert-delivery-queue": {
                "get": {
                    "summary": "List alert delivery tasks opened by submitted MLOps scheduler reports",
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
                            "description": "MLOps alert delivery queue",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/MlopsAlertDeliveryQueueResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/ops/models/{model_key}/mlops-alert-delivery-tasks/{task_id}/reviews": {
                "post": {
                    "summary": "Record a human receipt or escalation decision for an MLOps alert delivery task",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "task_id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitMlopsAlertDeliveryTaskReviewRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recorded alert delivery task review",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/MlopsAlertDeliveryTaskReviewResponse" }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid decision or missing evidence refs",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        },
                        "404": {
                            "description": "Alert delivery task not found",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
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
                    "summary": "Register external training output, candidate model, validation evaluation, and mined rule candidates",
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
                            "description": "Completed model retraining job output and saved mined rule candidates",
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
            "/api/v1/ops/models/{model_key}/versions/{model_version}/promotion-reviews": {
                "post": {
                    "summary": "Record a model promotion review decision for an explicit model version",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "model_version",
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
                            "description": "Recorded version-scoped model promotion review",
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
            "/api/v1/ops/models/{model_key}/versions/{model_version}/activate": {
                "post": {
                    "summary": "Activate an explicit governed model version for production routing",
                    "security": [{ "ApiKeyAuth": [] }],
                    "parameters": [
                        {
                            "name": "model_key",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        },
                        {
                            "name": "model_version",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "requestBody": model_lifecycle_request_body(),
                    "responses": {
                        "200": {
                            "description": "Model lifecycle status after version-scoped activation",
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
                        },
                        "403": {
                            "description": "Principal lacks tpa:knowledge:read",
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
    })
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
