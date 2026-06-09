use serde_json::{json, Value};

pub(super) fn pilot_paths() -> Value {
    json!({
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
