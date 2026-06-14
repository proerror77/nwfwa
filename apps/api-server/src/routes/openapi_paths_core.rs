use serde_json::{json, Value};

pub(super) fn core_paths() -> Value {
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
    })
}
