use serde_json::{json, Value};

pub(super) fn provider_paths() -> Value {
    json!({
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
        "/api/v1/ops/providers/sanctions-sync-reports": {
            "post": {
                "summary": "Submit OIG/SAM sanctions sync report provider upserts",
                "description": "Persists provider sanctions from a worker-generated sync report. This writes provider sanctions only; it does not change scoring policy, assign fraud labels, or adjudicate claims.",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitSanctionsSyncReportRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Persisted provider sanctions upserts",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitSanctionsSyncReportResponse" }
                            }
                        }
                    },
                    "403": {
                        "description": "Requires ops:providers:write permission",
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
    })
}
