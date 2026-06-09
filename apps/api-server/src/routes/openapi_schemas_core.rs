use serde_json::{json, Value};

pub(super) fn core_schemas() -> Value {
    json!({
                "InboxNormalizeRequest": {
                    "type": "object",
                    "description": "Customer-specific raw claim intake payload. MVP supports the AiClaim Core reportCase envelope.",
                    "required": ["systemCode", "transNo", "reportCase"],
                    "properties": {
                        "systemCode": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Source system code bound to the authenticated API key."
                        },
                        "transNo": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Source transaction id used with reportNo for idempotency."
                        },
                        "transDate": {
                            "type": ["string", "null"],
                            "description": "Source transaction timestamp when present."
                        },
                        "reportCase": {
                            "type": "object",
                            "description": "Raw source claim case payload. It may contain medical records, policy, invoice, product, and liability lists."
                        }
                    },
                    "additionalProperties": true
                },
                "InboxNormalizeResponse": {
                    "type": "object",
                    "required": [
                        "run_id",
                        "audit_id",
                        "mapping_version",
                        "raw_payload_checksum",
                        "validation_result",
                        "scoring_ready",
                        "validation_errors",
                        "canonical_claim_context",
                        "data_quality_signals",
                        "evidence_refs"
                    ],
                    "properties": {
                        "run_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "external_message_id": { "type": ["string", "null"] },
                        "idempotency_key": { "type": ["string", "null"] },
                        "raw_payload_checksum": { "type": "string" },
                        "mapping_version": { "type": "string" },
                        "validation_result": {
                            "type": "string",
                            "enum": ["accepted", "accepted_with_warnings", "rejected"]
                        },
                        "scoring_ready": { "type": "boolean" },
                        "raw_payload_ref": { "type": ["string", "null"] },
                        "validation_errors": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxValidationError" }
                        },
                        "canonical_claim_context": {
                            "$ref": "#/components/schemas/InboxCanonicalClaimContext"
                        },
                        "data_quality_signals": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxCanonicalClaimContext": {
                    "type": "object",
                    "required": [
                        "claim_header",
                        "member_policy_snapshot",
                        "provider_snapshot",
                        "itemized_bill_lines",
                        "document_evidence"
                    ],
                    "properties": {
                        "claim_header": { "$ref": "#/components/schemas/InboxClaimHeader" },
                        "member_policy_snapshot": { "$ref": "#/components/schemas/InboxMemberPolicySnapshot" },
                        "provider_snapshot": { "$ref": "#/components/schemas/InboxProviderSnapshot" },
                        "itemized_bill_lines": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxBillLine" }
                        },
                        "document_evidence": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxDocumentEvidence" }
                        }
                    }
                },
                "InboxClaimHeader": {
                    "type": "object",
                    "properties": {
                        "external_claim_id": { "type": "string" },
                        "source_system": { "type": "string" },
                        "service_date": { "type": ["string", "null"], "format": "date" },
                        "receive_date": { "type": ["string", "null"], "format": "date" },
                        "accident_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "service_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "receive_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "accident_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "accident_reason": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "currency": { "type": "string" },
                        "total_amount": { "type": ["number", "null"] }
                    }
                },
                "InboxMemberPolicySnapshot": {
                    "type": "object",
                    "properties": {
                        "masked_member_id": { "type": ["string", "null"] },
                        "masked_certificate_id": { "type": ["string", "null"] },
                        "certificate_type": { "type": ["string", "null"] },
                        "member_gender": { "type": ["string", "null"] },
                        "member_birth_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "member_birth_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "policy_id": { "type": ["string", "null"] },
                        "product_code": { "type": ["string", "null"] },
                        "liability_code": { "type": ["string", "null"] },
                        "liability_name": { "type": ["string", "null"] },
                        "policy_type": { "type": ["string", "null"] },
                        "policy_first_apply_date": { "type": ["string", "null"], "format": "date" },
                        "policy_first_apply_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "insured_with_social_insurance": { "type": ["boolean", "null"] },
                        "coverage_limit": { "type": ["number", "null"] },
                        "coverage_start_date": { "type": ["string", "null"], "format": "date" },
                        "coverage_end_date": { "type": ["string", "null"], "format": "date" },
                        "coverage_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "coverage_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_start_date": { "type": ["string", "null"], "format": "date" },
                        "liability_claim_start_date": { "type": ["string", "null"], "format": "date" },
                        "waiting_period_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_claim_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "product_liabilities": {
                            "type": "array",
                            "description": "All product and claim-liability windows from the source policies, preserving coverage and waiting-period candidates before scoring.",
                            "items": { "$ref": "#/components/schemas/InboxProductLiability" }
                        }
                    }
                },
                "InboxProductLiability": {
                    "type": "object",
                    "properties": {
                        "policy_id": { "type": ["string", "null"] },
                        "product_id": { "type": ["string", "null"] },
                        "product_code": { "type": ["string", "null"] },
                        "product_name": { "type": ["string", "null"] },
                        "plan_code": { "type": ["string", "null"] },
                        "plan_version": { "type": ["string", "null"] },
                        "product_start_date": { "type": ["string", "null"], "format": "date" },
                        "product_end_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "product_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "product_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_id": { "type": ["string", "null"] },
                        "liability_code": { "type": ["string", "null"] },
                        "liability_name": { "type": ["string", "null"] },
                        "liability_start_date": { "type": ["string", "null"], "format": "date" },
                        "liability_claim_start_date": { "type": ["string", "null"], "format": "date" },
                        "waiting_period_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_end_date": { "type": ["string", "null"], "format": "date" },
                        "liability_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_claim_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "liability_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "is_serious_disease_liability": { "type": ["boolean", "null"] },
                        "main_liability": { "type": ["boolean", "null"] },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxProviderSnapshot": {
                    "type": "object",
                    "properties": {
                        "provider_code": { "type": ["string", "null"] },
                        "name": { "type": ["string", "null"] },
                        "class": { "type": ["string", "null"] },
                        "type": { "type": ["string", "null"] },
                        "city": { "type": ["string", "null"] },
                        "province": { "type": ["string", "null"] },
                        "network_flags": { "$ref": "#/components/schemas/InboxProviderNetworkFlags" }
                    }
                },
                "InboxProviderNetworkFlags": {
                    "type": "object",
                    "properties": {
                        "is_hospital_institution": { "type": ["boolean", "null"] },
                        "primary_care": { "type": ["boolean", "null"] },
                        "red_flag": { "type": ["string", "null"] }
                    }
                },
                "InboxBillLine": {
                    "type": "object",
                    "properties": {
                        "invoice_id": { "type": ["string", "null"] },
                        "diagnosis_list": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/InboxDiagnosis" }
                        },
                        "fee_category": { "type": ["string", "null"] },
                        "item_name": { "type": ["string", "null"] },
                        "amount": { "type": ["number", "null"] },
                        "self_pay": { "type": ["number", "null"] },
                        "own_expense": { "type": ["number", "null"] },
                        "social_insurance_amount": { "type": ["number", "null"] },
                        "medical_category": { "type": ["string", "null"] },
                        "invoice_bill_type": { "type": ["string", "null"] },
                        "invoice_document_type": { "type": ["string", "null"] },
                        "social_insurance_type": { "type": ["string", "null"] },
                        "department": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "invoice_claim_nature": { "type": ["string", "null"] },
                        "invoice_start_date": { "type": ["string", "null"], "format": "date" },
                        "invoice_end_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "invoice_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "invoice_end_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "invoice_social_insurance_amount": { "type": ["number", "null"] },
                        "invoice_self_pay_amount": { "type": ["number", "null"] },
                        "invoice_own_expense_amount": { "type": ["number", "null"] },
                        "invoice_other_amount": { "type": ["number", "null"] },
                        "invoice_provider_code": { "type": ["string", "null"] },
                        "invoice_provider_name": { "type": ["string", "null"] },
                        "invoice_provider_class": { "type": ["string", "null"] },
                        "invoice_provider_type": { "type": ["string", "null"] },
                        "invoice_provider_city": { "type": ["string", "null"] },
                        "invoice_provider_province": { "type": ["string", "null"] },
                        "invoice_is_hospital_institution": { "type": ["boolean", "null"] },
                        "invoice_primary_care": { "type": ["boolean", "null"] },
                        "invoice_red_flag": { "type": ["string", "null"] },
                        "fee_group_amount": { "type": ["number", "null"] },
                        "fee_group_other_amount": { "type": ["number", "null"] },
                        "medicare_prorated": { "type": ["string", "null"] },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxDiagnosis": {
                    "type": "object",
                    "properties": {
                        "code": { "type": ["string", "null"] },
                        "name": { "type": ["string", "null"] }
                    }
                },
                "InboxDocumentEvidence": {
                    "type": "object",
                    "properties": {
                        "document_id": { "type": ["string", "null"] },
                        "department": { "type": ["string", "null"] },
                        "diagnosis": { "type": ["string", "null"] },
                        "claim_nature": { "type": ["string", "null"] },
                        "medical_record_type": { "type": ["string", "null"] },
                        "chief_complaint": { "type": ["string", "null"] },
                        "current_medical_history": { "type": ["string", "null"] },
                        "past_history": { "type": ["string", "null"] },
                        "extracted_diagnosis": { "type": ["string", "null"] },
                        "extracted_procedure": { "type": ["string", "null"] },
                        "extracted_prescription": { "type": ["string", "null"] },
                        "medical_type": { "type": ["string", "null"] },
                        "visit_date": { "type": ["string", "null"], "format": "date" },
                        "first_happen_date": { "type": ["string", "null"], "format": "date" },
                        "operation_start_date": { "type": ["string", "null"], "format": "date" },
                        "source_timezone": { "type": "string" },
                        "visit_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "first_happen_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "operation_start_date_raw_epoch_ms": { "type": ["integer", "null"], "format": "int64" },
                        "medical_record_text": { "type": ["string", "null"] },
                        "source_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "InboxValidationError": {
                    "type": "object",
                    "required": ["field_path", "severity", "remediation"],
                    "properties": {
                        "field_path": { "type": "string" },
                        "severity": { "type": "string", "enum": ["error", "warning"] },
                        "remediation": { "type": "string" }
                    }
                },
                "ScoreClaimRequest": {
                    "oneOf": [
                        {
                            "$ref": "#/components/schemas/ClaimIdScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/FullPayloadScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/CanonicalContextScoreClaimRequest"
                        },
                        {
                            "$ref": "#/components/schemas/InboxHandoffScoreClaimRequest"
                        }
                    ]
                },
                "ClaimIdScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "claim_id"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "claim_id": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Existing claim id to load from FWA storage."
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] },
                            { "required": ["documents"] },
                            { "required": ["provider_profile"] },
                            { "required": ["provider_relationships"] },
                            { "required": ["canonical_claim_context"] },
                            { "required": ["inbox_run_id"] },
                            { "required": ["inbox_idempotency_key"] }
                        ]
                    }
                },
                "CanonicalContextScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "canonical_claim_context"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        },
                        "canonical_claim_context": {
                            "$ref": "#/components/schemas/InboxCanonicalClaimContext"
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim_id"] },
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] },
                            { "required": ["documents"] },
                            { "required": ["provider_profile"] },
                            { "required": ["provider_relationships"] },
                            { "required": ["inbox_run_id"] },
                            { "required": ["inbox_idempotency_key"] }
                        ]
                    }
                },
                "InboxHandoffScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        },
                        "inbox_run_id": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Scoring-ready inbox normalization run id returned by /api/v1/inbox/claims/normalize."
                        },
                        "inbox_idempotency_key": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Stable scoring-ready inbox normalization idempotency key returned by /api/v1/inbox/claims/normalize."
                        }
                    },
                    "oneOf": [
                        { "required": ["inbox_run_id"] },
                        { "required": ["inbox_idempotency_key"] }
                    ],
                    "not": {
                        "anyOf": [
                            { "required": ["claim_id"] },
                            { "required": ["claim"] },
                            { "required": ["items"] },
                            { "required": ["member"] },
                            { "required": ["policy"] },
                            { "required": ["provider"] },
                            { "required": ["documents"] },
                            { "required": ["provider_profile"] },
                            { "required": ["provider_relationships"] },
                            { "required": ["canonical_claim_context"] }
                        ]
                    }
                },
                "FullPayloadScoreClaimRequest": {
                    "type": "object",
                    "required": ["source_system", "claim"],
                    "properties": {
                        "source_system": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Must match the source system bound to the authenticated API key.",
                            "examples": ["tpa-demo"]
                        },
                        "claim": {
                            "$ref": "#/components/schemas/FullClaimPayload"
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"],
                            "default": "pre_payment",
                            "description": "Runtime scoring context for pre-payment or post-payment review."
                        },
                        "items": {
                            "type": "array",
                            "description": "Top-level claim items for spec-style full payload requests. Do not send the same entity both nested under claim and at the top level.",
                            "items": {
                                "$ref": "#/components/schemas/ClaimItemPayload"
                            }
                        },
                        "member": {
                            "$ref": "#/components/schemas/MemberPayload"
                        },
                        "policy": {
                            "$ref": "#/components/schemas/PolicyPayload"
                        },
                        "provider": {
                            "$ref": "#/components/schemas/ProviderPayload"
                        },
                        "documents": {
                            "type": "array",
                            "description": "Clinical documents linked to claim items for evidence sufficiency review.",
                            "items": {
                                "$ref": "#/components/schemas/DocumentPayload"
                            }
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfilePayload"
                        },
                        "provider_relationships": {
                            "$ref": "#/components/schemas/ProviderRelationshipGraphPayload"
                        }
                    },
                    "not": {
                        "anyOf": [
                            { "required": ["claim_id"] },
                            { "required": ["canonical_claim_context"] },
                            { "required": ["inbox_run_id"] },
                            { "required": ["inbox_idempotency_key"] }
                        ]
                    }
                },
                "FullClaimPayload": {
                    "type": "object",
                    "required": ["external_claim_id", "claim_amount", "currency"],
                    "properties": {
                        "external_claim_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "claim_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Positive decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        },
                        "service_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "diagnosis_code": {
                            "type": "string",
                            "minLength": 1
                        },
                        "items": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/ClaimItemPayload"
                            }
                        },
                        "member": {
                            "$ref": "#/components/schemas/MemberPayload"
                        },
                        "policy": {
                            "$ref": "#/components/schemas/PolicyPayload"
                        },
                        "provider": {
                            "$ref": "#/components/schemas/ProviderPayload"
                        },
                        "documents": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/DocumentPayload"
                            }
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfilePayload"
                        },
                        "provider_relationships": {
                            "$ref": "#/components/schemas/ProviderRelationshipGraphPayload"
                        }
                    }
                },
                "ClaimItemPayload": {
                    "type": "object",
                    "required": ["item_code", "item_type", "description", "quantity", "unit_amount", "total_amount"],
                    "properties": {
                        "item_code": {
                            "type": "string",
                            "minLength": 1
                        },
                        "item_type": {
                            "type": "string",
                            "minLength": 1
                        },
                        "description": {
                            "type": "string",
                            "minLength": 1
                        },
                        "quantity": {
                            "type": "integer",
                            "minimum": 1
                        },
                        "unit_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Non-negative decimal string."
                        },
                        "total_amount": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Non-negative decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "MemberPayload": {
                    "type": "object",
                    "required": ["external_member_id"],
                    "properties": {
                        "external_member_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "dob": {
                            "type": "string",
                            "format": "date"
                        },
                        "gender": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "PolicyPayload": {
                    "type": "object",
                    "required": ["external_policy_id", "coverage_start_date", "coverage_end_date", "coverage_limit"],
                    "properties": {
                        "external_policy_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "product_code": {
                            "type": "string",
                            "minLength": 1
                        },
                        "coverage_start_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "coverage_end_date": {
                            "type": "string",
                            "format": "date"
                        },
                        "coverage_limit": {
                            "type": "string",
                            "format": "decimal",
                            "description": "Positive decimal string."
                        },
                        "currency": {
                            "type": "string",
                            "minLength": 1
                        }
                    }
                },
                "ProviderPayload": {
                    "type": "object",
                    "required": ["external_provider_id", "name", "provider_type", "region"],
                    "properties": {
                        "external_provider_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "name": {
                            "type": "string",
                            "minLength": 1
                        },
                        "provider_type": {
                            "type": "string",
                            "minLength": 1
                        },
                        "region": {
                            "type": "string",
                            "minLength": 1
                        },
                        "risk_tier": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High"]
                        }
                    }
                },
                "DocumentPayload": {
                    "type": "object",
                    "required": ["external_document_id", "document_type"],
                    "properties": {
                        "external_document_id": {
                            "type": "string",
                            "minLength": 1
                        },
                        "document_type": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Examples: medical_record, clinical_order, radiology_report, dental_xray, prescription_detail, operation_record, lab_result"
                        },
                        "linked_item_codes": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "minLength": 1
                            }
                        }
                    }
                },
                "ProviderProfilePayload": {
                    "type": "object",
                    "required": ["windows"],
                    "properties": {
                        "specialty": { "type": "string" },
                        "network_status": { "type": "string" },
                        "windows": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "$ref": "#/components/schemas/ProviderProfileWindowPayload" }
                        }
                    }
                },
                "ProviderProfileWindowPayload": {
                    "type": "object",
                    "required": [
                        "window_days",
                        "claim_count",
                        "total_claim_amount",
                        "high_cost_item_ratio",
                        "diagnosis_procedure_mismatch_rate",
                        "peer_amount_percentile",
                        "peer_frequency_percentile",
                        "review_failure_count",
                        "confirmed_fwa_count",
                        "false_positive_count"
                    ],
                    "properties": {
                        "window_days": { "type": "integer", "enum": [30, 90, 180] },
                        "claim_count": { "type": "integer", "minimum": 0 },
                        "total_claim_amount": { "type": "string", "format": "decimal", "description": "Non-negative decimal string." },
                        "high_cost_item_ratio": { "type": "number", "minimum": 0, "maximum": 1 },
                        "diagnosis_procedure_mismatch_rate": { "type": "number", "minimum": 0, "maximum": 1 },
                        "peer_amount_percentile": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "peer_frequency_percentile": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "review_failure_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 }
                    }
                },
                "ProviderRelationshipGraphPayload": {
                    "type": "object",
                    "required": [
                        "high_risk_neighbor_ratio",
                        "provider_patient_overlap_score",
                        "connected_confirmed_fwa_count"
                    ],
                    "properties": {
                        "high_risk_neighbor_ratio": { "type": "number", "minimum": 0, "maximum": 1 },
                        "provider_patient_overlap_score": { "type": "number", "minimum": 0, "maximum": 1 },
                        "referral_concentration_score": { "type": ["number", "null"], "minimum": 0, "maximum": 1 },
                        "connected_confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "network_component_risk_score": { "type": ["integer", "null"], "minimum": 0, "maximum": 100 },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ScoreClaimResponse": {
                    "type": "object",
                    "required": [
                        "run_id",
                        "audit_id",
                        "claim_id",
                        "review_mode",
                        "risk_score",
                        "rag",
                        "risk_level",
                        "recommended_action",
                        "decision_outcome",
                        "decision_authority",
                        "decision_confidence",
                        "appeal_or_review_required",
                        "reason_code",
                        "confidence_score",
                        "confidence",
                        "routing_reason",
                        "routing_policy",
                        "scores",
                        "model_score",
                        "alerts",
                        "top_reasons",
                        "layers",
                        "clinical_evidence",
                        "provider_profile",
                        "provider_relationships",
                        "similar_cases",
                        "feature_values",
                        "evidence_refs",
                        "agent_investigation_prefill"
                    ],
                    "properties": {
                        "run_id": {
                            "type": "string"
                        },
                        "audit_id": {
                            "type": "string"
                        },
                        "claim_id": {
                            "type": "string"
                        },
                        "review_mode": {
                            "type": "string",
                            "enum": ["pre_payment", "post_payment"]
                        },
                        "risk_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "rag": {
                            "type": "string",
                            "enum": ["Green", "Amber", "Red"]
                        },
                        "risk_level": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High", "Critical"]
                        },
                        "recommended_action": {
                            "type": "string",
                            "enum": [
                                "StandardProcessing",
                                "QaSample",
                                "ManualReview",
                                "RequestEvidence",
                                "EscalateInvestigation",
                                "PostPaymentAudit",
                                "ProviderReview",
                                "RecoveryReview"
                            ]
                        },
                        "decision_outcome": {
                            "type": "string",
                            "enum": [
                                "straight_through",
                                "auto_deny",
                                "pending_evidence",
                                "manual_review",
                                "qa_sample",
                                "post_payment_audit"
                            ]
                        },
                        "decision_authority": {
                            "type": "string",
                            "enum": [
                                "customer_policy_rule",
                                "clinical_policy_rule",
                                "risk_routing_policy",
                                "human_reviewer",
                                "qa_policy"
                            ]
                        },
                        "decision_confidence": {
                            "type": "string",
                            "enum": ["deterministic", "high", "medium", "low"]
                        },
                        "appeal_or_review_required": {
                            "type": "boolean"
                        },
                        "reason_code": {
                            "type": "string",
                            "minLength": 1
                        },
                        "confidence_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "confidence": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High"]
                        },
                        "routing_reason": {
                            "type": "string"
                        },
                        "routing_policy": {
                            "$ref": "#/components/schemas/RoutingPolicy"
                        },
                        "scores": {
                            "$ref": "#/components/schemas/ScoreBreakdown"
                        },
                        "model_score": {
                            "$ref": "#/components/schemas/ModelScore"
                        },
                        "alerts": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/AlertResponse"
                            }
                        },
                        "top_reasons": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "minLength": 1
                            }
                        },
                        "layers": {
                            "type": "array",
                            "minItems": 7,
                            "maxItems": 7,
                            "items": {
                                "$ref": "#/components/schemas/DetectionLayerScore"
                            }
                        },
                        "clinical_evidence": {
                            "$ref": "#/components/schemas/ClinicalEvidenceAssessment"
                        },
                        "provider_profile": {
                            "$ref": "#/components/schemas/ProviderProfileAssessment"
                        },
                        "provider_relationships": {
                            "$ref": "#/components/schemas/ProviderRelationshipGraphAssessment"
                        },
                        "similar_cases": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/SimilarCase"
                            }
                        },
                        "feature_values": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/FeatureValue"
                            }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "oneOf": [
                                    { "type": "object" },
                                    { "type": "string" }
                                ]
                            }
                        },
                        "agent_investigation_prefill": {
                            "$ref": "#/components/schemas/AgentInvestigationPrefill"
                        }
                    }
                },
                "ModelScore": {
                    "type": "object",
                    "required": [
                        "model_key",
                        "model_version",
                        "runtime_kind",
                        "execution_provider",
                        "score",
                        "label",
                        "explanations",
                        "metadata",
                        "latency_ms"
                    ],
                    "properties": {
                        "model_key": { "type": "string" },
                        "model_version": { "type": "string" },
                        "runtime_kind": { "type": "string" },
                        "execution_provider": { "type": "string" },
                        "score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "label": { "type": "string" },
                        "explanations": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ModelExplanation" }
                        },
                        "metadata": {
                            "type": "object",
                            "properties": {
                                "fraud_probability": { "type": "number", "minimum": 0, "maximum": 1 },
                                "abuse_probability": { "type": "number", "minimum": 0, "maximum": 1 },
                                "waste_probability": { "type": "number", "minimum": 0, "maximum": 1 }
                            },
                            "additionalProperties": true
                        },
                        "latency_ms": { "type": "integer", "minimum": 0 }
                    }
                },
                "ModelExplanation": {
                    "type": "object",
                    "required": ["feature", "direction", "contribution", "reason"],
                    "properties": {
                        "feature": { "type": "string" },
                        "direction": { "type": "string" },
                        "contribution": { "type": "number" },
                        "reason": { "type": "string" }
                    }
                },
                "FeatureValue": {
                    "type": "object",
                    "required": ["name", "version", "value", "evidence_refs"],
                    "properties": {
                        "name": { "type": "string" },
                        "version": { "type": "integer", "minimum": 0 },
                        "value": {},
                        "evidence_refs": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/EvidenceRef" }
                        }
                    }
                },
                "EvidenceRef": {
                    "type": "object",
                    "required": ["entity_type", "entity_id", "field"],
                    "properties": {
                        "entity_type": { "type": "string" },
                        "entity_id": { "type": "string" },
                        "field": { "type": "string" }
                    }
                },
                "DetectionLayerScore": {
                    "type": "object",
                    "required": ["layer_id", "name", "score", "status", "reason", "evidence_refs"],
                    "properties": {
                        "layer_id": {
                            "type": "string",
                            "enum": [
                                "L1_PEER_BENCHMARK",
                                "L2_RULE_DETECTION",
                                "L3_UNSUPERVISED_ANOMALY",
                                "L4_SUPERVISED_ML",
                                "L5_MEDICAL_REASONABLENESS",
                                "L6_PROVIDER_GRAPH_RISK",
                                "L7_RISK_FUSION_ROUTING"
                            ]
                        },
                        "name": { "type": "string" },
                        "score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "status": {
                            "type": "string",
                            "enum": ["active", "baseline", "no_data"]
                        },
                        "reason": { "type": "string" },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "oneOf": [
                                    { "type": "string" },
                                    { "$ref": "#/components/schemas/EvidenceRef" }
                                ]
                            }
                        }
                    }
                },
                "RoutingPolicy": {
                    "type": "object",
                    "required": ["policy_id", "version", "review_mode", "risk_thresholds", "confidence_thresholds", "provider_review_threshold"],
                    "properties": {
                        "policy_id": { "type": "string", "minLength": 1 },
                        "version": { "type": "integer", "minimum": 1 },
                        "review_mode": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] },
                        "risk_thresholds": { "$ref": "#/components/schemas/RiskThresholds" },
                        "confidence_thresholds": { "$ref": "#/components/schemas/ConfidenceThresholds" },
                        "provider_review_threshold": { "type": "integer", "minimum": 0, "maximum": 100 }
                    }
                },
                "RiskThresholds": {
                    "type": "object",
                    "required": ["low_max", "medium_min", "high_min", "critical_min"],
                    "properties": {
                        "low_max": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "medium_min": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "high_min": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "critical_min": { "type": "integer", "minimum": 0, "maximum": 100 }
                    }
                },
                "ConfidenceThresholds": {
                    "type": "object",
                    "required": ["low_confidence_below", "high_confidence_min"],
                    "properties": {
                        "low_confidence_below": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "high_confidence_min": { "type": "integer", "minimum": 0, "maximum": 100 }
                    }
                },
                "ProviderProfileAssessment": {
                    "type": "object",
                    "required": [
                        "provider_id",
                        "risk_score",
                        "risk_tier",
                        "review_required",
                        "review_route",
                        "review_failure_count",
                        "confirmed_fwa_count",
                        "false_positive_count",
                        "outlier_flags",
                        "window_findings",
                        "evidence_refs"
                    ],
                    "properties": {
                        "provider_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "risk_tier": { "type": "string", "enum": ["low", "medium", "high"] },
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string", "enum": ["none", "provider_review"] },
                        "specialty": { "type": ["string", "null"] },
                        "network_status": { "type": ["string", "null"] },
                        "review_failure_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 },
                        "outlier_flags": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "window_findings": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ProviderProfileWindowFinding" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ProviderProfileWindowFinding": {
                    "type": "object",
                    "required": [
                        "window_days",
                        "risk_score",
                        "outlier_flags",
                        "reason",
                        "evidence_ref"
                    ],
                    "properties": {
                        "window_days": { "type": "integer" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "outlier_flags": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "reason": { "type": "string" },
                        "evidence_ref": { "type": "string" }
                    }
                },
                "ProviderRelationshipGraphAssessment": {
                    "type": "object",
                    "required": [
                        "provider_id",
                        "risk_score",
                        "risk_tier",
                        "review_required",
                        "review_route",
                        "graph_reasons",
                        "findings",
                        "evidence_refs"
                    ],
                    "properties": {
                        "provider_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "risk_tier": { "type": "string", "enum": ["no_data", "low", "medium", "high"] },
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string", "enum": ["none", "provider_graph_review"] },
                        "graph_reasons": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "findings": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ProviderRelationshipGraphFinding" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ProviderRelationshipGraphFinding": {
                    "type": "object",
                    "required": ["signal", "risk_score", "reason", "evidence_ref"],
                    "properties": {
                        "signal": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "reason": { "type": "string" },
                        "evidence_ref": { "type": "string" }
                    }
                },
                "ProviderRiskSummaryItem": {
                    "type": "object",
                    "required": ["provider_id", "risk_score", "risk_tier", "review_required", "review_route", "claim_count", "specialty", "network_status", "review_failure_count", "confirmed_fwa_count", "false_positive_count", "network_risk_score", "outlier_flags", "graph_reasons", "evidence_refs"],
                    "properties": {
                        "provider_id": { "type": "string" },
                        "risk_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "risk_tier": { "type": "string" },
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string" },
                        "claim_count": { "type": "integer" },
                        "specialty": { "type": ["string", "null"] },
                        "network_status": { "type": ["string", "null"] },
                        "review_failure_count": { "type": "integer", "minimum": 0 },
                        "confirmed_fwa_count": { "type": "integer", "minimum": 0 },
                        "false_positive_count": { "type": "integer", "minimum": 0 },
                        "network_risk_score": { "type": ["integer", "null"], "minimum": 0, "maximum": 100 },
                        "latest_claim_id": { "type": ["string", "null"] },
                        "outlier_flags": { "type": "array", "items": { "type": "string" } },
                        "graph_reasons": { "type": "array", "items": { "type": "string" } },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "ProviderRiskSummaryResponse": {
                    "type": "object",
                    "required": ["provider_count", "review_required_count", "high_risk_count", "providers"],
                    "properties": {
                        "provider_count": { "type": "integer" },
                        "review_required_count": { "type": "integer" },
                        "high_risk_count": { "type": "integer" },
                        "providers": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ProviderRiskSummaryItem" }
                        }
                    }
                },
                "AnomalyClusteringReviewTaskInput": {
                    "type": "object",
                    "required": ["candidate_kind", "candidate_id", "task_kind", "review_queue", "required_review", "evidence_refs"],
                    "properties": {
                        "candidate_kind": {
                            "type": "string",
                            "enum": ["provider_peer_anomaly", "provider_graph_anomaly", "claim_entity_anomaly"]
                        },
                        "candidate_id": { "type": "string", "minLength": 1 },
                        "task_kind": { "type": "string", "minLength": 1 },
                        "review_queue": { "type": "string", "minLength": 1 },
                        "required_review": { "type": "string", "minLength": 1 },
                        "decision_options": {
                            "type": "array",
                            "items": { "type": "string", "minLength": 1 }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Must include anomaly_clustering_reports:{source_report_uri}; values must not contain PII."
                        },
                        "candidate_payload": {
                            "type": "object",
                            "additionalProperties": true,
                            "description": "Explainable candidate fields copied from the clustering report."
                        }
                    }
                },
                "SubmitAnomalyClusteringReportRequest": {
                    "type": "object",
                    "required": ["actor", "notes", "source_report_uri", "report_kind", "dataset_key", "dataset_version", "label_policy", "governance_boundary", "review_tasks", "evidence_refs"],
                    "properties": {
                        "actor": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Submission notes must not contain PII."
                        },
                        "source_report_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "URI of provider_peer_clustering_report.json, provider_graph_community_report.json, or claim_entity_clustering_report.json."
                        },
                        "report_kind": {
                            "type": "string",
                            "enum": ["provider_peer_clustering", "provider_graph_community_clustering", "claim_entity_clustering"]
                        },
                        "dataset_key": { "type": "string", "minLength": 1 },
                        "dataset_version": { "type": "string", "minLength": 1 },
                        "label_policy": { "type": "string", "minLength": 1 },
                        "governance_boundary": {
                            "type": "string",
                            "minLength": 1,
                            "description": "The source report boundary. Report submission opens review tasks only."
                        },
                        "review_tasks": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "$ref": "#/components/schemas/AnomalyClusteringReviewTaskInput" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Must include anomaly_clustering_reports:{source_report_uri}; values must not contain PII."
                        }
                    }
                },
                "SubmitAnomalyClusteringReportResponse": {
                    "type": "object",
                    "required": ["report_kind", "source_report_uri", "review_task_count", "accepted_for_review_queue", "active_rule_writeback", "model_activation", "label_assignment", "case_creation", "governance_boundary", "audit_event_type"],
                    "properties": {
                        "report_kind": { "type": "string" },
                        "source_report_uri": { "type": "string" },
                        "review_task_count": { "type": "integer" },
                        "accepted_for_review_queue": { "type": "boolean" },
                        "active_rule_writeback": { "type": "boolean", "const": false },
                        "model_activation": { "type": "boolean", "const": false },
                        "label_assignment": { "type": "boolean", "const": false },
                        "case_creation": { "type": "boolean", "const": false },
                        "governance_boundary": { "type": "string" },
                        "audit_event_type": { "type": "string", "enum": ["provider.anomaly_clustering.report_submitted"] }
                    }
                },
                "AnomalyReviewQueueTask": {
                    "type": "object",
                    "required": ["candidate_kind", "candidate_id", "task_kind", "review_queue", "required_review", "decision_options", "source_report_uri", "report_kind", "dataset_key", "dataset_version", "label_policy", "governance_boundary", "review_status", "candidate_payload", "evidence_refs"],
                    "properties": {
                        "candidate_kind": {
                            "type": "string",
                            "enum": ["provider_peer_anomaly", "provider_graph_anomaly", "claim_entity_anomaly"]
                        },
                        "candidate_id": { "type": "string" },
                        "task_kind": { "type": "string" },
                        "review_queue": { "type": "string" },
                        "required_review": { "type": "string" },
                        "decision_options": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "source_report_uri": { "type": "string" },
                        "report_kind": { "type": "string" },
                        "dataset_key": { "type": "string" },
                        "dataset_version": { "type": "string" },
                        "label_policy": { "type": "string" },
                        "governance_boundary": { "type": "string" },
                        "review_status": {
                            "type": "string",
                            "enum": ["pending_human_review", "reviewed"]
                        },
                        "reviewer": { "type": ["string", "null"] },
                        "decision": { "type": ["string", "null"] },
                        "candidate_payload": { "type": "object", "additionalProperties": true },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "AnomalyReviewQueueResponse": {
                    "type": "object",
                    "required": ["tasks"],
                    "properties": {
                        "tasks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/AnomalyReviewQueueTask" }
                        }
                    }
                },
                "ReviewAnomalyCandidateRequest": {
                    "type": "object",
                    "required": ["candidate_kind", "candidate_id", "source_report_uri", "decision", "reviewer", "notes", "evidence_refs"],
                    "properties": {
                        "candidate_kind": {
                            "type": "string",
                            "enum": ["provider_peer_anomaly", "provider_graph_anomaly", "claim_entity_anomaly"]
                        },
                        "candidate_id": { "type": "string", "minLength": 1 },
                        "source_report_uri": {
                            "type": "string",
                            "minLength": 1,
                            "description": "URI of provider_peer_clustering_report.json, provider_graph_community_report.json, or claim_entity_clustering_report.json."
                        },
                        "decision": {
                            "type": "string",
                            "enum": ["accepted_for_review", "rejected", "open_investigation_review", "request_more_evidence"]
                        },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Review notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 },
                            "description": "Must include anomaly_clustering_reports:{source_report_uri}; values must not contain PII."
                        },
                        "candidate_payload": {
                            "type": "object",
                            "additionalProperties": true,
                            "description": "Optional non-decisional candidate context from the clustering report."
                        }
                    }
                },
                "ReviewAnomalyCandidateResponse": {
                    "type": "object",
                    "required": ["candidate_kind", "candidate_id", "decision", "reviewer", "accepted_for_review", "active_rule_writeback", "model_activation", "label_assignment", "governance_boundary", "audit_event_type"],
                    "properties": {
                        "candidate_kind": { "type": "string" },
                        "candidate_id": { "type": "string" },
                        "decision": { "type": "string" },
                        "reviewer": { "type": "string" },
                        "accepted_for_review": { "type": "boolean" },
                        "active_rule_writeback": { "type": "boolean", "const": false },
                        "model_activation": { "type": "boolean", "const": false },
                        "label_assignment": { "type": "boolean", "const": false },
                        "governance_boundary": { "type": "string" },
                        "audit_event_type": { "type": "string", "enum": ["anomaly.candidate.reviewed"] }
                    }
                },
                "SubmitMedicalReviewResultRequest": {
                    "type": "object",
                    "required": ["claim_id", "scoring_audit_id", "reviewer", "decision", "notes", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string", "minLength": 1 },
                        "scoring_audit_id": { "type": "string", "minLength": 1 },
                        "reviewer": { "type": "string", "minLength": 1 },
                        "decision": {
                            "type": "string",
                            "enum": ["evidence_sufficient", "request_more_evidence", "medical_necessity_issue", "no_medical_issue"]
                        },
                        "clinical_outcomes": {
                            "type": "array",
                            "description": "Optional controlled clinical outcome fields for model training and rule tuning. When omitted, the platform derives one compatible outcome from decision.",
                            "items": {
                                "type": "string",
                                "enum": ["documentation_issue", "medical_necessity_review_required", "insufficient_evidence", "medical_necessity_issue", "clinical_evidence_sufficient", "false_positive"]
                            }
                        },
                        "notes": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Medical review notes must not contain PII."
                        },
                        "evidence_refs": {
                            "type": "array",
                            "description": "Structured evidence references must not contain PII. For claims with the referenced normalized scoring trace, canonical evidence refs from that trace are merged into the persisted medical review and response.",
                            "minItems": 1,
                            "items": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "MedicalReviewResultResponse": {
                    "type": "object",
                    "required": ["claim_id", "event_type", "event_status", "audit_id", "run_id", "review_status", "clinical_outcomes", "evidence_refs"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "event_type": { "type": "string" },
                        "event_status": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "review_status": { "type": "string" },
                        "clinical_outcomes": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "MedicalReviewQueueItem": {
                    "type": "object",
                    "required": ["claim_id", "run_id", "audit_id", "medical_reasonableness_score", "review_route", "evidence_status", "missing_evidence", "item_finding_count", "evidence_refs", "canonical_source_refs", "canonical_evidence_refs", "review_status"],
                    "properties": {
                        "claim_id": { "type": "string" },
                        "run_id": { "type": "string" },
                        "audit_id": { "type": "string" },
                        "medical_reasonableness_score": { "type": "integer", "minimum": 0, "maximum": 100 },
                        "review_route": { "type": "string" },
                        "evidence_status": { "type": "string" },
                        "missing_evidence": { "type": "array", "items": { "type": "string" } },
                        "item_finding_count": { "type": "integer" },
                        "first_item_code": { "type": ["string", "null"] },
                        "first_issue_type": { "type": ["string", "null"] },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "canonical_source_refs": { "type": "array", "items": { "type": "string" } },
                        "canonical_evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "created_at": { "type": ["string", "null"], "format": "date-time" },
                        "review_status": { "type": "string" },
                        "review_audit_id": { "type": ["string", "null"] },
                        "review_decision": { "type": ["string", "null"] },
                        "reviewer": { "type": ["string", "null"] },
                        "reviewed_at": { "type": ["string", "null"], "format": "date-time" }
                    }
                },
                "MedicalReviewQueueResponse": {
                    "type": "object",
                    "required": ["items"],
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/MedicalReviewQueueItem" }
                        }
                    }
                },
                "ClinicalEvidenceAssessment": {
                    "type": "object",
                    "required": [
                        "review_required",
                        "review_route",
                        "evidence_status",
                        "minimum_evidence",
                        "missing_evidence",
                        "item_findings",
                        "evidence_refs"
                    ],
                    "properties": {
                        "review_required": { "type": "boolean" },
                        "review_route": { "type": "string", "enum": ["none", "medical_review"] },
                        "evidence_status": {
                            "type": "string",
                            "enum": [
                                "no_clinical_evidence_required",
                                "sufficient_for_basic_review",
                                "missing_required_evidence"
                            ]
                        },
                        "minimum_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "missing_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "item_findings": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ClinicalEvidenceFinding" }
                        },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ClinicalEvidenceFinding": {
                    "type": "object",
                    "required": [
                        "item_code",
                        "issue_type",
                        "required_evidence",
                        "missing_evidence",
                        "reason",
                        "review_route",
                        "evidence_refs"
                    ],
                    "properties": {
                        "item_code": { "type": "string" },
                        "issue_type": { "type": "string" },
                        "required_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "missing_evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "reason": { "type": "string" },
                        "review_route": { "type": "string" },
                        "evidence_refs": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ScoreBreakdown": {
                    "type": "object",
                    "required": [
                        "peer_deviation_score",
                        "rule_score",
                        "anomaly_score",
                        "ml_score",
                        "medical_reasonableness_score",
                        "provider_network_score",
                        "similar_case_score",
                        "final_score"
                    ],
                    "properties": {
                        "peer_deviation_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "rule_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "anomaly_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "ml_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "medical_reasonableness_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "provider_network_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "similar_case_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        },
                        "final_score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 100
                        }
                    }
                },
                "AlertResponse": {
                    "type": "object",
                    "required": ["alert_code", "severity", "reason", "rule_id", "rule_version", "required_evidence"],
                    "properties": {
                        "alert_code": {
                            "type": "string"
                        },
                        "severity": {
                            "type": "string"
                        },
                        "reason": {
                            "type": "string"
                        },
                        "rule_id": {
                            "type": "string"
                        },
                        "rule_version": {
                            "type": "integer"
                        },
                        "required_evidence": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/RequiredEvidence" }
                        }
                    }
                },
                "RequiredEvidence": {
                    "type": "object",
                    "required": ["evidence_type", "blocking"],
                    "properties": {
                        "evidence_type": { "type": "string", "minLength": 1 },
                        "evidence_request_type": { "type": ["string", "null"] },
                        "blocking": { "type": "boolean", "default": true },
                        "policy_authority_ref": { "type": ["string", "null"] },
                        "exception_check": { "type": ["string", "null"] }
                    }
                },
                "AdjudicationPolicy": {
                    "type": "object",
                    "required": ["customer_approval_ref", "appeal_or_override_route", "effective_date", "rollback_plan_ref", "production_threshold_ref", "routing_impact_ref"],
                    "properties": {
                        "customer_approval_ref": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Customer-approved deterministic rule-list or policy approval reference."
                        },
                        "appeal_or_override_route": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Customer-approved appeal, exception, or reviewer override route."
                        },
                        "effective_date": { "type": "string", "minLength": 1 },
                        "rollback_plan_ref": { "type": "string", "minLength": 1 },
                        "production_threshold_ref": { "type": "string", "minLength": 1 },
                        "routing_impact_ref": { "type": "string", "minLength": 1 }
                    }
                },
                "ErrorResponse": {
                    "type": "object",
                    "required": ["code", "message"],
                    "properties": {
                        "code": {
                            "type": "string"
                        },
                        "message": {
                            "type": "string"
                        }
                    }
                },
                "HealthCheck": {
                    "type": "object",
                    "required": ["name", "status"],
                    "properties": {
                        "name": { "type": "string" },
                        "status": {
                            "type": "string",
                            "enum": ["ok", "configured", "local_dev_key", "local_demo_source", "local_dev_database", "local_dev_model_service", "heuristic_model_scorer", "local_demo_object_storage", "local_demo_customer_scope", "local_demo_retention_policy", "local_demo_backup_restore", "local_demo_pii_masking", "local_demo_key_rotation", "local_demo_network_allowlist", "local_demo_alert_routing", "local_demo_observability_exporter", "local_demo_agent_policy"],
                            "description": "Check status. local_dev_key indicates the API is using the local development key. local_demo_source indicates the API is using the local demo source system. local_dev_database indicates the API is using the local development database URL. local_dev_model_service indicates the API is using the local development model service URL. heuristic_model_scorer indicates the API is using the heuristic fallback scorer. local_demo_object_storage indicates the API is using the local demo object storage URI. local_demo_customer_scope indicates the API is using the local demo customer scope id. local_demo_retention_policy indicates the API is using the local demo retention policy id. local_demo_backup_restore indicates the API is using the local demo backup and restore plan id. local_demo_pii_masking indicates the API is using the local demo PII masking policy id. local_demo_key_rotation indicates the API is using the local demo key rotation policy id. local_demo_network_allowlist indicates the API is using the local demo network allowlist id. local_demo_alert_routing indicates the API is using the local demo alert routing policy id. local_demo_observability_exporter indicates the API is using the local demo observability exporter endpoint. local_demo_agent_policy indicates the API is using the local demo Agent tool policy id. These must be reconfigured before customer pilot or production use."
                        },
                        "runtime_kind": {
                            "type": "string",
                            "enum": ["python_http", "heuristic", "rust_artifact", "rust_serving_manifest"],
                            "description": "Model scorer runtime boundary when the check is model_scorer. Internal service URLs are intentionally not exposed."
                        },
                        "remediation": {
                            "type": "string",
                            "description": "Non-secret remediation hint returned for configuration checks that are not yet customer-pilot ready. Secret values and internal endpoint values are intentionally not exposed."
                        }
                    }
                },
                "PilotReadiness": {
                    "type": "object",
                    "required": ["status", "ready_for_customer_pilot", "required_check_names", "required_check_count", "ready_check_count", "blocking_check_count", "blocking_check_names", "remediation_summary", "ready_checks", "blocking_checks"],
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["ready", "not_ready"],
                            "description": "Aggregate customer pilot readiness derived from non-secret configuration checks."
                        },
                        "ready_for_customer_pilot": {
                            "type": "boolean",
                            "description": "True only when no required pilot configuration checks are blocking customer pilot traffic."
                        },
                        "required_check_names": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Configuration check names that must be configured before customer pilot traffic."
                        },
                        "required_check_count": {
                            "type": "integer",
                            "description": "Total number of required pilot configuration checks."
                        },
                        "ready_check_count": {
                            "type": "integer",
                            "description": "Number of required pilot configuration checks already configured."
                        },
                        "blocking_check_count": {
                            "type": "integer",
                            "description": "Number of required pilot configuration checks still blocking customer pilot readiness."
                        },
                        "blocking_check_names": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Compact list of blocking configuration check names for scripts and dashboards."
                        },
                        "remediation_summary": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Compact non-secret remediation hints for blocking readiness checks."
                        },
                        "ready_checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/HealthCheck" },
                            "description": "Configuration checks that are ready for customer pilot traffic."
                        },
                        "blocking_checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/HealthCheck" },
                            "description": "Configuration checks that must become configured before customer pilot traffic."
                        }
                    }
                },
                "HealthResponse": {
                    "type": "object",
                    "required": ["status", "service", "version", "pilot_readiness", "checks"],
                    "properties": {
                        "status": { "type": "string", "enum": ["ok"] },
                        "service": { "type": "string" },
                        "version": { "type": "string" },
                        "pilot_readiness": { "$ref": "#/components/schemas/PilotReadiness" },
                        "checks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/HealthCheck" }
                        }
                    }
                },
    })
}
