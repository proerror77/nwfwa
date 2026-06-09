use serde_json::{json, Value};

pub(super) fn data_ops_paths() -> Value {
    json!({
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
    })
}
