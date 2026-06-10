use serde_json::{json, Value};

pub(super) fn evidence_paths() -> Value {
    json!({
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
    })
}
