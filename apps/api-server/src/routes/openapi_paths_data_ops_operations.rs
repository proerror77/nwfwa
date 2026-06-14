use serde_json::{json, Value};

pub(super) fn operational_paths() -> Value {
    json!({
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
        "/api/v1/ops/agent-runs/{agent_run_id}/cancel": {
            "post": {
                "summary": "Cancel a queued or running agent run",
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
                            "schema": { "$ref": "#/components/schemas/CancelAgentRunRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Agent run cancellation accepted and audited",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CancelAgentRunResponse" }
                            }
                        }
                    },
                    "409": {
                        "description": "Agent run has already reached a terminal state",
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
