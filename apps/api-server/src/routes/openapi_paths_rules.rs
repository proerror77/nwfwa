use super::{rule_lifecycle_parameters, rule_lifecycle_request_body};
use serde_json::{json, Value};

pub(super) fn rule_paths() -> Value {
    json!({
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
    })
}
