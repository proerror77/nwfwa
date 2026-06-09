use super::{routing_policy_lifecycle_parameters, routing_policy_lifecycle_request_body};
use serde_json::{json, Value};

pub(super) fn governance_paths() -> Value {
    json!({
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
    })
}
