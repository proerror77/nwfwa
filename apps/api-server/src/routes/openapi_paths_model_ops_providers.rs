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
        "/api/v1/ops/providers/profile-window-rollups": {
            "post": {
                "summary": "Submit provider profile 30/90/365 window rollups",
                "description": "Persists provider profile windows from a worker-generated rollup report. This writes provider profile rollups only; it does not change scoring policy, assign fraud labels, or adjudicate claims.",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitProviderProfileWindowRollupRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Persisted provider profile window rollups",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitProviderProfileWindowRollupResponse" }
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
        "/api/v1/ops/providers/graph-signal-rollups": {
            "post": {
                "summary": "Submit provider graph signal rollups",
                "description": "Persists provider relationship graph signals from a worker-generated rollup report. This writes provider graph signals only; it does not change scoring policy, assign fraud labels, open cases, or adjudicate claims.",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitProviderGraphSignalRollupRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Persisted provider graph signal rollups",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitProviderGraphSignalRollupResponse" }
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
        "/api/v1/ops/providers/peer-benchmarks": {
            "post": {
                "summary": "Submit peer percentile benchmark groups",
                "description": "Persists peer percentile reference groups from a worker-generated benchmark report. This writes benchmark reference data only; it does not score claims, assign fraud labels, or change scoring/routing policy.",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitPeerBenchmarkRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Persisted peer benchmark groups",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitPeerBenchmarkResponse" }
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
        "/api/v1/ops/providers/episode-rollups": {
            "post": {
                "summary": "Submit member-provider episode rollups",
                "description": "Persists member-provider episode utilization rollups from a worker-generated aggregation report. This writes episode rollups only; it does not change scoring policy, assign fraud labels, open cases, deny claims, or adjudicate claims.",
                "security": [{ "ApiKeyAuth": [] }],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/SubmitEpisodeRollupRequest" }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Persisted episode rollups",
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SubmitEpisodeRollupResponse" }
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
